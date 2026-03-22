pub mod hotkey;
pub mod recorder;

use tauri::{AppHandle, Listener, Manager};

use crate::app::events::emit_all;
use crate::features::output::{self, FocusedWindowState};
use crate::features::settings::ConfigState;
use crate::features::speech;
use crate::features::speech::inference::InferenceState;
use crate::features::speech::vocabulary::GlobalDictionaryState;

use self::recorder::{RecordingState, VadConfig};

/// Set up the hotkey-action event listener that orchestrates the
/// record → transcribe → insert pipeline.
pub fn setup_hotkey_handler(app: &AppHandle) {
    let app_handle = app.clone();
    app.listen("hotkey-action", move |event| {
        let action = event.payload().trim_matches('"');
        let config = app_handle
            .state::<ConfigState>()
            .0
            .lock()
            .unwrap()
            .clone();

        match action {
            "start" => handle_start(&app_handle, &config),
            "stop" => handle_stop(&app_handle, &config),
            _ => {}
        }
    });
}

fn handle_start(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
) {
    // Capture the foreground window before recording
    let hwnd = output::capture_foreground_window();
    *app.state::<FocusedWindowState>().0.lock().unwrap() = hwnd;

    let recording_state = app.state::<RecordingState>();
    let mut guard = recording_state.0.lock().unwrap();
    // Stop any existing recording
    *guard = None;

    // Build VAD config if offline mode is active and inference is ready
    let vad_config = if config.transcription_mode == "offline" {
        let inference_state = app.state::<InferenceState>();
        let inference_ready = inference_state
            .0
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false);

        if inference_ready {
            match resolve_vad_model_path(app) {
                Ok(model_path) => Some(VadConfig {
                    silence_threshold_ms: config.vad_silence_threshold_ms,
                    model_path,
                    text_cleanup_enabled: config.text_cleanup_enabled,
                }),
                Err(e) => {
                    eprintln!("[capture] VAD model not found, falling back to non-VAD: {e}");
                    None
                }
            }
        } else {
            eprintln!("[capture] Inference not ready, falling back to non-VAD");
            None
        }
    } else {
        None
    };

    match recorder::start_recording(config.device_index, app.clone(), vad_config) {
        Ok(stream) => {
            *guard = Some(stream);
            emit_all(app, "recording-state", "recording");
            output::play_start_sound(&config.start_sound);
        }
        Err(e) => {
            eprintln!("Failed to start recording: {}", e);
        }
    }
}

fn handle_stop(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
) {
    let recording_state = app.state::<RecordingState>();
    let mut guard = recording_state.0.lock().unwrap();
    if let Some(stream) = guard.take() {
        let use_cloud = config.transcription_mode == "cloud";
        let has_vad = recorder::has_vad(&stream);

        if has_vad {
            // ── VAD path: segments already transcribed in the background.
            emit_all(app, "recording-state", "transcribing");

            let app = app.clone();
            let config = config.clone();
            let global_dict = app
                .state::<GlobalDictionaryState>()
                .0
                .lock()
                .unwrap()
                .clone();
            let hwnd = *app.state::<FocusedWindowState>().0.lock().unwrap();

            std::thread::spawn(move || {
                match recorder::stop_vad_recording(stream) {
                    Ok(raw_text) => {
                        finalize_and_insert(&app, &config, hwnd, raw_text, &global_dict);
                    }
                    Err(e) => {
                        eprintln!("VAD recording stop failed: {}", e);
                        emit_all(&app, "transcription-error", e);
                        emit_all(&app, "recording-state", "idle");
                    }
                }
            });
        } else {
            // ── Legacy single-shot path (cloud or offline without VAD)
            let audio_result: Result<AudioData, String> = if use_cloud {
                recorder::stop_and_save(stream)
                    .map(|path| AudioData::WavFile(path.to_string_lossy().to_string()))
            } else {
                recorder::stop_and_get_raw_samples(stream)
                    .map(|(samples, rate, channels)| AudioData::RawSamples {
                        samples,
                        sample_rate: rate,
                        channels,
                    })
            };

            match audio_result {
                Ok(audio_data) => {
                    emit_all(app, "recording-state", "transcribing");

                    let hwnd = *app
                        .state::<FocusedWindowState>()
                        .0
                        .lock()
                        .unwrap();

                    let app = app.clone();
                    let config = config.clone();
                    let global_dict = app
                        .state::<GlobalDictionaryState>()
                        .0
                        .lock()
                        .unwrap()
                        .clone();

                    std::thread::spawn(move || {
                        transcribe_and_insert(&app, &config, hwnd, audio_data, &global_dict);
                    });
                }
                Err(e) => {
                    eprintln!("Failed to capture recording: {}", e);
                    emit_all(app, "transcription-error", e.to_string());
                    emit_all(app, "recording-state", "idle");
                }
            }
        }
    } else {
        emit_all(app, "recording-state", "idle");
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve the VAD ONNX model path.
fn resolve_vad_model_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let dev_paths = [
        cwd.join("resources/silero_vad_v5.onnx"),
        cwd.join("../resources/silero_vad_v5.onnx"),
        cwd.join("src-tauri/resources/silero_vad_v5.onnx"),
    ];
    for p in &dev_paths {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Cannot resolve resource dir: {e}"))?;

    let prod_paths = [
        resource_dir.join("silero_vad_v5.onnx"),
        resource_dir.join("resources/silero_vad_v5.onnx"),
    ];
    for p in &prod_paths {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    Err("Silero VAD model not found (silero_vad_v5.onnx)".to_string())
}

enum AudioData {
    WavFile(String),
    RawSamples {
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    },
}

/// Finalize text: apply replacements, post-process, insert.
/// Shared by both VAD and legacy paths.
fn finalize_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    raw_text: String,
    global_dict: &speech::vocabulary::GlobalDictionary,
) {
    if raw_text.trim().is_empty() {
        eprintln!("[capture] Transcription produced empty text");
        output::play_done_sound(&config.stop_sound);
        emit_all(app, "recording-state", "done");
        return;
    }

    let raw_text = speech::vocabulary::apply_replacements(&raw_text, &global_dict.replacements);

    // Text cleanup: fillers, stutters, punctuation (can be disabled in settings)
    let raw_text = if config.text_cleanup_enabled {
        speech::cleanup::clean_text(&raw_text)
    } else {
        raw_text
    };

    let final_text = if config.post_processing_enabled && !raw_text.is_empty() {
        let profiles_dir = speech::get_profiles_dir(app).unwrap_or_default();
        let profiles = speech::list_profiles(&profiles_dir).unwrap_or_default();

        let profile = profiles
            .iter()
            .find(|p| p.id == config.active_profile_id)
            .cloned();

        if let Some(profile) = profile {
            match speech::post_process_text(
                &raw_text,
                &profile,
                &config.llm_provider,
                &config.llm_model,
                &config.llm_api_key,
                &config.llm_base_url,
            ) {
                Ok(processed) => processed,
                Err(e) => {
                    eprintln!("Post-processing failed, using raw: {}", e);
                    emit_all(
                        app,
                        "transcription-error",
                        format!("Post-processing failed: {}", e),
                    );
                    raw_text
                }
            }
        } else {
            raw_text
        }
    } else {
        raw_text
    };

    let final_text = speech::vocabulary::apply_replacements(&final_text, &global_dict.replacements);

    if !final_text.is_empty() {
        let text_to_insert = format!("{} ", final_text.trim());
        if let Err(e) = output::insert_text(&text_to_insert, hwnd) {
            eprintln!("Text insertion error: {}", e);
            emit_all(app, "transcription-error", e);
        }
    }
    output::play_done_sound(&config.stop_sound);
    emit_all(app, "recording-state", "done");
}

/// Background transcription pipeline (legacy non-VAD path).
fn transcribe_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    audio_data: AudioData,
    global_dict: &speech::vocabulary::GlobalDictionary,
) {
    let transcribe_result = match audio_data {
        AudioData::WavFile(wav_path) => speech::cloud_transcribe(
            &wav_path,
            &config.cloud_stt_provider,
            &config.cloud_stt_api_key,
            &config.language,
        ),
        AudioData::RawSamples {
            samples,
            sample_rate,
            channels,
        } => {
            // Safety cap: limit to ~5 minutes of audio to prevent runaway inference
            let max_samples = 5 * 60 * sample_rate as usize * channels as usize;
            let capped = if samples.len() > max_samples {
                &samples[..max_samples]
            } else {
                &samples
            };
            let inference_state = app.state::<InferenceState>();
            speech::transcribe_audio(&inference_state, capped, sample_rate, channels)
        }
    };

    match transcribe_result {
        Ok(raw_text) => {
            finalize_and_insert(app, config, hwnd, raw_text, global_dict);
        }
        Err(e) => {
            eprintln!("Transcription error: {}", e);
            emit_all(app, "transcription-error", e);
            emit_all(app, "recording-state", "idle");
        }
    }
}
