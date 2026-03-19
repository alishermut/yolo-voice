use std::sync::Mutex;

use tauri::State;

use crate::audio::{self, AudioStream, DeviceInfo};
use crate::config::{self, AppConfig, ConfigState};
use crate::recorder::{self, RecordingState};
use crate::sidecar::{self, SidecarState};
use crate::startup;
use crate::transcription::{
    self, GlobalDictionary, GlobalDictionaryState, IndustryPackInfo, ModelInfo, Profile,
};

pub struct AudioState(pub Mutex<Option<AudioStream>>);

/// Shared state for the pill UI to poll
pub struct PillUiState {
    pub recording_state: Mutex<String>,
    pub audio_level: Mutex<f32>,
}

impl Default for PillUiState {
    fn default() -> Self {
        Self {
            recording_state: Mutex::new("idle".to_string()),
            audio_level: Mutex::new(0.0),
        }
    }
}

#[tauri::command]
pub fn get_pill_state(state: State<'_, PillUiState>) -> Result<(String, f32), String> {
    let rs = state.recording_state.lock().map_err(|e| e.to_string())?;
    let level = state.audio_level.lock().map_err(|e| e.to_string())?;
    Ok((rs.clone(), *level))
}

#[tauri::command]
pub fn list_devices() -> Vec<DeviceInfo> {
    audio::list_input_devices()
}

#[tauri::command]
pub fn start_test(
    device_index: usize,
    app_handle: tauri::AppHandle,
    state: State<'_, AudioState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Stop any existing stream first
    *guard = None;
    let stream = audio::start_level_monitor(device_index, app_handle)?;
    *guard = Some(stream);
    Ok(())
}

#[tauri::command]
pub fn stop_test(state: State<'_, AudioState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn get_config(state: State<'_, ConfigState>) -> Result<AppConfig, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn save_config_cmd(
    new_config: AppConfig,
    app_handle: tauri::AppHandle,
    state: State<'_, ConfigState>,
) -> Result<(), String> {
    config::save_config(&app_handle, &new_config)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = new_config;
    Ok(())
}

#[tauri::command]
pub fn start_recording(
    device_index: usize,
    app_handle: tauri::AppHandle,
    state: State<'_, RecordingState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Stop any existing recording first
    *guard = None;
    let stream = recorder::start_recording(device_index, app_handle)?;
    *guard = Some(stream);
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, RecordingState>) -> Result<String, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let recording = guard
        .take()
        .ok_or_else(|| "No active recording".to_string())?;
    let path = recorder::stop_and_save(recording)?;
    Ok(path.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Phase 3: Transcription commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_models(
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<Vec<ModelInfo>, String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let models_dir = sidecar::get_models_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    transcription::list_downloaded_models(sc, &models_dir.to_string_lossy())
}

#[tauri::command]
pub fn download_model_cmd(
    model: String,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<(), String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let models_dir = sidecar::get_models_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    transcription::download_model(sc, &model, &models_dir.to_string_lossy(), &app_handle)
}

#[tauri::command]
pub fn set_whisper_model(
    model: String,
    device: String,
    compute_type: String,
    app_handle: tauri::AppHandle,
    sidecar_state: State<'_, SidecarState>,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    sidecar::ensure_running(&app_handle, &sidecar_state)?;
    let models_dir = sidecar::get_models_dir(&app_handle)?;

    // Load model in sidecar
    {
        let mut guard = sidecar_state.0.lock().map_err(|e| e.to_string())?;
        let sc = guard.as_mut().ok_or("Sidecar not running")?;
        transcription::load_model(
            sc,
            &model,
            &device,
            &compute_type,
            &models_dir.to_string_lossy(),
        )?;
    }

    // Persist to config
    let mut config_guard = config_state.0.lock().map_err(|e| e.to_string())?;
    config_guard.whisper_model = model;
    config_guard.device = device;
    config_guard.compute_type = compute_type;
    config::save_config(&app_handle, &config_guard)?;

    Ok(())
}

#[tauri::command]
pub fn get_gpu_available(
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<bool, String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    transcription::get_gpu_available(sc)
}

#[tauri::command]
pub fn get_sidecar_status(
    state: State<'_, SidecarState>,
) -> Result<String, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_mut() {
        Some(s) => {
            if s.is_alive() {
                Ok("running".to_string())
            } else {
                Ok("stopped".to_string())
            }
        }
        None => Ok("stopped".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Phase 5: Profile & post-processing commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_profiles(
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<Vec<Profile>, String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let profiles_dir = transcription::get_profiles_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    transcription::list_profiles(sc, &profiles_dir.to_string_lossy())
}

#[tauri::command]
pub fn save_profile_cmd(
    profile: Profile,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<(), String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let profiles_dir = transcription::get_profiles_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    transcription::save_profile(sc, &profiles_dir.to_string_lossy(), &profile)
}

#[tauri::command]
pub fn delete_profile_cmd(
    id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<(), String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let profiles_dir = transcription::get_profiles_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    transcription::delete_profile(sc, &profiles_dir.to_string_lossy(), &id)
}

#[tauri::command]
pub fn test_llm_connection(
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<String, String> {
    sidecar::ensure_running(&app_handle, &state)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;

    // Use a simple test profile
    let test_profile = Profile {
        id: "_test".to_string(),
        name: "Test".to_string(),
        builtin: false,
        system_prompt: "Fix the grammar. Output only the corrected text.".to_string(),
        dictionary: vec![],
        tone: "neutral".to_string(),
    };

    transcription::post_process_text(
        sc,
        "this is a test to check if the connection works",
        &test_profile,
        &provider,
        &model,
        &api_key,
        &base_url,
    )
}

// ---------------------------------------------------------------------------
// Phase 6: Startup & app info commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn set_launch_on_startup(
    enable: bool,
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    startup::set_launch_on_startup(enable)?;

    // Persist to config
    let mut guard = config_state.0.lock().map_err(|e| e.to_string())?;
    guard.launch_on_startup = enable;
    config::save_config(&app_handle, &guard)?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub launch_on_startup: bool,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "YOLO Voice".to_string(),
        launch_on_startup: startup::is_launch_on_startup(),
    }
}

#[tauri::command]
pub fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

// ---------------------------------------------------------------------------
// Sidecar setup commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_sidecar_setup_status(
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    sidecar::is_sidecar_setup(&app_handle)
}

#[tauri::command]
pub fn setup_sidecar_cmd(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    sidecar::setup_sidecar_python(&app_handle)
}

// ---------------------------------------------------------------------------
// Phase 7: Global Dictionary & Industry Packs
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_global_dictionary(
    state: State<'_, GlobalDictionaryState>,
) -> Result<GlobalDictionary, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn save_global_dictionary_cmd(
    dictionary: GlobalDictionary,
    app_handle: tauri::AppHandle,
    state: State<'_, GlobalDictionaryState>,
) -> Result<(), String> {
    transcription::save_global_dictionary(&app_handle, &dictionary)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = dictionary;
    Ok(())
}

#[tauri::command]
pub fn get_industry_packs(
    app_handle: tauri::AppHandle,
) -> Result<Vec<IndustryPackInfo>, String> {
    transcription::list_industry_packs(&app_handle)
}

#[tauri::command]
pub fn apply_industry_pack(
    pack_id: String,
    app_handle: tauri::AppHandle,
    dict_state: State<'_, GlobalDictionaryState>,
    config_state: State<'_, ConfigState>,
) -> Result<GlobalDictionary, String> {
    let pack = transcription::load_industry_pack(&app_handle, &pack_id)?;

    let mut guard = dict_state.0.lock().map_err(|e| e.to_string())?;

    // Merge vocabulary (add new, deduplicate)
    for word in &pack.vocabulary {
        if !guard.vocabulary.iter().any(|w| w.eq_ignore_ascii_case(word)) {
            guard.vocabulary.push(word.clone());
        }
    }

    // Merge replacements (add new, skip duplicates by find key)
    for rule in &pack.replacements {
        if !guard
            .replacements
            .iter()
            .any(|r| r.find.eq_ignore_ascii_case(&rule.find))
        {
            guard.replacements.push(rule.clone());
        }
    }

    // Save to disk
    transcription::save_global_dictionary(&app_handle, &guard)?;

    // Update config with active pack
    let mut config_guard = config_state.0.lock().map_err(|e| e.to_string())?;
    config_guard.active_industry_pack = pack_id;
    config::save_config(&app_handle, &config_guard)?;

    Ok(guard.clone())
}
