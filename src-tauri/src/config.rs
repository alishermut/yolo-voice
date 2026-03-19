use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkey: String,
    pub record_mode: RecordMode,
    pub device_index: usize,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    #[serde(default = "default_device")]
    pub device: String,
    #[serde(default = "default_compute_type")]
    pub compute_type: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub post_processing_enabled: bool,
    #[serde(default = "default_active_profile")]
    pub active_profile_id: String,
    #[serde(default = "default_llm_provider")]
    pub llm_provider: String,
    #[serde(default = "default_llm_model")]
    pub llm_model: String,
    #[serde(default)]
    pub llm_api_key: String,
    #[serde(default = "default_llm_base_url")]
    pub llm_base_url: String,
    // Phase 6: Cloud STT + startup
    #[serde(default = "default_transcription_mode")]
    pub transcription_mode: String,
    #[serde(default = "default_cloud_stt_provider")]
    pub cloud_stt_provider: String,
    #[serde(default)]
    pub cloud_stt_api_key: String,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub launch_on_startup: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_industry_pack")]
    pub active_industry_pack: String,
}

fn default_industry_pack() -> String { "general".to_string() }

fn default_whisper_model() -> String { "tiny".to_string() }
fn default_device() -> String { "auto".to_string() }
fn default_compute_type() -> String { "float16".to_string() }
fn default_language() -> String { "en".to_string() }
fn default_active_profile() -> String { "general".to_string() }
fn default_llm_provider() -> String { "ollama".to_string() }
fn default_llm_model() -> String { "llama3.1:8b".to_string() }
fn default_llm_base_url() -> String { "http://localhost:11434".to_string() }
fn default_transcription_mode() -> String { "offline".to_string() }
fn default_cloud_stt_provider() -> String { "groq".to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "CapsLock".to_string(),
            record_mode: RecordMode::Hold,
            device_index: 0,
            whisper_model: default_whisper_model(),
            device: default_device(),
            compute_type: default_compute_type(),
            language: default_language(),
            post_processing_enabled: false,
            active_profile_id: default_active_profile(),
            llm_provider: default_llm_provider(),
            llm_model: default_llm_model(),
            llm_api_key: String::new(),
            llm_base_url: default_llm_base_url(),
            transcription_mode: default_transcription_mode(),
            cloud_stt_provider: default_cloud_stt_provider(),
            cloud_stt_api_key: String::new(),
            onboarding_completed: false,
            launch_on_startup: false,
            start_minimized: false,
            active_industry_pack: default_industry_pack(),
        }
    }
}

pub struct ConfigState(pub Mutex<AppConfig>);

fn config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

pub fn load_config(app_handle: &AppHandle) -> AppConfig {
    let path = match config_path(app_handle) {
        Ok(p) => p,
        Err(_) => return AppConfig::default(),
    };

    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(app_handle: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app_handle)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
