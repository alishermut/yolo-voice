use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use keyring::Entry;
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
    #[serde(default = "default_offline_engine")]
    pub offline_engine: String,
    #[serde(default = "default_parakeet_segmented_mode_enabled")]
    pub parakeet_segmented_mode_enabled: bool,
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
    #[serde(default)]
    pub has_llm_api_key: bool,
    #[serde(default = "default_llm_base_url")]
    pub llm_base_url: String,
    #[serde(default = "default_transcription_mode")]
    pub transcription_mode: String,
    #[serde(default = "default_cloud_stt_provider")]
    pub cloud_stt_provider: String,
    #[serde(default)]
    pub cloud_stt_api_key: String,
    #[serde(default)]
    pub has_cloud_stt_api_key: bool,
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
    #[serde(default = "default_offline_accuracy_boost")]
    pub offline_accuracy_boost_enabled: bool,
    #[serde(default)]
    pub numerals_enabled: bool,
    #[serde(default = "default_ui_language")]
    pub ui_language: String,
    #[serde(default)]
    pub pill_pinned: bool,
    #[serde(default)]
    pub show_dictionary_migration_notice: bool,
    #[serde(default)]
    pub transcript_diagnostics_enabled: bool,
    #[serde(default = "default_history_mode_legacy")]
    pub history_mode: String,
    #[serde(default)]
    pub history_retention_days: u32,
    #[serde(default = "default_hallucination_filter")]
    pub hallucination_filter_enabled: bool,
    #[serde(default)]
    pub spoken_punctuation_enabled: bool,
    #[serde(default)]
    pub continuous_recording_enabled: bool,
    #[serde(default)]
    pub auto_pause_media_enabled: bool,

    // Command mode
    #[serde(default = "default_command_hotkey")]
    pub command_hotkey: String,
    #[serde(default = "default_command_provider")]
    pub command_provider: String,
    #[serde(default = "default_command_model")]
    pub command_model: String,
    #[serde(default)]
    pub command_api_key: String,
    #[serde(default)]
    pub has_command_api_key: bool,
    #[serde(default = "default_command_base_url")]
    pub command_base_url: String,
    #[serde(default = "default_command_system_prompt")]
    pub command_system_prompt: String,
}

fn default_text_cleanup() -> bool {
    true
}

fn default_offline_accuracy_boost() -> bool {
    false
}

fn default_vad_silence_threshold() -> u32 {
    700
}

fn default_ui_language() -> String {
    "en".to_string()
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
fn default_offline_engine() -> String {
    "parakeet".to_string()
}
fn default_parakeet_segmented_mode_enabled() -> bool {
    true
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
    "ControlLeft+AltLeft+Space".to_string()
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
fn default_hallucination_filter() -> bool {
    true
}
fn default_history_mode() -> String {
    "final_text".to_string()
}
fn default_history_mode_legacy() -> String {
    String::new()
}
fn default_history_retention_days() -> u32 {
    30
}
fn default_sounds_enabled() -> bool {
    true
}
fn default_start_sound() -> String {
    "click_soft".to_string()
}
fn default_stop_sound() -> String {
    "success_chime".to_string()
}

const SECRET_SERVICE: &str = "yolo-voice";

#[derive(Clone, Copy)]
enum SecretSlot {
    LlmApiKey,
    CloudSttApiKey,
    CommandApiKey,
}

impl SecretSlot {
    fn user(self) -> &'static str {
        match self {
            Self::LlmApiKey => "llm_api_key",
            Self::CloudSttApiKey => "cloud_stt_api_key",
            Self::CommandApiKey => "command_api_key",
        }
    }
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
            offline_engine: default_offline_engine(),
            parakeet_segmented_mode_enabled: default_parakeet_segmented_mode_enabled(),
            post_processing_enabled: false,
            active_profile_id: default_active_profile(),
            llm_provider: default_llm_provider(),
            llm_model: default_llm_model(),
            llm_api_key: String::new(),
            has_llm_api_key: false,
            llm_base_url: default_llm_base_url(),
            transcription_mode: default_transcription_mode(),
            cloud_stt_provider: default_cloud_stt_provider(),
            cloud_stt_api_key: String::new(),
            has_cloud_stt_api_key: false,
            onboarding_completed: false,
            launch_on_startup: false,
            start_minimized: false,
            active_industry_pack: default_industry_pack(),
            sounds_enabled: default_sounds_enabled(),
            start_sound: default_start_sound(),
            stop_sound: default_stop_sound(),
            vad_silence_threshold_ms: default_vad_silence_threshold(),
            text_cleanup_enabled: default_text_cleanup(),
            offline_accuracy_boost_enabled: default_offline_accuracy_boost(),
            numerals_enabled: false,
            ui_language: default_ui_language(),
            pill_pinned: false,
            show_dictionary_migration_notice: false,
            transcript_diagnostics_enabled: false,
            history_mode: default_history_mode(),
            history_retention_days: default_history_retention_days(),
            hallucination_filter_enabled: default_hallucination_filter(),
            spoken_punctuation_enabled: false,
            continuous_recording_enabled: false,
            auto_pause_media_enabled: false,
            command_hotkey: default_command_hotkey(),
            command_provider: default_command_provider(),
            command_model: default_command_model(),
            command_api_key: String::new(),
            has_command_api_key: false,
            command_base_url: default_command_base_url(),
            command_system_prompt: default_command_system_prompt(),
        }
    }
}

fn secret_entry(slot: SecretSlot) -> Result<Entry, String> {
    Entry::new(SECRET_SERVICE, slot.user()).map_err(|e| e.to_string())
}

fn read_secret(slot: SecretSlot) -> Result<Option<String>, String> {
    match secret_entry(slot)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn write_secret(slot: SecretSlot, secret: &str) -> Result<(), String> {
    let entry = secret_entry(slot)?;
    entry.set_password(secret).map_err(|e| e.to_string())?;
    let stored = entry.get_password().map_err(|e| e.to_string())?;
    if stored != secret {
        return Err(format!(
            "Stored keychain secret did not match for {}",
            slot.user()
        ));
    }
    Ok(())
}

fn clear_secret_slot(slot: SecretSlot) -> Result<(), String> {
    match secret_entry(slot)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn sync_secret_slot(
    current_secret: &mut String,
    has_secret: &mut bool,
    slot: SecretSlot,
) -> Result<bool, String> {
    if !current_secret.trim().is_empty() {
        write_secret(slot, current_secret.trim())?;
        *current_secret = current_secret.trim().to_string();
        *has_secret = true;
        return Ok(true);
    }

    if let Some(stored) = read_secret(slot)? {
        *current_secret = stored;
        *has_secret = true;
        return Ok(false);
    }

    *current_secret = String::new();
    *has_secret = false;
    Ok(false)
}

fn sync_secrets(config: &mut AppConfig) -> Result<bool, String> {
    let llm_migrated = sync_secret_slot(
        &mut config.llm_api_key,
        &mut config.has_llm_api_key,
        SecretSlot::LlmApiKey,
    )?;
    let cloud_migrated = sync_secret_slot(
        &mut config.cloud_stt_api_key,
        &mut config.has_cloud_stt_api_key,
        SecretSlot::CloudSttApiKey,
    )?;
    let command_migrated = sync_secret_slot(
        &mut config.command_api_key,
        &mut config.has_command_api_key,
        SecretSlot::CommandApiKey,
    )?;

    Ok(llm_migrated || cloud_migrated || command_migrated)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn validate_base_url(field: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let url = reqwest::Url::parse(trimmed).map_err(|err| format!("Invalid {}: {}", field, err))?;
    match url.scheme() {
        "https" => Ok(()),
        "http" if url.host_str().is_some_and(is_loopback_host) => Ok(()),
        "http" => Err(format!(
            "{} must use HTTPS unless it points to localhost or loopback.",
            field
        )),
        scheme => Err(format!(
            "{} must use http://localhost or HTTPS. Found unsupported scheme '{}'.",
            field, scheme
        )),
    }
}

fn sanitized_config(mut config: AppConfig) -> AppConfig {
    config.has_llm_api_key = !config.llm_api_key.trim().is_empty();
    config.has_cloud_stt_api_key = !config.cloud_stt_api_key.trim().is_empty();
    config.has_command_api_key = !config.command_api_key.trim().is_empty();
    config.llm_api_key.clear();
    config.cloud_stt_api_key.clear();
    config.command_api_key.clear();
    config
}

pub fn public_config(config: &AppConfig) -> AppConfig {
    sanitized_config(config.clone())
}

pub fn merge_runtime_config(existing: &AppConfig, incoming: &AppConfig) -> AppConfig {
    let mut merged = incoming.clone();
    if merged.llm_api_key.trim().is_empty() {
        merged.llm_api_key = existing.llm_api_key.clone();
    }
    if merged.cloud_stt_api_key.trim().is_empty() {
        merged.cloud_stt_api_key = existing.cloud_stt_api_key.clone();
    }
    if merged.command_api_key.trim().is_empty() {
        merged.command_api_key = existing.command_api_key.clone();
    }
    merged.has_llm_api_key = !merged.llm_api_key.trim().is_empty();
    merged.has_cloud_stt_api_key = !merged.cloud_stt_api_key.trim().is_empty();
    merged.has_command_api_key = !merged.command_api_key.trim().is_empty();
    merged
}

pub fn clear_secret(
    app_handle: &AppHandle,
    config: &mut AppConfig,
    slot: &str,
) -> Result<(), String> {
    match slot {
        "llm_api_key" => {
            clear_secret_slot(SecretSlot::LlmApiKey)?;
            config.llm_api_key.clear();
            config.has_llm_api_key = false;
        }
        "cloud_stt_api_key" => {
            clear_secret_slot(SecretSlot::CloudSttApiKey)?;
            config.cloud_stt_api_key.clear();
            config.has_cloud_stt_api_key = false;
        }
        "command_api_key" => {
            clear_secret_slot(SecretSlot::CommandApiKey)?;
            config.command_api_key.clear();
            config.has_command_api_key = false;
        }
        _ => return Err("Unknown secret slot".to_string()),
    }

    save_config(app_handle, config)
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn defaults_include_distil_and_parakeet_segmented_settings() {
        let config = AppConfig::default();
        assert_eq!(config.offline_engine, "parakeet");
        assert!(config.parakeet_segmented_mode_enabled);
    }

    #[test]
    fn older_config_without_new_fields_still_deserializes() {
        let json = r#"{
            "hotkey":"CapsLock",
            "device_index":0,
            "language":"en",
            "transcription_mode":"offline"
        }"#;

        let config: AppConfig = serde_json::from_str(json).expect("config should deserialize");
        assert_eq!(config.offline_engine, "parakeet");
        assert!(config.parakeet_segmented_mode_enabled);
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

    let mut config = match fs::read_to_string(&path) {
        Ok(contents) => normalize_config(serde_json::from_str(&contents).unwrap_or_default()),
        Err(_) => AppConfig::default(),
    };

    match sync_secrets(&mut config) {
        Ok(migrated) => {
            if migrated {
                let _ = save_config(app_handle, &config);
            }
        }
        Err(err) => {
            eprintln!("[settings] Failed to sync secrets from keychain: {}", err);
            config.has_llm_api_key = !config.llm_api_key.trim().is_empty();
            config.has_cloud_stt_api_key = !config.cloud_stt_api_key.trim().is_empty();
            config.has_command_api_key = !config.command_api_key.trim().is_empty();
        }
    }

    config
}

fn normalize_config(mut config: AppConfig) -> AppConfig {
    config.offline_engine = match config.offline_engine.as_str() {
        "parakeet" => "parakeet".to_string(),
        "distil_whisper" => "distil_whisper".to_string(),
        "cohere" => "distil_whisper".to_string(),
        _ => default_offline_engine(),
    };
    config.history_mode = match config.history_mode.trim() {
        "off" => "off".to_string(),
        "final_text" => "final_text".to_string(),
        "debug" => "debug".to_string(),
        _ if config.transcript_diagnostics_enabled => "debug".to_string(),
        _ => default_history_mode(),
    };
    if config.history_retention_days == 0 {
        config.history_retention_days = default_history_retention_days();
    }
    config.transcript_diagnostics_enabled = config.history_mode == "debug";
    config.has_llm_api_key = !config.llm_api_key.trim().is_empty();
    config.has_cloud_stt_api_key = !config.cloud_stt_api_key.trim().is_empty();
    config.has_command_api_key = !config.command_api_key.trim().is_empty();
    config
}

pub fn save_config(app_handle: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app_handle)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut runtime_config = config.clone();
    validate_base_url("llm_base_url", &runtime_config.llm_base_url)?;
    validate_base_url("command_base_url", &runtime_config.command_base_url)?;
    sync_secrets(&mut runtime_config)?;
    let sanitized = sanitized_config(runtime_config);
    let json = serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?;
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
    let exe_path = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
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
            let data_bytes =
                std::slice::from_raw_parts(value_data.as_ptr() as *const u8, value_data.len() * 2);

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
    dirs_next::home_dir().map(|h| {
        h.join("Library/LaunchAgents")
            .join(format!("{}.plist", LAUNCH_AGENT_LABEL))
    })
}

#[cfg(not(windows))]
pub fn set_launch_on_startup(enable: bool) -> Result<(), String> {
    let plist_path =
        launch_agent_path().ok_or_else(|| "Could not determine home directory".to_string())?;

    if enable {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
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
