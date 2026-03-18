use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Manager};

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

/// Resolve the Python executable and script path for the sidecar.
fn resolve_sidecar_paths(app_handle: &AppHandle) -> Result<(String, PathBuf), String> {
    if cfg!(debug_assertions) {
        // Dev mode: resolve relative to project root
        // Tauri dev CWD is typically the project root
        let project_root = std::env::current_dir().map_err(|e| e.to_string())?;

        // Check if we're in src-tauri/ or project root
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
        // Production: sidecar bundled as resource
        let resource_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e| e.to_string())?;
        let script_path = resource_dir.join("sidecar/transcribe.py");
        let python = resource_dir
            .join("sidecar/python/python.exe")
            .to_string_lossy()
            .to_string();

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

/// Ensure the sidecar is running, spawning it if needed.
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
        let sidecar = spawn_sidecar(app_handle)?;
        *guard = Some(sidecar);
    }

    Ok(())
}
