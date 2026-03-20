use std::sync::Mutex;

use tauri::{Manager, State};

use crate::features::capture::recorder::{self, RecordingState};
use crate::features::output;
use crate::features::settings::{self, AppConfig, ConfigState};
use crate::features::speech;
use crate::features::speech::vocabulary::{
    GlobalDictionary, GlobalDictionaryState, IndustryPackInfo,
};
use crate::infra::platform::{self, AudioStream, DeviceInfo};
use crate::infra::sidecar::{self, SidecarState};

pub struct AudioState(pub Mutex<Option<AudioStream>>);

// ---- Audio Devices ----

#[tauri::command]
pub fn list_devices() -> Vec<DeviceInfo> {
    platform::list_input_devices()
}

#[tauri::command]
pub fn start_test(
    device_index: usize,
    app_handle: tauri::AppHandle,
    state: State<'_, AudioState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    let stream = platform::start_level_monitor(device_index, app_handle)?;
    *guard = Some(stream);
    Ok(())
}

#[tauri::command]
pub fn stop_test(state: State<'_, AudioState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

// ---- Config ----

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
    settings::save_config(&app_handle, &new_config)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = new_config;
    Ok(())
}

// ---- Recording ----

#[tauri::command]
pub fn start_recording(
    device_index: usize,
    app_handle: tauri::AppHandle,
    state: State<'_, RecordingState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
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

// ---- Transcription / Models ----

fn ensure_sidecar_running(
    app_handle: &tauri::AppHandle,
    state: &SidecarState,
) -> Result<(), String> {
    let config = app_handle
        .state::<ConfigState>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    sidecar::ensure_running(
        app_handle,
        state,
        &config.whisper_model,
        &config.device,
        &config.compute_type,
    )
}

#[tauri::command]
pub fn get_models(
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<Vec<speech::ModelInfo>, String> {
    ensure_sidecar_running(&app_handle, &state)?;
    let models_dir = sidecar::get_models_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    speech::list_downloaded_models(sc, &models_dir.to_string_lossy())
}

#[tauri::command]
pub fn download_model_cmd(
    model: String,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<(), String> {
    ensure_sidecar_running(&app_handle, &state)?;
    let models_dir = sidecar::get_models_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    speech::download_model(sc, &model, &models_dir.to_string_lossy(), &app_handle)
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
    ensure_sidecar_running(&app_handle, &sidecar_state)?;
    let models_dir = sidecar::get_models_dir(&app_handle)?;

    {
        let mut guard = sidecar_state.0.lock().map_err(|e| e.to_string())?;
        let sc = guard.as_mut().ok_or("Sidecar not running")?;
        speech::load_model(
            sc,
            &model,
            &device,
            &compute_type,
            &models_dir.to_string_lossy(),
        )?;
    }

    let mut config_guard = config_state.0.lock().map_err(|e| e.to_string())?;
    config_guard.whisper_model = model;
    config_guard.device = device;
    config_guard.compute_type = compute_type;
    settings::save_config(&app_handle, &config_guard)?;

    Ok(())
}

#[tauri::command]
pub fn get_gpu_available(
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<bool, String> {
    ensure_sidecar_running(&app_handle, &state)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    speech::get_gpu_available(sc)
}

#[tauri::command]
pub fn get_sidecar_status(state: State<'_, SidecarState>) -> Result<String, String> {
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

// ---- Profiles ----

#[tauri::command]
pub fn get_profiles(
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<Vec<speech::Profile>, String> {
    ensure_sidecar_running(&app_handle, &state)?;
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    speech::list_profiles(sc, &profiles_dir.to_string_lossy())
}

#[tauri::command]
pub fn save_profile_cmd(
    profile: speech::Profile,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<(), String> {
    ensure_sidecar_running(&app_handle, &state)?;
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    speech::save_profile(sc, &profiles_dir.to_string_lossy(), &profile)
}

#[tauri::command]
pub fn delete_profile_cmd(
    id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, SidecarState>,
) -> Result<(), String> {
    ensure_sidecar_running(&app_handle, &state)?;
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;
    speech::delete_profile(sc, &profiles_dir.to_string_lossy(), &id)
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
    ensure_sidecar_running(&app_handle, &state)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let sc = guard.as_mut().ok_or("Sidecar not running")?;

    let test_profile = speech::Profile {
        id: "_test".to_string(),
        name: "Test".to_string(),
        builtin: false,
        system_prompt: "Fix the grammar. Output only the corrected text.".to_string(),
        dictionary: vec![],
        tone: "neutral".to_string(),
    };

    speech::post_process_text(
        sc,
        "this is a test to check if the connection works",
        &test_profile,
        &provider,
        &model,
        &api_key,
        &base_url,
    )
}

// ---- Startup & App Info ----

#[tauri::command]
pub fn set_launch_on_startup(
    enable: bool,
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    settings::set_launch_on_startup(enable)?;

    let mut guard = config_state.0.lock().map_err(|e| e.to_string())?;
    guard.launch_on_startup = enable;
    settings::save_config(&app_handle, &guard)?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub launch_on_startup: bool,
    pub log_path: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    let log_path = dirs_next::data_dir()
        .map(|d| d.join("com.alish.yolo-voice").join("yolo-voice.log"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "YOLO Voice".to_string(),
        launch_on_startup: settings::is_launch_on_startup(),
        log_path,
    }
}

#[tauri::command]
pub fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

// ---- Sound ----

#[tauri::command]
pub fn preview_sound(sound_name: String) -> Result<(), String> {
    output::play_sound(&sound_name);
    Ok(())
}

#[tauri::command]
pub fn get_available_sounds() -> Vec<String> {
    output::AVAILABLE_SOUNDS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---- Sidecar Setup ----

#[tauri::command]
pub fn get_sidecar_setup_status(app_handle: tauri::AppHandle) -> Result<bool, String> {
    sidecar::is_sidecar_setup(&app_handle)
}

#[tauri::command]
pub fn setup_sidecar_cmd(app_handle: tauri::AppHandle) -> Result<(), String> {
    sidecar::setup_sidecar_python(&app_handle)
}

// ---- Global Dictionary & Industry Packs ----

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
    speech::vocabulary::save_global_dictionary(&app_handle, &dictionary)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = dictionary;
    speech::vocabulary::invalidate_regex_cache();
    Ok(())
}

#[tauri::command]
pub fn get_industry_packs(app_handle: tauri::AppHandle) -> Result<Vec<IndustryPackInfo>, String> {
    speech::vocabulary::list_industry_packs(&app_handle)
}

#[tauri::command]
pub fn apply_industry_pack(
    pack_id: String,
    app_handle: tauri::AppHandle,
    dict_state: State<'_, GlobalDictionaryState>,
    config_state: State<'_, ConfigState>,
) -> Result<GlobalDictionary, String> {
    let pack = speech::vocabulary::load_industry_pack(&app_handle, &pack_id)?;

    let mut guard = dict_state.0.lock().map_err(|e| e.to_string())?;

    speech::vocabulary::merge_pack_into_dictionary(&mut guard, &pack);

    speech::vocabulary::save_global_dictionary(&app_handle, &guard)?;
    speech::vocabulary::invalidate_regex_cache();

    let mut config_guard = config_state.0.lock().map_err(|e| e.to_string())?;
    config_guard.active_industry_pack = pack_id;
    settings::save_config(&app_handle, &config_guard)?;

    Ok(guard.clone())
}
