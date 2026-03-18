mod audio;
mod commands;
mod config;
mod hotkey;
mod recorder;
mod sidecar;
mod startup;
mod text_insert;
mod transcription;

use commands::{AudioState, PillUiState};
use config::ConfigState;
use recorder::RecordingState;
use sidecar::SidecarState;
use std::sync::Mutex;
use text_insert::FocusedWindowState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, EventTarget, Listener, Manager, WindowEvent,
};

/// Emit an event to ALL windows AND update shared PillUiState for polling.
fn emit_all<S: serde::Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: S) {
    // Update shared PillUiState so pill can poll it
    if event == "recording-state" {
        if let Ok(json) = serde_json::to_string(&payload) {
            let pill_state = app.state::<PillUiState>();
            let mut rs = pill_state.recording_state.lock().unwrap();
            *rs = json.trim_matches('"').to_string();
            drop(rs);
        }
    }

    // Emit to all targets
    let _ = app.emit(event, payload.clone());
    let _ = app.emit_to(EventTarget::labeled("pill"), event, payload.clone());
    let _ = app.emit_to(EventTarget::labeled("main"), event, payload);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AudioState(Mutex::new(None)))
        .manage(ConfigState(Mutex::new(config::AppConfig::default())))
        .manage(RecordingState(Mutex::new(None)))
        .manage(FocusedWindowState(Mutex::new(0)))
        .manage(SidecarState(Mutex::new(None)))
        .manage(PillUiState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::start_test,
            commands::stop_test,
            commands::get_config,
            commands::save_config_cmd,
            commands::start_recording,
            commands::stop_recording,
            commands::get_models,
            commands::download_model_cmd,
            commands::set_whisper_model,
            commands::get_gpu_available,
            commands::get_sidecar_status,
            commands::get_profiles,
            commands::save_profile_cmd,
            commands::delete_profile_cmd,
            commands::test_llm_connection,
            commands::set_launch_on_startup,
            commands::get_app_info,
            commands::quit_app,
            commands::get_pill_state,
        ])
        .setup(|app| {
            // Load persisted config
            let saved_config = config::load_config(&app.handle());
            let config_state = app.state::<ConfigState>();
            *config_state.0.lock().unwrap() = saved_config.clone();

            // Build tray menu
            let show_item =
                MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item =
                MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Build tray icon
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        if let Some(w) =
                            tray.app_handle().get_webview_window("main")
                        {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Position pill window above the taskbar, centered
            if let Some(pill) = app.get_webview_window("pill") {
                // Set webview background to transparent (critical for Windows WebView2)
                let _ = pill.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

                // Use Win32 to get the work area (excludes taskbar)
                let work_area = unsafe {
                    let mut rect = windows::Win32::Foundation::RECT::default();
                    let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
                        windows::Win32::UI::WindowsAndMessaging::SPI_GETWORKAREA,
                        0,
                        Some(&mut rect as *mut _ as *mut _),
                        windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                    );
                    rect
                };

                let pill_width = 280;
                let pill_height = 50;
                let x = (work_area.right - work_area.left - pill_width) / 2 + work_area.left;
                let y = work_area.bottom - pill_height - 20; // 20px above taskbar
                let _ = pill.set_position(tauri::PhysicalPosition::new(x, y));
            }

            // Start minimized: hide main window if configured
            if saved_config.start_minimized {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.hide();
                }
            }

            // Start global hotkey listener
            hotkey::start_hotkey_listener(app.handle().clone());

            // Spawn sidecar in the background (non-blocking)
            let sidecar_handle = app.handle().clone();
            let sidecar_config = saved_config.clone();
            std::thread::spawn(move || {
                match sidecar::spawn_sidecar(&sidecar_handle) {
                    Ok(mut sc) => {
                        // Try to load the configured model
                        let models_dir = sidecar::get_models_dir(&sidecar_handle)
                            .unwrap_or_default();
                        let _ = transcription::load_model(
                            &mut sc,
                            &sidecar_config.whisper_model,
                            &sidecar_config.device,
                            &sidecar_config.compute_type,
                            &models_dir.to_string_lossy(),
                        );
                        let state = sidecar_handle.state::<SidecarState>();
                        *state.0.lock().unwrap() = Some(sc);
                        eprintln!("[app] Sidecar started and model load attempted");
                    }
                    Err(e) => {
                        eprintln!("[app] Failed to start sidecar (will retry on first transcription): {}", e);
                    }
                }
            });

            // Handle hotkey-action events: start/stop recording
            let app_handle = app.handle().clone();
            app.listen("hotkey-action", move |event| {
                let action = event.payload().trim_matches('"');
                let config = app_handle
                    .state::<ConfigState>()
                    .0
                    .lock()
                    .unwrap()
                    .clone();

                match action {
                    "start" => {
                        // Capture the foreground window before recording
                        let hwnd = text_insert::capture_foreground_window();
                        *app_handle.state::<FocusedWindowState>().0.lock().unwrap() = hwnd;

                        let recording_state = app_handle.state::<RecordingState>();
                        let mut guard = recording_state.0.lock().unwrap();
                        // Stop any existing recording
                        *guard = None;
                        match recorder::start_recording(
                            config.device_index,
                            app_handle.clone(),
                        ) {
                            Ok(stream) => {
                                *guard = Some(stream);
                                emit_all(&app_handle, "recording-state", "recording");
                                text_insert::play_start_sound();
                            }
                            Err(e) => {
                                eprintln!("Failed to start recording: {}", e);
                            }
                        }
                    }
                    "stop" => {
                        // Play stop sound immediately for instant feedback
                        text_insert::play_done_sound();

                        let recording_state = app_handle.state::<RecordingState>();
                        let mut guard = recording_state.0.lock().unwrap();
                        if let Some(stream) = guard.take() {
                            match recorder::stop_and_save(stream) {
                                Ok(path) => {
                                    eprintln!("Recording saved to: {:?}", path);
                                    emit_all(&app_handle, "recording-state", "transcribing");

                                    let hwnd = *app_handle
                                        .state::<FocusedWindowState>()
                                        .0
                                        .lock()
                                        .unwrap();

                                    // Transcribe in a background thread
                                    let app = app_handle.clone();
                                    let wav_path = path.to_string_lossy().to_string();
                                    let language = config.language.clone();
                                    let pp_enabled = config.post_processing_enabled;
                                    let pp_profile_id = config.active_profile_id.clone();
                                    let pp_provider = config.llm_provider.clone();
                                    let pp_model = config.llm_model.clone();
                                    let pp_api_key = config.llm_api_key.clone();
                                    let pp_base_url = config.llm_base_url.clone();
                                    let transcription_mode = config.transcription_mode.clone();
                                    let cloud_provider = config.cloud_stt_provider.clone();
                                    let cloud_api_key = config.cloud_stt_api_key.clone();

                                    std::thread::spawn(move || {
                                        // Ensure sidecar is running
                                        let sidecar_state = app.state::<SidecarState>();
                                        if let Err(e) = sidecar::ensure_running(&app, &sidecar_state) {
                                            emit_all(&app, "transcription-error", format!("Sidecar error: {}", e));
                                            emit_all(&app, "recording-state", "idle");
                                            return;
                                        }

                                        let mut guard = sidecar_state.0.lock().unwrap();
                                        let sc = match guard.as_mut() {
                                            Some(s) => s,
                                            None => {
                                                emit_all(&app, "transcription-error", "Sidecar not available");
                                                emit_all(&app, "recording-state", "idle");
                                                return;
                                            }
                                        };

                                        // Choose transcription method
                                        let transcribe_result = if transcription_mode == "cloud" {
                                            transcription::cloud_transcribe(sc, &wav_path, &cloud_provider, &cloud_api_key, &language)
                                        } else {
                                            transcription::transcribe_wav(sc, &wav_path, &language)
                                        };

                                        match transcribe_result {
                                            Ok(raw_text) => {
                                                let final_text = if pp_enabled && !raw_text.is_empty() {
                                                    // Load the active profile and post-process
                                                    let profiles_dir = transcription::get_profiles_dir(&app)
                                                        .unwrap_or_default();
                                                    let profiles = transcription::list_profiles(
                                                        sc,
                                                        &profiles_dir.to_string_lossy(),
                                                    )
                                                    .unwrap_or_default();

                                                    let profile = profiles
                                                        .iter()
                                                        .find(|p| p.id == pp_profile_id)
                                                        .cloned();

                                                    if let Some(profile) = profile {
                                                        match transcription::post_process_text(
                                                            sc,
                                                            &raw_text,
                                                            &profile,
                                                            &pp_provider,
                                                            &pp_model,
                                                            &pp_api_key,
                                                            &pp_base_url,
                                                        ) {
                                                            Ok(processed) => processed,
                                                            Err(e) => {
                                                                eprintln!("Post-processing failed, using raw: {}", e);
                                                                emit_all(&app, "transcription-error", format!("Post-processing failed: {}", e));
                                                                raw_text
                                                            }
                                                        }
                                                    } else {
                                                        raw_text
                                                    }
                                                } else {
                                                    raw_text
                                                };

                                                if !final_text.is_empty() {
                                                    // Auto-append period if text doesn't end with punctuation
                                                    let text_to_insert = {
                                                        let trimmed = final_text.trim();
                                                        if !trimmed.is_empty()
                                                            && !trimmed.ends_with('.')
                                                            && !trimmed.ends_with('!')
                                                            && !trimmed.ends_with('?')
                                                            && !trimmed.ends_with(':')
                                                            && !trimmed.ends_with(';')
                                                            && !trimmed.ends_with(',')
                                                        {
                                                            format!("{}.", trimmed)
                                                        } else {
                                                            trimmed.to_string()
                                                        }
                                                    };

                                                    if let Err(e) = text_insert::insert_text(&text_to_insert, hwnd) {
                                                        eprintln!("Text insertion error: {}", e);
                                                        emit_all(&app, "transcription-error", e);
                                                    }
                                                }
                                                text_insert::play_done_sound();
                                                emit_all(&app, "recording-state", "done");
                                            }
                                            Err(e) => {
                                                eprintln!("Transcription error: {}", e);
                                                emit_all(&app, "transcription-error", e);
                                                emit_all(&app, "recording-state", "idle");
                                            }
                                        }
                                    });
                                }
                                Err(e) => {
                                    eprintln!("Failed to save recording: {}", e);
                                    emit_all(&app_handle, "transcription-error", e.to_string());
                                    emit_all(&app_handle, "recording-state", "idle");
                                }
                            }
                        } else {
                            emit_all(&app_handle, "recording-state", "idle");
                        }
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Only hide-on-close for the main window, not the pill
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
