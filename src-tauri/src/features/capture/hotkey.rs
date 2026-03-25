use std::time::Instant;

use rdev::{listen, Event, EventType, Key};
use tauri::{AppHandle, Emitter, Manager};

use crate::features::settings::ConfigState;

fn parse_hotkey(hotkey_str: &str) -> Option<Key> {
    let primary = hotkey_str.split('+').last().unwrap_or(hotkey_str);
    match primary {
        "Alt" => Some(Key::Alt),
        "Backspace" | "BackSpace" => Some(Key::Backspace),
        "CapsLock" => Some(Key::CapsLock),
        "ControlLeft" => Some(Key::ControlLeft),
        "ControlRight" => Some(Key::ControlRight),
        "Delete" => Some(Key::Delete),
        "DownArrow" => Some(Key::DownArrow),
        "End" => Some(Key::End),
        "Escape" => Some(Key::Escape),
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        "Home" => Some(Key::Home),
        "LeftArrow" => Some(Key::LeftArrow),
        "MetaLeft" => Some(Key::MetaLeft),
        "MetaRight" => Some(Key::MetaRight),
        "PageDown" => Some(Key::PageDown),
        "PageUp" => Some(Key::PageUp),
        "Return" | "Enter" => Some(Key::Return),
        "RightArrow" => Some(Key::RightArrow),
        "ShiftLeft" => Some(Key::ShiftLeft),
        "ShiftRight" => Some(Key::ShiftRight),
        "Space" => Some(Key::Space),
        "Tab" => Some(Key::Tab),
        "UpArrow" => Some(Key::UpArrow),
        "PrintScreen" => Some(Key::PrintScreen),
        "ScrollLock" => Some(Key::ScrollLock),
        "Pause" => Some(Key::Pause),
        "NumLock" => Some(Key::NumLock),
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Idle,
    Pressed,
    WaitingForDoubleTap,
    ToggleRecording,
}

const HOLD_THRESHOLD_MS: u128 = 500;
const DOUBLE_TAP_WINDOW_MS: u128 = 400;

pub fn start_hotkey_listener(app_handle: AppHandle) {
    std::thread::spawn(move || {
        let mut state = State::Idle;
        let mut press_time: Option<Instant> = None;
        let mut release_time: Option<Instant> = None;

        let app = app_handle.clone();
        let callback = move |event: Event| {
            let config_state = app.state::<ConfigState>();
            let config = match config_state.0.lock() {
                Ok(c) => c.clone(),
                Err(_) => return,
            };

            let target_key = match parse_hotkey(&config.hotkey) {
                Some(k) => k,
                None => return,
            };

            match event.event_type {
                EventType::KeyPress(key) if key == target_key => {
                    match state {
                        State::Idle => {
                            state = State::Pressed;
                            press_time = Some(Instant::now());
                            let _ = app.emit("hotkey-action", "start");
                        }
                        State::Pressed => {
                            // Key repeat — ignore
                        }
                        State::WaitingForDoubleTap => {
                            let in_window = release_time
                                .map(|t| t.elapsed().as_millis() < DOUBLE_TAP_WINDOW_MS)
                                .unwrap_or(false);

                            if in_window {
                                state = State::ToggleRecording;
                            } else {
                                state = State::Pressed;
                                press_time = Some(Instant::now());
                                let _ = app.emit("hotkey-action", "start");
                            }
                        }
                        State::ToggleRecording => {
                            state = State::Idle;
                            let _ = app.emit("hotkey-action", "stop");
                        }
                    }
                }
                EventType::KeyRelease(key) if key == target_key => {
                    match state {
                        State::Pressed => {
                            let held_ms =
                                press_time.map(|t| t.elapsed().as_millis()).unwrap_or(0);

                            if held_ms >= HOLD_THRESHOLD_MS {
                                state = State::Idle;
                                let _ = app.emit("hotkey-action", "stop");
                            } else {
                                state = State::WaitingForDoubleTap;
                                release_time = Some(Instant::now());
                            }
                        }
                        State::ToggleRecording => {
                            // Release during toggle — ignore
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            // Check timeout for WaitingForDoubleTap
            if state == State::WaitingForDoubleTap {
                if let Some(rt) = release_time {
                    if rt.elapsed().as_millis() > DOUBLE_TAP_WINDOW_MS {
                        state = State::Idle;
                        let _ = app.emit("hotkey-action", "stop");
                    }
                }
            }
        };

        if let Err(e) = listen(callback) {
            eprintln!("Hotkey listener error: {:?}", e);
        }
    });
}
