use std::sync::{Arc, Mutex};
use std::time::Instant;

use rdev::{listen, Event, EventType, Key};
use tauri::{AppHandle, Emitter, Manager};

use super::ActiveStyleKey;

// ── Cached hotkey config ────────────────────────────────────────────────────

/// Lightweight cache of the parsed hotkey keys.
/// Updated only when config changes, read on every rdev event.
#[derive(Clone)]
pub struct HotkeyCache(pub Arc<Mutex<HotkeyCacheInner>>);

pub struct HotkeyCacheInner {
    pub dict_key: Option<Key>,
    pub cmd_key: Option<Key>,
}

impl HotkeyCache {
    /// Build a new cache from the raw hotkey config strings.
    pub fn new(hotkey: &str, command_hotkey: &str) -> Self {
        Self(Arc::new(Mutex::new(HotkeyCacheInner {
            dict_key: parse_key(hotkey),
            cmd_key: parse_key(command_hotkey),
        })))
    }

    /// Re-parse keys when the user saves new config.
    pub fn update(&self, hotkey: &str, command_hotkey: &str) {
        if let Ok(mut inner) = self.0.lock() {
            inner.dict_key = parse_key(hotkey);
            inner.cmd_key = parse_key(command_hotkey);
        }
    }
}

// ── Key parsing ─────────────────────────────────────────────────────────────

fn parse_key(name: &str) -> Option<Key> {
    match name {
        "Alt" => Some(Key::Alt),
        "AltLeft" => Some(Key::Alt),
        "AltRight" => Some(Key::AltGr),
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
        "Insert" => Some(Key::Insert),
        // Number row digits
        "Digit0" => Some(Key::Num0),
        "Digit1" => Some(Key::Num1),
        "Digit2" => Some(Key::Num2),
        "Digit3" => Some(Key::Num3),
        "Digit4" => Some(Key::Num4),
        "Digit5" => Some(Key::Num5),
        "Digit6" => Some(Key::Num6),
        "Digit7" => Some(Key::Num7),
        "Digit8" => Some(Key::Num8),
        "Digit9" => Some(Key::Num9),
        // Numpad keys
        "Kp0" => Some(Key::Kp0),
        "Kp1" => Some(Key::Kp1),
        "Kp2" => Some(Key::Kp2),
        "Kp3" => Some(Key::Kp3),
        "Kp4" => Some(Key::Kp4),
        "Kp5" => Some(Key::Kp5),
        "Kp6" => Some(Key::Kp6),
        "Kp7" => Some(Key::Kp7),
        "Kp8" => Some(Key::Kp8),
        "Kp9" => Some(Key::Kp9),
        "KpReturn" => Some(Key::KpReturn),
        "KpPlus" => Some(Key::KpPlus),
        "KpMinus" => Some(Key::KpMinus),
        "KpMultiply" => Some(Key::KpMultiply),
        "KpDivide" => Some(Key::KpDivide),
        "KpDelete" => Some(Key::KpDelete),
        _ => None,
    }
}

/// Convert an rdev Key back to the config string name.
/// This is the reverse of parse_key, plus letter/number keys.
fn key_to_rdev_name(key: &Key) -> String {
    match key {
        Key::Alt => "AltLeft".to_string(),
        Key::AltGr => "AltRight".to_string(),
        Key::Backspace => "BackSpace".to_string(),
        Key::CapsLock => "CapsLock".to_string(),
        Key::ControlLeft => "ControlLeft".to_string(),
        Key::ControlRight => "ControlRight".to_string(),
        Key::Delete => "Delete".to_string(),
        Key::DownArrow => "DownArrow".to_string(),
        Key::End => "End".to_string(),
        Key::Escape => "Escape".to_string(),
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(),
        Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(),
        Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(),
        Key::F12 => "F12".to_string(),
        Key::Home => "Home".to_string(),
        Key::LeftArrow => "LeftArrow".to_string(),
        Key::MetaLeft => "MetaLeft".to_string(),
        Key::MetaRight => "MetaRight".to_string(),
        Key::PageDown => "PageDown".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::Return => "Return".to_string(),
        Key::RightArrow => "RightArrow".to_string(),
        Key::ShiftLeft => "ShiftLeft".to_string(),
        Key::ShiftRight => "ShiftRight".to_string(),
        Key::Space => "Space".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::UpArrow => "UpArrow".to_string(),
        Key::PrintScreen => "PrintScreen".to_string(),
        Key::ScrollLock => "ScrollLock".to_string(),
        Key::Pause => "Pause".to_string(),
        Key::NumLock => "NumLock".to_string(),
        // Letter keys
        Key::KeyA => "A".to_string(),
        Key::KeyB => "B".to_string(),
        Key::KeyC => "C".to_string(),
        Key::KeyD => "D".to_string(),
        Key::KeyE => "E".to_string(),
        Key::KeyF => "F".to_string(),
        Key::KeyG => "G".to_string(),
        Key::KeyH => "H".to_string(),
        Key::KeyI => "I".to_string(),
        Key::KeyJ => "J".to_string(),
        Key::KeyK => "K".to_string(),
        Key::KeyL => "L".to_string(),
        Key::KeyM => "M".to_string(),
        Key::KeyN => "N".to_string(),
        Key::KeyO => "O".to_string(),
        Key::KeyP => "P".to_string(),
        Key::KeyQ => "Q".to_string(),
        Key::KeyR => "R".to_string(),
        Key::KeyS => "S".to_string(),
        Key::KeyT => "T".to_string(),
        Key::KeyU => "U".to_string(),
        Key::KeyV => "V".to_string(),
        Key::KeyW => "W".to_string(),
        Key::KeyX => "X".to_string(),
        Key::KeyY => "Y".to_string(),
        Key::KeyZ => "Z".to_string(),
        // Number keys
        Key::Num0 => "Digit0".to_string(),
        Key::Num1 => "Digit1".to_string(),
        Key::Num2 => "Digit2".to_string(),
        Key::Num3 => "Digit3".to_string(),
        Key::Num4 => "Digit4".to_string(),
        Key::Num5 => "Digit5".to_string(),
        Key::Num6 => "Digit6".to_string(),
        Key::Num7 => "Digit7".to_string(),
        Key::Num8 => "Digit8".to_string(),
        Key::Num9 => "Digit9".to_string(),
        // Numpad keys
        Key::Kp0 => "Kp0".to_string(),
        Key::Kp1 => "Kp1".to_string(),
        Key::Kp2 => "Kp2".to_string(),
        Key::Kp3 => "Kp3".to_string(),
        Key::Kp4 => "Kp4".to_string(),
        Key::Kp5 => "Kp5".to_string(),
        Key::Kp6 => "Kp6".to_string(),
        Key::Kp7 => "Kp7".to_string(),
        Key::Kp8 => "Kp8".to_string(),
        Key::Kp9 => "Kp9".to_string(),
        Key::KpReturn => "KpReturn".to_string(),
        Key::KpPlus => "KpPlus".to_string(),
        Key::KpMinus => "KpMinus".to_string(),
        Key::KpMultiply => "KpMultiply".to_string(),
        Key::KpDivide => "KpDivide".to_string(),
        Key::KpDelete => "KpDelete".to_string(),
        Key::Insert => "Insert".to_string(),
        _ => String::new(),
    }
}

// ── Dictation state machine ─────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum DictationState {
    Idle,
    Pressed,
    WaitingForDoubleTap,
    ToggleRecording,
}

const HOLD_THRESHOLD_MS: u128 = 500;
const DOUBLE_TAP_WINDOW_MS: u128 = 400;

// ── Command state machine ───────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum CommandState {
    Idle,
    Recording,
}

const COMMAND_MIN_HOLD_MS: u128 = 200;

// ── Which mode is actively recording ────────────────────────────────────────

#[derive(Debug, PartialEq, Clone, Copy)]
enum ActiveRecording {
    None,
    Dictation,
    Command,
}

// ── Listener ────────────────────────────────────────────────────────────────

pub fn start_hotkey_listener(app_handle: AppHandle, cache: HotkeyCache) {
    std::thread::spawn(move || {
        // Dictation state
        let mut dict_state = DictationState::Idle;
        let mut dict_press_time: Option<Instant> = None;
        let mut dict_release_time: Option<Instant> = None;

        // Command state
        let mut cmd_state = CommandState::Idle;
        let mut cmd_press_time: Option<Instant> = None;

        // Style key tracking (for two-key styled dictation)
        let mut style_key_held: Option<Key> = None;

        // Shared
        let mut active = ActiveRecording::None;

        let app = app_handle.clone();
        let hotkey_cache = cache.clone();
        let callback = move |event: Event| {
            // Read cached hotkey keys (cheap: only two Option<Key> behind a Mutex)
            let (dict_key, cmd_key) = match hotkey_cache.0.lock() {
                Ok(inner) => (inner.dict_key, inner.cmd_key),
                Err(_) => return,
            };

            // ── Dictation hotkey handling ────────────────────────────────
            if let Some(target_key) = dict_key {
                match event.event_type {
                    EventType::KeyPress(key) if key == target_key => {
                        if active == ActiveRecording::Command {
                            // Command mode active — ignore dictation
                        } else {
                            match dict_state {
                                DictationState::Idle => {
                                    dict_state = DictationState::Pressed;
                                    dict_press_time = Some(Instant::now());
                                    active = ActiveRecording::Dictation;
                                    let _ = app.emit("hotkey-action", "start");
                                }
                                DictationState::Pressed => {
                                    // Key repeat — ignore
                                }
                                DictationState::WaitingForDoubleTap => {
                                    let in_window = dict_release_time
                                        .map(|t| t.elapsed().as_millis() < DOUBLE_TAP_WINDOW_MS)
                                        .unwrap_or(false);

                                    if in_window {
                                        dict_state = DictationState::ToggleRecording;
                                    } else {
                                        dict_state = DictationState::Pressed;
                                        dict_press_time = Some(Instant::now());
                                        active = ActiveRecording::Dictation;
                                        let _ = app.emit("hotkey-action", "start");
                                    }
                                }
                                DictationState::ToggleRecording => {
                                    dict_state = DictationState::Idle;
                                    active = ActiveRecording::None;
                                    let _ = app.emit("hotkey-action", "stop");
                                }
                            }
                        }
                    }

                    // While dictation key is held, another key pressed → activate style
                    // Recording continues — release either key to stop with style applied
                    EventType::KeyPress(key)
                        if dict_state == DictationState::Pressed
                            && key != target_key
                            && active == ActiveRecording::Dictation =>
                    {
                        let key_name = key_to_rdev_name(&key);
                        if !key_name.is_empty() {
                            style_key_held = Some(key);
                            // Store style key in managed state so pipeline can read it
                            if let Ok(mut sk) = app.state::<ActiveStyleKey>().0.lock() {
                                *sk = Some(key_name.clone());
                            }
                            let _ = app.emit("style-switch", key_name);
                        }
                    }

                    // Style key released while dictation key is still held → stop recording
                    EventType::KeyRelease(key)
                        if style_key_held == Some(key)
                            && dict_state == DictationState::Pressed
                            && active == ActiveRecording::Dictation =>
                    {
                        style_key_held = None;
                        dict_state = DictationState::Idle;
                        active = ActiveRecording::None;
                        let _ = app.emit("hotkey-action", "stop");
                    }

                    EventType::KeyRelease(key) if key == target_key => {
                        // Clear style key tracking when dictation key is released
                        style_key_held = None;

                        match dict_state {
                            DictationState::Pressed => {
                                let held_ms = dict_press_time
                                    .map(|t| t.elapsed().as_millis())
                                    .unwrap_or(0);

                                if held_ms >= HOLD_THRESHOLD_MS {
                                    dict_state = DictationState::Idle;
                                    active = ActiveRecording::None;
                                    let _ = app.emit("hotkey-action", "stop");
                                } else {
                                    dict_state = DictationState::WaitingForDoubleTap;
                                    dict_release_time = Some(Instant::now());
                                }
                            }
                            DictationState::ToggleRecording => {
                                // Release during toggle — ignore
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }

                // Check timeout for WaitingForDoubleTap
                if dict_state == DictationState::WaitingForDoubleTap {
                    if let Some(rt) = dict_release_time {
                        if rt.elapsed().as_millis() > DOUBLE_TAP_WINDOW_MS {
                            dict_state = DictationState::Idle;
                            active = ActiveRecording::None;
                            let _ = app.emit("hotkey-action", "stop");
                        }
                    }
                }
            }

            // ── Command hotkey handling ──────────────────────────────────
            if let Some(target_key) = cmd_key {
                match event.event_type {
                    // Command key pressed → start recording immediately
                    EventType::KeyPress(key) if key == target_key => {
                        if active == ActiveRecording::Dictation {
                            // Dictation active — ignore
                        } else if cmd_state == CommandState::Idle {
                            cmd_state = CommandState::Recording;
                            cmd_press_time = Some(Instant::now());
                            active = ActiveRecording::Command;
                            let _ = app.emit("command-hotkey-action", "start");
                        }
                    }

                    // Command key released → stop recording
                    EventType::KeyRelease(key) if key == target_key => {
                        if cmd_state == CommandState::Recording {
                            let held_ms = cmd_press_time
                                .map(|t| t.elapsed().as_millis())
                                .unwrap_or(0);

                            cmd_state = CommandState::Idle;
                            active = ActiveRecording::None;

                            if held_ms >= COMMAND_MIN_HOLD_MS {
                                let _ = app.emit("command-hotkey-action", "stop");
                            } else {
                                // Too short — cancel
                                let _ = app.emit("command-hotkey-action", "cancel");
                            }
                        }
                    }

                    _ => {}
                }
            }
        };

        if let Err(e) = listen(callback) {
            eprintln!("Hotkey listener error: {:?}", e);
        }
    });
}
