mod app;
mod features;
mod infra;

use app::commands::AudioState;
use features::capture::recorder::RecordingState;
use features::output::FocusedWindowState;
use features::settings::ConfigState;
use features::speech::inference::InferenceState;
use features::speech::vocabulary::GlobalDictionaryState;
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
        .manage(InferenceState(Mutex::new(None)))
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
            app::commands::download_model_cmd,
            app::commands::get_gpu_available,
            app::commands::get_gpu_info,
            app::commands::get_model_status,
            app::commands::get_profiles,
            app::commands::save_profile_cmd,
            app::commands::delete_profile_cmd,
            app::commands::test_llm_connection,
            app::commands::set_launch_on_startup,
            app::commands::get_app_info,
            app::commands::quit_app,
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

                match features::speech::inference::InferenceSession::new(&models_dir) {
                    Ok(session) => {
                        let gpu = session.is_gpu();
                        let state = inference_handle.state::<InferenceState>();
                        *state.0.lock().unwrap() = Some(session);
                        let _ = inference_handle.emit("model-status", "ready");
                        if !gpu {
                            let _ = inference_handle.emit("gpu-fallback", "CPU (GPU not available)");
                        }
                    }
                    Err(e) => {
                        eprintln!("[app] Failed to init inference engine: {}", e);
                        let _ = inference_handle.emit("model-status", "error");
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
