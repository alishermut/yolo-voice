pub mod hotkey;
pub mod recorder;

use serde_json;
use tauri::{AppHandle, Listener, Manager};

use crate::app::events::emit_all;
use crate::features::diagnostics::{
    current_timestamp_ms, TranscriptDiagnosticsState, TranscriptSample,
};
use crate::features::output::{self, FocusedWindowState};
use crate::features::settings::ConfigState;
use crate::features::speech;
use crate::features::speech::inference::InferenceState;
use crate::features::speech::vocabulary::{RuntimeDictionary, UserDictionaryState};

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
            let user_dict = app
                .state::<UserDictionaryState>()
                .0
                .lock()
                .unwrap()
                .clone();
            let hwnd = *app.state::<FocusedWindowState>().0.lock().unwrap();

            std::thread::spawn(move || {
                match recorder::stop_vad_recording(stream) {
                    Ok(transcript) => {
                        let runtime_dict = resolve_runtime_dictionary(&app, &config, &user_dict);
                        finalize_and_insert(
                            &app,
                            &config,
                            hwnd,
                            TranscriptPipelineInput {
                                raw_segments: transcript.raw_segments,
                                joined_text: transcript.joined_text,
                                stt_provider: "parakeet-tdt".to_string(),
                            },
                            &runtime_dict,
                        );
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
                    let user_dict = app
                        .state::<UserDictionaryState>()
                        .0
                        .lock()
                        .unwrap()
                        .clone();

                    std::thread::spawn(move || {
                        let runtime_dict = resolve_runtime_dictionary(&app, &config, &user_dict);
                        transcribe_and_insert(&app, &config, hwnd, audio_data, &runtime_dict);
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

#[derive(Debug, Clone)]
struct TranscriptPipelineInput {
    raw_segments: Vec<String>,
    joined_text: String,
    stt_provider: String,
}

/// Finalize text: apply replacements, post-process, insert.
/// Shared by both VAD and legacy paths.
fn finalize_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    transcript: TranscriptPipelineInput,
    runtime_dict: &RuntimeDictionary,
) {
    if transcript.raw_segments.is_empty() && transcript.joined_text.trim().is_empty() {
        eprintln!("[capture] Transcription produced empty text");
        output::play_done_sound(&config.stop_sound);
        emit_all(app, "recording-state", "done");
        return;
    }

    let normalized_text = speech::vocabulary::apply_normalization_rules(
        &transcript.joined_text,
        &runtime_dict.normalization_rules,
    );

    // Final deterministic cleanup happens once after full assembly.
    let cleaned_text = if config.text_cleanup_enabled {
        speech::cleanup::clean_final_text(&normalized_text)
    } else {
        normalized_text.clone()
    };

    let post_processed_text = if config.post_processing_enabled && !cleaned_text.is_empty() {
        let profiles_dir = speech::get_profiles_dir(app).unwrap_or_default();
        let profiles = speech::list_profiles(&profiles_dir).unwrap_or_default();

        let profile = profiles
            .iter()
            .find(|p| p.id == config.active_profile_id)
            .cloned();

        if let Some(profile) = profile {
            match speech::post_process_text(
                &cleaned_text,
                &profile,
                &config.llm_provider,
                &config.llm_model,
                &config.llm_api_key,
                &config.llm_base_url,
            ) {
                Ok(processed) => Some(processed),
                Err(e) => {
                    eprintln!("Post-processing failed, using cleaned text: {}", e);
                    emit_all(
                        app,
                        "transcription-error",
                        format!("Post-processing failed: {}", e),
                    );
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let pre_final_text = post_processed_text
        .as_deref()
        .unwrap_or(cleaned_text.as_str());

    let final_text = speech::vocabulary::apply_normalization_rules(
        pre_final_text,
        &runtime_dict.normalization_rules,
    );

    let inserted_text = if final_text.is_empty() {
        None
    } else {
        Some(format!("{} ", final_text.trim()))
    };

    let mut insert_success = false;
    if let Some(text_to_insert) = inserted_text.as_deref() {
        if let Err(e) = output::insert_text(text_to_insert, hwnd) {
            eprintln!("Text insertion error: {}", e);
            emit_all(app, "transcription-error", e);
        } else {
            insert_success = true;
        }
    }

    maybe_log_transcript_sample(
        app,
        config,
        &transcript,
        option_if_not_empty(&normalized_text),
        if config.text_cleanup_enabled {
            option_if_not_empty(&cleaned_text)
        } else {
            None
        },
        post_processed_text
            .as_deref()
            .and_then(option_if_not_empty),
        option_if_not_empty(&final_text),
        inserted_text,
        insert_success,
    );

    output::play_done_sound(&config.stop_sound);
    emit_all(app, "recording-state", "done");
}

/// Background transcription pipeline (legacy non-VAD path).
fn transcribe_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    audio_data: AudioData,
    runtime_dict: &RuntimeDictionary,
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
            finalize_and_insert(
                app,
                config,
                hwnd,
                TranscriptPipelineInput {
                    raw_segments: vec![raw_text.clone()],
                    joined_text: raw_text,
                    stt_provider: resolve_stt_provider(config),
                },
                runtime_dict,
            );
        }
        Err(e) => {
            eprintln!("Transcription error: {}", e);
            emit_all(app, "transcription-error", e);
            emit_all(app, "recording-state", "idle");
        }
    }
}

fn resolve_runtime_dictionary(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    user_dict: &speech::vocabulary::UserDictionary,
) -> RuntimeDictionary {
    match speech::vocabulary::resolve_runtime_dictionary_for_pack(
        app,
        user_dict,
        &config.active_industry_pack,
    ) {
        Ok(runtime_dict) => runtime_dict,
        Err(err) => {
            eprintln!(
                "[capture] Failed to resolve industry pack '{}': {}. Falling back to personal dictionary only.",
                config.active_industry_pack, err
            );
            speech::vocabulary::runtime_dictionary_from_user_dictionary(user_dict)
        }
    }
}

fn resolve_stt_provider(config: &crate::features::settings::AppConfig) -> String {
    if config.transcription_mode == "cloud" {
        config.cloud_stt_provider.clone()
    } else {
        "parakeet-tdt".to_string()
    }
}

fn maybe_log_transcript_sample(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    transcript: &TranscriptPipelineInput,
    normalized_text: Option<String>,
    cleaned_text: Option<String>,
    post_processed_text: Option<String>,
    final_text: Option<String>,
    inserted_text: Option<String>,
    insert_success: bool,
) {
    if !config.transcript_diagnostics_enabled {
        return;
    }

    let diagnostics_state = app.state::<TranscriptDiagnosticsState>();
    let raw_segments_json = match serde_json::to_string(&transcript.raw_segments) {
        Ok(json) => json,
        Err(err) => {
            eprintln!(
                "[capture] Failed to serialize transcript diagnostics segments: {}",
                err
            );
            return;
        }
    };

    diagnostics_state.0.log_sample(TranscriptSample {
        created_at: current_timestamp_ms(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        session_id: diagnostics_state.0.session_id().to_string(),
        utterance_id: diagnostics_state.0.next_utterance_id(),
        transcription_mode: config.transcription_mode.clone(),
        stt_provider: transcript.stt_provider.clone(),
        active_industry_pack: config.active_industry_pack.clone(),
        active_profile_id: config.active_profile_id.clone(),
        cleanup_enabled: config.text_cleanup_enabled,
        post_processing_enabled: config.post_processing_enabled,
        vad_silence_threshold_ms: config.vad_silence_threshold_ms,
        raw_segments_json,
        joined_text: option_if_not_empty(&transcript.joined_text),
        normalized_text,
        cleaned_text,
        post_processed_text,
        final_text,
        inserted_text,
        insert_success,
    });
}

fn option_if_not_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::settings::AppConfig;

    #[test]
    fn resolves_cloud_and_offline_stt_provider() {
        let mut config = AppConfig::default();
        assert_eq!(resolve_stt_provider(&config), "parakeet-tdt");

        config.transcription_mode = "cloud".to_string();
        config.cloud_stt_provider = "deepgram".to_string();
        assert_eq!(resolve_stt_provider(&config), "deepgram");
    }

    #[test]
    fn option_if_not_empty_trims_blank_strings() {
        assert_eq!(option_if_not_empty(""), None);
        assert_eq!(option_if_not_empty("   "), None);
        assert_eq!(option_if_not_empty(" hello "), Some("hello".to_string()));
    }

    #[test]
    fn legacy_pipeline_input_keeps_single_raw_segment() {
        let input = TranscriptPipelineInput {
            raw_segments: vec!["deploy to staging".to_string()],
            joined_text: "deploy to staging".to_string(),
            stt_provider: "parakeet-tdt".to_string(),
        };

        let raw_segments_json = serde_json::to_string(&input.raw_segments).unwrap();
        assert_eq!(raw_segments_json, "[\"deploy to staging\"]");
        assert_eq!(input.joined_text, "deploy to staging");
    }

    #[test]
    fn vad_pipeline_input_keeps_multiple_raw_segments() {
        let input = TranscriptPipelineInput {
            raw_segments: vec!["open the".to_string(), "settings page".to_string()],
            joined_text: "open the settings page".to_string(),
            stt_provider: "parakeet-tdt".to_string(),
        };

        let raw_segments_json = serde_json::to_string(&input.raw_segments).unwrap();
        assert_eq!(raw_segments_json, "[\"open the\",\"settings page\"]");
        assert_eq!(input.joined_text, "open the settings page");
    }
}
