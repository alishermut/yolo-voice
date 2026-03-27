use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{Emitter, Manager, State};

use crate::features::capture::hotkey::HotkeyCache;
use crate::features::capture::recorder::{self, RecordingState};
use crate::features::capture::RuntimeDictionaryCache;
use crate::features::diagnostics::{TranscriptDiagnosticsState, TranscriptDiagnosticsStatus};
use crate::features::output;
use crate::features::settings::{self, AppConfig, ConfigState};
use crate::features::speech;
use crate::features::speech::inference::InferenceState;
use crate::features::speech::vocabulary::{IndustryPack, IndustryPackInfo};
use crate::infra::platform::{self, AudioStream, DeviceInfo};

pub struct AudioState(pub Mutex<Option<AudioStream>>);

/// Invalidate both the regex cache and the runtime dictionary cache.
/// Call this whenever vocabulary or replacement rules change.
fn invalidate_vocabulary_caches(app: &tauri::AppHandle) {
    speech::vocabulary::invalidate_regex_cache();
    if let Ok(mut guard) = app.state::<RuntimeDictionaryCache>().0.lock() {
        *guard = None;
    }
}

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
    hotkey_cache: State<'_, HotkeyCache>,
) -> Result<(), String> {
    let (old_lang, old_pill_pinned, old_device_index) = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        (guard.ui_language.clone(), guard.pill_pinned, guard.device_index)
    };

    settings::save_config(&app_handle, &new_config)?;
    // Update cached hotkey keys so the rdev listener picks up changes immediately
    hotkey_cache.update(&new_config.hotkey, &new_config.command_hotkey);

    // Notify all windows when UI language changes
    if new_config.ui_language != old_lang {
        let _ = app_handle.emit("ui-language-changed", &new_config.ui_language);
    }
    // Notify all windows when pill pinned state changes
    if new_config.pill_pinned != old_pill_pinned {
        let _ = app_handle.emit("pill-pinned-changed", new_config.pill_pinned);
    }
    // Re-warm audio device if microphone changed
    if new_config.device_index != old_device_index {
        use crate::features::capture::recorder::{WarmDeviceState, spawn_warm_device};
        if let Some(warm) = app_handle.try_state::<WarmDeviceState>() {
            if let Ok(mut g) = warm.0.lock() {
                *g = None;
            }
        }
        spawn_warm_device(&app_handle, new_config.device_index);
    }

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
    let stream = recorder::start_recording(device_index, app_handle, None, None)?;
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

static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn download_model_cmd(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Prevent duplicate concurrent downloads
    if DOWNLOAD_IN_PROGRESS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Download already in progress".to_string());
    }

    DOWNLOAD_CANCELLED.store(false, Ordering::SeqCst);

    let handle = app_handle.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let models_dir = crate::infra::model::get_models_dir(&handle)?;
            crate::infra::model::download_model(&models_dir, &handle, &DOWNLOAD_CANCELLED)?;

            // Signal the UI that we're now initializing (loading ONNX into memory)
            let _ = handle.emit(
                "model-download-progress",
                serde_json::json!({ "status": "initializing" }),
            );
            let _ = handle.emit("model-status", "loading");

            let session = speech::inference::InferenceSession::new(&models_dir)
                .map_err(|e| format!("Model downloaded but failed to initialize: {}", e))?;

            let gpu = session.is_gpu();
            let state = handle.state::<InferenceState>();
            *state.0.lock().map_err(|e| e.to_string())? = Some(session);

            let _ = handle.emit("model-status", "ready");
            if !gpu {
                let _ = handle.emit("gpu-fallback", "CPU (GPU not available)");
            }
            Ok(())
        })();

        DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);

        if let Err(e) = result {
            eprintln!("[download] Error: {}", e);
            let _ = handle.emit(
                "model-download-progress",
                serde_json::json!({ "status": "error", "error": e }),
            );
            let _ = handle.emit("model-status", "error");
        }
    });

    Ok(()) // Returns immediately — download runs in background
}

#[tauri::command]
pub fn cancel_model_download_cmd() -> Result<(), String> {
    if !DOWNLOAD_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("No download in progress".to_string());
    }
    DOWNLOAD_CANCELLED.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn delete_model_cmd(
    app_handle: tauri::AppHandle,
    state: State<'_, InferenceState>,
) -> Result<(), String> {
    // Prevent deleting while a download is in progress
    if DOWNLOAD_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("Cannot delete model while download is in progress".to_string());
    }

    // Unload the inference session (frees GPU/CPU memory)
    *state.0.lock().map_err(|e| e.to_string())? = None;

    // Delete model files from disk
    let models_dir = crate::infra::model::get_models_dir(&app_handle)?;
    crate::infra::model::delete_model_files(&models_dir)?;

    let _ = app_handle.emit("model-status", "not-downloaded");
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
pub fn reload_model_cmd(
    use_gpu: bool,
    app_handle: tauri::AppHandle,
    state: State<'_, InferenceState>,
) -> Result<(), String> {
    let models_dir = crate::infra::model::get_models_dir(&app_handle)?;
    if !crate::infra::model::is_model_downloaded(&models_dir) {
        return Err("Model not downloaded".to_string());
    }

    // Drop existing session
    *state.0.lock().map_err(|e| e.to_string())? = None;

    let _ = app_handle.emit("model-status", "loading");

    let handle = app_handle.clone();
    std::thread::spawn(move || {
        match speech::inference::InferenceSession::with_gpu(&models_dir, use_gpu) {
            Ok(session) => {
                let gpu = session.is_gpu();
                let state = handle.state::<InferenceState>();
                *state.0.lock().unwrap() = Some(session);
                let _ = handle.emit("model-status", "ready");
                if !gpu {
                    let _ = handle.emit("gpu-fallback", "CPU (GPU not available)");
                }
            }
            Err(e) => {
                eprintln!("[reload] Failed: {}", e);
                let _ = handle.emit("model-status", "error");
            }
        }
    });

    Ok(())
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
pub fn reset_profile_to_default(
    id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    speech::reset_profile_to_default(&profiles_dir, &id, &app_handle)
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
        terminology_hints: vec![],
        tone: "neutral".to_string(),
        shortcut_key: String::new(),
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

#[tauri::command]
pub fn test_command_llm_connection(
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
) -> Result<String, String> {
    speech::command_llm_call(
        "write hello world in python",
        "You are a voice command assistant. Output only the requested text.",
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
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "YOLO Voice".to_string(),
        launch_on_startup: settings::is_launch_on_startup(),
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

// ---- Industry Packs & Vocabulary ----

#[tauri::command]
pub fn get_industry_packs(app_handle: tauri::AppHandle) -> Result<Vec<IndustryPackInfo>, String> {
    speech::vocabulary::list_industry_packs(&app_handle)
}

#[tauri::command]
pub fn apply_industry_pack(
    pack_id: String,
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    speech::vocabulary::load_industry_pack(&app_handle, &pack_id)?;

    let mut config_guard = config_state.0.lock().map_err(|e| e.to_string())?;
    config_guard.active_industry_pack = pack_id;
    settings::save_config(&app_handle, &config_guard)?;
    invalidate_vocabulary_caches(&app_handle);

    Ok(())
}

// ---- General Vocabulary & Editable Packs ----

#[tauri::command]
pub fn load_industry_pack_cmd(
    id: String,
    app_handle: tauri::AppHandle,
) -> Result<IndustryPack, String> {
    speech::vocabulary::load_industry_pack(&app_handle, &id)
}

#[tauri::command]
pub fn get_general_vocabulary(
    app_handle: tauri::AppHandle,
) -> Result<IndustryPack, String> {
    speech::vocabulary::load_general_vocabulary(&app_handle)
}

#[tauri::command]
pub fn save_general_vocabulary_cmd(
    pack: IndustryPack,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    speech::vocabulary::save_general_vocabulary(&app_handle, &pack)?;
    invalidate_vocabulary_caches(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn save_industry_pack_cmd(
    pack: IndustryPack,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    speech::vocabulary::save_industry_pack(&app_handle, &pack)?;
    invalidate_vocabulary_caches(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn reset_industry_pack_cmd(
    id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    speech::vocabulary::reset_industry_pack(&app_handle, &id)?;
    invalidate_vocabulary_caches(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn generate_vocab_variants(
    term: String,
    config_state: State<'_, ConfigState>,
) -> Result<Vec<String>, String> {
    let api_key = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.command_api_key.clone()
    };
    speech::llm::generate_misspelling_variants(&term, &api_key)
}

// ---- Transcript Diagnostics ----

#[tauri::command]
pub fn get_transcript_diagnostics_status(
    config_state: State<'_, ConfigState>,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<TranscriptDiagnosticsStatus, String> {
    let enabled = config_state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .transcript_diagnostics_enabled;

    diagnostics_state.0.status(enabled)
}

#[tauri::command]
pub fn clear_transcript_diagnostics(
    config_state: State<'_, ConfigState>,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<TranscriptDiagnosticsStatus, String> {
    let enabled = config_state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .transcript_diagnostics_enabled;

    diagnostics_state.0.clear(enabled)
}

// ---- Transcript History ----

#[tauri::command]
pub fn get_transcript_history(
    limit: u32,
    offset: u32,
    search: Option<String>,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<Vec<crate::features::diagnostics::TranscriptHistoryEntry>, String> {
    diagnostics_state
        .0
        .list_history(limit, offset, search.as_deref())
}

#[tauri::command]
pub fn delete_transcript_entry(
    id: i64,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<(), String> {
    diagnostics_state.0.delete_entry(id)
}

#[tauri::command]
pub fn get_transcript_entry_words(
    id: i64,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<Vec<String>, String> {
    diagnostics_state.0.get_entry_words(id)
}

// ---- Auto-Learn Dictionary ----

#[tauri::command]
pub fn add_words_to_dictionary(
    words: Vec<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut general = speech::vocabulary::load_general_vocabulary(&app_handle)?;

    for word in &words {
        let trimmed = word.trim().to_string();
        if !trimmed.is_empty()
            && !general
                .vocabulary
                .iter()
                .any(|v| v.eq_ignore_ascii_case(&trimmed))
        {
            general.vocabulary.push(trimmed);
        }
    }

    speech::vocabulary::save_general_vocabulary(&app_handle, &general)?;
    invalidate_vocabulary_caches(&app_handle);
    Ok(())
}
