use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::{fs, io::BufRead, io::BufReader};

use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::features::capture::hotkey::HotkeyCache;
use crate::features::capture::recorder::{self, RecordingState};
use crate::features::capture::RuntimeDictionaryCache;
use crate::features::diagnostics::{
    distil_whisper_events_path, maybe_log_support_event, parakeet_events_path, runtime_events_path,
    SupportDiagnosticsBundleSummary, SupportDiagnosticsExport, TranscriptDiagnosticsState,
    TranscriptDiagnosticsStatus, TranscriptHistoryExport,
};
use crate::features::output;
use crate::features::settings::{self, AppConfig, ConfigState};
use crate::features::speech;
use crate::features::speech::distil_whisper::{
    maybe_prepare_in_background, DistilWhisperState, DistilWhisperStatus,
};
use crate::features::speech::inference::InferenceState;
use crate::features::speech::vocabulary::{IndustryPack, IndustryPackInfo};
use crate::features::speech::TextAction;
use crate::infra::platform::{self, AudioStream, DeviceInfo};

pub struct AudioState(pub Mutex<Option<AudioStream>>);
pub struct OnboardingPreviewState(pub Mutex<Option<recorder::RecordingStream>>);

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OnboardingPreviewRequest {
    pub transcription_mode: String,
    pub cloud_stt_provider: String,
    pub cloud_stt_api_key: String,
    pub language: String,
    #[serde(default)]
    pub offline_engine: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OnboardingPreviewResult {
    pub transcript: String,
    pub effective_provider: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageOverview {
    pub app_data_dir: String,
    pub config_path: String,
    pub models_dir: String,
    pub parakeet_models_dir: String,
    pub distil_whisper_models_dir: String,
    pub diagnostics_dir: String,
    pub transcript_history_db_path: String,
    pub support_exports_dir: String,
    pub profiles_dir: String,
    pub text_actions_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageLocationKind {
    AppData,
    Config,
    Models,
    Diagnostics,
    History,
    SupportExports,
    Profiles,
    TextActions,
}

impl TryFrom<&str> for StorageLocationKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "app_data" => Ok(Self::AppData),
            "config" => Ok(Self::Config),
            "models" => Ok(Self::Models),
            "diagnostics" => Ok(Self::Diagnostics),
            "history" => Ok(Self::History),
            "support_exports" => Ok(Self::SupportExports),
            "profiles" => Ok(Self::Profiles),
            "text_actions" => Ok(Self::TextActions),
            other => Err(format!("Unknown storage location kind: {}", other)),
        }
    }
}

/// Invalidate both the regex cache and the runtime dictionary cache.
/// Call this whenever vocabulary or replacement rules change.
fn invalidate_vocabulary_caches(app: &tauri::AppHandle) {
    speech::vocabulary::invalidate_regex_cache();
    if let Ok(mut guard) = app.state::<RuntimeDictionaryCache>().0.lock() {
        *guard = None;
    }
}

fn ensure_text_actions_initialized(
    app_handle: &tauri::AppHandle,
    config_state: &State<'_, ConfigState>,
) -> Result<(), String> {
    let mut config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    let changed = speech::ensure_text_actions_ready(app_handle, &mut config)?;
    if changed {
        let saved = settings::save_config(app_handle, &config)?;
        let mut guard = config_state.0.lock().map_err(|e| e.to_string())?;
        *guard = saved;
    }
    Ok(())
}

fn storage_overview_from_app_data(
    app_data_dir: &std::path::Path,
    config_path: &std::path::Path,
    models_dir: &std::path::Path,
    parakeet_models_dir: &std::path::Path,
    distil_whisper_models_dir: &std::path::Path,
    diagnostics_dir: &std::path::Path,
    transcript_history_db_path: &std::path::Path,
    support_exports_dir: &std::path::Path,
    profiles_dir: &std::path::Path,
    text_actions_dir: &std::path::Path,
) -> StorageOverview {
    StorageOverview {
        app_data_dir: app_data_dir.to_string_lossy().to_string(),
        config_path: config_path.to_string_lossy().to_string(),
        models_dir: models_dir.to_string_lossy().to_string(),
        parakeet_models_dir: parakeet_models_dir.to_string_lossy().to_string(),
        distil_whisper_models_dir: distil_whisper_models_dir.to_string_lossy().to_string(),
        diagnostics_dir: diagnostics_dir.to_string_lossy().to_string(),
        transcript_history_db_path: transcript_history_db_path.to_string_lossy().to_string(),
        support_exports_dir: support_exports_dir.to_string_lossy().to_string(),
        profiles_dir: profiles_dir.to_string_lossy().to_string(),
        text_actions_dir: text_actions_dir.to_string_lossy().to_string(),
    }
}

fn build_storage_overview(app_handle: &tauri::AppHandle) -> Result<StorageOverview, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let config_path = settings::get_config_path(app_handle)?;
    let models_dir = crate::infra::model::get_models_root_dir(app_handle)?;
    let parakeet_models_dir = crate::infra::model::get_models_dir(app_handle)?;
    let distil_whisper_models_dir = crate::infra::model::get_distil_whisper_models_dir(app_handle)?;
    let diagnostics_dir = crate::features::diagnostics::diagnostics_dir(app_handle)?;
    let transcript_history_db_path = crate::features::diagnostics::diagnostics_db_path(app_handle)?;
    let support_exports_dir = crate::features::diagnostics::support_exports_dir(app_handle)?;
    let profiles_dir = speech::get_profiles_dir(app_handle)?;
    let text_actions_dir = speech::get_text_actions_dir(app_handle)?;

    Ok(storage_overview_from_app_data(
        &app_data_dir,
        &config_path,
        &models_dir,
        &parakeet_models_dir,
        &distil_whisper_models_dir,
        &diagnostics_dir,
        &transcript_history_db_path,
        &support_exports_dir,
        &profiles_dir,
        &text_actions_dir,
    ))
}

fn resolve_storage_location_path(
    app_handle: &tauri::AppHandle,
    kind: StorageLocationKind,
) -> Result<std::path::PathBuf, String> {
    match kind {
        StorageLocationKind::AppData => {
            let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Ok(dir)
        }
        StorageLocationKind::Config => settings::get_config_path(app_handle)?
            .parent()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| "Config folder not available".to_string()),
        StorageLocationKind::Models => crate::infra::model::get_models_root_dir(app_handle),
        StorageLocationKind::Diagnostics => crate::features::diagnostics::diagnostics_dir(app_handle),
        StorageLocationKind::History => crate::features::diagnostics::diagnostics_db_path(app_handle)?
            .parent()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| "History folder not available".to_string()),
        StorageLocationKind::SupportExports => {
            let dir = crate::features::diagnostics::support_exports_dir(app_handle)?;
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Ok(dir)
        }
        StorageLocationKind::Profiles => speech::get_profiles_dir(app_handle),
        StorageLocationKind::TextActions => speech::get_text_actions_dir(app_handle),
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
pub fn get_storage_overview(app_handle: tauri::AppHandle) -> Result<StorageOverview, String> {
    build_storage_overview(&app_handle)
}

#[tauri::command]
pub fn open_storage_location(
    kind: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let kind = StorageLocationKind::try_from(kind.as_str())?;
    let path = resolve_storage_location_path(&app_handle, kind)?;
    app_handle
        .opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config_cmd(
    new_config: AppConfig,
    app_handle: tauri::AppHandle,
    state: State<'_, ConfigState>,
    hotkey_cache: State<'_, HotkeyCache>,
) -> Result<AppConfig, String> {
    let (
        old_lang,
        old_pill_pinned,
        old_device_index,
        old_offline_engine,
        old_hotkey,
        old_command_hotkey,
        old_activation_mode,
    ) = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        (
            guard.ui_language.clone(),
            guard.pill_pinned,
            guard.device_index,
            guard.offline_engine.clone(),
            guard.hotkey.clone(),
            guard.command_hotkey.clone(),
            guard.dictation_activation_mode.clone(),
        )
    };

    let saved_config = settings::save_config(&app_handle, &new_config)?;
    // Update cached hotkey keys so the rdev listener picks up changes immediately
    hotkey_cache.update(&saved_config.hotkey, &saved_config.command_hotkey);
    if saved_config.hotkey != old_hotkey
        || saved_config.command_hotkey != old_command_hotkey
        || saved_config.dictation_activation_mode != old_activation_mode
    {
        app_handle
            .state::<crate::features::capture::HotkeyRuntimeState>()
            .reset_listener_state();
    }

    // Notify all windows when UI language changes
    if saved_config.ui_language != old_lang {
        let _ = app_handle.emit("ui-language-changed", &saved_config.ui_language);
    }
    // Notify all windows when pill pinned state changes
    if saved_config.pill_pinned != old_pill_pinned {
        let _ = app_handle.emit("pill-pinned-changed", saved_config.pill_pinned);
    }
    // Re-warm audio device if microphone changed
    if saved_config.device_index != old_device_index {
        use crate::features::capture::recorder::{spawn_warm_device, WarmDeviceState};
        if let Some(warm) = app_handle.try_state::<WarmDeviceState>() {
            if let Ok(mut g) = warm.0.lock() {
                *g = None;
            }
        }
        spawn_warm_device(&app_handle, saved_config.device_index);
    }

    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = saved_config.clone();
    let should_prepare_distil = guard.transcription_mode == "offline"
        && guard.offline_engine == "distil_whisper"
        && old_offline_engine != "distil_whisper";
    drop(guard);

    if should_prepare_distil {
        let _ = maybe_prepare_in_background(&app_handle);
    }
    Ok(saved_config)
}

// ---- Text Actions ----

#[tauri::command]
pub fn get_text_actions(
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<Vec<TextAction>, String> {
    ensure_text_actions_initialized(&app_handle, &config_state)?;
    let text_actions_dir = speech::get_text_actions_dir(&app_handle)?;
    speech::list_text_actions(&text_actions_dir)
}

#[tauri::command]
pub fn save_text_action(
    action: TextAction,
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    ensure_text_actions_initialized(&app_handle, &config_state)?;
    let text_actions_dir = speech::get_text_actions_dir(&app_handle)?;
    speech::save_text_action(&text_actions_dir, &action)
}

#[tauri::command]
pub fn delete_text_action(
    id: String,
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    ensure_text_actions_initialized(&app_handle, &config_state)?;
    let text_actions_dir = speech::get_text_actions_dir(&app_handle)?;
    speech::delete_text_action(&text_actions_dir, &id)
}

#[tauri::command]
pub fn reset_text_action_to_default(
    id: String,
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    ensure_text_actions_initialized(&app_handle, &config_state)?;
    let text_actions_dir = speech::get_text_actions_dir(&app_handle)?;
    speech::reset_text_action_to_default(&text_actions_dir, &id)
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

#[tauri::command]
pub fn start_onboarding_preview_recording(
    device_index: usize,
    app_handle: tauri::AppHandle,
    state: State<'_, OnboardingPreviewState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    let app_handle_for_state = app_handle.clone();
    let warm_state = app_handle_for_state.try_state::<recorder::WarmDeviceState>();
    let stream = recorder::start_recording(device_index, app_handle, None, warm_state.as_deref())?;
    *guard = Some(stream);
    Ok(())
}

#[tauri::command]
pub fn cancel_onboarding_preview_recording(
    state: State<'_, OnboardingPreviewState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn finish_onboarding_preview(
    request: OnboardingPreviewRequest,
    app_handle: tauri::AppHandle,
    preview_state: State<'_, OnboardingPreviewState>,
    inference_state: State<'_, InferenceState>,
    distil_state: State<'_, DistilWhisperState>,
    config_state: State<'_, ConfigState>,
) -> Result<OnboardingPreviewResult, String> {
    let recording = {
        let mut guard = preview_state.0.lock().map_err(|e| e.to_string())?;
        guard
            .take()
            .ok_or_else(|| "No onboarding preview recording is active.".to_string())?
    };

    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();
    let transcription_mode = if request.transcription_mode.trim().is_empty() {
        config.transcription_mode.clone()
    } else {
        request.transcription_mode.trim().to_string()
    };
    let offline_engine = if request.offline_engine.trim().is_empty() {
        config.offline_engine.clone()
    } else {
        request.offline_engine.trim().to_string()
    };
    let language = if request.language.trim().is_empty() {
        config.language.clone()
    } else {
        request.language.trim().to_string()
    };

    let (raw_transcript, effective_provider) = if transcription_mode == "cloud" {
        if request.cloud_stt_api_key.trim().is_empty() {
            return Err("Enter your API key to test cloud transcription.".to_string());
        }

        let path = recorder::stop_and_save(recording)?;
        let transcript = speech::cloud_transcribe(
            &path.to_string_lossy(),
            &request.cloud_stt_provider,
            &request.cloud_stt_api_key,
            &language,
        )?;
        (transcript, request.cloud_stt_provider.trim().to_string())
    } else if offline_engine == "distil_whisper" {
        let wav_bytes = recorder::stop_and_get_wav_bytes(recording)?;
        let transcript = {
            let mut guard = distil_state.0.lock().map_err(|e| e.to_string())?;
            guard.transcribe_local_wav_bytes(&app_handle, &wav_bytes)?.text
        };
        (transcript, "distil_whisper".to_string())
    } else {
        let (samples, sample_rate, channels) = recorder::stop_and_get_raw_samples(recording)?;
        let transcript = speech::transcribe_audio(&inference_state, &samples, sample_rate, channels)?;
        (transcript, "parakeet".to_string())
    };

    Ok(build_onboarding_preview_result(
        raw_transcript,
        &effective_provider,
    )?)
}

// ---- Model / Inference ----

static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn download_model_cmd(app_handle: tauri::AppHandle) -> Result<(), String> {
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
        maybe_log_support_event(
            &handle,
            "parakeet",
            "download_requested",
            "Downloading Parakeet model",
            serde_json::json!({}),
        );
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
            maybe_log_support_event(
                &handle,
                "parakeet",
                "load_success",
                "Downloaded and initialized Parakeet model",
                serde_json::json!({
                    "gpu": gpu,
                }),
            );

            let _ = handle.emit("model-status", "ready");
            if !gpu {
                let _ = handle.emit("gpu-fallback", "CPU (GPU not available)");
            }
            Ok(())
        })();

        DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);

        if let Err(e) = result {
            eprintln!("[download] Error: {}", e);
            maybe_log_support_event(
                &handle,
                "parakeet",
                "download_error",
                "Failed to download or initialize Parakeet model",
                serde_json::json!({
                    "error": e,
                }),
            );
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
pub fn get_gpu_available(state: State<'_, InferenceState>) -> Result<bool, String> {
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
        maybe_log_support_event(
            &handle,
            "parakeet",
            "reload_requested",
            "Reloading Parakeet model",
            serde_json::json!({
                "use_gpu": use_gpu,
            }),
        );
        match speech::inference::InferenceSession::with_gpu(&models_dir, use_gpu) {
            Ok(session) => {
                let gpu = session.is_gpu();
                let state = handle.state::<InferenceState>();
                *state.0.lock().unwrap() = Some(session);
                maybe_log_support_event(
                    &handle,
                    "parakeet",
                    "reload_success",
                    "Reloaded Parakeet model",
                    serde_json::json!({
                        "gpu": gpu,
                    }),
                );
                let _ = handle.emit("model-status", "ready");
                if !gpu {
                    let _ = handle.emit("gpu-fallback", "CPU (GPU not available)");
                }
            }
            Err(e) => {
                eprintln!("[reload] Failed: {}", e);
                maybe_log_support_event(
                    &handle,
                    "parakeet",
                    "reload_error",
                    "Failed to reload Parakeet model",
                    serde_json::json!({
                        "use_gpu": use_gpu,
                        "error": e,
                    }),
                );
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
        None => match crate::infra::model::get_models_dir(&app_handle) {
            Ok(models_dir) => {
                if crate::infra::model::is_model_downloaded(&models_dir) {
                    Ok("error".to_string())
                } else {
                    Ok("not-downloaded".to_string())
                }
            }
            Err(_) => Ok("not-downloaded".to_string()),
        },
    }
}

fn build_onboarding_preview_result(
    raw_transcript: String,
    effective_provider: &str,
) -> Result<OnboardingPreviewResult, String> {
    let transcript = raw_transcript.trim().to_string();
    if transcript.is_empty() {
        return Err("No speech detected. Try speaking a little longer and closer to the microphone.".to_string());
    }

    Ok(OnboardingPreviewResult {
        transcript,
        effective_provider: effective_provider.to_string(),
    })
}

#[tauri::command]
pub fn open_distil_whisper_model_page_cmd(app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle
        .opener()
        .open_url(
            crate::features::speech::distil_whisper::DISTIL_WHISPER_URL,
            None::<&str>,
        )
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_distil_whisper_model_status(
    app_handle: tauri::AppHandle,
    state: State<'_, DistilWhisperState>,
) -> Result<DistilWhisperStatus, String> {
    match state.0.try_lock() {
        Ok(mut guard) => Ok(guard.status(&app_handle)),
        Err(_) => Ok(DistilWhisperStatus {
            status: "preparing".to_string(),
            downloaded: crate::infra::model::get_distil_whisper_models_dir(&app_handle)
                .map(|dir| crate::infra::model::is_distil_whisper_model_downloaded(&dir))
                .unwrap_or(false),
            ready: false,
            device: None,
            gpu_available: false,
            runtime: "transformers-distil-whisper".to_string(),
            message: None,
        }),
    }
}

#[cfg(test)]
mod onboarding_preview_tests {
    use super::build_onboarding_preview_result;

    #[test]
    fn preview_result_trims_transcript() {
        let result =
            build_onboarding_preview_result("  hello this is a test  ".to_string(), "parakeet")
                .expect("preview result should succeed");

        assert_eq!(result.transcript, "hello this is a test");
        assert_eq!(result.effective_provider, "parakeet");
    }

    #[test]
    fn preview_result_rejects_blank_transcript() {
        let err = build_onboarding_preview_result("   ".to_string(), "groq")
            .expect_err("blank transcripts should fail");

        assert!(err.contains("No speech detected"));
    }
}

#[tauri::command]
pub fn download_distil_whisper_model_cmd(
    app_handle: tauri::AppHandle,
    state: State<'_, DistilWhisperState>,
) -> Result<DistilWhisperStatus, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.download_model(&app_handle)
}

#[tauri::command]
pub fn prepare_distil_whisper_model_cmd(
    app_handle: tauri::AppHandle,
    state: State<'_, DistilWhisperState>,
) -> Result<DistilWhisperStatus, String> {
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let status = guard.status(&app_handle);
        if !status.downloaded {
            return Err("Download Distil-Whisper first.".to_string());
        }
        if status.ready {
            return Ok(status);
        }
    }

    maybe_prepare_in_background(&app_handle)?;

    match state.0.try_lock() {
        Ok(mut guard) => Ok(guard.status(&app_handle)),
        Err(_) => Ok(DistilWhisperStatus {
            status: "preparing".to_string(),
            downloaded: true,
            ready: false,
            device: None,
            gpu_available: false,
            runtime: "transformers-distil-whisper".to_string(),
            message: None,
        }),
    }
}

#[tauri::command]
pub fn reload_distil_whisper_model_cmd(
    use_gpu: bool,
    app_handle: tauri::AppHandle,
    state: State<'_, DistilWhisperState>,
) -> Result<DistilWhisperStatus, String> {
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let status = guard.status(&app_handle);
        if !status.downloaded {
            return Err("Download Distil-Whisper first.".to_string());
        }
        guard.set_preferred_device(use_gpu);
        guard.shutdown()?;
    }

    maybe_prepare_in_background(&app_handle)?;

    match state.0.try_lock() {
        Ok(mut guard) => Ok(guard.status(&app_handle)),
        Err(_) => Ok(DistilWhisperStatus {
            status: "preparing".to_string(),
            downloaded: true,
            ready: false,
            device: None,
            gpu_available: false,
            runtime: "transformers-distil-whisper".to_string(),
            message: None,
        }),
    }
}

#[tauri::command]
pub fn delete_distil_whisper_model_cmd(
    app_handle: tauri::AppHandle,
    state: State<'_, DistilWhisperState>,
) -> Result<DistilWhisperStatus, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    guard.delete_model(&app_handle)
}

// ---- Profiles ----

#[tauri::command]
pub fn get_profiles(app_handle: tauri::AppHandle) -> Result<Vec<speech::Profile>, String> {
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
pub fn delete_profile_cmd(id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let profiles_dir = speech::get_profiles_dir(&app_handle)?;
    speech::delete_profile(&profiles_dir, &id)
}

#[tauri::command]
pub fn reset_profile_to_default(id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
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
    let _ = settings::save_config(&app_handle, &guard)?;
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
    let _ = settings::save_config(&app_handle, &config_guard)?;
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
pub fn get_general_vocabulary(app_handle: tauri::AppHandle) -> Result<IndustryPack, String> {
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
pub fn reset_industry_pack_cmd(id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
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

#[tauri::command]
pub fn export_support_diagnostics(
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
    inference_state: State<'_, InferenceState>,
    distil_state: State<'_, DistilWhisperState>,
) -> Result<SupportDiagnosticsExport, String> {
    let config = config_state.0.lock().map_err(|e| e.to_string())?.clone();

    let diagnostics_status = diagnostics_state
        .0
        .status(config.transcript_diagnostics_enabled)?;

    let parakeet_gpu_loaded = speech::get_gpu_available(&inference_state);
    let distil_status = distil_state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .status(&app_handle);

    let runtime_event_count = count_lines_if_exists(&runtime_events_path(&app_handle)?);
    let distil_event_count = count_lines_if_exists(&distil_whisper_events_path(&app_handle)?);
    let parakeet_event_count = count_lines_if_exists(&parakeet_events_path(&app_handle)?);

    let summary = SupportDiagnosticsBundleSummary {
        generated_at: crate::features::diagnostics::current_timestamp_ms(),
        app_name: "YOLO Voice".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        session_id: diagnostics_state.0.session_id().to_string(),
        diagnostics_enabled: config.transcript_diagnostics_enabled,
        runtime_event_count,
        distil_event_count,
        parakeet_event_count,
        sample_count: diagnostics_status.sample_count,
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        bundle_contents: vec![
            "summary.json".to_string(),
            "config.redacted.json".to_string(),
            "diagnostics/runtime_events.jsonl".to_string(),
            "diagnostics/distil_whisper_events.jsonl".to_string(),
            "diagnostics/parakeet_events.jsonl".to_string(),
        ],
    };

    let redacted_config = serde_json::json!({
        "transcription_mode": config.transcription_mode,
        "offline_engine": config.offline_engine,
        "cloud_stt_provider": config.cloud_stt_provider,
        "language": config.language,
        "device_index": config.device_index,
        "sounds_enabled": config.sounds_enabled,
        "start_sound": config.start_sound,
        "stop_sound": config.stop_sound,
        "text_cleanup_enabled": config.text_cleanup_enabled,
        "numerals_enabled": config.numerals_enabled,
        "hallucination_filter_enabled": config.hallucination_filter_enabled,
        "spoken_punctuation_enabled": config.spoken_punctuation_enabled,
        "parakeet_segmented_mode_enabled": config.parakeet_segmented_mode_enabled,
        "dictation_activation_mode": config.dictation_activation_mode,
        "post_processing_enabled": config.post_processing_enabled,
        "vad_silence_threshold_ms": config.vad_silence_threshold_ms,
        "continuous_recording_enabled": config.continuous_recording_enabled,
        "auto_pause_media_enabled": config.auto_pause_media_enabled,
        "active_profile_id": config.active_profile_id,
        "active_industry_pack": config.active_industry_pack,
        "ui_language": config.ui_language,
        "transcript_diagnostics_enabled": config.transcript_diagnostics_enabled,
        "command_provider": config.command_provider,
        "command_model": config.command_model,
        "default_text_action_id": config.default_text_action_id,
        "llm_provider": config.llm_provider,
        "llm_model": config.llm_model,
        "cloud_stt_api_key": redact_secret(&config.cloud_stt_api_key),
        "llm_api_key": redact_secret(&config.llm_api_key),
        "command_api_key": redact_secret(&config.command_api_key),
        "llm_base_url": config.llm_base_url,
        "command_base_url": config.command_base_url,
        "parakeet_gpu_loaded": parakeet_gpu_loaded,
        "distil_status": distil_status,
    });

    diagnostics_state
        .0
        .export_support_bundle(&app_handle, &summary, &redacted_config)
}

#[tauri::command]
pub fn export_transcript_history(
    app_handle: tauri::AppHandle,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<TranscriptHistoryExport, String> {
    diagnostics_state.0.export_transcript_history(&app_handle)
}

fn count_lines_if_exists(path: &std::path::Path) -> u64 {
    if !path.is_file() {
        return 0;
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return 0,
    };

    BufReader::new(file)
        .lines()
        .filter(|line| line.is_ok())
        .count() as u64
}

fn redact_secret(value: &str) -> String {
    if value.trim().is_empty() {
        return String::new();
    }

    let visible = value.chars().rev().take(4).collect::<String>();
    let suffix = visible.chars().rev().collect::<String>();
    format!("***{}", suffix)
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

#[cfg(test)]
mod storage_location_tests {
    use super::{storage_overview_from_app_data, StorageLocationKind};
    use std::path::Path;

    #[test]
    fn storage_location_kind_rejects_unknown_values() {
        assert!(StorageLocationKind::try_from("unknown").is_err());
        assert_eq!(
            StorageLocationKind::try_from("history").unwrap(),
            StorageLocationKind::History
        );
    }

    #[test]
    fn storage_overview_uses_expected_path_layout() {
        let base = Path::new("C:/Users/Test/AppData/Roaming/com.yolo.voice");
        let overview = storage_overview_from_app_data(
            base,
            &base.join("config.json"),
            &base.join("models"),
            &base.join("models").join("parakeet-tdt-v3"),
            &base.join("models").join("distil-whisper"),
            &base.join("diagnostics"),
            &base.join("diagnostics").join("transcript_samples.sqlite3"),
            &base.join("diagnostics").join("exports"),
            &base.join("profiles"),
            &base.join("text_actions"),
        );
        let normalized_support_exports = overview.support_exports_dir.replace('\\', "/");

        assert!(overview.config_path.ends_with("config.json"));
        assert!(overview.models_dir.ends_with("models"));
        assert!(overview.parakeet_models_dir.ends_with("parakeet-tdt-v3"));
        assert!(overview.distil_whisper_models_dir.ends_with("distil-whisper"));
        assert!(overview.diagnostics_dir.ends_with("diagnostics"));
        assert!(overview.transcript_history_db_path.ends_with("transcript_samples.sqlite3"));
        assert!(normalized_support_exports.ends_with("diagnostics/exports"));
        assert!(overview.profiles_dir.ends_with("profiles"));
        assert!(overview.text_actions_dir.ends_with("text_actions"));
    }
}

#[tauri::command]
pub fn delete_transcript_entry(
    id: i64,
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<(), String> {
    diagnostics_state.0.delete_entry(id)
}

#[tauri::command]
pub fn clear_transcript_history(
    diagnostics_state: State<'_, TranscriptDiagnosticsState>,
) -> Result<(), String> {
    diagnostics_state.0.clear_history()
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
