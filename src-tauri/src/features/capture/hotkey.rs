use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rdev::{listen, Event, EventType, Key};
use tauri::{AppHandle, Emitter, Manager};

use crate::features::speech;

use super::{
    ActiveStyleKey, DictationRuntimePhase, HotkeyRecordingMode, HotkeyRuntimeState,
};

/// Lightweight cache of the parsed hotkey keys.
/// Updated only when config changes, read on every rdev event.
#[derive(Clone)]
pub struct HotkeyCache(pub Arc<Mutex<HotkeyCacheInner>>);

pub struct HotkeyCacheInner {
    pub dict_key: Option<Key>,
    /// Command hotkey as a chord - all keys must be held simultaneously.
    pub cmd_chord: Vec<Key>,
}

impl HotkeyCache {
    /// Build a new cache from the raw hotkey config strings.
    pub fn new(hotkey: &str, command_hotkey: &str) -> Self {
        Self(Arc::new(Mutex::new(HotkeyCacheInner {
            dict_key: parse_key(hotkey),
            cmd_chord: parse_chord(command_hotkey),
        })))
    }

    /// Re-parse keys when the user saves new config.
    pub fn update(&self, hotkey: &str, command_hotkey: &str) {
        if let Ok(mut inner) = self.0.lock() {
            inner.dict_key = parse_key(hotkey);
            inner.cmd_chord = parse_chord(command_hotkey);
        }
    }
}

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

/// Parse a "+"-delimited chord string like "ControlLeft+ShiftLeft" into a Vec<Key>.
/// Falls back to single-key parsing when no "+" is present.
fn parse_chord(name: &str) -> Vec<Key> {
    if name.is_empty() {
        return Vec::new();
    }

    name.split('+')
        .filter_map(|part| {
            let trimmed = part.trim();
            parse_key(trimmed).or_else(|| {
                if trimmed.len() == 1
                    && trimmed
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_alphabetic())
                        .unwrap_or(false)
                {
                    let upper = trimmed.to_uppercase();
                    parse_key(&format!("Key{}", upper)).or_else(|| match upper.as_str() {
                        "A" => Some(Key::KeyA),
                        "B" => Some(Key::KeyB),
                        "C" => Some(Key::KeyC),
                        "D" => Some(Key::KeyD),
                        "E" => Some(Key::KeyE),
                        "F" => Some(Key::KeyF),
                        "G" => Some(Key::KeyG),
                        "H" => Some(Key::KeyH),
                        "I" => Some(Key::KeyI),
                        "J" => Some(Key::KeyJ),
                        "K" => Some(Key::KeyK),
                        "L" => Some(Key::KeyL),
                        "M" => Some(Key::KeyM),
                        "N" => Some(Key::KeyN),
                        "O" => Some(Key::KeyO),
                        "P" => Some(Key::KeyP),
                        "Q" => Some(Key::KeyQ),
                        "R" => Some(Key::KeyR),
                        "S" => Some(Key::KeyS),
                        "T" => Some(Key::KeyT),
                        "U" => Some(Key::KeyU),
                        "V" => Some(Key::KeyV),
                        "W" => Some(Key::KeyW),
                        "X" => Some(Key::KeyX),
                        "Y" => Some(Key::KeyY),
                        "Z" => Some(Key::KeyZ),
                        _ => None,
                    })
                } else {
                    None
                }
            })
        })
        .collect()
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

#[derive(Debug, PartialEq)]
enum DictationState {
    Idle,
    Pressed,
    WaitingForDoubleTap,
    ToggleRecording,
}

const HOLD_THRESHOLD_MS: u128 = 500;
const DOUBLE_TAP_WINDOW_MS: u128 = 400;

#[derive(Debug, PartialEq)]
enum CommandState {
    Idle,
    Recording,
}

const COMMAND_MIN_HOLD_MS: u128 = 200;

struct ListenerState {
    dict_state: DictationState,
    dict_press_time: Option<Instant>,
    dict_release_time: Option<Instant>,
    cmd_state: CommandState,
    cmd_press_time: Option<Instant>,
    held_keys: HashSet<Key>,
    style_key_held: Option<Key>,
    last_reset_generation: u64,
}

impl ListenerState {
    fn new(last_reset_generation: u64) -> Self {
        Self {
            dict_state: DictationState::Idle,
            dict_press_time: None,
            dict_release_time: None,
            cmd_state: CommandState::Idle,
            cmd_press_time: None,
            held_keys: HashSet::new(),
            style_key_held: None,
            last_reset_generation,
        }
    }

    fn reset_dictation(&mut self) {
        self.dict_state = DictationState::Idle;
        self.dict_press_time = None;
        self.dict_release_time = None;
        self.style_key_held = None;
    }

    fn reset_command(&mut self) {
        self.cmd_state = CommandState::Idle;
        self.cmd_press_time = None;
    }

    fn reset_all(&mut self) {
        self.reset_dictation();
        self.reset_command();
    }
}

fn apply_runtime_reset_if_needed(
    app: &AppHandle,
    runtime: &HotkeyRuntimeState,
    state: &mut ListenerState,
) {
    let reset_generation = runtime
        .reset_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    if reset_generation == state.last_reset_generation {
        return;
    }

    state.last_reset_generation = reset_generation;
    state.reset_all();

    if let Ok(mut active_style_key) = app.state::<ActiveStyleKey>().0.lock() {
        *active_style_key = None;
    }
}

fn cancel_pending_dictation_stop(runtime: &HotkeyRuntimeState) {
    runtime.cancel_pending_dictation_stop();
}

fn schedule_delayed_dictation_stop(app: AppHandle, runtime: HotkeyRuntimeState) {
    let token = runtime
        .dictation_stop_token
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(DOUBLE_TAP_WINDOW_MS as u64));

        if runtime
            .dictation_stop_token
            .load(std::sync::atomic::Ordering::SeqCst)
            != token
        {
            return;
        }

        if runtime.recording_mode() == HotkeyRecordingMode::Dictation {
            let _ = app.emit("hotkey-action", "stop");
        }
    });
}

fn start_dictation_press(app: &AppHandle, state: &mut ListenerState) {
    state.dict_state = DictationState::Pressed;
    state.dict_press_time = Some(Instant::now());
    state.dict_release_time = None;
    state.style_key_held = None;
    let _ = app.emit("hotkey-action", "start");
}

fn stop_dictation(app: &AppHandle, runtime: &HotkeyRuntimeState, state: &mut ListenerState) {
    cancel_pending_dictation_stop(runtime);
    state.reset_dictation();
    let _ = app.emit("hotkey-action", "stop");
}

fn resolve_style_shortcut_key(app: &AppHandle, key: Key) -> Option<String> {
    let key_name = key_to_rdev_name(&key);
    if key_name.is_empty() {
        return None;
    }

    let profiles_dir = speech::get_profiles_dir(app).ok()?;
    let profiles = speech::list_profiles(&profiles_dir).ok()?;
    profiles
        .iter()
        .any(|profile| profile.shortcut_key.eq_ignore_ascii_case(&key_name))
        .then_some(key_name)
}

pub fn start_hotkey_listener(app_handle: AppHandle, cache: HotkeyCache) {
    let hotkey_runtime = app_handle.state::<HotkeyRuntimeState>().inner().clone();

    std::thread::spawn(move || {
        let mut state = ListenerState::new(
            hotkey_runtime
                .reset_generation
                .load(std::sync::atomic::Ordering::SeqCst),
        );

        let app = app_handle.clone();
        let hotkey_cache = cache.clone();
        let runtime = hotkey_runtime.clone();
        let callback = move |event: Event| {
            apply_runtime_reset_if_needed(&app, &runtime, &mut state);

            let key_was_held = match event.event_type {
                EventType::KeyPress(key) => state.held_keys.contains(&key),
                _ => false,
            };

            match &event.event_type {
                EventType::KeyPress(key) => {
                    state.held_keys.insert(*key);
                }
                EventType::KeyRelease(key) => {
                    state.held_keys.remove(key);
                }
                _ => {}
            }

            let (dict_key, cmd_chord) = match hotkey_cache.0.lock() {
                Ok(inner) => (inner.dict_key, inner.cmd_chord.clone()),
                Err(_) => return,
            };

            let backend_mode = runtime.recording_mode();
            let voice_activated = app
                .state::<crate::features::settings::ConfigState>()
                .0
                .lock()
                .map(|config| config.dictation_activation_mode == "voice_activated")
                .unwrap_or(false);

            if let Some(target_key) = dict_key {
                if voice_activated {
                    match event.event_type {
                        EventType::KeyPress(key)
                            if key == target_key
                                && !key_was_held
                                && backend_mode != HotkeyRecordingMode::Command =>
                        {
                            match backend_mode {
                                HotkeyRecordingMode::None => {
                                    state.reset_dictation();
                                    let _ = app.emit("hotkey-action", "start");
                                }
                                HotkeyRecordingMode::Dictation => {
                                    state.reset_dictation();
                                    let action = if runtime.dictation_phase()
                                        == DictationRuntimePhase::Listening
                                    {
                                        "cancel"
                                    } else {
                                        "stop"
                                    };
                                    let _ = app.emit("hotkey-action", action);
                                }
                                HotkeyRecordingMode::Command => {}
                            }
                        }
                        _ => {}
                    }
                } else {
                match event.event_type {
                    EventType::KeyPress(key) if key == target_key => {
                        if backend_mode == HotkeyRecordingMode::Command {
                            // Command mode active - ignore dictation.
                        } else {
                            match state.dict_state {
                                DictationState::Idle => {
                                    if backend_mode == HotkeyRecordingMode::Dictation {
                                        stop_dictation(&app, &runtime, &mut state);
                                    } else {
                                        start_dictation_press(&app, &mut state);
                                    }
                                }
                                DictationState::Pressed => {
                                    // Key repeat - ignore.
                                }
                                DictationState::WaitingForDoubleTap => {
                                    let in_window = state
                                        .dict_release_time
                                        .map(|time| {
                                            time.elapsed().as_millis() < DOUBLE_TAP_WINDOW_MS
                                        })
                                        .unwrap_or(false);

                                    if in_window {
                                        cancel_pending_dictation_stop(&runtime);
                                        state.dict_state = DictationState::ToggleRecording;
                                        state.dict_press_time = None;
                                        state.dict_release_time = None;
                                    } else {
                                        if runtime.recording_mode()
                                            == HotkeyRecordingMode::Dictation
                                        {
                                            stop_dictation(&app, &runtime, &mut state);
                                        } else {
                                            start_dictation_press(&app, &mut state);
                                        }
                                    }
                                }
                                DictationState::ToggleRecording => {
                                    stop_dictation(&app, &runtime, &mut state);
                                }
                            }
                        }
                    }
                    EventType::KeyPress(key)
                        if state.dict_state == DictationState::Pressed
                            && key != target_key
                            && !key_was_held
                            && backend_mode == HotkeyRecordingMode::Dictation =>
                    {
                        if let Some(key_name) = resolve_style_shortcut_key(&app, key) {
                            state.style_key_held = Some(key);
                            if let Ok(mut active_style_key) = app.state::<ActiveStyleKey>().0.lock()
                            {
                                *active_style_key = Some(key_name.clone());
                            }
                            let _ = app.emit("style-switch", key_name);
                        }
                    }
                    EventType::KeyRelease(key)
                        if state.style_key_held == Some(key)
                            && state.dict_state == DictationState::Pressed
                            && backend_mode == HotkeyRecordingMode::Dictation =>
                    {
                        state.style_key_held = None;
                        stop_dictation(&app, &runtime, &mut state);
                    }
                    EventType::KeyRelease(key) if key == target_key => {
                        state.style_key_held = None;

                        match state.dict_state {
                            DictationState::Pressed => {
                                let held_ms = state
                                    .dict_press_time
                                    .map(|time| time.elapsed().as_millis())
                                    .unwrap_or(0);

                                if held_ms >= HOLD_THRESHOLD_MS {
                                    stop_dictation(&app, &runtime, &mut state);
                                } else {
                                    state.dict_state = DictationState::WaitingForDoubleTap;
                                    state.dict_release_time = Some(Instant::now());
                                    schedule_delayed_dictation_stop(app.clone(), runtime.clone());
                                }
                            }
                            DictationState::ToggleRecording => {
                                // Release during toggle - ignore.
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                }
            }

            if !cmd_chord.is_empty() {
                let all_chord_held = cmd_chord.iter().all(|key| state.held_keys.contains(key));

                match event.event_type {
                    EventType::KeyPress(_)
                        if !key_was_held
                            && all_chord_held
                            && state.cmd_state == CommandState::Idle
                            && runtime.recording_mode() == HotkeyRecordingMode::None =>
                    {
                        state.cmd_state = CommandState::Recording;
                        state.cmd_press_time = Some(Instant::now());
                        let _ = app.emit("command-hotkey-action", "start");
                    }
                    EventType::KeyRelease(key)
                        if state.cmd_state == CommandState::Recording
                            && cmd_chord.contains(&key) =>
                    {
                        let held_ms = state
                            .cmd_press_time
                            .map(|time| time.elapsed().as_millis())
                            .unwrap_or(0);

                        state.reset_command();

                        if held_ms >= COMMAND_MIN_HOLD_MS {
                            let _ = app.emit("command-hotkey-action", "stop");
                        } else {
                            let _ = app.emit("command-hotkey-action", "cancel");
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
