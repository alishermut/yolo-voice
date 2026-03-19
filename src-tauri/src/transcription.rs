use std::sync::Mutex;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::sidecar::SidecarProcess;

// ---------------------------------------------------------------------------
// Phase 7: Global Dictionary & Replacement Rules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementRule {
    pub find: String,
    pub replace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalDictionary {
    #[serde(default)]
    pub vocabulary: Vec<String>,
    #[serde(default)]
    pub replacements: Vec<ReplacementRule>,
}

pub struct GlobalDictionaryState(pub Mutex<GlobalDictionary>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryPack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub vocabulary: Vec<String>,
    #[serde(default)]
    pub replacements: Vec<ReplacementRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryPackInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vocabulary_count: usize,
    pub replacement_count: usize,
}

/// Apply replacement rules to text (case-insensitive, word-boundary-aware).
pub fn apply_replacements(text: &str, rules: &[ReplacementRule]) -> String {
    let mut result = text.to_string();
    for rule in rules {
        if rule.find.is_empty() {
            continue;
        }
        // For multi-word patterns, use \b only at outer edges
        let escaped = regex::escape(&rule.find);
        let pattern = format!(r"(?i)\b{}\b", escaped);
        if let Ok(re) = Regex::new(&pattern) {
            result = re.replace_all(&result, rule.replace.as_str()).to_string();
        }
    }
    result
}

fn dictionary_path(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    Ok(dir.join("global_dictionary.json"))
}

pub fn load_global_dictionary(app_handle: &AppHandle) -> GlobalDictionary {
    let path = match dictionary_path(app_handle) {
        Ok(p) => p,
        Err(_) => return GlobalDictionary::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => GlobalDictionary::default(),
    }
}

pub fn save_global_dictionary(
    app_handle: &AppHandle,
    dict: &GlobalDictionary,
) -> Result<(), String> {
    let path = dictionary_path(app_handle)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(dict).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// List available industry packs from the sidecar/industry_packs directory.
pub fn list_industry_packs() -> Result<Vec<IndustryPackInfo>, String> {
    let packs_dir = get_industry_packs_dir()?;
    let mut packs = Vec::new();

    if !packs_dir.exists() {
        return Ok(packs);
    }

    for entry in std::fs::read_dir(&packs_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(pack) = serde_json::from_str::<IndustryPack>(&contents) {
                packs.push(IndustryPackInfo {
                    id: pack.id,
                    name: pack.name,
                    description: pack.description,
                    vocabulary_count: pack.vocabulary.len(),
                    replacement_count: pack.replacements.len(),
                });
            }
        }
    }

    packs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packs)
}

/// Load a specific industry pack by id.
pub fn load_industry_pack(id: &str) -> Result<IndustryPack, String> {
    let packs_dir = get_industry_packs_dir()?;
    let path = packs_dir.join(format!("{}.json", id));
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Pack '{}' not found: {}", id, e))?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

fn get_industry_packs_dir() -> Result<std::path::PathBuf, String> {
    // Dev mode: relative to project root
    if cfg!(debug_assertions) {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let dir = if cwd.join("sidecar/industry_packs").exists() {
            cwd.join("sidecar/industry_packs")
        } else if cwd.join("../sidecar/industry_packs").exists() {
            cwd.join("../sidecar/industry_packs")
        } else {
            return Err("Industry packs directory not found".to_string());
        };
        Ok(dir)
    } else {
        // Production: bundled as resource
        Err("Industry packs not available in production yet".to_string())
    }
}

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
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    let mut cmd = json!({
        "cmd": "transcribe",
        "wav_path": wav_path,
        "language": language,
    });
    if let Some(prompt) = initial_prompt {
        cmd["initial_prompt"] = serde_json::Value::String(prompt.to_string());
    }
    let response = sidecar.send_command(cmd)?;

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
