use base64::Engine;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Emitter;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::infra::model;

const DISTIL_WHISPER_REPO: &str = "distil-whisper/distil-large-v3";
pub const DISTIL_WHISPER_URL: &str = "https://huggingface.co/distil-whisper/distil-large-v3";
const DISTIL_WHISPER_SYSTEM_PYTHON: &str = "python";
static DISTIL_PREPARE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

pub struct DistilWhisperState(pub Mutex<DistilWhisperManager>);

#[derive(Debug, Clone, Serialize)]
pub struct DistilWhisperStatus {
    pub status: String,
    pub downloaded: bool,
    pub ready: bool,
    pub device: Option<String>,
    pub runtime: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DistilWhisperResult {
    pub text: String,
}

#[derive(Default)]
pub struct DistilWhisperManager {
    process: Option<DistilWhisperProcess>,
    last_error: Option<String>,
    preferred_device: DistilWhisperDevicePreference,
}

#[derive(Debug, Clone, Copy, Default)]
enum DistilWhisperDevicePreference {
    #[default]
    Auto,
    Cpu,
    Gpu,
}

impl DistilWhisperDevicePreference {
    fn as_request_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }
}

struct DistilWhisperProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    loaded: bool,
    device: String,
}

struct Launcher {
    program: String,
    prefix_args: Vec<String>,
    display: String,
    script_path: PathBuf,
}

impl Drop for DistilWhisperManager {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl DistilWhisperManager {
    pub fn status(&mut self, app: &AppHandle) -> DistilWhisperStatus {
        self.refresh_process_state();
        let downloaded = distil_whisper_model_downloaded(app);
        let preparing = DISTIL_PREPARE_IN_PROGRESS.load(Ordering::SeqCst);
        let ready = self.process.as_ref().is_some_and(|proc| proc.loaded);
        let device = self
            .process
            .as_ref()
            .and_then(|proc| (ready || preparing).then(|| proc.device.clone()));

        DistilWhisperStatus {
            status: if ready {
                "ready".to_string()
            } else if preparing {
                "preparing".to_string()
            } else if self.last_error.is_some() {
                "error".to_string()
            } else if downloaded {
                "downloaded".to_string()
            } else {
                "not-downloaded".to_string()
            },
            downloaded,
            ready,
            device,
            runtime: "transformers-distil-whisper".to_string(),
            message: self.last_error.clone(),
        }
    }

    pub fn download_model(&mut self, app: &AppHandle) -> Result<DistilWhisperStatus, String> {
        let model_dir = model::get_distil_whisper_models_dir(app)?;
        let proc = self.ensure_process(app)?;
        let response = proc.send_request(json!({
            "cmd": "download_model",
            "model_id": DISTIL_WHISPER_REPO,
            "target_dir": model_dir.display().to_string(),
        }))?;
        ensure_ok_response("download_model", &response)?;
        self.last_error = None;
        maybe_prepare_in_background(app)?;
        Ok(self.status(app))
    }

    pub fn prepare_model(&mut self, app: &AppHandle) -> Result<DistilWhisperStatus, String> {
        self.ensure_model_loaded(app)?;
        Ok(self.status(app))
    }

    pub fn delete_model(&mut self, app: &AppHandle) -> Result<DistilWhisperStatus, String> {
        let model_dir = model::get_distil_whisper_models_dir(app)?;
        model::delete_distil_whisper_model_files(&model_dir)?;
        self.shutdown()?;
        self.last_error = None;
        Ok(self.status(app))
    }

    pub fn transcribe_local_wav_bytes(
        &mut self,
        app: &AppHandle,
        wav_bytes: &[u8],
    ) -> Result<DistilWhisperResult, String> {
        let proc = self.ensure_model_loaded(app)?;
        let response = proc.send_request(json!({
            "cmd": "transcribe_audio",
            "audio_data": base64::engine::general_purpose::STANDARD.encode(wav_bytes),
        }))?;
        ensure_ok_response("transcribe_audio", &response)?;

        Ok(DistilWhisperResult {
            text: response
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        if let Some(mut proc) = self.process.take() {
            let _ = proc.send_request(json!({ "cmd": "shutdown" }));
            let _ = proc.child.kill();
            let _ = proc.child.wait();
        }
        Ok(())
    }

    pub fn set_preferred_device(&mut self, use_gpu: bool) {
        self.preferred_device = if use_gpu {
            DistilWhisperDevicePreference::Gpu
        } else {
            DistilWhisperDevicePreference::Cpu
        };
    }

    fn refresh_process_state(&mut self) {
        let Some(proc) = self.process.as_mut() else {
            return;
        };

        match proc.child.try_wait() {
            Ok(Some(status)) => {
                self.last_error = Some(format!("Distil-Whisper sidecar exited: {}", status));
                self.process = None;
            }
            Ok(None) => {}
            Err(err) => {
                self.last_error = Some(format!("Failed to poll Distil-Whisper sidecar: {}", err));
                self.process = None;
            }
        }
    }

    fn ensure_model_loaded(
        &mut self,
        app: &AppHandle,
    ) -> Result<&mut DistilWhisperProcess, String> {
        if !distil_whisper_model_downloaded(app) {
            return Err("Download Distil-Whisper first.".to_string());
        }

        let model_dir = model::get_distil_whisper_models_dir(app)?;
        let device_preference = self.preferred_device.as_request_value().to_string();
        let needs_load = {
            let proc = self.ensure_process(app)?;
            !proc.loaded
        };
        if needs_load {
            eprintln!(
                "[distil-whisper] Requesting load_model with model_dir={} preference={}",
                model_dir.display(),
                device_preference
            );
            let response = {
                let proc = self.ensure_process(app)?;
                proc.send_request(json!({
                    "cmd": "load_model",
                    "model_source": model_dir.display().to_string(),
                    "device_preference": device_preference,
                }))?
            };
            if let Err(err) = ensure_ok_response("load_model", &response) {
                let message = format!(
                    "Distil-Whisper failed to load from the pinned local snapshot: {}",
                    err
                );
                self.last_error = Some(message.clone());
                return Err(message);
            }
            let proc = self.ensure_process(app)?;
            proc.loaded = true;
            proc.device = response
                .get("device")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            self.last_error = None;
        }
        self.ensure_process(app)
    }

    fn ensure_process(&mut self, app: &AppHandle) -> Result<&mut DistilWhisperProcess, String> {
        self.refresh_process_state();
        if self.process.is_none() {
            let launcher = resolve_launcher(app)?;
            self.process = Some(DistilWhisperProcess::spawn(launcher)?);
        }
        self.process
            .as_mut()
            .ok_or_else(|| "Failed to start Distil-Whisper sidecar".to_string())
    }
}

pub fn maybe_prepare_in_background(app: &AppHandle) -> Result<(), String> {
    if DISTIL_PREPARE_IN_PROGRESS.load(Ordering::SeqCst) {
        return Ok(());
    }

    let state = app.state::<DistilWhisperState>();
    let mut guard = match state.0.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Ok(()),
    };

    let status = guard.status(app);
    if !status.downloaded || status.ready || status.status == "preparing" {
        return Ok(());
    }
    drop(guard);

    if DISTIL_PREPARE_IN_PROGRESS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }

    let handle = app.clone();
    let _ = handle.emit("distil-whisper-status", "preparing");
    std::thread::spawn(move || {
        let final_status = match handle.state::<DistilWhisperState>().0.lock() {
            Ok(mut guard) => match guard.prepare_model(&handle) {
                Ok(status) => status.status,
                Err(err) => {
                    guard.last_error = Some(err);
                    "error".to_string()
                }
            },
            Err(err) => {
                eprintln!("[distil-whisper] prepare lock error: {}", err);
                "error".to_string()
            }
        };
        DISTIL_PREPARE_IN_PROGRESS.store(false, Ordering::SeqCst);
        let _ = handle.emit("distil-whisper-status", final_status);
    });

    Ok(())
}

impl DistilWhisperProcess {
    fn spawn(launcher: Launcher) -> Result<Self, String> {
        let mut command = Command::new(&launcher.program);
        command
            .args(&launcher.prefix_args)
            .arg("-u")
            .arg(&launcher.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("HF_HUB_DISABLE_PROGRESS_BARS", "1");

        #[cfg(windows)]
        command.creation_flags(0x08000000);

        let mut child = command.spawn().map_err(|e| {
            format!(
                "Failed to start Distil-Whisper sidecar with {}: {}",
                launcher.display, e
            )
        })?;

        if let Some(stderr) = child.stderr.take() {
            spawn_stderr_forwarder(stderr);
        }

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to open Distil-Whisper sidecar stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to open Distil-Whisper sidecar stdout".to_string())?;

        let mut proc = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            loaded: false,
            device: "unknown".to_string(),
        };

        let response = proc.send_request(json!({ "cmd": "ping" }))?;
        ensure_ok_response("ping", &response)?;
        proc.device = response
            .get("device")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        Ok(proc)
    }

    fn send_request(&mut self, payload: Value) -> Result<Value, String> {
        let line = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("Failed to send request to Distil-Whisper sidecar: {}", e))?;

        let mut response = String::new();
        let bytes_read = self
            .stdout
            .read_line(&mut response)
            .map_err(|e| format!("Failed to read Distil-Whisper sidecar response: {}", e))?;
        if bytes_read == 0 {
            return Err("Distil-Whisper sidecar closed unexpectedly".to_string());
        }

        serde_json::from_str(response.trim())
            .map_err(|e| format!("Invalid Distil-Whisper sidecar response: {}", e))
    }
}

fn spawn_stderr_forwarder(stderr: ChildStderr) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) if !line.trim().is_empty() => {
                    eprintln!("[distil-whisper] {}", line);
                }
                Ok(_) => {}
                Err(err) => {
                    eprintln!("[distil-whisper] Failed to read sidecar stderr: {}", err);
                    break;
                }
            }
        }
    });
}

fn ensure_ok_response(cmd: &str, response: &Value) -> Result<(), String> {
    if response.get("status").and_then(Value::as_str) == Some("ok") {
        return Ok(());
    }
    Err(response
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(&format!("{} failed", cmd))
        .to_string())
}

fn distil_whisper_model_downloaded(app: &AppHandle) -> bool {
    let Ok(dir) = model::get_distil_whisper_models_dir(app) else {
        return false;
    };
    model::is_distil_whisper_model_downloaded(&dir)
}

fn resolve_launcher(app: &AppHandle) -> Result<Launcher, String> {
    let script_path = resolve_sidecar_script(app)?;
    let python = resolve_python_command(app)?;
    eprintln!(
        "[distil-whisper] Resolved launcher python={} script={}",
        python.2,
        script_path.display()
    );
    Ok(Launcher {
        program: python.0,
        prefix_args: python.1,
        display: python.2,
        script_path,
    })
}

fn resolve_sidecar_script(app: &AppHandle) -> Result<PathBuf, String> {
    for candidate in candidate_roots(app).into_iter().flat_map(|root| {
        [
            root.join("sidecar").join("distil_whisper.py"),
            root.join("distil_whisper.py"),
            root.join("_up_").join("sidecar").join("distil_whisper.py"),
        ]
    }) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(
        "Could not locate sidecar/distil_whisper.py. Rebuild or run from the repo root."
            .to_string(),
    )
}

fn resolve_python_command(app: &AppHandle) -> Result<(String, Vec<String>, String), String> {
    if let Ok(path) = std::env::var("YOLO_VOICE_PYTHON") {
        return Ok((path.clone(), Vec::new(), path));
    }

    for root in candidate_roots(app) {
        let bundled = root.join("sidecar").join("python-env").join("python.exe");
        if bundled.is_file() {
            let display = bundled.display().to_string();
            return Ok((display.clone(), Vec::new(), display));
        }

        let flat = root.join("python-env").join("python.exe");
        if flat.is_file() {
            let display = flat.display().to_string();
            return Ok((display.clone(), Vec::new(), display));
        }

        let updater_bundled = root
            .join("_up_")
            .join("sidecar")
            .join("python-env")
            .join("python.exe");
        if updater_bundled.is_file() {
            let display = updater_bundled.display().to_string();
            return Ok((display.clone(), Vec::new(), display));
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled_script_present = [
            resource_dir.join("sidecar").join("distil_whisper.py"),
            resource_dir.join("distil_whisper.py"),
            resource_dir.join("_up_").join("sidecar").join("distil_whisper.py"),
            resource_dir.join("resources").join("sidecar").join("distil_whisper.py"),
            resource_dir.join("resources").join("distil_whisper.py"),
        ]
        .into_iter()
        .any(|path| path.is_file());

        if bundled_script_present {
            return Err(format!(
                "Bundled Distil-Whisper Python runtime not found near {}. The packaged app is missing sidecar/python-env.",
                resource_dir.display()
            ));
        }
    }

    Ok((
        DISTIL_WHISPER_SYSTEM_PYTHON.to_string(),
        Vec::new(),
        DISTIL_WHISPER_SYSTEM_PYTHON.to_string(),
    ))
}

fn candidate_roots(app: &AppHandle) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push_unique = |path: PathBuf| {
        if seen.insert(path.clone()) {
            roots.push(path);
        }
    };

    if let Ok(cwd) = std::env::current_dir() {
        push_unique(cwd.clone());
        if let Some(parent) = cwd.parent() {
            push_unique(parent.to_path_buf());
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            push_unique(dir.to_path_buf());
            if let Some(parent) = dir.parent() {
                push_unique(parent.to_path_buf());
                if let Some(grandparent) = parent.parent() {
                    push_unique(grandparent.to_path_buf());
                }
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        push_unique(resource_dir.clone());
        push_unique(resource_dir.join("resources"));
    }

    roots
}
