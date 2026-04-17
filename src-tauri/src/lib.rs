mod app;
mod features;
mod infra;

use app::commands::{AudioState, OnboardingPreviewState};
use features::capture::recorder::{RecordingState, WarmDeviceState};
use features::capture::{
    ActiveStyleKey, ContinuousGeneration, HotkeyRuntimeState, RuntimeDictionaryCache,
};
use features::diagnostics::{maybe_log_support_event, TranscriptDiagnosticsState};
use features::output::FocusedWindowState;
use features::settings::ConfigState;
use features::speech::distil_whisper::DistilWhisperState;
use features::speech::inference::InferenceState;
use features::speech::vocabulary::{UserDictionaryMigration, UserDictionaryState};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AudioState(Mutex::new(None)))
        .manage(OnboardingPreviewState(Mutex::new(None)))
        .manage(ConfigState(Mutex::new(features::settings::AppConfig::default())))
        .manage(RecordingState(Mutex::new(None)))
        .manage(WarmDeviceState(Mutex::new(None)))
        .manage(FocusedWindowState(Mutex::new(0)))
        .manage(InferenceState(Mutex::new(None)))
        .manage(DistilWhisperState(Mutex::new(
            features::speech::distil_whisper::DistilWhisperManager::default(),
        )))
        .manage(RuntimeDictionaryCache(Mutex::new(None)))
        .manage(ActiveStyleKey(Mutex::new(None)))
        .manage(HotkeyRuntimeState::new())
        .manage(ContinuousGeneration(std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0))))
        .manage(UserDictionaryState(Mutex::new(
            features::speech::vocabulary::UserDictionary::default(),
        )))
        .invoke_handler(tauri::generate_handler![
            app::commands::list_devices,
            app::commands::start_test,
            app::commands::stop_test,
            app::commands::get_config,
            app::commands::get_storage_overview,
            app::commands::open_storage_location,
            app::commands::save_config_cmd,
            app::commands::get_text_actions,
            app::commands::save_text_action,
            app::commands::delete_text_action,
            app::commands::reset_text_action_to_default,
            app::commands::start_recording,
            app::commands::stop_recording,
            app::commands::start_onboarding_preview_recording,
            app::commands::cancel_onboarding_preview_recording,
            app::commands::finish_onboarding_preview,
            app::commands::download_model_cmd,
            app::commands::cancel_model_download_cmd,
            app::commands::delete_model_cmd,
            app::commands::get_gpu_available,
            app::commands::get_gpu_info,
            app::commands::reload_model_cmd,
            app::commands::get_model_status,
            app::commands::open_distil_whisper_model_page_cmd,
            app::commands::get_distil_whisper_model_status,
            app::commands::download_distil_whisper_model_cmd,
            app::commands::prepare_distil_whisper_model_cmd,
            app::commands::reload_distil_whisper_model_cmd,
            app::commands::delete_distil_whisper_model_cmd,
            app::commands::get_profiles,
            app::commands::save_profile_cmd,
            app::commands::delete_profile_cmd,
            app::commands::reset_profile_to_default,
            app::commands::test_llm_connection,
            app::commands::set_launch_on_startup,
            app::commands::get_app_info,
            app::commands::quit_app,
            app::commands::get_industry_packs,
            app::commands::apply_industry_pack,
            app::commands::get_transcript_diagnostics_status,
            app::commands::clear_transcript_diagnostics,
            app::commands::export_support_diagnostics,
            app::commands::export_transcript_history,
            app::commands::preview_sound,
            app::commands::get_available_sounds,
            app::commands::test_command_llm_connection,
            app::commands::load_industry_pack_cmd,
            app::commands::get_general_vocabulary,
            app::commands::save_general_vocabulary_cmd,
            app::commands::save_industry_pack_cmd,
            app::commands::reset_industry_pack_cmd,
            app::commands::generate_vocab_variants,
            app::commands::get_transcript_history,
            app::commands::clear_transcript_history,
            app::commands::delete_transcript_entry,
            app::commands::get_transcript_entry_words,
            app::commands::add_words_to_dictionary,
        ])
        .setup(|app| {
            // Load persisted config
            let mut saved_config = features::settings::load_config(&app.handle());
            let text_actions_changed =
                features::speech::ensure_text_actions_ready(&app.handle(), &mut saved_config)
                    .unwrap_or_else(|err| {
                        eprintln!("[app] Failed to initialize text actions: {}", err);
                        false
                    });
            let config_state = app.state::<ConfigState>();
            *config_state.0.lock().unwrap() = saved_config.clone();
            if text_actions_changed {
                if let Err(err) = features::settings::save_config(&app.handle(), &saved_config) {
                    eprintln!("[app] Failed to persist text action migration: {}", err);
                }
            }

            // Load user dictionary and migrate legacy merged dictionaries if needed.
            let load_result = features::speech::vocabulary::load_user_dictionary(&app.handle());
            if let UserDictionaryMigration::LegacyReset { backup_path } = &load_result.migration {
                let mut config_guard = config_state.0.lock().unwrap();
                config_guard.show_dictionary_migration_notice = true;
                if let Err(err) = features::settings::save_config(&app.handle(), &config_guard) {
                    eprintln!("[app] Failed to persist dictionary migration notice: {}", err);
                }
                eprintln!(
                    "[app] Reset legacy merged dictionary and wrote backup to {}",
                    backup_path.display()
                );
            }

            let dict_state = app.state::<UserDictionaryState>();
            *dict_state.0.lock().unwrap() = load_result.dictionary;

            let diagnostics_store =
                features::diagnostics::TranscriptDiagnosticsStore::new(&app.handle())?;
            app.manage(TranscriptDiagnosticsState(diagnostics_store));

            // Build tray menu
            let show_item = MenuItem::with_id(app, "show", "Show app", true, None::<&str>)?;
            let transcriptions_item =
                MenuItem::with_id(app, "transcriptions", "Transcriptions", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let history_item = MenuItem::with_id(app, "history", "History", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &transcriptions_item,
                    &settings_item,
                    &separator,
                    &history_item,
                    &quit_item,
                ],
            )?;

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
                    "transcriptions" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                        let _ = app.emit("open-settings-section", "transcription");
                    }
                    "settings" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                        let _ = app.emit("open-settings-section", "general");
                    }
                    "history" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                        let _ = app.emit("open-settings-section", "history");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    {
                        if button == MouseButton::Left && button_state == MouseButtonState::Up {
                            if let Some(w) = tray.app_handle().get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Position pill window above the taskbar/dock, centered
            if let Some(pill) = app.get_webview_window("pill") {
                let _ =
                    pill.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

                let pill_width = 280i32;
                let pill_height = 50i32;

                #[cfg(windows)]
                {
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
                    let x = (work_area.right - work_area.left - pill_width) / 2 + work_area.left;
                    let y = work_area.bottom - pill_height - 20;
                    let _ = pill.set_position(tauri::PhysicalPosition::new(x, y));
                }

                #[cfg(not(windows))]
                {
                    // Use Tauri monitor API for cross-platform positioning
                    if let Ok(Some(monitor)) = pill.current_monitor() {
                        let size = monitor.size();
                        let pos = monitor.position();
                        let x = pos.x + (size.width as i32 - pill_width) / 2;
                        let y = pos.y + size.height as i32 - pill_height - 80;
                        let _ = pill.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                }

                // Install WinEvent hooks that re-assert HWND_TOPMOST whenever
                // another app (Electron, Discord, etc.) steals z-order.
                #[cfg(windows)]
                if let Ok(hwnd) = pill.hwnd() {
                    infra::topmost_guard::install(hwnd.0);
                }
            }

            // Start minimized: hide main window if configured
            if saved_config.start_minimized {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.hide();
                }
            }

            // Start global hotkey listener with cached keys
            let hotkey_cache = features::capture::hotkey::HotkeyCache::new(
                &saved_config.hotkey,
                &saved_config.command_hotkey,
            );
            app.manage(hotkey_cache.clone());
            features::capture::hotkey::start_hotkey_listener(app.handle().clone(), hotkey_cache);

            // Pre-warm the audio device so the first recording starts faster
            features::capture::recorder::spawn_warm_device(
                &app.handle(),
                saved_config.device_index,
            );

            // Clean up old whisper models from previous versions
            let _ = infra::model::cleanup_old_models(&app.handle());

            // Ensure default profiles are seeded
            if let Ok(profiles_dir) = features::speech::profiles::get_profiles_dir(&app.handle()) {
                let _ = features::speech::profiles::ensure_profiles_dir(&profiles_dir, &app.handle());
            }

            // Initialize inference engine in the background
            let inference_handle = app.handle().clone();
            std::thread::spawn(move || {
                let models_dir = match infra::model::get_models_dir(&inference_handle) {
                    Ok(dir) => dir,
                    Err(e) => {
                        eprintln!("[app] Failed to get models dir: {}", e);
                        let _ = inference_handle.emit("model-status", "error");
                        return;
                    }
                };

                if !infra::model::is_model_downloaded(&models_dir) {
                    eprintln!("[app] Model not downloaded yet");
                    let _ = inference_handle.emit("model-status", "not-downloaded");
                    return;
                }

                let _ = inference_handle.emit("model-status", "loading");
                maybe_log_support_event(
                    &inference_handle,
                    "parakeet",
                    "startup_load_requested",
                    "Initializing Parakeet model during app startup",
                    serde_json::json!({}),
                );

                match features::speech::inference::InferenceSession::new(&models_dir) {
                    Ok(session) => {
                        let gpu = session.is_gpu();
                        let state = inference_handle.state::<InferenceState>();
                        match state.0.lock() {
                            Ok(mut g) => *g = Some(session),
                            Err(e) => {
                                eprintln!("[app] InferenceState mutex poisoned: {}", e);
                                let _ = inference_handle.emit("model-status", "error");
                                return;
                            }
                        }
                        maybe_log_support_event(
                            &inference_handle,
                            "parakeet",
                            "startup_load_success",
                            "Initialized Parakeet model during app startup",
                            serde_json::json!({
                                "gpu": gpu,
                            }),
                        );
                        let _ = inference_handle.emit("model-status", "ready");
                        if !gpu {
                            let _ = inference_handle.emit("gpu-fallback", "CPU (GPU not available)");
                        }
                    }
                    Err(e) => {
                        eprintln!("[app] Failed to init inference engine: {}", e);
                        maybe_log_support_event(
                            &inference_handle,
                            "parakeet",
                            "startup_load_error",
                            "Failed to initialize Parakeet model during app startup",
                            serde_json::json!({
                                "error": e,
                            }),
                        );
                        let _ = inference_handle.emit("model-status", "error");
                    }
                }

            });

            // Set up the hotkey-action event handler (record → transcribe → insert pipeline)
            if saved_config.transcription_mode == "offline"
                && saved_config.offline_engine == "distil_whisper"
            {
                let _ = features::speech::distil_whisper::maybe_prepare_in_background(&app.handle());
            }
            features::capture::setup_hotkey_handler(&app.handle());

            // Set up the command-hotkey-action event handler (command pipeline)
            features::capture::setup_command_hotkey_handler(&app.handle());

            // Set up the style-switch event handler (command key + letter)
            features::capture::setup_style_switch_handler(&app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
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
