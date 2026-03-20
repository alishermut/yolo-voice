pub mod hotkey;
pub mod recorder;

use tauri::{AppHandle, Listener, Manager};

use crate::app::events::emit_all;
use crate::features::output::{self, FocusedWindowState};
use crate::features::settings::ConfigState;
use crate::features::speech;
use crate::features::speech::vocabulary::GlobalDictionaryState;
use crate::infra::sidecar::SidecarState;

use self::recorder::RecordingState;

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
    match recorder::start_recording(config.device_index, app.clone()) {
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

        let audio_result: Result<(Option<String>, Option<Vec<u8>>), String> = if use_cloud {
            recorder::stop_and_save(stream)
                .map(|path| (Some(path.to_string_lossy().to_string()), None))
        } else {
            recorder::stop_and_get_wav_bytes(stream).map(|bytes| (None, Some(bytes)))
        };

        match audio_result {
            Ok((wav_path, wav_bytes)) => {
                eprintln!(
                    "Recording captured (in-memory: {})",
                    wav_bytes.is_some()
                );
                emit_all(app, "recording-state", "transcribing");

                let hwnd = *app
                    .state::<FocusedWindowState>()
                    .0
                    .lock()
                    .unwrap();

                // Clone everything needed for the background thread
                let app = app.clone();
                let config = config.clone();
                let global_dict = app
                    .state::<GlobalDictionaryState>()
                    .0
                    .lock()
                    .unwrap()
                    .clone();

                std::thread::spawn(move || {
                    transcribe_and_insert(
                        &app, &config, hwnd, wav_path, wav_bytes, &global_dict,
                    );
                });
            }
            Err(e) => {
                eprintln!("Failed to save recording: {}", e);
                emit_all(app, "transcription-error", e.to_string());
                emit_all(app, "recording-state", "idle");
            }
        }
    } else {
        emit_all(app, "recording-state", "idle");
    }
}

/// Background transcription pipeline: transcribe → post-process → insert text.
fn transcribe_and_insert(
    app: &AppHandle,
    config: &crate::features::settings::AppConfig,
    hwnd: isize,
    wav_path: Option<String>,
    wav_bytes: Option<Vec<u8>>,
    global_dict: &speech::vocabulary::GlobalDictionary,
) {
    let sidecar_state = app.state::<SidecarState>();

    // Ensure sidecar is running
    if let Err(e) = crate::infra::sidecar::ensure_running(
        app,
        &sidecar_state,
        &config.whisper_model,
        &config.device,
        &config.compute_type,
    ) {
        emit_all(app, "transcription-error", format!("Sidecar error: {}", e));
        emit_all(app, "recording-state", "idle");
        return;
    }

    let mut guard = sidecar_state.0.lock().unwrap();
    let sc = match guard.as_mut() {
        Some(s) => s,
        None => {
            emit_all(app, "transcription-error", "Sidecar not available");
            emit_all(app, "recording-state", "idle");
            return;
        }
    };

    // Build initial_prompt from global vocabulary
    let vocab_words = global_dict.vocabulary.clone();
    let initial_prompt = if vocab_words.is_empty() {
        None
    } else {
        Some(vocab_words.join(", "))
    };

    // Choose transcription method
    let transcribe_result = if let Some(ref wav_path) = wav_path {
        speech::cloud_transcribe(
            sc,
            wav_path,
            &config.cloud_stt_provider,
            &config.cloud_stt_api_key,
            &config.language,
        )
    } else if let Some(ref bytes) = wav_bytes {
        speech::transcribe_audio(sc, bytes, &config.language, initial_prompt.as_deref())
    } else {
        Err("No audio data available".to_string())
    };

    match transcribe_result {
        Ok(raw_text) => {
            // Apply replacement rules
            let raw_text =
                speech::vocabulary::apply_replacements(&raw_text, &global_dict.replacements);
            let final_text = if config.post_processing_enabled && !raw_text.is_empty() {
                // Load the active profile and post-process
                let profiles_dir = speech::get_profiles_dir(app).unwrap_or_default();
                let profiles =
                    speech::list_profiles(sc, &profiles_dir.to_string_lossy())
                        .unwrap_or_default();

                let profile = profiles
                    .iter()
                    .find(|p| p.id == config.active_profile_id)
                    .cloned();

                if let Some(profile) = profile {
                    match speech::post_process_text(
                        sc,
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

            // Apply replacements again after post-processing
            let final_text =
                speech::vocabulary::apply_replacements(&final_text, &global_dict.replacements);

            if !final_text.is_empty() {
                let text_to_insert = final_text.trim().to_string();

                if let Err(e) = output::insert_text(&text_to_insert, hwnd) {
                    eprintln!("Text insertion error: {}", e);
                    emit_all(app, "transcription-error", e);
                }
            }
            output::play_done_sound(&config.stop_sound);
            emit_all(app, "recording-state", "done");
        }
        Err(e) => {
            eprintln!("Transcription error: {}", e);
            emit_all(app, "transcription-error", e);
            emit_all(app, "recording-state", "idle");
        }
    }
}
