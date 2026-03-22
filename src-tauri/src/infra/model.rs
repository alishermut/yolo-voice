use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

const REPO_ID: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";
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

/// Download Parakeet model files from HuggingFace with progress reporting.
pub fn download_model(models_dir: &Path, app_handle: &AppHandle) -> Result<(), String> {
    download_hf_files(
        REPO_ID,
        MODEL_FILES,
        models_dir,
        app_handle,
        "model-download-progress",
    )
}

// ── Shared download helper ───────────────────────────────────────────────────

/// Download files from a HuggingFace repo, emitting progress via the given event name.
fn download_hf_files(
    repo_id: &str,
    files: &[(&str, bool)],
    dest_dir: &Path,
    app_handle: &AppHandle,
    progress_event: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;

    let client = reqwest::blocking::Client::builder()
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

    // Get total size
    let mut total_bytes: u64 = 0;
    let mut file_sizes: Vec<(&str, u64)> = Vec::new();
    for (name, _) in &files_to_download {
        let url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, name);
        match client.head(&url).send() {
            Ok(resp) => {
                let size = resp
                    .headers()
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                file_sizes.push((name, size));
                total_bytes += size;
            }
            Err(_) => {
                file_sizes.push((name, 0));
            }
        }
    }

    let mut downloaded_bytes: u64 = 0;

    for (name, expected_size) in &file_sizes {
        let url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, name);
        let dest = dest_dir.join(name);

        // Ensure subdirectories exist (e.g. "onnx/")
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let temp_dest = dest_dir.join(format!("{}.tmp", name.replace('/', "_")));

        let _ = app_handle.emit(
            progress_event,
            serde_json::json!({
                "status": "downloading",
                "file": name,
                "percent": if total_bytes > 0 {
                    (downloaded_bytes as f64 / total_bytes as f64 * 100.0).round()
                } else { 0.0 },
                "downloaded_mb": downloaded_bytes / (1024 * 1024),
                "total_mb": total_bytes / (1024 * 1024),
            }),
        );

        let response = client
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
                return Err(format!("Failed to download {}: HTTP {}", name, response.status()));
            }
            continue;
        }

        use std::io::Write;
        let mut file = std::fs::File::create(&temp_dest)
            .map_err(|e| format!("Failed to create {}: {}", name, e))?;

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read response for {}: {}", name, e))?;

        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write {}: {}", name, e))?;

        std::fs::rename(&temp_dest, &dest)
            .map_err(|e| format!("Failed to finalize {}: {}", name, e))?;

        downloaded_bytes += *expected_size;

        let _ = app_handle.emit(
            progress_event,
            serde_json::json!({
                "status": "downloading",
                "file": name,
                "percent": if total_bytes > 0 {
                    (downloaded_bytes as f64 / total_bytes as f64 * 100.0).round()
                } else { 100.0 },
                "downloaded_mb": downloaded_bytes / (1024 * 1024),
                "total_mb": total_bytes / (1024 * 1024),
            }),
        );

    }

    let _ = app_handle.emit(
        progress_event,
        serde_json::json!({
            "status": "complete",
            "percent": 100.0,
            "downloaded_mb": total_bytes / (1024 * 1024),
            "total_mb": total_bytes / (1024 * 1024),
        }),
    );

    Ok(())
}

/// Clean up old whisper models from previous versions.
pub fn cleanup_old_models(app_handle: &AppHandle) -> Result<(), String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let models_dir = dir.join("models");

    if !models_dir.exists() {
        return Ok(());
    }

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Remove old faster-whisper model directories
            if entry.path().is_dir() && name.contains("whisper") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(())
}
