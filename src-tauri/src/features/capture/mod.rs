pub mod hotkey;
pub mod recorder;

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json;
use tauri::{AppHandle, Listener, Manager};

use crate::app::events::emit_all;
use crate::features::diagnostics::{
    current_timestamp_ms, maybe_log_support_event, DistilWhisperDiagnosticsEvent,
    ParakeetDiagnosticsEvent, TranscriptDiagnosticsState, TranscriptSample,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyRecordingMode {
    None = 0,
    Dictation = 1,
    Command = 2,
}

impl HotkeyRecordingMode {
    fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Dictation,
            2 => Self::Command,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictationRuntimePhase {
    Idle = 0,
    Listening = 1,
    Recording = 2,
}

impl DictationRuntimePhase {
    fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Listening,
            2 => Self::Recording,
            _ => Self::Idle,
        }
    }
}

#[derive(Clone)]
pub struct HotkeyRuntimeState {
    pub recording_mode: Arc<AtomicU8>,
    pub dictation_phase: Arc<AtomicU8>,
    pub dictation_stop_token: Arc<AtomicU64>,
    pub reset_generation: Arc<AtomicU64>,
    /// Set when a dictation start is enqueued; cleared when handle_start finishes.
    pub dictation_start_pending: Arc<AtomicBool>,
    /// Bumped when installing / invalidating a hotkey listener instance.
    pub listener_generation: Arc<AtomicU64>,
    /// Millis (since `hook_epoch`, +1 so 0 means "never") of the last event delivered to
    /// the keyboard hook callback. Atomic rather than a mutex because this is written on
    /// the WH_KEYBOARD_LL thread for *every* input event, including mouse moves.
    pub last_hook_event_ms: Arc<AtomicU64>,
    /// Baseline for `last_hook_event_ms`.
    pub hook_epoch: Arc<Instant>,
}

impl HotkeyRuntimeState {
    pub fn new() -> Self {
        Self {
            recording_mode: Arc::new(AtomicU8::new(HotkeyRecordingMode::None as u8)),
            dictation_phase: Arc::new(AtomicU8::new(DictationRuntimePhase::Idle as u8)),
            dictation_stop_token: Arc::new(AtomicU64::new(0)),
            reset_generation: Arc::new(AtomicU64::new(0)),
            dictation_start_pending: Arc::new(AtomicBool::new(false)),
            listener_generation: Arc::new(AtomicU64::new(0)),
            last_hook_event_ms: Arc::new(AtomicU64::new(0)),
            hook_epoch: Arc::new(Instant::now()),
        }
    }

    pub fn recording_mode(&self) -> HotkeyRecordingMode {
        HotkeyRecordingMode::from_raw(self.recording_mode.load(Ordering::SeqCst))
    }

    pub fn set_recording_mode(&self, mode: HotkeyRecordingMode) {
        self.recording_mode.store(mode as u8, Ordering::SeqCst);
    }

    pub fn dictation_phase(&self) -> DictationRuntimePhase {
        DictationRuntimePhase::from_raw(self.dictation_phase.load(Ordering::SeqCst))
    }

    pub fn set_dictation_phase(&self, phase: DictationRuntimePhase) {
        self.dictation_phase.store(phase as u8, Ordering::SeqCst);
    }

    pub fn cancel_pending_dictation_stop(&self) {
        self.dictation_stop_token.fetch_add(1, Ordering::SeqCst);
    }

    pub fn dictation_start_pending(&self) -> bool {
        self.dictation_start_pending.load(Ordering::SeqCst)
    }

    pub fn set_dictation_start_pending(&self, pending: bool) {
        self.dictation_start_pending.store(pending, Ordering::SeqCst);
    }

    pub fn listener_generation(&self) -> u64 {
        self.listener_generation.load(Ordering::SeqCst)
    }

    pub fn bump_listener_generation(&self) -> u64 {
        self.listener_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Runs on the hook thread for every input event — must stay lock-free.
    pub fn mark_hook_event(&self) {
        let ms = self.hook_epoch.elapsed().as_millis() as u64;
        self.last_hook_event_ms
            .store(ms.saturating_add(1), Ordering::Relaxed);
    }

    /// Time since the hook last delivered an event, or `None` if it never has.
    pub fn last_hook_event_elapsed(&self) -> Option<Duration> {
        let raw = self.last_hook_event_ms.load(Ordering::Relaxed);
        if raw == 0 {
            return None;
        }
        let then = Duration::from_millis(raw - 1);
        Some(self.hook_epoch.elapsed().saturating_sub(then))
    }

    pub fn reset_listener_state(&self) {
        self.cancel_pending_dictation_stop();
        self.set_dictation_start_pending(false);
        self.set_dictation_phase(DictationRuntimePhase::Idle);
        self.reset_generation.fetch_add(1, Ordering::SeqCst);
    }
}

fn set_hotkey_recording_mode(app: &AppHandle, mode: HotkeyRecordingMode) {
    app.state::<HotkeyRuntimeState>().set_recording_mode(mode);
}

fn set_dictation_phase(app: &AppHandle, phase: DictationRuntimePhase) {
    app.state::<HotkeyRuntimeState>().set_dictation_phase(phase);
}

fn reset_hotkey_runtime(app: &AppHandle) {
    app.state::<HotkeyRuntimeState>().reset_listener_state();
}

fn parakeet_segmented_supported(config: &crate::features::settings::AppConfig) -> bool {
    config.transcription_mode == "offline"
        && crate::features::settings::is_parakeet_variant(config)
        && config.parakeet_segmented_mode_enabled
}

/// True when the configured offline engine is either Parakeet variant.
fn is_parakeet_variant(config: &crate::features::settings::AppConfig) -> bool {
    crate::features::settings::is_parakeet_variant(config)
}

/// True when in offline mode using either Parakeet variant. Used to decide
/// whether to emit Parakeet diagnostics events.
fn offline_parakeet_variant(config: &crate::features::settings::AppConfig) -> bool {
    config.transcription_mode == "offline" && is_parakeet_variant(config)
}

fn voice_activated_enabled(config: &crate::features::settings::AppConfig) -> bool {
    config.dictation_activation_mode == "voice_activated"
        && crate::features::settings::voice_activation_supported(config)
}

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

        maybe_log_support_event(
            &app_handle,
            "capture",
            "hotkey_action",
            format!("Received hotkey action '{action}'"),
            serde_json::json!({
                "action": action,
                "offline_engine": config.offline_engine,
                "transcription_mode": config.transcription_mode,
                "dictation_activation_mode": config.dictation_activation_mode,
                "continuous_recording_enabled": config.continuous_recording_enabled,
            }),
        );

        match action {
            "start" => handle_start(&app_handle, &config),
            "stop" => handle_stop(&app_handle, &config),
            "cancel" => handle_dictation_cancel(&app_handle),
            _ => {}
        }
    });
}

fn handle_start(app: &AppHandle, config: &crate::features::settings::AppConfig) {
    // Always clear the start-pending flag when this handler returns (success or error).
    struct ClearStartPending<'a>(&'a AppHandle);
    impl Drop for ClearStartPending<'_> {
        fn drop(&mut self) {
            self.0
                .state::<HotkeyRuntimeState>()
                .set_dictation_start_pending(false);
        }
    }
    let _clear_start_pending = ClearStartPending(app);

    let voice_activated = voice_activated_enabled(config);
    let should_use_continuous = config.continuous_recording_enabled && !voice_activated;

    maybe_log_support_event(
        app,
        "capture",
        "recording_start_requested",
        "Starting dictation recording",
        serde_json::json!({
            "pipeline_mode": "dictation",
            "device_index": config.device_index,
            "offline_engine": config.offline_engine,
            "transcription_mode": config.transcription_mode,
            "parakeet_segmented_mode_enabled": config.parakeet_segmented_mode_enabled,
            "dictation_activation_mode": config.dictation_activation_mode,
        }),
    );

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
            set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
            reset_hotkey_runtime(app);
            return;
        }
    };
    // Stop any existing recording
    *guard = None;

    // Build VAD config if the Parakeet segmented path is active and inference is ready.
    let vad_config = if parakeet_segmented_supported(config) {
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
                    emit_speech_start_event: voice_activated,
                    auto_stop_after_segment: voice_activated,
                }),
                Err(e) => {
                    if voice_activated {
                        emit_all(
                            app,
                            "transcription-error",
                            "Voice activated mode is not ready yet. Finish loading Parakeet and try again."
                                .to_string(),
                        );
                        set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
                        reset_hotkey_runtime(app);
                        emit_all(app, "recording-state", "idle");
                        return;
                    }
                    eprintln!("[capture] VAD model not found, falling back to non-VAD: {e}");
                    None
                }
            }
        } else {
            if voice_activated {
                emit_all(
                    app,
                    "transcription-error",
                    "Voice activated mode needs Parakeet ready before it can listen for speech."
                        .to_string(),
                );
                set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
                reset_hotkey_runtime(app);
                emit_all(app, "recording-state", "idle");
                return;
            }
            eprintln!("[capture] Inference not ready, falling back to non-VAD");
            None
        }
    } else {
        None
    };

    // Track continuous recording generation
    if should_use_continuous {
        app.state::<ContinuousGeneration>()
            .0
            .fetch_add(1, Ordering::SeqCst);
    }

    // Emit events BEFORE audio init so the pill reacts instantly.
    emit_all(app, "active-mode", "dictation");
    emit_all(
        app,
        "recording-state",
        if voice_activated {
            "listening"
        } else {
            "recording"
        },
    );
    set_dictation_phase(
        app,
        if voice_activated {
            DictationRuntimePhase::Listening
        } else {
            DictationRuntimePhase::Recording
        },
    );
    if should_use_continuous {
        emit_all(app, "continuous-active", "true");
    }
    if config.auto_pause_media_enabled {
        output::send_media_play_pause();
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
            set_hotkey_recording_mode(app, HotkeyRecordingMode::Dictation);
            maybe_log_support_event(
                app,
                "capture",
                "recording_start_success",
                "Dictation recording started",
                serde_json::json!({
                    "pipeline_mode": "dictation",
                    "device_index": config.device_index,
                    "vad_enabled": parakeet_segmented_supported(config),
                    "voice_activated": voice_activated,
                }),
            );
            if config.sounds_enabled {
                output::play_start_sound(&config.start_sound);
            }
        }
        Err(e) => {
            // Rollback: audio init failed after we already showed "recording"
            log::error!(target: "yolo_voice::capture", "Failed to start recording: {}", e);
            if config.transcription_mode == "offline" && config.offline_engine == "distil_whisper" {
                log_distil_whisper_event(
                    app,
                    DistilWhisperEventContext {
                        event_type: "recording_start_error",
                        pipeline_mode: "dictation",
                        input_source: "dictation",
                        utterance_id: None,
                        utterance_duration_ms: None,
                        compacted_duration_ms: None,
                        wav_bytes: None,
                        compacted_wav_bytes: None,
                        speech_region_count: None,
                        fallback_to_raw_audio: None,
                        requested_mode: None,
                        effective_mode: None,
                        device: None,
                        total_ms: None,
                        text_len: None,
                        success: false,
                        error: Some(e.clone()),
                    },
                );
            } else if offline_parakeet_variant(config) {
                log_parakeet_event(
                    app,
                    ParakeetEventContext {
                        event_type: "recording_start_error",
                        pipeline_mode: "dictation",
                        input_source: "dictation",
                        utterance_id: None,
                        utterance_duration_ms: None,
                        preview_segment_count: None,
                        raw_segment_count: None,
                        gpu_available: Some(speech::get_gpu_available(
                            &app.state::<InferenceState>(),
                        )),
                        vad_enabled: Some(false),
                        stop_ms: None,
                        transcribe_ms: None,
                        total_ms: None,
                        text_len: None,
                        success: false,
                        error: Some(e.clone()),
                    },
                );
            }
            maybe_log_support_event(
                app,
                "capture",
                "recording_start_error",
                "Failed to start dictation recording",
                serde_json::json!({
                    "pipeline_mode": "dictation",
                    "device_index": config.device_index,
                    "error": e,
                }),
            );
            set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
            reset_hotkey_runtime(app);
            set_dictation_phase(app, DictationRuntimePhase::Idle);
            emit_all(app, "recording-state", "idle");
            emit_all(app, "transcription-error", format!("Recording failed: {e}"));
        }
    }

    // Re-warm device for next recording (non-blocking)
    recorder::spawn_warm_device(app, config.device_index);
}

fn handle_stop(app: &AppHandle, config: &crate::features::settings::AppConfig) {
    maybe_log_support_event(
        app,
        "capture",
        "recording_stop_requested",
        "Stopping dictation recording",
        serde_json::json!({
            "pipeline_mode": "dictation",
            "offline_engine": config.offline_engine,
            "transcription_mode": config.transcription_mode,
        }),
    );

    // Break continuous recording cycle — user explicitly pressed stop
    app.state::<ContinuousGeneration>()
        .0
        .store(0, Ordering::SeqCst);
    emit_all(app, "continuous-active", "false");
    set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
    set_dictation_phase(app, DictationRuntimePhase::Idle);
    reset_hotkey_runtime(app);

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
            let hwnd = app
                .state::<FocusedWindowState>()
                .0
                .lock()
                .map(|g| *g)
                .unwrap_or(0);
            let style_id = style_profile_id.clone();
            let stop_started = Instant::now();

            std::thread::spawn(move || match recorder::stop_vad_recording(stream) {
                Ok(recording) => {
                    let stop_elapsed = stop_started.elapsed();
                    maybe_log_support_event(
                        &app,
                        "capture",
                        "vad_finalize_success",
                        "Completed VAD dictation finalize",
                        serde_json::json!({
                            "pipeline_mode": "dictation",
                            "utterance_duration_ms": recording.utterance_duration_ms,
                            "raw_segment_count": recording.transcript.raw_segments.len(),
                            "joined_text_len": recording.transcript.joined_text.len(),
                            "stop_ms": stop_elapsed.as_millis() as u64,
                        }),
                    );
                    let runtime_dict = resolve_runtime_dictionary(&app, &config);
                    let preview_analysis =
                        language::analyze_preview_segments(&recording.transcript.raw_segments);
                    let preview_segment_count = preview_analysis.non_empty_segments;
                    if offline_parakeet_variant(&config)
                    {
                        log_parakeet_event(
                            &app,
                            ParakeetEventContext {
                                event_type: "vad_finalize_success",
                                pipeline_mode: "dictation",
                                input_source: "dictation",
                                utterance_id: None,
                                utterance_duration_ms: Some(recording.utterance_duration_ms),
                                preview_segment_count: Some(preview_segment_count as u32),
                                raw_segment_count: Some(
                                    recording.transcript.raw_segments.len() as u32
                                ),
                                gpu_available: Some(speech::get_gpu_available(
                                    &app.state::<InferenceState>(),
                                )),
                                vad_enabled: Some(true),
                                stop_ms: Some(stop_elapsed.as_millis() as u64),
                                transcribe_ms: None,
                                total_ms: Some(stop_elapsed.as_millis() as u64),
                                text_len: Some(recording.transcript.joined_text.len()),
                                success: true,
                                error: None,
                            },
                        );
                    }

                    finalize_and_insert(
                        &app,
                        &config,
                        hwnd,
                        TranscriptPipelineInput {
                            raw_segments: recording.transcript.raw_segments,
                            joined_text: recording.transcript.joined_text,
                            stt_provider: resolve_stt_provider(&config),
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
                    maybe_log_support_event(
                        &app,
                        "capture",
                        "vad_finalize_error",
                        "Failed to finalize VAD dictation recording",
                        serde_json::json!({
                            "pipeline_mode": "dictation",
                            "error": e,
                        }),
                    );
                    if offline_parakeet_variant(&config)
                    {
                        log_parakeet_event(
                            &app,
                            ParakeetEventContext {
                                event_type: "vad_finalize_error",
                                pipeline_mode: "dictation",
                                input_source: "dictation",
                                utterance_id: None,
                                utterance_duration_ms: None,
                                preview_segment_count: None,
                                raw_segment_count: None,
                                gpu_available: Some(speech::get_gpu_available(
                                    &app.state::<InferenceState>(),
                                )),
                                vad_enabled: Some(true),
                                stop_ms: Some(stop_started.elapsed().as_millis() as u64),
                                transcribe_ms: None,
                                total_ms: Some(stop_started.elapsed().as_millis() as u64),
                                text_len: None,
                                success: false,
                                error: Some(e.clone()),
                            },
                        );
                    }
                    emit_all(&app, "transcription-error", e);
                    emit_all(&app, "recording-state", "idle");
                }
            });
        } else {
            // ── Legacy single-shot path (cloud or offline without VAD)
            let use_distil_offline =
                config.transcription_mode == "offline" && config.offline_engine == "distil_whisper";
            let audio_result: Result<AudioData, String> = if use_cloud {
                recorder::stop_and_save(stream)
                    .map(|path| AudioData::WavFile(path.to_string_lossy().to_string()))
            } else if use_distil_offline {
                let diagnostics_utterance_id = app
                    .state::<TranscriptDiagnosticsState>()
                    .0
                    .next_utterance_id();
                recorder::stop_and_get_raw_samples(stream).map(|(samples, rate, channels)| {
                    AudioData::DistilWhisperSamples {
                        samples,
                        sample_rate: rate,
                        channels,
                        diagnostics_utterance_id: Some(diagnostics_utterance_id.clone()),
                    }
                })
            } else {
                recorder::stop_and_get_raw_samples(stream).map(|(samples, rate, channels)| {
                    AudioData::RawSamples {
                        samples,
                        sample_rate: rate,
                        channels,
                    }
                })
            };

            match audio_result {
                Ok(audio_data) => {
                    let audio_details = match &audio_data {
                        AudioData::WavFile(path) => serde_json::json!({
                            "pipeline_mode": "dictation",
                            "audio_kind": "wav_file",
                            "path": path,
                        }),
                        AudioData::DistilWhisperSamples {
                            samples,
                            sample_rate,
                            channels,
                            ..
                        } => serde_json::json!({
                            "pipeline_mode": "dictation",
                            "audio_kind": "distil_samples",
                            "sample_count": samples.len(),
                            "sample_rate": sample_rate,
                            "channels": channels,
                        }),
                        AudioData::RawSamples {
                            samples,
                            sample_rate,
                            channels,
                        } => serde_json::json!({
                            "pipeline_mode": "dictation",
                            "audio_kind": "raw_samples",
                            "sample_count": samples.len(),
                            "sample_rate": sample_rate,
                            "channels": channels,
                        }),
                    };
                    maybe_log_support_event(
                        app,
                        "capture",
                        "recording_capture_success",
                        "Captured dictation audio successfully",
                        audio_details,
                    );
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
                        transcribe_and_insert(
                            &app,
                            &config,
                            hwnd,
                            audio_data,
                            &runtime_dict,
                            style_id,
                        );
                    });
                }
                Err(e) => {
                    eprintln!("Failed to capture recording: {}", e);
                    maybe_log_support_event(
                        app,
                        "capture",
                        "recording_capture_error",
                        "Failed to capture dictation audio",
                        serde_json::json!({
                            "pipeline_mode": "dictation",
                            "error": e,
                        }),
                    );
                    if use_distil_offline {
                        log_distil_whisper_event(
                            app,
                            DistilWhisperEventContext {
                                event_type: "capture_stop_error",
                                pipeline_mode: "dictation",
                                input_source: "dictation",
                                utterance_id: None,
                                utterance_duration_ms: None,
                                compacted_duration_ms: None,
                                wav_bytes: None,
                                compacted_wav_bytes: None,
                                speech_region_count: None,
                                fallback_to_raw_audio: None,
                                requested_mode: None,
                                effective_mode: None,
                                device: None,
                                total_ms: None,
                                text_len: None,
                                success: false,
                                error: Some(e.clone()),
                            },
                        );
                    } else if offline_parakeet_variant(config)
                    {
                        log_parakeet_event(
                            app,
                            ParakeetEventContext {
                                event_type: "capture_stop_error",
                                pipeline_mode: "dictation",
                                input_source: "dictation",
                                utterance_id: None,
                                utterance_duration_ms: None,
                                preview_segment_count: None,
                                raw_segment_count: None,
                                gpu_available: Some(speech::get_gpu_available(
                                    &app.state::<InferenceState>(),
                                )),
                                vad_enabled: Some(false),
                                stop_ms: None,
                                transcribe_ms: None,
                                total_ms: None,
                                text_len: None,
                                success: false,
                                error: Some(e.clone()),
                            },
                        );
                    }
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
    DistilWhisperSamples {
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
        diagnostics_utterance_id: Option<String>,
    },
    RawSamples {
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    },
}

struct PreparedSpeechAudio {
    effective_wav_bytes: Vec<u8>,
}

#[allow(dead_code)]
struct DistilWhisperEventContext<'a> {
    event_type: &'a str,
    pipeline_mode: &'a str,
    input_source: &'a str,
    utterance_id: Option<String>,
    utterance_duration_ms: Option<u64>,
    compacted_duration_ms: Option<u64>,
    wav_bytes: Option<usize>,
    compacted_wav_bytes: Option<usize>,
    speech_region_count: Option<u32>,
    fallback_to_raw_audio: Option<bool>,
    requested_mode: Option<String>,
    effective_mode: Option<String>,
    device: Option<String>,
    total_ms: Option<u64>,
    text_len: Option<usize>,
    success: bool,
    error: Option<String>,
}

#[allow(dead_code)]
struct ParakeetEventContext<'a> {
    event_type: &'a str,
    pipeline_mode: &'a str,
    input_source: &'a str,
    utterance_id: Option<String>,
    utterance_duration_ms: Option<u64>,
    preview_segment_count: Option<u32>,
    raw_segment_count: Option<u32>,
    gpu_available: Option<bool>,
    vad_enabled: Option<bool>,
    stop_ms: Option<u64>,
    transcribe_ms: Option<u64>,
    total_ms: Option<u64>,
    text_len: Option<usize>,
    success: bool,
    error: Option<String>,
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
    let continuous_gen = app.state::<ContinuousGeneration>().0.load(Ordering::SeqCst);

    if transcript.raw_segments.is_empty() && transcript.joined_text.trim().is_empty() {
        eprintln!("[capture] Transcription produced empty text");
        maybe_log_support_event(
            app,
            "capture",
            "finalize_empty",
            "Transcription finalized with empty text",
            serde_json::json!({
                "pipeline_mode": "dictation",
                "stt_provider": transcript.stt_provider,
                "utterance_duration_ms": transcript.utterance_duration_ms,
            }),
        );
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
    let use_distil_light_cleanup =
        config.transcription_mode == "offline" && config.offline_engine == "distil_whisper";

    // Spoken punctuation: "period" → ".", "comma" → "," etc.
    // Runs BEFORE cleanup so inserted periods trigger sentence capitalization.
    let normalized_text = if config.spoken_punctuation_enabled
        && allow_english_shaped_transforms
        && !use_distil_light_cleanup
    {
        speech::cleanup::replace_spoken_punctuation(&normalized_text)
    } else {
        normalized_text
    };

    // Final deterministic cleanup happens once after full assembly.
    let cleaned_text = if config.text_cleanup_enabled {
        if use_distil_light_cleanup {
            speech::cleanup::normalize_text_light(&normalized_text)
        } else {
            speech::cleanup::clean_final_text_for_language(&normalized_text, cleanup_language)
        }
    } else {
        normalized_text.clone()
    };

    // Hallucination filtering: remove known phantom phrases and repetition loops.
    let cleaned_text = if config.hallucination_filter_enabled {
        if use_distil_light_cleanup {
            speech::cleanup::filter_hallucination_loops(&cleaned_text)
        } else {
            speech::cleanup::filter_hallucinations(&cleaned_text)
        }
    } else {
        cleaned_text
    };

    // Number-word → digit conversion (e.g. "twenty three" → "23").
    let cleaned_text = if config.numerals_enabled
        && allow_english_shaped_transforms
        && !use_distil_light_cleanup
    {
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
                eprintln!(
                    "[capture] Style profile '{}' not found, skipping LLM",
                    profile_id
                );
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
            maybe_log_support_event(
                app,
                "capture",
                "insert_success",
                "Inserted dictated text into the app window",
                serde_json::json!({
                    "pipeline_mode": "dictation",
                    "stt_provider": transcript.stt_provider,
                    "final_text_len": final_text.len(),
                    "insert_target": "self_window",
                }),
            );
        } else {
            match output::insert_text(text_to_insert, hwnd) {
                Ok(report) => {
                    insert_success = true;
                    maybe_log_support_event(
                        app,
                        "capture",
                        "insert_success",
                        "Inserted dictated text",
                        serde_json::json!({
                            "pipeline_mode": "dictation",
                            "stt_provider": transcript.stt_provider,
                            "final_text_len": final_text.len(),
                            "insert_target": "focused_window",
                            "output_report": report,
                        }),
                    );
                }
                Err(error) => {
                    maybe_log_support_event(
                        app,
                        "capture",
                        "insert_error",
                        "Failed to insert dictated text",
                        serde_json::json!({
                            "pipeline_mode": "dictation",
                            "stt_provider": transcript.stt_provider,
                            "final_text_len": final_text.len(),
                            "error": error.to_string(),
                            "output_report": error.report,
                        }),
                    );
                    emit_all(app, "transcription-error", error.to_string());
                }
            }
        }
    }

    maybe_log_support_event(
        app,
        "capture",
        "finalize_complete",
        "Completed dictation finalize pipeline",
        serde_json::json!({
            "pipeline_mode": "dictation",
            "stt_provider": transcript.stt_provider,
            "joined_text_len": transcript.joined_text.len(),
            "final_text_len": final_text.len(),
            "insert_success": insert_success,
            "final_pass_used": transcript.final_pass_used,
            "final_pass_reason": transcript.final_pass_reason,
        }),
    );

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
        post_processed_text.as_deref().and_then(option_if_not_empty),
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
    if config.continuous_recording_enabled
        && config.dictation_activation_mode == "manual"
        && continuous_gen > 0
    {
        let current = app.state::<ContinuousGeneration>().0.load(Ordering::SeqCst);
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
    let pipeline_started = Instant::now();
    let utterance_duration_ms = match &audio_data {
        AudioData::WavFile(_) => 0,
        AudioData::DistilWhisperSamples {
            samples,
            sample_rate,
            channels,
            ..
        } => duration_ms_from_samples(samples.len(), *sample_rate, *channels),
        AudioData::RawSamples {
            samples,
            sample_rate,
            channels,
        } => duration_ms_from_samples(samples.len(), *sample_rate, *channels),
    };
    let diagnostics_utterance_id = match &audio_data {
        AudioData::DistilWhisperSamples {
            diagnostics_utterance_id,
            ..
        } => diagnostics_utterance_id.clone(),
        _ => None,
    };
    let parakeet_gpu_available = offline_parakeet_variant(config)
        && speech::get_gpu_available(&app.state::<InferenceState>());

    maybe_log_support_event(
        app,
        "capture",
        "transcribe_started",
        "Starting dictation transcription",
        serde_json::json!({
            "pipeline_mode": "dictation",
            "offline_engine": config.offline_engine,
            "transcription_mode": config.transcription_mode,
            "utterance_duration_ms": utterance_duration_ms,
            "audio_kind": match &audio_data {
                AudioData::WavFile(_) => "wav_file",
                AudioData::DistilWhisperSamples { .. } => "distil_samples",
                AudioData::RawSamples { .. } => "raw_samples",
            },
        }),
    );

    let transcribe_result = match audio_data {
        AudioData::WavFile(wav_path) => speech::cloud_transcribe(
            &wav_path,
            &config.cloud_stt_provider,
            &config.cloud_stt_api_key,
            &config.language,
        ),
        AudioData::DistilWhisperSamples {
            samples,
            sample_rate,
            channels,
            ..
        } => {
            let prepared = prepare_compacted_audio(app, config, &samples, sample_rate, channels);
            match prepared {
                Ok(prepared) => match app
                    .state::<crate::features::speech::distil_whisper::DistilWhisperState>()
                    .0
                    .lock()
                {
                    Ok(mut guard) => guard
                        .transcribe_local_wav_bytes(app, &prepared.effective_wav_bytes)
                        .map(|result| result.text),
                    Err(err) => Err(err.to_string()),
                },
                Err(err) => Err(err),
            }
        }
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
            let transcribe_started = Instant::now();
            let result = speech::transcribe_audio(&inference_state, capped, sample_rate, channels);
            if offline_parakeet_variant(config) {
                match &result {
                    Ok(text) => {
                        eprintln!(
                            "[capture][parakeet-dictation] raw_ms={} vad=false gpu_available={} transcribe_s={:.2} total_s={:.2} text_len={}",
                            utterance_duration_ms,
                            parakeet_gpu_available,
                            transcribe_started.elapsed().as_secs_f32(),
                            pipeline_started.elapsed().as_secs_f32(),
                            text.len(),
                        );
                        log_parakeet_event(
                            app,
                            ParakeetEventContext {
                                event_type: "transcribe_success",
                                pipeline_mode: "dictation",
                                input_source: "dictation",
                                utterance_id: None,
                                utterance_duration_ms: Some(utterance_duration_ms),
                                preview_segment_count: Some(1),
                                raw_segment_count: Some(1),
                                gpu_available: Some(parakeet_gpu_available),
                                vad_enabled: Some(false),
                                stop_ms: None,
                                transcribe_ms: Some(transcribe_started.elapsed().as_millis() as u64),
                                total_ms: Some(pipeline_started.elapsed().as_millis() as u64),
                                text_len: Some(text.len()),
                                success: true,
                                error: None,
                            },
                        );
                    }
                    Err(err) => {
                        log_parakeet_event(
                            app,
                            ParakeetEventContext {
                                event_type: "transcribe_error",
                                pipeline_mode: "dictation",
                                input_source: "dictation",
                                utterance_id: None,
                                utterance_duration_ms: Some(utterance_duration_ms),
                                preview_segment_count: None,
                                raw_segment_count: None,
                                gpu_available: Some(parakeet_gpu_available),
                                vad_enabled: Some(false),
                                stop_ms: None,
                                transcribe_ms: Some(transcribe_started.elapsed().as_millis() as u64),
                                total_ms: Some(pipeline_started.elapsed().as_millis() as u64),
                                text_len: None,
                                success: false,
                                error: Some(err.clone()),
                            },
                        );
                    }
                }
            }
            result
        }
    };

    match transcribe_result {
        Ok(raw_text) => {
            maybe_log_support_event(
                app,
                "capture",
                "transcribe_success",
                "Dictation transcription completed",
                serde_json::json!({
                    "pipeline_mode": "dictation",
                    "stt_provider": resolve_stt_provider(config),
                    "utterance_duration_ms": utterance_duration_ms,
                    "text_len": raw_text.len(),
                    "elapsed_ms": pipeline_started.elapsed().as_millis() as u64,
                }),
            );
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
            maybe_log_support_event(
                app,
                "capture",
                "transcribe_error",
                "Dictation transcription failed",
                serde_json::json!({
                    "pipeline_mode": "dictation",
                    "offline_engine": config.offline_engine,
                    "utterance_duration_ms": utterance_duration_ms,
                    "elapsed_ms": pipeline_started.elapsed().as_millis() as u64,
                    "error": e,
                }),
            );
            if config.transcription_mode == "offline" && config.offline_engine == "distil_whisper" {
                log_distil_whisper_event(
                    app,
                    DistilWhisperEventContext {
                        event_type: "transcribe_error",
                        pipeline_mode: "dictation",
                        input_source: "dictation",
                        utterance_id: diagnostics_utterance_id,
                        utterance_duration_ms: Some(utterance_duration_ms),
                        compacted_duration_ms: None,
                        wav_bytes: None,
                        compacted_wav_bytes: None,
                        speech_region_count: None,
                        fallback_to_raw_audio: None,
                        requested_mode: None,
                        effective_mode: None,
                        device: None,
                        total_ms: None,
                        text_len: None,
                        success: false,
                        error: Some(e.clone()),
                    },
                );
            }
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
    } else if config.offline_engine == "distil_whisper" {
        "distil-whisper".to_string()
    } else if config.offline_engine == "parakeet_en" {
        "parakeet-tdt-v2".to_string()
    } else {
        "parakeet-tdt-v3".to_string()
    }
}

fn log_distil_whisper_event(app: &AppHandle, event: DistilWhisperEventContext<'_>) {
    if !crate::features::diagnostics::support_diagnostics_enabled(app) {
        return;
    }

    let diagnostics_state = app.state::<TranscriptDiagnosticsState>();
    diagnostics_state
        .0
        .log_distil_whisper_event(DistilWhisperDiagnosticsEvent {
            created_at: current_timestamp_ms(),
            session_id: diagnostics_state.0.session_id().to_string(),
            utterance_id: event.utterance_id,
            event_type: event.event_type.to_string(),
            pipeline_mode: event.pipeline_mode.to_string(),
            input_source: event.input_source.to_string(),
            utterance_duration_ms: event.utterance_duration_ms,
            compacted_duration_ms: event.compacted_duration_ms,
            wav_bytes: event.wav_bytes,
            compacted_wav_bytes: event.compacted_wav_bytes,
            speech_region_count: event.speech_region_count,
            fallback_to_raw_audio: event.fallback_to_raw_audio,
            requested_mode: event.requested_mode,
            effective_mode: event.effective_mode,
            device: event.device,
            total_ms: event.total_ms,
            text_len: event.text_len,
            success: event.success,
            error: event.error,
        });
}

fn log_parakeet_event(app: &AppHandle, event: ParakeetEventContext<'_>) {
    if !crate::features::diagnostics::support_diagnostics_enabled(app) {
        return;
    }

    let diagnostics_state = app.state::<TranscriptDiagnosticsState>();
    diagnostics_state
        .0
        .log_parakeet_event(ParakeetDiagnosticsEvent {
            created_at: current_timestamp_ms(),
            session_id: diagnostics_state.0.session_id().to_string(),
            utterance_id: event.utterance_id,
            event_type: event.event_type.to_string(),
            pipeline_mode: event.pipeline_mode.to_string(),
            input_source: event.input_source.to_string(),
            utterance_duration_ms: event.utterance_duration_ms,
            preview_segment_count: event.preview_segment_count,
            raw_segment_count: event.raw_segment_count,
            gpu_available: event.gpu_available,
            vad_enabled: event.vad_enabled,
            stop_ms: event.stop_ms,
            transcribe_ms: event.transcribe_ms,
            total_ms: event.total_ms,
            text_len: event.text_len,
            success: event.success,
            error: event.error,
        });
}

fn prepare_compacted_audio(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<PreparedSpeechAudio, String> {
    let raw_wav_bytes = recorder::encode_wav_bytes(samples, sample_rate, channels);

    let model_path = resolve_vad_model_path(app)?;
    let compacted = speech::vad::compact_speech(
        samples,
        sample_rate,
        channels,
        &model_path,
        config.vad_silence_threshold_ms,
    )?;

    let compacted_duration_ms =
        duration_ms_from_samples(compacted.compacted_samples_16k.len(), 16_000, 1);
    let compacted_wav_bytes = if !compacted.compacted_samples_16k.is_empty() {
        Some(recorder::encode_wav_bytes(
            &compacted.compacted_samples_16k,
            16_000,
            1,
        ))
    } else {
        None
    };

    let use_compacted_audio = should_use_compacted_audio(
        compacted.speech_region_count,
        compacted_duration_ms,
        compacted_wav_bytes.is_some(),
    );

    let effective_wav_bytes = compacted_wav_bytes
        .clone()
        .filter(|_| use_compacted_audio)
        .unwrap_or_else(|| raw_wav_bytes.clone());

    Ok(PreparedSpeechAudio {
        effective_wav_bytes,
    })
}

fn duration_ms_from_samples(sample_count: usize, sample_rate: u32, channels: u16) -> u64 {
    let channel_count = u64::from(channels.max(1));
    let sample_rate = u64::from(sample_rate.max(1));
    let frames = sample_count as u64 / channel_count;
    (frames * 1000) / sample_rate
}

fn should_use_compacted_audio(
    speech_region_count: usize,
    compacted_duration_ms: u64,
    has_compacted_wav: bool,
) -> bool {
    speech_region_count > 0 && compacted_duration_ms >= 200 && has_compacted_wav
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

fn log_command_history_sample(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    raw_transcript: &str,
    inserted_text: Option<String>,
    insert_success: bool,
) {
    let diagnostics_state = app.state::<TranscriptDiagnosticsState>();
    let raw_segments_json = match serde_json::to_string(&vec![raw_transcript.to_string()]) {
        Ok(json) => json,
        Err(err) => {
            eprintln!(
                "[capture] Failed to serialize command transcript history: {}",
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
        pipeline_mode: "command".to_string(),
        transcription_mode: config.transcription_mode.clone(),
        stt_provider: resolve_stt_provider(config),
        active_industry_pack: config.active_industry_pack.clone(),
        active_profile_id: config.active_profile_id.clone(),
        cleanup_enabled: false,
        post_processing_enabled: true,
        vad_silence_threshold_ms: config.vad_silence_threshold_ms,
        utterance_duration_ms: 0,
        preview_segment_count: 1,
        final_pass_used: false,
        final_pass_reason: "command".to_string(),
        final_pass_latency_ms: None,
        language_family: "unknown".to_string(),
        language_lock_confidence: "none".to_string(),
        mixed_script_detected: false,
        raw_segments_json,
        joined_text: option_if_not_empty(raw_transcript),
        normalized_text: None,
        cleaned_text: None,
        post_processed_text: None,
        final_text: inserted_text.clone(),
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
            eprintln!(
                "[capture] Style switched to '{}' (key: {})",
                profile.name, key_name
            );
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
    maybe_log_support_event(
        app,
        "capture",
        "recording_cancelled",
        "Dictation recording cancelled",
        serde_json::json!({
            "pipeline_mode": "dictation",
        }),
    );
    // Break continuous recording cycle
    app.state::<ContinuousGeneration>()
        .0
        .store(0, Ordering::SeqCst);
    emit_all(app, "continuous-active", "false");
    set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
    set_dictation_phase(app, DictationRuntimePhase::Idle);
    reset_hotkey_runtime(app);

    if let Ok(mut style_key) = app.state::<ActiveStyleKey>().0.lock() {
        *style_key = None;
    }

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
    maybe_log_support_event(
        app,
        "capture",
        "recording_cancelled",
        "Command recording cancelled",
        serde_json::json!({
            "pipeline_mode": "command",
        }),
    );
    // Silently discard any in-progress command recording
    set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
    reset_hotkey_runtime(app);
    let recording_state = app.state::<RecordingState>();
    if let Ok(mut guard) = recording_state.0.lock() {
        if guard.take().is_some() {
            eprintln!("[capture] Command recording cancelled (style switch or short press)");
        }
    }
    emit_all(app, "recording-state", "idle");
}

fn handle_command_start(app: &AppHandle, config: &crate::features::settings::AppConfig) {
    maybe_log_support_event(
        app,
        "capture",
        "recording_start_requested",
        "Starting command recording",
        serde_json::json!({
            "pipeline_mode": "command",
            "device_index": config.device_index,
            "offline_engine": config.offline_engine,
            "transcription_mode": config.transcription_mode,
        }),
    );

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
            set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
            reset_hotkey_runtime(app);
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
    // Command mode: always record without VAD (commands are short utterances)
    let warm_state = app.try_state::<recorder::WarmDeviceState>();
    match recorder::start_recording(
        config.device_index,
        app.clone(),
        None,
        warm_state.as_deref(),
    ) {
        Ok(stream) => {
            *guard = Some(stream);
            set_hotkey_recording_mode(app, HotkeyRecordingMode::Command);
            maybe_log_support_event(
                app,
                "capture",
                "recording_start_success",
                "Command recording started",
                serde_json::json!({
                    "pipeline_mode": "command",
                    "device_index": config.device_index,
                }),
            );
            if config.sounds_enabled {
                output::play_start_sound(&config.start_sound);
            }
        }
        Err(e) => {
            eprintln!("[capture] Failed to start command recording: {}", e);
            maybe_log_support_event(
                app,
                "capture",
                "recording_start_error",
                "Failed to start command recording",
                serde_json::json!({
                    "pipeline_mode": "command",
                    "device_index": config.device_index,
                    "error": e,
                }),
            );
            set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
            reset_hotkey_runtime(app);
            emit_all(app, "recording-state", "idle");
            emit_all(app, "transcription-error", format!("Recording failed: {e}"));
        }
    }

    recorder::spawn_warm_device(app, config.device_index);
}

fn handle_command_stop(app: &AppHandle, config: &crate::features::settings::AppConfig) {
    maybe_log_support_event(
        app,
        "capture",
        "recording_stop_requested",
        "Stopping command recording",
        serde_json::json!({
            "pipeline_mode": "command",
            "offline_engine": config.offline_engine,
            "transcription_mode": config.transcription_mode,
        }),
    );
    set_hotkey_recording_mode(app, HotkeyRecordingMode::None);
    reset_hotkey_runtime(app);
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
            let use_distil_offline =
                config.transcription_mode == "offline" && config.offline_engine == "distil_whisper";
            let transcribe_result = if use_cloud {
                recorder::stop_and_save(stream).and_then(|path| {
                    speech::cloud_transcribe(
                        &path.to_string_lossy(),
                        &config.cloud_stt_provider,
                        &config.cloud_stt_api_key,
                        &config.language,
                    )
                })
            } else if use_distil_offline {
                recorder::stop_and_get_raw_samples(stream).and_then(
                    |(samples, sample_rate, channels)| {
                        let prepared =
                            prepare_compacted_audio(&app, &config, &samples, sample_rate, channels)?;
                        let distil_state =
                            app.state::<crate::features::speech::distil_whisper::DistilWhisperState>();
                        let mut guard = distil_state.0.lock().map_err(|e| e.to_string())?;
                        guard
                            .transcribe_local_wav_bytes(&app, &prepared.effective_wav_bytes)
                            .map(|result| result.text)
                    },
                )
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
                    maybe_log_support_event(
                        &app,
                        "capture",
                        "transcribe_success",
                        "Command transcription completed",
                        serde_json::json!({
                            "pipeline_mode": "command",
                            "stt_provider": resolve_stt_provider(&config),
                            "text_len": transcript.len(),
                        }),
                    );
                    let hwnd = app
                        .state::<FocusedWindowState>()
                        .0
                        .lock()
                        .map(|g| *g)
                        .unwrap_or(0);
                    command_finalize(&app, &config, hwnd, transcript);
                }
                Err(e) => {
                    eprintln!("[capture] Command transcription error: {}", e);
                    maybe_log_support_event(
                        &app,
                        "capture",
                        "transcribe_error",
                        "Command transcription failed",
                        serde_json::json!({
                            "pipeline_mode": "command",
                            "offline_engine": config.offline_engine,
                            "error": e,
                        }),
                    );
                    if use_distil_offline {
                        log_distil_whisper_event(
                            &app,
                            DistilWhisperEventContext {
                                event_type: "transcribe_error",
                                pipeline_mode: "command",
                                input_source: "command",
                                utterance_id: None,
                                utterance_duration_ms: None,
                                compacted_duration_ms: None,
                                wav_bytes: None,
                                compacted_wav_bytes: None,
                                speech_region_count: None,
                                fallback_to_raw_audio: None,
                                requested_mode: None,
                                effective_mode: None,
                                device: None,
                                total_ms: None,
                                text_len: None,
                                success: false,
                                error: Some(e.clone()),
                            },
                        );
                    }
                    emit_all(&app, "transcription-error", e);
                    emit_all(&app, "recording-state", "idle");
                }
            }
        });
    } else {
        emit_all(app, "recording-state", "idle");
    }
}

/// Finalize a text action: send transcript through the selected action and
/// paste the result using the existing command pipeline.
fn command_finalize(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    raw_transcript: String,
) {
    if raw_transcript.trim().is_empty() {
        eprintln!("[capture] Command transcription produced empty text");
        maybe_log_support_event(
            app,
            "capture",
            "finalize_empty",
            "Command transcription finalized with empty text",
            serde_json::json!({
                "pipeline_mode": "command",
            }),
        );
        if config.sounds_enabled {
            output::play_done_sound(&config.stop_sound);
        }
        emit_all(app, "recording-state", "done");
        return;
    }

    let transcript = raw_transcript.trim();
    let text_action = match resolve_default_text_action(app, config) {
        Ok(action) => action,
        Err(e) => {
            emit_all(app, "transcription-error", e);
            emit_all(app, "recording-state", "idle");
            return;
        }
    };
    eprintln!("[capture] Command transcript: {}", transcript);
    maybe_log_support_event(
        app,
        "capture",
        "text_action_finalize_started",
        "Starting text action finalize pipeline",
        serde_json::json!({
            "pipeline_mode": "command",
            "raw_text_len": transcript.len(),
            "text_action_id": text_action.id,
            "text_action_name": text_action.name,
            "provider": config.command_provider,
            "model": config.command_model,
        }),
    );

    let is_freeform_action = text_action.id == speech::freeform_command_action_id();

    // Check if this is a vocabulary addition command
    if is_freeform_action {
        if let Some(vocab_cmd) =
            speech::llm::detect_vocab_command(transcript, &config.command_api_key)
        {
            eprintln!(
                "[capture] Vocab command detected: term='{}', full_form={:?}",
                vocab_cmd.term, vocab_cmd.full_form
            );
            match add_term_to_general_vocabulary(app, &vocab_cmd, &config.command_api_key) {
                Ok(_) => {
                    let msg = format!("Added: {}", vocab_cmd.term);
                    emit_all(app, "vocab-added", &msg);
                    maybe_log_support_event(
                        app,
                        "capture",
                        "command_vocab_add_success",
                        "Added vocabulary term from command mode",
                        serde_json::json!({
                            "pipeline_mode": "command",
                            "term": vocab_cmd.term,
                        }),
                    );
                }
                Err(e) => {
                    eprintln!("[capture] Failed to add vocab term: {}", e);
                    maybe_log_support_event(
                        app,
                        "capture",
                        "command_vocab_add_error",
                        "Failed to add vocabulary term from command mode",
                        serde_json::json!({
                            "pipeline_mode": "command",
                            "term": vocab_cmd.term,
                            "error": e,
                        }),
                    );
                    emit_all(
                        app,
                        "transcription-error",
                        format!("Vocab add failed: {}", e),
                    );
                }
            }
            if config.sounds_enabled {
                output::play_done_sound(&config.stop_sound);
            }
            emit_all(app, "recording-state", "done");
            return;
        }
    }

    let result = if is_freeform_action {
        text_only_command(transcript, text_action.prompt.trim(), config)
    } else {
        transform_with_text_action(transcript, &text_action, config)
    };

    match result {
        Ok(text) => {
            let inserted_text = option_if_not_empty(&text);
            let mut insert_success = false;
            if !text.is_empty() {
                match output::insert_text(&text, hwnd) {
                    Ok(report) => {
                        insert_success = true;
                        maybe_log_support_event(
                            app,
                            "capture",
                            "insert_success",
                            "Inserted command output",
                            serde_json::json!({
                                "pipeline_mode": "command",
                                "final_text_len": text.len(),
                                "output_report": report,
                            }),
                        );
                    }
                    Err(error) => {
                        eprintln!("[capture] Command text insertion error: {}", error);
                        maybe_log_support_event(
                            app,
                            "capture",
                            "insert_error",
                            "Failed to insert command output",
                            serde_json::json!({
                                "pipeline_mode": "command",
                                "final_text_len": text.len(),
                                "error": error.to_string(),
                                "output_report": error.report,
                            }),
                        );
                        emit_all(app, "transcription-error", error.to_string());
                    }
                }
            }
            log_command_history_sample(app, config, transcript, inserted_text, insert_success);
        }
        Err(e) => {
            eprintln!("[capture] Command error: {}", e);
            maybe_log_support_event(
                app,
                "capture",
                "text_action_error",
                "Text action pipeline failed",
                serde_json::json!({
                    "pipeline_mode": "command",
                    "text_action_id": text_action.id,
                    "text_action_name": text_action.name,
                    "provider": config.command_provider,
                    "model": config.command_model,
                    "error": e,
                }),
            );
            emit_all(
                app,
                "transcription-error",
                format!("Text action error: {}", e),
            );
        }
    }

    if config.sounds_enabled {
        output::play_done_sound(&config.stop_sound);
    }
    emit_all(app, "recording-state", "done");
}

fn resolve_default_text_action(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
) -> Result<speech::TextAction, String> {
    let text_actions_dir = speech::get_text_actions_dir(app)?;
    speech::get_text_action(&text_actions_dir, &config.default_text_action_id)
        .or_else(|_| speech::get_text_action(&text_actions_dir, speech::default_text_action_id()))
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

fn transform_with_text_action(
    source_text: &str,
    action: &speech::TextAction,
    config: &crate::features::settings::AppConfig,
) -> Result<String, String> {
    speech::text_action_llm_call(
        source_text,
        &action.prompt,
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
    if !general
        .vocabulary
        .iter()
        .any(|v| v.eq_ignore_ascii_case(&term))
    {
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
        // Default is the v3 multilingual Parakeet variant.
        assert_eq!(resolve_stt_provider(&config), "parakeet-tdt-v3");

        config.offline_engine = "parakeet_en".to_string();
        assert_eq!(resolve_stt_provider(&config), "parakeet-tdt-v2");

        config.offline_engine = "distil_whisper".to_string();
        assert_eq!(resolve_stt_provider(&config), "distil-whisper");

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
    fn compacted_audio_fallback_requires_regions_and_min_duration() {
        assert!(!should_use_compacted_audio(0, 1_500, true));
        assert!(!should_use_compacted_audio(2, 150, true));
        assert!(!should_use_compacted_audio(2, 900, false));
        assert!(should_use_compacted_audio(2, 900, true));
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
