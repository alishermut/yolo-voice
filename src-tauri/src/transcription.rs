use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::sidecar::SidecarProcess;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size_mb: u64,
}

/// Transcribe a WAV file and return the text.
pub fn transcribe_wav(
    sidecar: &mut SidecarProcess,
    wav_path: &str,
    language: &str,
) -> Result<String, String> {
    let response = sidecar.send_command(json!({
        "cmd": "transcribe",
        "wav_path": wav_path,
        "language": language,
    }))?;

    let text = response
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Ok(text)
}

/// Load a Whisper model in the sidecar.
pub fn load_model(
    sidecar: &mut SidecarProcess,
    model: &str,
    device: &str,
    compute_type: &str,
    models_dir: &str,
) -> Result<(), String> {
    sidecar.send_command(json!({
        "cmd": "load_model",
        "model": model,
        "device": device,
        "compute_type": compute_type,
        "models_dir": models_dir,
    }))?;
    Ok(())
}

/// List models that are downloaded on disk.
pub fn list_downloaded_models(
    sidecar: &mut SidecarProcess,
    models_dir: &str,
) -> Result<Vec<ModelInfo>, String> {
    let response = sidecar.send_command(json!({
        "cmd": "list_models",
        "models_dir": models_dir,
    }))?;

    let models = response
        .get("models")
        .and_then(|m| serde_json::from_value::<Vec<ModelInfo>>(m.clone()).ok())
        .unwrap_or_default();

    Ok(models)
}

/// Download a model with progress reporting via Tauri events.
pub fn download_model(
    sidecar: &mut SidecarProcess,
    model: &str,
    models_dir: &str,
    app_handle: &AppHandle,
) -> Result<(), String> {
    sidecar.send_command_with_progress(
        json!({
            "cmd": "download_model",
            "model": model,
            "models_dir": models_dir,
        }),
        |progress| {
            let _ = app_handle.emit("model-download-progress", progress);
        },
    )?;
    Ok(())
}

/// Check if GPU is available via the sidecar.
pub fn get_gpu_available(sidecar: &mut SidecarProcess) -> Result<bool, String> {
    let response = sidecar.send_command(json!({"cmd": "ping"}))?;
    Ok(response
        .get("gpu_available")
        .and_then(|g| g.as_bool())
        .unwrap_or(false))
}

// ---------------------------------------------------------------------------
// Phase 6: Cloud transcription
// ---------------------------------------------------------------------------

/// Transcribe a WAV file via cloud API (Groq or Deepgram).
pub fn cloud_transcribe(
    sidecar: &mut SidecarProcess,
    wav_path: &str,
    provider: &str,
    api_key: &str,
    language: &str,
) -> Result<String, String> {
    let response = sidecar.send_command(json!({
        "cmd": "cloud_transcribe",
        "wav_path": wav_path,
        "provider": provider,
        "api_key": api_key,
        "language": language,
    }))?;

    let text = response
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Ok(text)
}

// ---------------------------------------------------------------------------
// Phase 5: Post-processing & profiles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub builtin: bool,
    pub system_prompt: String,
    #[serde(default)]
    pub dictionary: Vec<String>,
    #[serde(default = "default_tone")]
    pub tone: String,
}

fn default_tone() -> String {
    "neutral".to_string()
}

/// Post-process transcribed text through an LLM.
pub fn post_process_text(
    sidecar: &mut SidecarProcess,
    text: &str,
    profile: &Profile,
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, String> {
    let response = sidecar.send_command(json!({
        "cmd": "post_process",
        "text": text,
        "profile": {
            "system_prompt": profile.system_prompt,
            "dictionary": profile.dictionary,
            "tone": profile.tone,
        },
        "provider": provider,
        "model": model,
        "api_key": api_key,
        "base_url": base_url,
    }))?;

    let result = response
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or(text)
        .to_string();

    Ok(result)
}

/// List profiles from disk.
pub fn list_profiles(
    sidecar: &mut SidecarProcess,
    profiles_dir: &str,
) -> Result<Vec<Profile>, String> {
    let response = sidecar.send_command(json!({
        "cmd": "list_profiles",
        "profiles_dir": profiles_dir,
    }))?;

    let profiles = response
        .get("profiles")
        .and_then(|p| serde_json::from_value::<Vec<Profile>>(p.clone()).ok())
        .unwrap_or_default();

    Ok(profiles)
}

/// Save a profile to disk.
pub fn save_profile(
    sidecar: &mut SidecarProcess,
    profiles_dir: &str,
    profile: &Profile,
) -> Result<(), String> {
    sidecar.send_command(json!({
        "cmd": "save_profile",
        "profiles_dir": profiles_dir,
        "profile": profile,
    }))?;
    Ok(())
}

/// Delete a profile from disk.
pub fn delete_profile(
    sidecar: &mut SidecarProcess,
    profiles_dir: &str,
    id: &str,
) -> Result<(), String> {
    sidecar.send_command(json!({
        "cmd": "delete_profile",
        "profiles_dir": profiles_dir,
        "id": id,
    }))?;
    Ok(())
}

/// Get the profiles directory path.
pub fn get_profiles_dir(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    let profiles_dir = dir.join("profiles");
    std::fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    Ok(profiles_dir)
}
