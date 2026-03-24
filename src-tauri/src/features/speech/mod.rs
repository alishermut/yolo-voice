pub mod accumulator;
pub mod cleanup;
pub mod cloud;
pub mod inference;
pub mod llm;
pub mod profiles;
pub mod vad;
pub mod vocabulary;

use serde::{Deserialize, Serialize};

use self::inference::InferenceState;

// ---- Types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ModelInfo {
    pub name: String,
    pub size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub builtin: bool,
    pub system_prompt: String,
    #[serde(default, alias = "dictionary")]
    pub terminology_hints: Vec<String>,
    #[serde(default = "default_tone")]
    pub tone: String,
}

fn default_tone() -> String {
    "neutral".to_string()
}

// ---- Transcription ----

/// Transcribe raw audio samples via parakeet-rs (offline, in-process).
pub fn transcribe_audio(
    state: &InferenceState,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<String, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let session = guard
        .as_mut()
        .ok_or_else(|| "Inference engine not initialized. Download the model first.".to_string())?;

    session.transcribe(samples, sample_rate, channels)
}

/// Transcribe via cloud API (Groq or Deepgram).
pub fn cloud_transcribe(
    wav_path: &str,
    provider: &str,
    api_key: &str,
    language: &str,
) -> Result<String, String> {
    cloud::cloud_transcribe(wav_path, provider, api_key, language)
}

// ---- GPU Detection ----

/// Check if GPU acceleration is available.
pub fn get_gpu_available(state: &InferenceState) -> bool {
    state
        .0
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|s| s.is_gpu()))
        .unwrap_or(false)
}

// ---- Post-processing & Profiles ----

/// Post-process transcribed text through an LLM.
pub fn post_process_text(
    text: &str,
    profile: &Profile,
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
) -> Result<String, String> {
    llm::post_process_text(text, profile, provider, model, api_key, base_url)
}

/// Get the profiles directory path.
pub fn get_profiles_dir(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    profiles::get_profiles_dir(app_handle)
}

/// List profiles from disk.
pub fn list_profiles(profiles_dir: &std::path::Path) -> Result<Vec<Profile>, String> {
    profiles::list_profiles(profiles_dir)
}

/// Save a profile to disk.
pub fn save_profile(
    profiles_dir: &std::path::Path,
    profile: &Profile,
) -> Result<(), String> {
    profiles::save_profile(profiles_dir, profile)
}

/// Delete a profile from disk.
pub fn delete_profile(profiles_dir: &std::path::Path, id: &str) -> Result<(), String> {
    profiles::delete_profile(profiles_dir, id)
}
