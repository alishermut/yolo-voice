use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

const REPO_ID: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";
const REPO_REVISION: &str = "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";
const DISTIL_WHISPER_MODEL_DIR: &str = "distil-whisper";
const MODEL_FILES: &[(&str, bool)] = &[
    ("encoder-model.onnx", true),        // ~41MB graph, required
    ("encoder-model.onnx.data", true),   // ~2.3GB weights (external data), required
    ("decoder_joint-model.onnx", true),  // ~70MB, required
    ("vocab.txt", true),                 // required
    ("config.json", false),              // optional metadata
    ("preprocessor_config.json", false), // optional metadata
    ("tokenizer.json", false),           // optional (parakeet-rs may use vocab.txt)
];

/// Get the models directory path: AppData/models/parakeet-tdt-v3/
pub fn get_models_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let models_dir = dir.join("models").join("parakeet-tdt-v3");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    Ok(models_dir)
}

pub fn get_distil_whisper_models_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    migrate_legacy_distil_whisper_dir(&dir)?;
    let models_dir = dir.join("models").join(DISTIL_WHISPER_MODEL_DIR);
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    Ok(models_dir)
}

fn migrate_legacy_distil_whisper_dir(app_data_dir: &Path) -> Result<(), String> {
    let models_root = app_data_dir.join("models");
    let legacy_dir = models_root.join("asr-benchmark").join("distil_whisper");
    let target_dir = models_root.join(DISTIL_WHISPER_MODEL_DIR);

    if target_dir.exists() || !legacy_dir.exists() {
        return Ok(());
    }

    if let Some(parent) = target_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::rename(&legacy_dir, &target_dir).map_err(|e| {
        format!(
            "Failed to migrate Distil-Whisper from benchmark storage: {}",
            e
        )
    })?;

    let legacy_root = models_root.join("asr-benchmark");
    if legacy_root.exists() {
        let is_empty = std::fs::read_dir(&legacy_root)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(&legacy_root);
        }
    }

    Ok(())
}

/// Check if the required model files are downloaded.
pub fn is_model_downloaded(models_dir: &Path) -> bool {
    MODEL_FILES
        .iter()
        .filter(|(_, required)| *required)
        .all(|(name, _)| {
            let path = models_dir.join(name);
            path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
        })
}

pub fn is_distil_whisper_model_downloaded(models_dir: &Path) -> bool {
    let required = [
        "model.safetensors",
        "config.json",
        "preprocessor_config.json",
        "tokenizer.json",
    ];

    required.iter().all(|name| {
        let path = models_dir.join(name);
        path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
    })
}

/// Download Parakeet model files from HuggingFace with progress reporting.
pub fn download_model(
    models_dir: &Path,
    app_handle: &AppHandle,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    download_hf_files(
        REPO_ID,
        REPO_REVISION,
        MODEL_FILES,
        models_dir,
        app_handle,
        "model-download-progress",
        cancelled,
    )
}

// ── Shared download helper ───────────────────────────────────────────────────

/// Emit a progress event with speed and ETA information.
fn emit_progress(
    app_handle: &AppHandle,
    progress_event: &str,
    status: &str,
    file: &str,
    file_index: usize,
    file_count: usize,
    downloaded_bytes: u64,
    total_bytes: u64,
    start_time: Instant,
) {
    let percent = if total_bytes > 0 {
        ((downloaded_bytes as f64 / total_bytes as f64) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    let elapsed = start_time.elapsed().as_secs_f64();
    let speed: u64 = if elapsed > 0.5 {
        (downloaded_bytes as f64 / elapsed) as u64
    } else {
        0
    };
    let remaining = total_bytes.saturating_sub(downloaded_bytes);
    let eta: u64 = if speed > 0 { remaining / speed } else { 0 };

    let _ = app_handle.emit(
        progress_event,
        serde_json::json!({
            "status": status,
            "file": file,
            "file_index": file_index,
            "file_count": file_count,
            "percent": percent,
            "downloaded_bytes": downloaded_bytes,
            "total_bytes": total_bytes,
            "speed_bytes_per_sec": speed,
            "eta_seconds": eta,
        }),
    );
}

/// Download files from a HuggingFace repo, emitting progress via the given event name.
/// Uses chunked streaming to keep memory usage low and provide real-time progress.
fn download_hf_files(
    repo_id: &str,
    repo_revision: &str,
    files: &[(&str, bool)],
    dest_dir: &Path,
    app_handle: &AppHandle,
    progress_event: &str,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let files_to_download: Vec<(&str, bool)> = files
        .iter()
        .filter(|(name, _)| {
            let path = dest_dir.join(name);
            !path.exists() || path.metadata().map(|m| m.len() == 0).unwrap_or(true)
        })
        .copied()
        .collect();

    if files_to_download.is_empty() {
        return Ok(());
    }

    // Get total size via HEAD requests
    let mut total_bytes: u64 = 0;
    let mut file_sizes: Vec<(&str, u64)> = Vec::new();
    for (name, _) in &files_to_download {
        let url = format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            repo_id, repo_revision, name
        );
        match client.head(&url).send() {
            Ok(resp) => {
                let size: u64 = resp
                    .headers()
                    .get("content-length")
                    .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
                    .and_then(|v: &str| v.parse::<u64>().ok())
                    .unwrap_or(0);
                file_sizes.push((name, size));
                total_bytes += size;
            }
            Err(_) => {
                file_sizes.push((name, 0));
            }
        }
    }

    let file_count = file_sizes.len();
    let mut downloaded_bytes: u64 = 0;
    let start_time = Instant::now();

    for (i, (name, _expected_size)) in file_sizes.iter().enumerate() {
        let file_index = i + 1;
        let url = format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            repo_id, repo_revision, name
        );
        let dest = dest_dir.join(name);

        // Ensure subdirectories exist (e.g. "onnx/")
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let temp_dest = dest_dir.join(format!("{}.tmp", name.replace('/', "_")));

        emit_progress(
            app_handle,
            progress_event,
            "downloading",
            name,
            file_index,
            file_count,
            downloaded_bytes,
            total_bytes,
            start_time,
        );

        let mut response = client
            .get(&url)
            .send()
            .map_err(|e| format!("Download failed for {}: {}", name, e))?;

        if !response.status().is_success() {
            let is_required = files_to_download
                .iter()
                .find(|(n, _)| *n == *name)
                .map(|(_, r)| *r)
                .unwrap_or(false);
            if is_required {
                return Err(format!(
                    "Failed to download {}: HTTP {}",
                    name,
                    response.status()
                ));
            }
            continue;
        }

        // Stream the response in 64KB chunks to keep memory usage low
        use std::io::Write;
        let mut file = std::fs::File::create(&temp_dest)
            .map_err(|e| format!("Failed to create {}: {}", name, e))?;

        let mut buf = [0u8; 65_536];
        let mut last_emit = Instant::now();

        loop {
            // Check for cancellation
            if cancelled.load(Ordering::Relaxed) {
                drop(file);
                let _ = std::fs::remove_file(&temp_dest);
                return Err("Download cancelled".to_string());
            }

            let n = response
                .read(&mut buf)
                .map_err(|e| format!("Failed to read {}: {}", name, e))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])
                .map_err(|e| format!("Failed to write {}: {}", name, e))?;
            downloaded_bytes += n as u64;

            // Throttle progress events to every 250ms
            if last_emit.elapsed() >= std::time::Duration::from_millis(250) {
                emit_progress(
                    app_handle,
                    progress_event,
                    "downloading",
                    name,
                    file_index,
                    file_count,
                    downloaded_bytes,
                    total_bytes,
                    start_time,
                );
                last_emit = Instant::now();
            }
        }

        // Finalize: rename temp file to final destination
        std::fs::rename(&temp_dest, &dest)
            .map_err(|e| format!("Failed to finalize {}: {}", name, e))?;

        // Emit progress after each file completes
        emit_progress(
            app_handle,
            progress_event,
            "downloading",
            name,
            file_index,
            file_count,
            downloaded_bytes,
            total_bytes,
            start_time,
        );
    }

    let _ = app_handle.emit(
        progress_event,
        serde_json::json!({
            "status": "complete",
            "percent": 100.0,
            "downloaded_bytes": total_bytes,
            "total_bytes": total_bytes,
        }),
    );

    Ok(())
}

/// Delete all downloaded model files to free disk space.
pub fn delete_model_files(models_dir: &Path) -> Result<(), String> {
    if models_dir.exists() {
        std::fs::remove_dir_all(models_dir)
            .map_err(|e| format!("Failed to delete model files: {}", e))?;
    }
    Ok(())
}

pub fn delete_distil_whisper_model_files(models_dir: &Path) -> Result<(), String> {
    if models_dir.exists() {
        std::fs::remove_dir_all(models_dir)
            .map_err(|e| format!("Failed to delete Distil-Whisper model files: {}", e))?;
    }
    Ok(())
}

/// Clean up old whisper models from previous versions.
pub fn cleanup_old_models(app_handle: &AppHandle) -> Result<(), String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let models_dir = dir.join("models");

    let _ = migrate_legacy_distil_whisper_dir(&dir);

    if !models_dir.exists() {
        return Ok(());
    }

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir() {
                match name.as_str() {
                    "faster-whisper" | "whisper-tiny" | "whisper-base" | "whisper-small" => {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                    "asr-benchmark" => {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
