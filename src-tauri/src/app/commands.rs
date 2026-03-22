use std::sync::Mutex;

use tauri::{Manager, State};

use crate::features::capture::recorder::{self, RecordingState};
use crate::features::output;
use crate::features::settings::{self, AppConfig, ConfigState};
use crate::features::speech;
use crate::features::speech::inference::InferenceState;
use crate::features::speech::vocabulary::{
    GlobalDictionary, GlobalDictionaryState, IndustryPackInfo,
};
use crate::infra::platform::{self, AudioStream, DeviceInfo};

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
    let stream = recorder::start_recording(device_index, app_handle, None)?;
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

// ---- Model / Inference ----

#[tauri::command]
pub fn download_model_cmd(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let models_dir = crate::infra::model::get_models_dir(&app_handle)?;
    crate::infra::model::download_model(&models_dir, &app_handle)?;

    // After download, initialize the inference engine
    let session = speech::inference::InferenceSession::new(&models_dir)
        .map_err(|e| format!("Model downloaded but failed to initialize: {}", e))?;

    let gpu = session.is_gpu();
    let state = app_handle.state::<InferenceState>();
    *state.0.lock().map_err(|e| e.to_string())? = Some(session);

    let _ = tauri::Emitter::emit(&app_handle, "model-status", "ready");

    // Notify frontend if GPU was unavailable and we fell back to CPU
    if !gpu {
        let _ = tauri::Emitter::emit(&app_handle, "gpu-fallback", "CPU (GPU not available)");
    }

    Ok(())
}

#[tauri::command]
pub fn get_gpu_available(
    state: State<'_, InferenceState>,
) -> Result<bool, String> {
    Ok(speech::get_gpu_available(&state))
}

#[derive(serde::Serialize)]
pub struct GpuInfo {
    pub available: bool,
    pub execution_provider: String,
}

#[tauri::command]
pub fn get_gpu_info(state: State<'_, InferenceState>) -> Result<GpuInfo, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(session) => Ok(GpuInfo {
            available: session.is_gpu(),
            execution_provider: if session.is_gpu() {
                "DirectML".to_string()
            } else {
                "CPU".to_string()
            },
        }),
        None => Ok(GpuInfo {
            available: false,
            execution_provider: "Not loaded".to_string(),
        }),
    }
}

#[tauri::command]
pub fn get_model_status(
    state: State<'_, InferenceState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(_) => Ok("ready".to_string()),
        None => {
            match crate::infra::model::get_models_dir(&app_handle) {
                Ok(models_dir) => {
                    if crate::infra::model::is_model_downloaded(&models_dir) {
                        Ok("error".to_string())
                    } else {
                        Ok("not-downloaded".to_string())
                    }
                }
                Err(_) => Ok("not-downloaded".to_string()),
            }
        }
    }
}

// ---- Profiles ----

#[tauri::command]
pub fn get_profiles(
    app_handle: tauri::AppHandle,
) -> Result<Vec<speech::Profile>, String> {
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    speech::list_profiles(&profiles_dir)
}

#[tauri::command]
pub fn save_profile_cmd(
    profile: speech::Profile,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    speech::save_profile(&profiles_dir, &profile)
}

#[tauri::command]
pub fn delete_profile_cmd(
    id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    speech::delete_profile(&profiles_dir, &id)
}

#[tauri::command]
pub fn test_llm_connection(
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
) -> Result<String, String> {
    let test_profile = speech::Profile {
        id: "_test".to_string(),
        name: "Test".to_string(),
        builtin: false,
        system_prompt: "Fix the grammar. Output only the corrected text.".to_string(),
        dictionary: vec![],
        tone: "neutral".to_string(),
    };

    speech::post_process_text(
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

