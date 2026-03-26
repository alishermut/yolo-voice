use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
};

// ---- Config ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkey: String,
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
    #[serde(default = "default_sounds_enabled")]
    pub sounds_enabled: bool,
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

    // Command mode
    #[serde(default = "default_command_hotkey")]
    pub command_hotkey: String,
    #[serde(default = "default_command_provider")]
    pub command_provider: String,
    #[serde(default = "default_command_model")]
    pub command_model: String,
    #[serde(default)]
    pub command_api_key: String,
    #[serde(default = "default_command_base_url")]
    pub command_base_url: String,
    #[serde(default = "default_command_system_prompt")]
    pub command_system_prompt: String,

    // Vision (command mode only)
    #[serde(default)]
    pub cloud_vision_enabled: bool,
    #[serde(default = "default_vision_provider")]
    pub cloud_vision_provider: String,
    #[serde(default)]
    pub cloud_vision_model: String,
    #[serde(default)]
    pub cloud_vision_api_key: String,
    #[serde(default = "default_vision_scope")]
    pub vision_capture_scope: String,
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
fn default_command_hotkey() -> String {
    "ControlLeft+Alt+Space".to_string()
}
fn default_command_provider() -> String {
    "groq".to_string()
}
fn default_command_model() -> String {
    "openai/gpt-oss-120b".to_string()
}
fn default_command_base_url() -> String {
    "https://api.groq.com/openai".to_string()
}
fn default_command_system_prompt() -> String {
    "You are a voice command assistant. The user speaks a command and you produce \
     the exact text they want inserted. Do not explain, do not add commentary. \
     Output only the requested text."
        .to_string()
}
fn default_vision_provider() -> String {
    "openai".to_string()
}
fn default_vision_scope() -> String {
    "focused_window".to_string()
}
fn default_sounds_enabled() -> bool {
    true
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
            sounds_enabled: default_sounds_enabled(),
            start_sound: default_start_sound(),
            stop_sound: default_stop_sound(),
            vad_silence_threshold_ms: default_vad_silence_threshold(),
            text_cleanup_enabled: default_text_cleanup(),
            show_dictionary_migration_notice: false,
            transcript_diagnostics_enabled: false,
            command_hotkey: default_command_hotkey(),
            command_provider: default_command_provider(),
            command_model: default_command_model(),
            command_api_key: String::new(),
            command_base_url: default_command_base_url(),
            command_system_prompt: default_command_system_prompt(),
            cloud_vision_enabled: false,
            cloud_vision_provider: default_vision_provider(),
            cloud_vision_model: String::new(),
            cloud_vision_api_key: String::new(),
            vision_capture_scope: default_vision_scope(),
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
    // Ensure the write is flushed to disk (prevents data loss on crash/close)
    if let Ok(file) = fs::File::open(&path) {
        let _ = file.sync_all();
    }
    Ok(())
}

// ---- Startup ----

#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "YOLOVoice";

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
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

#[cfg(windows)]
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

#[cfg(not(windows))]
const LAUNCH_AGENT_LABEL: &str = "com.alish.yolo-voice";

#[cfg(not(windows))]
fn launch_agent_path() -> Option<std::path::PathBuf> {
    dirs_next::home_dir().map(|h| h.join("Library/LaunchAgents").join(format!("{}.plist", LAUNCH_AGENT_LABEL)))
}

#[cfg(not(windows))]
pub fn set_launch_on_startup(enable: bool) -> Result<(), String> {
    let plist_path = launch_agent_path()
        .ok_or_else(|| "Could not determine home directory".to_string())?;

    if enable {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get exe path: {}", e))?;
        let exe_str = exe_path.to_string_lossy();

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>"#,
            LAUNCH_AGENT_LABEL, exe_str
        );

        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;
        }

        fs::write(&plist_path, plist_content)
            .map_err(|e| format!("Failed to write LaunchAgent plist: {}", e))?;
    } else {
        // Remove the plist; ignore error if it doesn't exist
        let _ = fs::remove_file(&plist_path);
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn is_launch_on_startup() -> bool {
    launch_agent_path().map_or(false, |p| p.exists())
}
