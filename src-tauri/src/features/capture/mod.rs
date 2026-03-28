pub mod hotkey;
pub mod recorder;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
use crate::features::speech::language::{self, LanguageFamily, LanguageLockConfidence};
use crate::features::speech::vocabulary::RuntimeDictionary;

use self::recorder::{RecordingState, VadConfig};

/// Cached RuntimeDictionary to avoid re-reading JSON files from disk on every
/// transcription stop. Invalidated whenever the user edits vocabulary.
pub struct RuntimeDictionaryCache(pub Mutex<Option<RuntimeDictionary>>);

/// Holds the rdev key name of the style shortcut pressed during dictation.
/// Set by the hotkey listener on style-key press, read + cleared by handle_stop.
pub struct ActiveStyleKey(pub Mutex<Option<String>>);

/// Generation counter for continuous recording.
/// Incremented on each `handle_start` when continuous mode is active.
/// Set to 0 on explicit user stop or cancel.  `finalize_and_insert` captures
/// the generation at pipeline start and only auto-restarts if it still matches.
pub struct ContinuousGeneration(pub Arc<AtomicU64>);

const DEFAULT_COMMAND_SYSTEM_PROMPT: &str =
    "You are a voice command assistant. The user speaks a command and you produce \
     the exact text they want inserted. Do not explain, do not add commentary. \
     Output only the requested text.";
/// Set up the hotkey-action event listener that orchestrates the
/// record → transcribe → insert pipeline.
pub fn setup_hotkey_handler(app: &AppHandle) {
    let app_handle = app.clone();
    app.listen("hotkey-action", move |event| {
        let action = event.payload().trim_matches('"');
        let config = match app_handle.state::<ConfigState>().0.lock() {
            Ok(g) => g.clone(),
            Err(e) => {
                eprintln!("[capture] Config mutex poisoned: {}", e);
                return;
            }
        };

        match action {
            "start" => handle_start(&app_handle, &config),
            "stop" => handle_stop(&app_handle, &config),
            "cancel" => handle_dictation_cancel(&app_handle),
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
    if let Ok(mut g) = app.state::<FocusedWindowState>().0.lock() {
        *g = hwnd;
    }

    let recording_state = app.state::<RecordingState>();
    let mut guard = match recording_state.0.lock() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[capture] RecordingState mutex poisoned: {}", e);
            return;
        }
    };
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

    // Track continuous recording generation
    if config.continuous_recording_enabled {
        app.state::<ContinuousGeneration>()
            .0
            .fetch_add(1, Ordering::SeqCst);
    }

    // Emit events BEFORE audio init so the pill reacts instantly.
    emit_all(app, "active-mode", "dictation");
    emit_all(app, "recording-state", "recording");
    if config.continuous_recording_enabled {
        emit_all(app, "continuous-active", "true");
    }
    if config.auto_pause_media_enabled {
        output::send_media_play_pause();
    }
    if config.sounds_enabled {
        output::play_start_sound(&config.start_sound);
    }

    let warm_state = app.try_state::<recorder::WarmDeviceState>();
    match recorder::start_recording(
        config.device_index,
        app.clone(),
        vad_config,
        warm_state.as_deref(),
    ) {
        Ok(stream) => {
            *guard = Some(stream);
        }
        Err(e) => {
            // Rollback: audio init failed after we already showed "recording"
            eprintln!("Failed to start recording: {}", e);
            emit_all(app, "recording-state", "idle");
            emit_all(app, "transcription-error", format!("Recording failed: {e}"));
        }
    }

    // Re-warm device for next recording (non-blocking)
    recorder::spawn_warm_device(app, config.device_index);
}

fn handle_stop(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
) {
    // Break continuous recording cycle — user explicitly pressed stop
    app.state::<ContinuousGeneration>()
        .0
        .store(0, Ordering::SeqCst);
    emit_all(app, "continuous-active", "false");

    // Read and clear the active style key (set by hotkey listener if style key was held)
    let style_key = app
        .state::<ActiveStyleKey>()
        .0
        .lock()
        .ok()
        .and_then(|mut sk| sk.take());

    // Resolve style key → profile ID by matching shortcut_key
    let style_profile_id = style_key.and_then(|key_name| {
        let profiles_dir = speech::get_profiles_dir(app).unwrap_or_default();
        let profiles = speech::list_profiles(&profiles_dir).unwrap_or_default();
        profiles
            .iter()
            .find(|p| p.shortcut_key.eq_ignore_ascii_case(&key_name))
            .map(|p| p.id.clone())
    });

    let recording_state = app.state::<RecordingState>();
    let mut guard = match recording_state.0.lock() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[capture] RecordingState mutex poisoned: {}", e);
            emit_all(app, "recording-state", "idle");
            return;
        }
    };
    if let Some(stream) = guard.take() {
        let use_cloud = config.transcription_mode == "cloud";
        let has_vad = recorder::has_vad(&stream);

        if has_vad {
            // ── VAD path: segments already transcribed in the background.
            emit_all(app, "recording-state", "transcribing");

            let app = app.clone();
            let config = config.clone();
            let hwnd = app.state::<FocusedWindowState>().0.lock().map(|g| *g).unwrap_or(0);
            let style_id = style_profile_id.clone();

            std::thread::spawn(move || {
                match recorder::stop_vad_recording(stream) {
                    Ok(recording) => {
                        let runtime_dict = resolve_runtime_dictionary(&app, &config);
                        let preview_analysis =
                            language::analyze_preview_segments(&recording.transcript.raw_segments);
                        let preview_segment_count = preview_analysis.non_empty_segments;

                        finalize_and_insert(
                            &app,
                            &config,
                            hwnd,
                            TranscriptPipelineInput {
                                raw_segments: recording.transcript.raw_segments,
                                joined_text: recording.transcript.joined_text,
                                stt_provider: "parakeet-tdt".to_string(),
                                utterance_duration_ms: recording.utterance_duration_ms,
                                preview_segment_count,
                                preview_language_family: preview_analysis.family,
                                preview_language_lock_confidence: preview_analysis.confidence,
                                mixed_script_detected: preview_analysis.mixed_script_detected,
                                final_pass_used: false,
                                final_pass_reason: "disabled".to_string(),
                                final_pass_latency_ms: None,
                            },
                            &runtime_dict,
                            style_id,
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

                    let hwnd = app
                        .state::<FocusedWindowState>()
                        .0
                        .lock()
                        .map(|g| *g)
                        .unwrap_or(0);

                    let app = app.clone();
                    let config = config.clone();
                    let style_id = style_profile_id.clone();

                    std::thread::spawn(move || {
                        let runtime_dict = resolve_runtime_dictionary(&app, &config);
                        transcribe_and_insert(&app, &config, hwnd, audio_data, &runtime_dict, style_id);
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
    utterance_duration_ms: u64,
    preview_segment_count: usize,
    preview_language_family: LanguageFamily,
    preview_language_lock_confidence: LanguageLockConfidence,
    mixed_script_detected: bool,
    final_pass_used: bool,
    final_pass_reason: String,
    final_pass_latency_ms: Option<u64>,
}

/// Finalize text: apply replacements, post-process, insert.
/// Shared by both VAD and legacy paths.
/// `style_profile_id` — if Some, applies that style's LLM post-processing.
/// If None, no LLM is used (plain dictation).
fn finalize_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    transcript: TranscriptPipelineInput,
    runtime_dict: &RuntimeDictionary,
    style_profile_id: Option<String>,
) {
    // Capture continuous recording generation at pipeline start
    let continuous_gen = app
        .state::<ContinuousGeneration>()
        .0
        .load(Ordering::SeqCst);

    if transcript.raw_segments.is_empty() && transcript.joined_text.trim().is_empty() {
        eprintln!("[capture] Transcription produced empty text");
        if config.sounds_enabled {
            output::play_done_sound(&config.stop_sound);
        }
        emit_all(app, "recording-state", "done");
        return;
    }

    let normalized_text = speech::vocabulary::apply_normalization_rules(
        &transcript.joined_text,
        &runtime_dict.normalization_rules,
    );
    let cleanup_language = effective_cleanup_language(&transcript, &normalized_text);
    let allow_english_shaped_transforms = !matches!(cleanup_language, LanguageFamily::Cyrillic);

    // Spoken punctuation: "period" → ".", "comma" → "," etc.
    // Runs BEFORE cleanup so inserted periods trigger sentence capitalization.
    let normalized_text = if config.spoken_punctuation_enabled && allow_english_shaped_transforms {
        speech::cleanup::replace_spoken_punctuation(&normalized_text)
    } else {
        normalized_text
    };

    // Final deterministic cleanup happens once after full assembly.
    let cleaned_text = if config.text_cleanup_enabled {
        speech::cleanup::clean_final_text_for_language(&normalized_text, cleanup_language)
    } else {
        normalized_text.clone()
    };

    // Hallucination filtering: remove known phantom phrases and repetition loops.
    let cleaned_text = if config.hallucination_filter_enabled {
        speech::cleanup::filter_hallucinations(&cleaned_text)
    } else {
        cleaned_text
    };

    // Number-word → digit conversion (e.g. "twenty three" → "23").
    let cleaned_text = if config.numerals_enabled && allow_english_shaped_transforms {
        speech::cleanup::convert_number_words(&cleaned_text)
    } else {
        cleaned_text
    };

    // Only run LLM post-processing if a style was active during recording
    let post_processed_text = if let Some(ref profile_id) = style_profile_id {
        if cleaned_text.is_empty() || config.command_api_key.is_empty() {
            None
        } else {
            let profiles_dir = speech::get_profiles_dir(app).unwrap_or_default();
            let profiles = speech::list_profiles(&profiles_dir).unwrap_or_default();

            let profile = profiles.iter().find(|p| p.id == *profile_id).cloned();

            if let Some(profile) = profile {
                match speech::post_process_text(
                    &cleaned_text,
                    &profile,
                    &config.command_provider,
                    &config.command_model,
                    &config.command_api_key,
                    &config.command_base_url,
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
                eprintln!("[capture] Style profile '{}' not found, skipping LLM", profile_id);
                None
            }
        }
    } else {
        None // No style active → plain dictation, no LLM
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
        if output::is_own_window(hwnd) {
            // Target is our own app — emit event so the frontend can insert into the focused input
            emit_all(app, "self-insert-text", text_to_insert.to_string());
            insert_success = true;
        } else if let Err(e) = output::insert_text(text_to_insert, hwnd) {
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

    if config.auto_pause_media_enabled {
        output::send_media_play_pause();
    }
    if config.sounds_enabled {
        output::play_done_sound(&config.stop_sound);
    }
    emit_all(app, "recording-state", "done");

    // Auto-restart if continuous recording is still active (generation unchanged)
    if config.continuous_recording_enabled && continuous_gen > 0 {
        let current = app
            .state::<ContinuousGeneration>()
            .0
            .load(Ordering::SeqCst);
        if current == continuous_gen {
            let app_clone = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(300));
                emit_all(&app_clone, "hotkey-action", "start");
            });
        }
    }
}

/// Background transcription pipeline (legacy non-VAD path).
fn transcribe_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    audio_data: AudioData,
    runtime_dict: &RuntimeDictionary,
    style_profile_id: Option<String>,
) {
    let utterance_duration_ms = match &audio_data {
        AudioData::WavFile(_) => 0,
        AudioData::RawSamples {
            samples,
            sample_rate,
            channels,
        } => {
            let channel_count = u64::from((*channels).max(1));
            let sample_rate = u64::from((*sample_rate).max(1));
            let frames = samples.len() as u64 / channel_count;
            (frames * 1000) / sample_rate
        }
    };

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
            let raw_segments = vec![raw_text.clone()];
            let preview_analysis = language::analyze_preview_segments(&raw_segments);
            finalize_and_insert(
                app,
                config,
                hwnd,
                TranscriptPipelineInput {
                    raw_segments,
                    joined_text: raw_text,
                    stt_provider: resolve_stt_provider(config),
                    utterance_duration_ms,
                    preview_segment_count: preview_analysis.non_empty_segments,
                    preview_language_family: preview_analysis.family,
                    preview_language_lock_confidence: preview_analysis.confidence,
                    mixed_script_detected: preview_analysis.mixed_script_detected,
                    final_pass_used: false,
                    final_pass_reason: "single-pass".to_string(),
                    final_pass_latency_ms: None,
                },
                runtime_dict,
                style_profile_id,
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
) -> RuntimeDictionary {
    // Check cache first — vocabulary only changes when the user explicitly edits it,
    // at which point the cache is invalidated.
    let cache = app.state::<RuntimeDictionaryCache>();
    if let Ok(guard) = cache.0.lock() {
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
    }

    let general_vocab = match speech::vocabulary::load_general_vocabulary(app) {
        Ok(vocab) => vocab,
        Err(err) => {
            eprintln!(
                "[capture] Failed to load general vocabulary: {}. Using empty vocabulary.",
                err
            );
            speech::vocabulary::IndustryPack {
                id: "general".to_string(),
                name: "General Vocabulary".to_string(),
                description: String::new(),
                vocabulary: Vec::new(),
                replacements: Vec::new(),
            }
        }
    };

    let runtime_dict = match speech::vocabulary::resolve_runtime_dictionary_for_pack(
        app,
        &general_vocab,
        &config.active_industry_pack,
    ) {
        Ok(runtime_dict) => runtime_dict,
        Err(err) => {
            eprintln!(
                "[capture] Failed to resolve industry pack '{}': {}. Falling back to general vocabulary only.",
                config.active_industry_pack, err
            );
            speech::vocabulary::resolve_runtime_dictionary(&general_vocab, None)
        }
    };

    // Cache the result
    if let Ok(mut guard) = cache.0.lock() {
        *guard = Some(runtime_dict.clone());
    }

    runtime_dict
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
        pipeline_mode: "dictation".to_string(),
        transcription_mode: config.transcription_mode.clone(),
        stt_provider: transcript.stt_provider.clone(),
        active_industry_pack: config.active_industry_pack.clone(),
        active_profile_id: config.active_profile_id.clone(),
        cleanup_enabled: config.text_cleanup_enabled,
        post_processing_enabled: config.post_processing_enabled,
        vad_silence_threshold_ms: config.vad_silence_threshold_ms,
        utterance_duration_ms: transcript.utterance_duration_ms,
        preview_segment_count: transcript.preview_segment_count as u32,
        final_pass_used: transcript.final_pass_used,
        final_pass_reason: transcript.final_pass_reason.clone(),
        final_pass_latency_ms: transcript.final_pass_latency_ms.map(|ms| ms as i64),
        language_family: transcript.preview_language_family.as_str().to_string(),
        language_lock_confidence: transcript
            .preview_language_lock_confidence
            .as_str()
            .to_string(),
        mixed_script_detected: transcript.mixed_script_detected,
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

fn effective_cleanup_language(
    transcript: &TranscriptPipelineInput,
    normalized_text: &str,
) -> LanguageFamily {
    if transcript.preview_language_lock_confidence == LanguageLockConfidence::High
        && transcript.preview_language_family != LanguageFamily::Unknown
    {
        transcript.preview_language_family
    } else {
        language::detect_language_family(normalized_text)
    }
}

// ── Style switching ──────────────────────────────────────────────────────────

/// Set up the style-switch event listener (dictation key + any key).
pub fn setup_style_switch_handler(app: &AppHandle) {
    let app_handle = app.clone();
    app.listen("style-switch", move |event| {
        let key_name = event.payload().trim_matches('"').to_string();
        if key_name.is_empty() {
            return;
        }

        // Find the profile with this shortcut_key
        let profiles_dir = match speech::get_profiles_dir(&app_handle) {
            Ok(dir) => dir,
            Err(_) => return,
        };
        let profiles = match speech::list_profiles(&profiles_dir) {
            Ok(p) => p,
            Err(_) => return,
        };

        if let Some(profile) = profiles.iter().find(|p| p.shortcut_key == key_name) {
            let config_state = app_handle.state::<ConfigState>();
            if let Ok(mut config) = config_state.0.lock() {
                config.active_profile_id = profile.id.clone();
                config.post_processing_enabled = true;
                let _ = crate::features::settings::save_config(&app_handle, &config);
            }
            eprintln!("[capture] Style switched to '{}' (key: {})", profile.name, key_name);
            emit_all(&app_handle, "style-switched", &profile.name);
        } else {
            eprintln!("[capture] No profile with shortcut_key '{}'", key_name);
        }
    });
}

// ── Command mode pipeline ────────────────────────────────────────────────────

/// Set up the command-hotkey-action event listener for the command pipeline.
pub fn setup_command_hotkey_handler(app: &AppHandle) {
    let app_handle = app.clone();
    app.listen("command-hotkey-action", move |event| {
        let action = event.payload().trim_matches('"');
        let config = match app_handle.state::<ConfigState>().0.lock() {
            Ok(g) => g.clone(),
            Err(e) => {
                eprintln!("[capture] Config mutex poisoned: {}", e);
                return;
            }
        };

        match action {
            "start" => handle_command_start(&app_handle, &config),
            "stop" => handle_command_stop(&app_handle, &config),
            "cancel" => handle_command_cancel(&app_handle),
            _ => {}
        }
    });
}

fn handle_dictation_cancel(app: &AppHandle) {
    // Break continuous recording cycle
    app.state::<ContinuousGeneration>()
        .0
        .store(0, Ordering::SeqCst);
    emit_all(app, "continuous-active", "false");

    // Silently discard any in-progress dictation recording (e.g., style switch)
    let recording_state = app.state::<RecordingState>();
    if let Ok(mut guard) = recording_state.0.lock() {
        if guard.take().is_some() {
            eprintln!("[capture] Dictation recording cancelled (style switch)");
        }
    }
    emit_all(app, "recording-state", "idle");
}

fn handle_command_cancel(app: &AppHandle) {
    // Silently discard any in-progress command recording
    let recording_state = app.state::<RecordingState>();
    if let Ok(mut guard) = recording_state.0.lock() {
        if guard.take().is_some() {
            eprintln!("[capture] Command recording cancelled (style switch or short press)");
        }
    }
    emit_all(app, "recording-state", "idle");
}

fn handle_command_start(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
) {
    // Capture the foreground window before recording
    let hwnd = output::capture_foreground_window();
    if let Ok(mut g) = app.state::<FocusedWindowState>().0.lock() {
        *g = hwnd;
    }

    let recording_state = app.state::<RecordingState>();
    let mut guard = match recording_state.0.lock() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[capture] RecordingState mutex poisoned: {}", e);
            return;
        }
    };

    // If already recording (dictation in progress), ignore
    if guard.is_some() {
        return;
    }

    // Emit events BEFORE audio init so the pill reacts instantly.
    emit_all(app, "active-mode", "command");
    emit_all(app, "recording-state", "recording");
    if config.sounds_enabled {
        output::play_start_sound(&config.start_sound);
    }

    // Command mode: always record without VAD (commands are short utterances)
    let warm_state = app.try_state::<recorder::WarmDeviceState>();
    match recorder::start_recording(config.device_index, app.clone(), None, warm_state.as_deref()) {
        Ok(stream) => {
            *guard = Some(stream);
        }
        Err(e) => {
            eprintln!("[capture] Failed to start command recording: {}", e);
            emit_all(app, "recording-state", "idle");
            emit_all(app, "transcription-error", format!("Recording failed: {e}"));
        }
    }

    recorder::spawn_warm_device(app, config.device_index);
}

fn handle_command_stop(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
) {
    let recording_state = app.state::<RecordingState>();
    let mut guard = match recording_state.0.lock() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[capture] RecordingState mutex poisoned: {}", e);
            emit_all(app, "recording-state", "idle");
            return;
        }
    };

    if let Some(stream) = guard.take() {
        emit_all(app, "recording-state", "transcribing");

        let use_cloud = config.transcription_mode == "cloud";
        let app = app.clone();
        let config = config.clone();

        std::thread::spawn(move || {
            // Step 1: Get audio data and transcribe
            let transcribe_result = if use_cloud {
                recorder::stop_and_save(stream).and_then(|path| {
                    speech::cloud_transcribe(
                        &path.to_string_lossy(),
                        &config.cloud_stt_provider,
                        &config.cloud_stt_api_key,
                        &config.language,
                    )
                })
            } else {
                recorder::stop_and_get_raw_samples(stream).and_then(
                    |(samples, sample_rate, channels)| {
                        let max_samples = 60 * sample_rate as usize * channels as usize;
                        let capped = if samples.len() > max_samples {
                            &samples[..max_samples]
                        } else {
                            &samples
                        };
                        let inference_state = app.state::<InferenceState>();
                        speech::transcribe_audio(&inference_state, capped, sample_rate, channels)
                    },
                )
            };

            match transcribe_result {
                Ok(transcript) => {
                    let hwnd = app.state::<FocusedWindowState>().0.lock().map(|g| *g).unwrap_or(0);
                    command_finalize(&app, &config, hwnd, transcript);
                }
                Err(e) => {
                    eprintln!("[capture] Command transcription error: {}", e);
                    emit_all(&app, "transcription-error", e);
                    emit_all(&app, "recording-state", "idle");
                }
            }
        });
    } else {
        emit_all(app, "recording-state", "idle");
    }
}

/// Finalize a command: send transcript to command LLM and paste the result.
/// If vision is enabled, runs intent classification first, then captures
/// a screenshot only when the command references on-screen content.
fn command_finalize(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    raw_transcript: String,
) {
    if raw_transcript.trim().is_empty() {
        eprintln!("[capture] Command transcription produced empty text");
        if config.sounds_enabled {
            output::play_done_sound(&config.stop_sound);
        }
        emit_all(app, "recording-state", "done");
        return;
    }

    let transcript = raw_transcript.trim();
    eprintln!("[capture] Command transcript: {}", transcript);

    // Check if this is a vocabulary addition command
    if let Some(vocab_cmd) = speech::llm::detect_vocab_command(transcript, &config.command_api_key)
    {
        eprintln!(
            "[capture] Vocab command detected: term='{}', full_form={:?}",
            vocab_cmd.term, vocab_cmd.full_form
        );
        match add_term_to_general_vocabulary(app, &vocab_cmd, &config.command_api_key) {
            Ok(_) => {
                let msg = format!("Added: {}", vocab_cmd.term);
                emit_all(app, "vocab-added", &msg);
            }
            Err(e) => {
                eprintln!("[capture] Failed to add vocab term: {}", e);
                emit_all(app, "transcription-error", format!("Vocab add failed: {}", e));
            }
        }
        if config.sounds_enabled {
            output::play_done_sound(&config.stop_sound);
        }
        emit_all(app, "recording-state", "done");
        return;
    }

    let system_prompt = if config.command_system_prompt.trim().is_empty() {
        DEFAULT_COMMAND_SYSTEM_PROMPT
    } else {
        config.command_system_prompt.trim()
    };

    let result = text_only_command(transcript, system_prompt, config);

    match result {
        Ok(text) => {
            if !text.is_empty() {
                if let Err(e) = output::insert_text(&text, hwnd) {
                    eprintln!("[capture] Command text insertion error: {}", e);
                    emit_all(app, "transcription-error", e);
                }
            }
        }
        Err(e) => {
            eprintln!("[capture] Command error: {}", e);
            emit_all(
                app,
                "transcription-error",
                format!("Command error: {}", e),
            );
        }
    }

    if config.sounds_enabled {
        output::play_done_sound(&config.stop_sound);
    }
    emit_all(app, "recording-state", "done");
}

fn text_only_command(
    transcript: &str,
    system_prompt: &str,
    config: &crate::features::settings::AppConfig,
) -> Result<String, String> {
    speech::command_llm_call(
        transcript,
        system_prompt,
        &config.command_provider,
        &config.command_model,
        &config.command_api_key,
        &config.command_base_url,
    )
}

/// Add a term to the general vocabulary with AI-generated misspelling variants.
fn add_term_to_general_vocabulary(
    app: &AppHandle,
    vocab_cmd: &speech::llm::VocabCommand,
    api_key: &str,
) -> Result<(), String> {
    let mut general = speech::vocabulary::load_general_vocabulary(app)?;

    // Add term to vocabulary list if not already present
    let term = vocab_cmd.term.trim().to_string();
    if !general.vocabulary.iter().any(|v| v.eq_ignore_ascii_case(&term)) {
        general.vocabulary.push(term.clone());
    }

    // Generate misspelling variants
    let variants = speech::llm::generate_misspelling_variants(&term, api_key).unwrap_or_default();

    // Add generated variants as replacement rules (skip conflicts)
    for variant in &variants {
        let find = variant.to_lowercase();
        if !general
            .replacements
            .iter()
            .any(|r| r.find.eq_ignore_ascii_case(&find))
        {
            general
                .replacements
                .push(speech::vocabulary::ReplacementRule {
                    find,
                    replace: term.clone(),
                });
        }
    }

    // If full_form provided, add that as a replacement too
    if let Some(ref full_form) = vocab_cmd.full_form {
        let find = full_form.to_lowercase();
        if !general
            .replacements
            .iter()
            .any(|r| r.find.eq_ignore_ascii_case(&find))
        {
            general
                .replacements
                .push(speech::vocabulary::ReplacementRule {
                    find,
                    replace: term.clone(),
                });
        }
    }

    speech::vocabulary::save_general_vocabulary(app, &general)?;
    speech::vocabulary::invalidate_regex_cache();
    if let Ok(mut guard) = app.state::<RuntimeDictionaryCache>().0.lock() {
        *guard = None;
    }
    eprintln!(
        "[capture] Added '{}' to general vocabulary with {} variant rules",
        term,
        variants.len()
    );
    Ok(())
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
            utterance_duration_ms: 1_500,
            preview_segment_count: 1,
            preview_language_family: LanguageFamily::Latin,
            preview_language_lock_confidence: LanguageLockConfidence::High,
            mixed_script_detected: false,
            final_pass_used: false,
            final_pass_reason: "single-pass".to_string(),
            final_pass_latency_ms: None,
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
            utterance_duration_ms: 4_000,
            preview_segment_count: 2,
            preview_language_family: LanguageFamily::Latin,
            preview_language_lock_confidence: LanguageLockConfidence::High,
            mixed_script_detected: false,
            final_pass_used: false,
            final_pass_reason: "preview-only".to_string(),
            final_pass_latency_ms: None,
        };

        let raw_segments_json = serde_json::to_string(&input.raw_segments).unwrap();
        assert_eq!(raw_segments_json, "[\"open the\",\"settings page\"]");
        assert_eq!(input.joined_text, "open the settings page");
    }

}
