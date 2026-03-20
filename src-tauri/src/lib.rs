mod app;
mod features;
mod infra;

use app::commands::AudioState;
use features::capture::recorder::RecordingState;
use features::output::FocusedWindowState;
use features::settings::ConfigState;
use features::speech::vocabulary::GlobalDictionaryState;
use infra::sidecar::SidecarState;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AudioState(Mutex::new(None)))
        .manage(ConfigState(Mutex::new(features::settings::AppConfig::default())))
        .manage(RecordingState(Mutex::new(None)))
        .manage(FocusedWindowState(Mutex::new(0)))
        .manage(SidecarState(Mutex::new(None)))
        .manage(GlobalDictionaryState(Mutex::new(
            features::speech::vocabulary::GlobalDictionary::default(),
        )))
        .invoke_handler(tauri::generate_handler![
            app::commands::list_devices,
            app::commands::start_test,
            app::commands::stop_test,
            app::commands::get_config,
            app::commands::save_config_cmd,
            app::commands::start_recording,
            app::commands::stop_recording,
            app::commands::get_models,
            app::commands::download_model_cmd,
            app::commands::set_whisper_model,
            app::commands::get_gpu_available,
            app::commands::get_sidecar_status,
            app::commands::get_profiles,
            app::commands::save_profile_cmd,
            app::commands::delete_profile_cmd,
            app::commands::test_llm_connection,
            app::commands::set_launch_on_startup,
            app::commands::get_app_info,
            app::commands::quit_app,
            app::commands::get_sidecar_setup_status,
            app::commands::setup_sidecar_cmd,
            app::commands::get_global_dictionary,
            app::commands::save_global_dictionary_cmd,
            app::commands::get_industry_packs,
            app::commands::apply_industry_pack,
            app::commands::preview_sound,
            app::commands::get_available_sounds,
        ])
        .setup(|app| {
            // Load persisted config
            let saved_config = features::settings::load_config(&app.handle());
            let config_state = app.state::<ConfigState>();
            *config_state.0.lock().unwrap() = saved_config.clone();

            // Load global dictionary (auto-apply all industry packs on first install)
            let mut saved_dict =
                features::speech::vocabulary::load_global_dictionary(&app.handle());
            features::speech::vocabulary::auto_apply_all_packs(&app.handle(), &mut saved_dict);
            let dict_state = app.state::<GlobalDictionaryState>();
            *dict_state.0.lock().unwrap() = saved_dict;

            // Build tray menu
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
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
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Position pill window above the taskbar, centered
            if let Some(pill) = app.get_webview_window("pill") {
                let _ =
                    pill.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

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
                let y = work_area.bottom - pill_height - 20;
                let _ = pill.set_position(tauri::PhysicalPosition::new(x, y));
            }

            // Start minimized: hide main window if configured
            if saved_config.start_minimized {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.hide();
                }
            }

            // Start global hotkey listener
            features::capture::hotkey::start_hotkey_listener(app.handle().clone());

            // Product policy: always use tiny model.
            // Enforce at startup in case config was edited externally.
            {
                let config_state = app.state::<ConfigState>();
                let mut guard = config_state.0.lock().unwrap();
                if guard.whisper_model != "tiny" {
                    guard.whisper_model = "tiny".to_string();
                    let _ = features::settings::save_config(&app.handle(), &guard);
                }
            }

            // Ensure bundled Python environment is copied to AppData
            if !cfg!(debug_assertions) {
                if let Err(e) = infra::sidecar::ensure_bundled_env_copied(&app.handle()) {
                    eprintln!("[app] Failed to copy bundled Python env: {}", e);
                }
            }

            // Spawn sidecar in the background
            let sidecar_handle = app.handle().clone();
            let sidecar_config = app.state::<ConfigState>().0.lock().unwrap().clone();
            std::thread::spawn(move || {
                let _ = infra::sidecar::cleanup_models(&sidecar_handle, "tiny");

                match infra::sidecar::spawn_sidecar(&sidecar_handle) {
                    Ok(mut sc) => {
                        let models_dir = infra::sidecar::get_models_dir(&sidecar_handle)
                            .unwrap_or_default();
                        match features::speech::load_model(
                            &mut sc,
                            "tiny",
                            &sidecar_config.device,
                            &sidecar_config.compute_type,
                            &models_dir.to_string_lossy(),
                        ) {
                            Ok(()) => {
                                eprintln!("[app] Sidecar started and tiny model loaded")
                            }
                            Err(e) => eprintln!(
                                "[app] Sidecar started but tiny model failed to load: {}",
                                e
                            ),
                        }
                        let state = sidecar_handle.state::<SidecarState>();
                        *state.0.lock().unwrap() = Some(sc);
                        let _ = sidecar_handle.emit("sidecar-status", "running");
                    }
                    Err(e) => {
                        eprintln!(
                            "[app] Failed to start sidecar (will retry on first transcription): {}",
                            e
                        );
                        let _ = sidecar_handle.emit("sidecar-status", "stopped");
                    }
                }
            });

            // Set up the hotkey-action event handler (record → transcribe → insert pipeline)
            features::capture::setup_hotkey_handler(&app.handle());

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
