use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Manager};

fn bundled_sidecar_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;

    eprintln!("[sidecar] resource_dir = {}", resource_dir.display());

    // Check for nested sidecar dir (Tauri bundles resources here)
    let nested_sidecar = resource_dir.join("sidecar");
    if nested_sidecar.exists() {
        eprintln!("[sidecar] Found nested sidecar dir: {}", nested_sidecar.display());
        return Ok(nested_sidecar);
    }

    // Check for _up_/sidecar (updater extracts here)
    let up_sidecar = resource_dir.join("_up_/sidecar");
    if up_sidecar.exists() {
        eprintln!("[sidecar] Found updater sidecar dir: {}", up_sidecar.display());
        return Ok(up_sidecar);
    }

    // List directory contents for debugging
    if let Ok(entries) = std::fs::read_dir(&resource_dir) {
        eprintln!("[sidecar] Contents of resource_dir:");
        for entry in entries.flatten() {
            eprintln!("[sidecar]   {}", entry.file_name().to_string_lossy());
        }
    }

    Ok(resource_dir)
}

pub struct SidecarProcess {
    child: Child,
    stdin: BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
}

pub struct SidecarState(pub Mutex<Option<SidecarProcess>>);

impl Drop for SidecarProcess {
    fn drop(&mut self) {
        let shutdown_cmd = serde_json::json!({"cmd": "shutdown"});
        if let Ok(line) = serde_json::to_string(&shutdown_cmd) {
            let _ = writeln!(self.stdin, "{}", line);
            let _ = self.stdin.flush();
        }

        std::thread::sleep(Duration::from_millis(500));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl SidecarProcess {
    /// Send a JSON command and read a single JSON response.
    pub fn send_command(&mut self, cmd: Value) -> Result<Value, String> {
        let line = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
        writeln!(self.stdin, "{}", line)
            .map_err(|e| format!("Failed to write to sidecar stdin: {}", e))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Failed to flush sidecar stdin: {}", e))?;

        let mut response_line = String::new();
        self.stdout
            .read_line(&mut response_line)
            .map_err(|e| format!("Failed to read from sidecar stdout: {}", e))?;

        if response_line.is_empty() {
            return Err("Sidecar process exited unexpectedly".to_string());
        }

        let response: Value = serde_json::from_str(response_line.trim())
            .map_err(|e| format!("Invalid JSON from sidecar: {}", e))?;

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
    pub fn send_command_with_progress(
        &mut self,
        cmd: Value,
        mut progress_cb: impl FnMut(Value),
    ) -> Result<Value, String> {
        let line = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
        writeln!(self.stdin, "{}", line)
            .map_err(|e| format!("Failed to write to sidecar stdin: {}", e))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Failed to flush sidecar stdin: {}", e))?;

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

/// Get the sidecar environment directory inside app data.
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
        return Ok(true);
    }
    let sidecar_dir = get_sidecar_dir(app_handle)?;
    let python = sidecar_dir.join("python/python.exe");
    Ok(python.exists())
}

/// Copy the bundled Python environment from the resource directory to AppData.
pub fn ensure_bundled_env_copied(app_handle: &AppHandle) -> Result<(), String> {
    let sidecar_dir = get_sidecar_dir(app_handle)?;
    let python_dir = sidecar_dir.join("python");
    let version_file = sidecar_dir.join(".bundle_version");
    let current_version = env!("CARGO_PKG_VERSION");

    if python_dir.join("python.exe").exists() {
        if let Ok(stored_version) = std::fs::read_to_string(&version_file) {
            if stored_version.trim() == current_version {
                eprintln!(
                    "[sidecar] Bundled env already up-to-date (v{})",
                    current_version
                );
                return Ok(());
            }
            eprintln!(
                "[sidecar] Bundle version mismatch ({} vs {}), re-copying...",
                stored_version.trim(),
                current_version
            );
        }
    }

    let bundled_sidecar_dir = bundled_sidecar_dir(app_handle)?;
    let bundled_python = bundled_sidecar_dir.join("python-env");

    if !bundled_python.join("python.exe").exists() {
        return Err(format!(
            "Bundled Python environment not found at {}. The installer may be corrupted.",
            bundled_python.display()
        ));
    }

    eprintln!(
        "[sidecar] Copying bundled Python env to {}...",
        python_dir.display()
    );

    if python_dir.exists() {
        let _ = std::fs::remove_dir_all(&python_dir);
    }
    std::fs::create_dir_all(&python_dir).map_err(|e| e.to_string())?;

    copy_dir_all(&bundled_python, &python_dir)
        .map_err(|e| format!("Failed to copy Python environment: {}", e))?;

    // Copy bundled tiny model if not already in AppData
    let models_dir = get_models_dir(app_handle)?;
    let bundled_model = bundled_sidecar_dir
        .join("models")
        .join("faster-whisper-tiny");
    let target_model = models_dir.join("faster-whisper-tiny");

    if bundled_model.exists() && !target_model.exists() {
        eprintln!("[sidecar] Copying bundled tiny model...");
        std::fs::create_dir_all(&target_model).map_err(|e| e.to_string())?;
        copy_dir_all(&bundled_model, &target_model)
            .map_err(|e| format!("Failed to copy bundled model: {}", e))?;
    }

    std::fs::write(&version_file, current_version).map_err(|e| e.to_string())?;

    eprintln!(
        "[sidecar] Bundled environment copied successfully (v{})",
        current_version
    );
    Ok(())
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Legacy setup function — kept for backwards compatibility.
pub fn setup_sidecar_python(app_handle: &AppHandle) -> Result<(), String> {
    ensure_bundled_env_copied(app_handle)
}

fn resolve_sidecar_paths(app_handle: &AppHandle) -> Result<(String, PathBuf), String> {
    if cfg!(debug_assertions) {
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
            return Err(format!(
                "Sidecar script not found: {}",
                script_path.display()
            ));
        }

        let venv_python = sidecar_dir.join(".venv/Scripts/python.exe");
        let python312 =
            PathBuf::from(r"C:\Users\Alish\AppData\Local\Programs\Python\Python312\python.exe");
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
        ensure_bundled_env_copied(app_handle)?;

        let bundled_sidecar_dir = bundled_sidecar_dir(app_handle)?;
        let script_path = bundled_sidecar_dir.join("transcribe.py");

        let sidecar_dir = get_sidecar_dir(app_handle)?;
        let python = sidecar_dir
            .join("python/python.exe")
            .to_string_lossy()
            .to_string();

        if !std::path::Path::new(&python).exists() {
            return Err(
                "Python environment not found. Please reinstall the application.".to_string(),
            );
        }

        if !script_path.exists() {
            return Err(format!(
                "Bundled sidecar script not found at {}. The installer may be corrupted.",
                script_path.display()
            ));
        }

        Ok((python, script_path))
    }
}

/// Spawn the Python sidecar process.
pub fn spawn_sidecar(app_handle: &AppHandle) -> Result<SidecarProcess, String> {
    let (python, script_path) = resolve_sidecar_paths(app_handle)?;

    eprintln!("[sidecar] Spawning: {} {}", python, script_path.display());

    let mut child = Command::new(&python)
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to spawn sidecar ({} {}): {}",
                python,
                script_path.display(),
                e
            )
        })?;

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
        return Err(format!(
            "Unexpected first message from sidecar: {}",
            ready_line.trim()
        ));
    }

    eprintln!(
        "[sidecar] Ready. GPU available: {}",
        ready
            .get("gpu_available")
            .and_then(|g| g.as_bool())
            .unwrap_or(false)
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

    let is_keep = |name: &str| -> bool {
        name == format!("faster-whisper-{}", keep_model)
            || name == format!("models--Systran--faster-whisper-{}", keep_model)
    };

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir() && name.contains("whisper") && !is_keep(&name) {
                eprintln!("[sidecar] Removing unused model: {}", name);
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(())
}

/// Ensure the sidecar is running, spawning it if needed.
/// If restarted, reloads the whisper model so transcription continues to work.
pub fn ensure_running(
    app_handle: &AppHandle,
    state: &SidecarState,
    whisper_model: &str,
    device: &str,
    compute_type: &str,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    let needs_restart = match guard.as_mut() {
        Some(s) => !s.is_alive(),
        None => true,
    };

    if needs_restart {
        *guard = None;
        let mut sidecar = spawn_sidecar(app_handle)?;

        // Reload the whisper model so local transcription works after restart
        let models_dir = get_models_dir(app_handle).unwrap_or_default();
        if let Err(e) = crate::features::speech::load_model(
            &mut sidecar,
            whisper_model,
            device,
            compute_type,
            &models_dir.to_string_lossy(),
        ) {
            eprintln!(
                "[sidecar] Warning: failed to reload model after restart: {}",
                e
            );
        }

        *guard = Some(sidecar);
    }

    Ok(())
}
