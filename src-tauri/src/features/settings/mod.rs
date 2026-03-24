use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
};

// ---- Config ----

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
    /// DEPRECATED: Legacy field from Python sidecar era. Kept for config.json backward compatibility.
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    /// DEPRECATED: Legacy field from Python sidecar era. Kept for config.json backward compatibility.
    #[serde(default = "default_device")]
    pub device: String,
    /// DEPRECATED: Legacy field from Python sidecar era. Kept for config.json backward compatibility.
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
    #[serde(default = "default_start_sound")]
    pub start_sound: String,
    #[serde(default = "default_stop_sound")]
    pub stop_sound: String,
    #[serde(default = "default_vad_silence_threshold")]
    pub vad_silence_threshold_ms: u32,
    #[serde(default = "default_text_cleanup")]
    pub text_cleanup_enabled: bool,
    #[serde(default)]
    pub show_dictionary_migration_notice: bool,
    #[serde(default)]
    pub transcript_diagnostics_enabled: bool,
}

fn default_text_cleanup() -> bool {
    true
}

fn default_vad_silence_threshold() -> u32 {
    500
}

fn default_industry_pack() -> String {
    "general".to_string()
}
fn default_whisper_model() -> String {
    "tiny".to_string()
}
fn default_device() -> String {
    "auto".to_string()
}
fn default_compute_type() -> String {
    "float16".to_string()
}
fn default_language() -> String {
    "en".to_string()
}
fn default_active_profile() -> String {
    "general".to_string()
}
fn default_llm_provider() -> String {
    "ollama".to_string()
}
fn default_llm_model() -> String {
    "llama3.1:8b".to_string()
}
fn default_llm_base_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_transcription_mode() -> String {
    "offline".to_string()
}
fn default_cloud_stt_provider() -> String {
    "groq".to_string()
}
fn default_start_sound() -> String {
    "chime".to_string()
}
fn default_stop_sound() -> String {
    "ding".to_string()
}

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
            start_sound: default_start_sound(),
            stop_sound: default_stop_sound(),
            vad_silence_threshold_ms: default_vad_silence_threshold(),
            text_cleanup_enabled: default_text_cleanup(),
            show_dictionary_migration_notice: false,
            transcript_diagnostics_enabled: false,
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

// ---- Startup (Windows Registry) ----

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "YOLOVoice";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn set_launch_on_startup(enable: bool) -> Result<(), String> {
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();

    unsafe {
        let mut key = HKEY::default();
        let subkey = to_wide(RUN_KEY);

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        );

        if result.is_err() {
            return Err(format!("Failed to open registry key: {:?}", result));
        }

        let value_name = to_wide(VALUE_NAME);

        if enable {
            let value_data = to_wide(&format!("\"{}\"", exe_str));
            let data_bytes = std::slice::from_raw_parts(
                value_data.as_ptr() as *const u8,
                value_data.len() * 2,
            );

            let result = RegSetValueExW(
                key,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(data_bytes),
            );

            let _ = RegCloseKey(key);
            if result.is_err() {
                return Err(format!("Failed to set registry value: {:?}", result));
            }
        } else {
            let result = RegDeleteValueW(key, PCWSTR(value_name.as_ptr()));
            let _ = RegCloseKey(key);
            // Ignore error — key may not exist
            let _ = result;
        }
    }

    Ok(())
}

pub fn is_launch_on_startup() -> bool {
    unsafe {
        let mut key = HKEY::default();
        let subkey = to_wide(RUN_KEY);

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut key,
        );

        if result.is_err() {
            return false;
        }

        let value_name = to_wide(VALUE_NAME);
        let mut data_size: u32 = 0;

        let result = RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            None,
            Some(&mut data_size),
        );

        let _ = RegCloseKey(key);
        result.is_ok() && data_size > 0
    }
}
