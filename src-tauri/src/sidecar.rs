use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

pub struct SidecarProcess {
    child: Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
}

pub struct SidecarState(pub Mutex<Option<SidecarProcess>>);

impl Drop for SidecarProcess {
    fn drop(&mut self) {
        // Try graceful shutdown first
        let shutdown_cmd = serde_json::json!({"cmd": "shutdown"});
        if let Ok(line) = serde_json::to_string(&shutdown_cmd) {
            let _ = writeln!(self.stdin, "{}", line);
            let _ = self.stdin.flush();
        }

        // Give it a moment then force kill
        std::thread::sleep(Duration::from_millis(500));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl SidecarProcess {
    /// Send a JSON command and read a single JSON response.
    pub fn send_command(&mut self, cmd: Value) -> Result<Value, String> {
        let line = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
        writeln!(self.stdin, "{}", line).map_err(|e| format!("Failed to write to sidecar stdin: {}", e))?;
        self.stdin.flush().map_err(|e| format!("Failed to flush sidecar stdin: {}", e))?;

        let mut response_line = String::new();
        self.stdout
            .read_line(&mut response_line)
            .map_err(|e| format!("Failed to read from sidecar stdout: {}", e))?;

        if response_line.is_empty() {
            return Err("Sidecar process exited unexpectedly".to_string());
        }

        let response: Value =
            serde_json::from_str(response_line.trim()).map_err(|e| format!("Invalid JSON from sidecar: {}", e))?;

        if response.get("status").and_then(|s| s.as_str()) == Some("error") {
            let msg = response
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown sidecar error");
            return Err(msg.to_string());
        }

        Ok(response)
    }

    /// Send a command that may produce progress lines before the final response.
    /// Calls `progress_cb` for each progress line.
    pub fn send_command_with_progress(
        &mut self,
        cmd: Value,
        mut progress_cb: impl FnMut(Value),
    ) -> Result<Value, String> {
        let line = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
        writeln!(self.stdin, "{}", line).map_err(|e| format!("Failed to write to sidecar stdin: {}", e))?;
        self.stdin.flush().map_err(|e| format!("Failed to flush sidecar stdin: {}", e))?;

        loop {
            let mut response_line = String::new();
            self.stdout
                .read_line(&mut response_line)
                .map_err(|e| format!("Failed to read from sidecar stdout: {}", e))?;

            if response_line.is_empty() {
                return Err("Sidecar process exited unexpectedly".to_string());
            }

            let response: Value = serde_json::from_str(response_line.trim())
                .map_err(|e| format!("Invalid JSON from sidecar: {}", e))?;

            match response.get("status").and_then(|s| s.as_str()) {
                Some("progress") => {
                    progress_cb(response);
                }
                Some("error") => {
                    let msg = response
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown sidecar error");
                    return Err(msg.to_string());
                }
                _ => {
                    return Ok(response);
                }
            }
        }
    }

    /// Check if the child process is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

/// Get the sidecar environment directory inside app data (for Python runtime).
pub fn get_sidecar_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let sidecar_dir = dir.join("sidecar");
    std::fs::create_dir_all(&sidecar_dir).map_err(|e| e.to_string())?;
    Ok(sidecar_dir)
}

/// Check whether the Python sidecar environment is set up.
pub fn is_sidecar_setup(app_handle: &AppHandle) -> Result<bool, String> {
    if cfg!(debug_assertions) {
        return Ok(true); // Dev mode always has Python available
    }
    let sidecar_dir = get_sidecar_dir(app_handle)?;
    let python = sidecar_dir.join("python/python.exe");
    Ok(python.exists())
}

/// Download and set up the embedded Python environment for production.
pub fn setup_sidecar_python(app_handle: &AppHandle) -> Result<(), String> {
    let sidecar_dir = get_sidecar_dir(app_handle)?;
    let python_dir = sidecar_dir.join("python");

    if python_dir.join("python.exe").exists() {
        eprintln!("[sidecar] Python already set up at {}", python_dir.display());
        return Ok(());
    }

    std::fs::create_dir_all(&python_dir).map_err(|e| e.to_string())?;

    let _ = app_handle.emit("sidecar-setup-progress", serde_json::json!({
        "step": "downloading_python",
        "message": "Downloading Python runtime...",
        "percent": 0
    }));

    // Download Python 3.12 embeddable zip
    let python_url = "https://www.python.org/ftp/python/3.12.8/python-3.12.8-embed-amd64.zip";
    let zip_path = sidecar_dir.join("python-embed.zip");

    download_file(python_url, &zip_path).map_err(|e| format!("Failed to download Python: {}", e))?;

    let _ = app_handle.emit("sidecar-setup-progress", serde_json::json!({
        "step": "extracting_python",
        "message": "Extracting Python...",
        "percent": 25
    }));

    // Extract the zip
    extract_zip(&zip_path, &python_dir)?;
    let _ = std::fs::remove_file(&zip_path);

    // Enable pip: uncomment "import site" in python312._pth
    let pth_path = python_dir.join("python312._pth");
    if pth_path.exists() {
        let content = std::fs::read_to_string(&pth_path).map_err(|e| e.to_string())?;
        let new_content = content.replace("#import site", "import site");
        std::fs::write(&pth_path, new_content).map_err(|e| e.to_string())?;
    }

    let _ = app_handle.emit("sidecar-setup-progress", serde_json::json!({
        "step": "installing_pip",
        "message": "Installing pip...",
        "percent": 40
    }));

    // Download and run get-pip.py
    let get_pip_path = sidecar_dir.join("get-pip.py");
    download_file("https://bootstrap.pypa.io/get-pip.py", &get_pip_path)
        .map_err(|e| format!("Failed to download get-pip.py: {}", e))?;

    let python_exe = python_dir.join("python.exe");
    let pip_output = Command::new(&python_exe)
        .arg(&get_pip_path)
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to run get-pip.py: {}", e))?;

    if !pip_output.status.success() {
        let stderr = String::from_utf8_lossy(&pip_output.stderr);
        return Err(format!("get-pip.py failed: {}", stderr));
    }

    let _ = std::fs::remove_file(&get_pip_path);

    let _ = app_handle.emit("sidecar-setup-progress", serde_json::json!({
        "step": "installing_deps",
        "message": "Installing dependencies (this may take a few minutes)...",
        "percent": 60
    }));

    // Find requirements.txt from bundled resources or sidecar dir
    let requirements_path = if cfg!(debug_assertions) {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        if cwd.join("sidecar/requirements.txt").exists() {
            cwd.join("sidecar/requirements.txt")
        } else {
            cwd.join("../sidecar/requirements.txt")
        }
    } else {
        let resource_dir = app_handle.path().resource_dir().map_err(|e| e.to_string())?;
        resource_dir.join("requirements.txt")
    };

    // Install requirements using pip
    let pip_install_output = Command::new(&python_exe)
        .args(["-m", "pip", "install", "-r"])
        .arg(&requirements_path)
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to run pip install: {}", e))?;

    if !pip_install_output.status.success() {
        let stderr = String::from_utf8_lossy(&pip_install_output.stderr);
        return Err(format!("pip install failed: {}", stderr));
    }

    let _ = app_handle.emit("sidecar-setup-progress", serde_json::json!({
        "step": "downloading_model",
        "message": "Downloading speech model (~39 MB)...",
        "percent": 85
    }));

    // Auto-download the default tiny model so it's ready on first use
    let models_dir = get_models_dir(app_handle)?;
    let model_dir = models_dir.join("faster-whisper-tiny");
    let model_dir_hf = models_dir.join("models--Systran--faster-whisper-tiny");
    if !model_dir.exists() && !model_dir_hf.exists() {
        // Spawn sidecar temporarily to download the model
        let mut temp_sidecar = spawn_sidecar(app_handle)?;
        let _ = crate::transcription::download_model(
            &mut temp_sidecar,
            "tiny",
            &models_dir.to_string_lossy(),
            app_handle,
        );
        // Sidecar will be dropped here (graceful shutdown)
    }

    let _ = app_handle.emit("sidecar-setup-progress", serde_json::json!({
        "step": "done",
        "message": "Setup complete!",
        "percent": 100
    }));

    eprintln!("[sidecar] Python environment setup complete at {}", python_dir.display());
    Ok(())
}

/// Download a file from a URL to a local path.
fn download_file(url: &str, dest: &std::path::Path) -> Result<(), String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                url,
                dest.to_string_lossy()
            ),
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to run PowerShell download: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Download failed: {}", stderr));
    }

    Ok(())
}

/// Extract a zip file to a destination directory.
fn extract_zip(zip_path: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let out_path = dest.join(entry.mangled_name());

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Resolve the Python executable and script path for the sidecar.
fn resolve_sidecar_paths(app_handle: &AppHandle) -> Result<(String, PathBuf), String> {
    if cfg!(debug_assertions) {
        // Dev mode: resolve relative to project root
        let project_root = std::env::current_dir().map_err(|e| e.to_string())?;

        let sidecar_dir = if project_root.join("sidecar").exists() {
            project_root.join("sidecar")
        } else if project_root.join("../sidecar").exists() {
            project_root.join("../sidecar")
        } else {
            return Err(format!(
                "Cannot find sidecar directory. CWD: {}",
                project_root.display()
            ));
        };

        let script_path = sidecar_dir.join("transcribe.py");
        if !script_path.exists() {
            return Err(format!("Sidecar script not found: {}", script_path.display()));
        }

        // Look for Python in order of preference:
        // 1. venv in sidecar dir
        // 2. Python 3.12 (has CUDA support for faster-whisper)
        // 3. System python
        let venv_python = sidecar_dir.join(".venv/Scripts/python.exe");
        let python312 = PathBuf::from(r"C:\Users\Alish\AppData\Local\Programs\Python\Python312\python.exe");
        let python = if venv_python.exists() {
            venv_python.to_string_lossy().to_string()
        } else if python312.exists() {
            eprintln!("[sidecar] Using Python 3.12 for CUDA/GPU support");
            python312.to_string_lossy().to_string()
        } else {
            "python".to_string()
        };

        Ok((python, script_path))
    } else {
        // Production: scripts bundled as resources (at root of resource dir), Python in app data
        let resource_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e| e.to_string())?;
        let script_path = resource_dir.join("transcribe.py");

        // Python environment lives in app_data_dir (downloaded during setup)
        let sidecar_dir = get_sidecar_dir(app_handle)?;
        let python = sidecar_dir
            .join("python/python.exe")
            .to_string_lossy()
            .to_string();

        if !std::path::Path::new(&python).exists() {
            return Err("Python environment not set up. Please run Setup in the Settings page.".to_string());
        }

        Ok((python, script_path))
    }
}

/// Spawn the Python sidecar process.
pub fn spawn_sidecar(app_handle: &AppHandle) -> Result<SidecarProcess, String> {
    let (python, script_path) = resolve_sidecar_paths(app_handle)?;

    eprintln!(
        "[sidecar] Spawning: {} {}",
        python,
        script_path.display()
    );

    let mut child = Command::new(&python)
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        // Prevent a console window from flashing on Windows
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("Failed to spawn sidecar ({} {}): {}", python, script_path.display(), e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to capture sidecar stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture sidecar stdout".to_string())?;

    let mut sidecar = SidecarProcess {
        child,
        stdin: BufWriter::new(stdin),
        stdout: BufReader::new(stdout),
    };

    // Wait for the "ready" message
    let mut ready_line = String::new();
    sidecar
        .stdout
        .read_line(&mut ready_line)
        .map_err(|e| format!("Failed to read sidecar ready signal: {}", e))?;

    if ready_line.is_empty() {
        return Err("Sidecar exited before sending ready signal".to_string());
    }

    let ready: Value = serde_json::from_str(ready_line.trim())
        .map_err(|e| format!("Invalid ready JSON from sidecar: {}", e))?;

    if ready.get("cmd").and_then(|c| c.as_str()) != Some("ready") {
        return Err(format!("Unexpected first message from sidecar: {}", ready_line.trim()));
    }

    eprintln!(
        "[sidecar] Ready. GPU available: {}",
        ready.get("gpu_available").and_then(|g| g.as_bool()).unwrap_or(false)
    );

    Ok(sidecar)
}

/// Get the models directory path.
pub fn get_models_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let models_dir = dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    Ok(models_dir)
}

/// Delete all downloaded models except the specified one.
pub fn cleanup_models(app_handle: &AppHandle, keep_model: &str) -> Result<(), String> {
    let models_dir = get_models_dir(app_handle)?;

    // Match both naming formats:
    //   "faster-whisper-tiny" (direct download)
    //   "models--Systran--faster-whisper-tiny" (HuggingFace cache)
    let is_keep = |name: &str| -> bool {
        name == format!("faster-whisper-{}", keep_model)
            || name == format!("models--Systran--faster-whisper-{}", keep_model)
    };

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Only delete model directories (contain "whisper"), skip .locks etc.
            if entry.path().is_dir() && name.contains("whisper") && !is_keep(&name) {
                eprintln!("[sidecar] Removing unused model: {}", name);
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(())
}

/// Ensure the sidecar is running, spawning it if needed.
/// If the sidecar was restarted, also reloads the whisper model so local
/// transcription continues to work.
pub fn ensure_running(
    app_handle: &AppHandle,
    state: &SidecarState,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    let needs_restart = match guard.as_mut() {
        Some(s) => !s.is_alive(),
        None => true,
    };

    if needs_restart {
        *guard = None; // Drop the old one
        let mut sidecar = spawn_sidecar(app_handle)?;

        // Reload the whisper model so local transcription works after restart
        let config = app_handle
            .state::<crate::config::ConfigState>()
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        let models_dir = get_models_dir(app_handle).unwrap_or_default();
        if let Err(e) = crate::transcription::load_model(
            &mut sidecar,
            &config.whisper_model,
            &config.device,
            &config.compute_type,
            &models_dir.to_string_lossy(),
        ) {
            eprintln!("[sidecar] Warning: failed to reload model after restart: {}", e);
        }

        *guard = Some(sidecar);
    }

    Ok(())
}
