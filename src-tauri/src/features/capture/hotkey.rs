use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rdev::{listen, Event, EventType, Key};
use tauri::{AppHandle, Emitter, Manager};

use crate::features::speech;

use super::{ActiveStyleKey, DictationRuntimePhase, HotkeyRecordingMode, HotkeyRuntimeState};

/// Lightweight cache of hotkey-related state for the rdev callback.
/// Updated only when config/profiles change — never reads disk or ConfigState
/// on the keyboard-hook thread.
#[derive(Clone)]
pub struct HotkeyCache(pub Arc<Mutex<HotkeyCacheInner>>);

pub struct HotkeyCacheInner {
    pub dict_key: Option<Key>,
    /// Command hotkey as a chord — all keys must be held simultaneously.
    pub cmd_chord: Arc<Vec<Key>>,
    pub voice_activated: bool,
    /// Style profile shortcut keys: rdev Key → config shortcut name.
    pub style_shortcuts: Arc<HashMap<Key, String>>,
}

impl HotkeyCache {
    pub fn new(
        hotkey: &str,
        command_hotkey: &str,
        voice_activated: bool,
        style_shortcuts: HashMap<Key, String>,
    ) -> Self {
        Self(Arc::new(Mutex::new(HotkeyCacheInner {
            dict_key: parse_key(hotkey),
            cmd_chord: Arc::new(parse_chord(command_hotkey)),
            voice_activated,
            style_shortcuts: Arc::new(style_shortcuts),
        })))
    }

    /// Re-parse keys / activation mode when the user saves new config.
    pub fn update(&self, hotkey: &str, command_hotkey: &str, voice_activated: bool) {
        if let Ok(mut inner) = self.0.lock() {
            inner.dict_key = parse_key(hotkey);
            inner.cmd_chord = Arc::new(parse_chord(command_hotkey));
            inner.voice_activated = voice_activated;
        }
    }

    pub fn update_style_shortcuts(&self, style_shortcuts: HashMap<Key, String>) {
        if let Ok(mut inner) = self.0.lock() {
            inner.style_shortcuts = Arc::new(style_shortcuts);
        }
    }
}

/// Build the style-shortcut map from on-disk profiles (call off the hook thread).
pub fn load_style_shortcuts(app: &AppHandle) -> HashMap<Key, String> {
    let mut map = HashMap::new();
    let Ok(profiles_dir) = speech::get_profiles_dir(app) else {
        return map;
    };
    let Ok(profiles) = speech::list_profiles(&profiles_dir) else {
        return map;
    };
    for profile in profiles {
        let name = profile.shortcut_key.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(key) = parse_key(name) {
            map.insert(key, name.to_string());
        }
    }
    map
}

fn parse_letter_key(name: &str) -> Option<Key> {
    let letter = if name.len() == 1 {
        name.chars().next()
    } else if let Some(rest) = name.strip_prefix("Key") {
        if rest.len() == 1 {
            rest.chars().next()
        } else {
            None
        }
    } else {
        None
    }?;

    if !letter.is_ascii_alphabetic() {
        return None;
    }

    match letter.to_ascii_uppercase() {
        'A' => Some(Key::KeyA),
        'B' => Some(Key::KeyB),
        'C' => Some(Key::KeyC),
        'D' => Some(Key::KeyD),
        'E' => Some(Key::KeyE),
        'F' => Some(Key::KeyF),
        'G' => Some(Key::KeyG),
        'H' => Some(Key::KeyH),
        'I' => Some(Key::KeyI),
        'J' => Some(Key::KeyJ),
        'K' => Some(Key::KeyK),
        'L' => Some(Key::KeyL),
        'M' => Some(Key::KeyM),
        'N' => Some(Key::KeyN),
        'O' => Some(Key::KeyO),
        'P' => Some(Key::KeyP),
        'Q' => Some(Key::KeyQ),
        'R' => Some(Key::KeyR),
        'S' => Some(Key::KeyS),
        'T' => Some(Key::KeyT),
        'U' => Some(Key::KeyU),
        'V' => Some(Key::KeyV),
        'W' => Some(Key::KeyW),
        'X' => Some(Key::KeyX),
        'Y' => Some(Key::KeyY),
        'Z' => Some(Key::KeyZ),
        _ => None,
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
        other => parse_letter_key(other),
    }
}

/// Parse a "+"-delimited chord string like "ControlLeft+ShiftLeft" into a Vec<Key>.
/// Falls back to single-key parsing when no "+" is present.
fn parse_chord(name: &str) -> Vec<Key> {
    if name.is_empty() {
        return Vec::new();
    }

    name.split('+')
        .filter_map(|part| parse_key(part.trim()))
        .collect()
}

/// Convert an rdev Key back to the config string name.
/// This is the reverse of parse_key, plus letter/number keys.
#[cfg(test)]
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

/// Actions enqueued by the hook callback; executed on a dedicated worker thread
/// so heavy Tauri handlers never run inside the OS low-level keyboard hook.
#[derive(Debug)]
enum HotkeyAction {
    DictationStart,
    DictationStop,
    DictationCancel,
    StyleSwitch(String),
    CommandStart,
    CommandStop,
    CommandCancel,
}

#[derive(Debug, PartialEq)]
enum DictationState {
    Idle,
    Pressed,
    WaitingForDoubleTap,
    ToggleRecording,
}

const HOLD_THRESHOLD_MS: u128 = 500;
const DOUBLE_TAP_WINDOW_MS: u128 = 600;
const START_PENDING_WAIT_MS: u64 = 2000;
const ACTION_CHANNEL_CAPACITY: usize = 64;
/// Separate from the action channel so a burst of diagnostics can never crowd out a real
/// hotkey action. Overflow drops log lines, which is the correct trade here.
const HOOK_LOG_CHANNEL_CAPACITY: usize = 256;
const LISTENER_HEALTH_POLL_MS: u64 = 2000;
/// Must be comfortably larger than the poll interval. At 2000/2000 a single keystroke
/// landing just after a poll sample, followed by a normal reading pause, reads as a dead
/// hook — and every false positive costs a leaked thread and a leaked OS hook (see
/// `start_hotkey_listener`).
const HOOK_STALE_AFTER_ACTIVITY_MS: u128 = 15_000;
const HOOK_STALE_CONFIRM_MS: u64 = 1000;
/// Consecutive stale observations required before declaring the hook dead.
const HOOK_DEAD_CONSECUTIVE: u32 = 3;
/// Reinstalling leaks a thread and an OS hook every time, so cap the damage rather than
/// letting a persistently-unhealthy machine restart forever.
const MAX_LISTENER_RESTARTS: u32 = 5;

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
    log_tx: SyncSender<String>,
}

impl ListenerState {
    fn new(last_reset_generation: u64, log_tx: SyncSender<String>) -> Self {
        Self {
            dict_state: DictationState::Idle,
            dict_press_time: None,
            dict_release_time: None,
            cmd_state: CommandState::Idle,
            cmd_press_time: None,
            held_keys: HashSet::new(),
            style_key_held: None,
            last_reset_generation,
            log_tx,
        }
    }

    /// Queue a log line for the log worker. Never touches the log sink directly —
    /// this is called from the WH_KEYBOARD_LL callback, where a synchronous file
    /// write can blow the OS hook timeout and get us silently unhooked.
    fn log(&self, message: String) {
        let _ = self.log_tx.try_send(message);
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
        // Held keys are part of the listener's view of the world; leaving them behind
        // after a resync lets a stale modifier complete a chord the user never pressed.
        self.held_keys.clear();
    }
}

/// Drop modifiers we think are held but that Windows reports as physically up.
///
/// The hook stops receiving events across a secure-desktop transition (UAC prompt,
/// Ctrl+Alt+Del), so key-up for anything held at that moment is never delivered and
/// `held_keys` keeps it forever. Modifiers are the ones that matter: they don't
/// auto-repeat, so nothing else ever corrects them, and a stale pair is enough to make
/// the next unrelated keypress look like a completed command chord.
#[cfg(windows)]
fn prune_stale_modifiers(held: &mut HashSet<Key>) {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    fn vk_for(key: &Key) -> Option<i32> {
        Some(match key {
            Key::ShiftLeft => 0xA0,
            Key::ShiftRight => 0xA1,
            Key::ControlLeft => 0xA2,
            Key::ControlRight => 0xA3,
            Key::Alt => 0xA4,
            Key::AltGr => 0xA5,
            Key::MetaLeft => 0x5B,
            Key::MetaRight => 0x5C,
            _ => return None,
        })
    }

    held.retain(|key| match vk_for(key) {
        // High bit set means the key is currently down.
        Some(vk) => unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 },
        None => true,
    });
}

#[cfg(not(windows))]
fn prune_stale_modifiers(_held: &mut HashSet<Key>) {}

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

fn enqueue_action(
    tx: &SyncSender<HotkeyAction>,
    runtime: &HotkeyRuntimeState,
    action: HotkeyAction,
) {
    let is_start = matches!(action, HotkeyAction::DictationStart);
    if is_start {
        runtime.set_dictation_start_pending(true);
    }

    if let Err(err) = tx.try_send(action) {
        log::error!(
            target: "yolo_voice::hotkey",
            "hotkey action queue full or disconnected: {err}"
        );
        if is_start {
            // Start never reached the worker — don't leave the pending flag stuck.
            runtime.set_dictation_start_pending(false);
        }
    }
}

fn schedule_delayed_dictation_stop(
    runtime: HotkeyRuntimeState,
    tx: SyncSender<HotkeyAction>,
) {
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

        // Wait for a slow start (cold device init) so a short tap still stops.
        let deadline = Instant::now() + Duration::from_millis(START_PENDING_WAIT_MS);
        while runtime.dictation_start_pending() {
            if Instant::now() >= deadline {
                break;
            }
            if runtime
                .dictation_stop_token
                .load(std::sync::atomic::Ordering::SeqCst)
                != token
            {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }

        if runtime
            .dictation_stop_token
            .load(std::sync::atomic::Ordering::SeqCst)
            != token
        {
            return;
        }

        if runtime.recording_mode() == HotkeyRecordingMode::Dictation
            || runtime.dictation_start_pending()
        {
            log::info!(target: "yolo_voice::hotkey", "delayed short-tap stop emitted");
            enqueue_action(&tx, &runtime, HotkeyAction::DictationStop);
        }
    });
}

fn start_dictation_press(
    runtime: &HotkeyRuntimeState,
    tx: &SyncSender<HotkeyAction>,
    state: &mut ListenerState,
) {
    state.log("dictation start enqueued".to_string());
    state.dict_state = DictationState::Pressed;
    state.dict_press_time = Some(Instant::now());
    state.dict_release_time = None;
    state.style_key_held = None;
    enqueue_action(tx, runtime, HotkeyAction::DictationStart);
}

fn stop_dictation(
    runtime: &HotkeyRuntimeState,
    tx: &SyncSender<HotkeyAction>,
    state: &mut ListenerState,
) {
    state.log("dictation stop enqueued".to_string());
    cancel_pending_dictation_stop(runtime);
    state.reset_dictation();
    enqueue_action(tx, runtime, HotkeyAction::DictationStop);
}

/// Drains hook-thread log lines. Dropping lines under flood is fine — these are
/// diagnostics, and the alternative is stalling the keyboard hook.
fn start_hook_log_worker(rx: Receiver<String>) {
    thread::spawn(move || {
        while let Ok(line) = rx.recv() {
            log::info!(target: "yolo_voice::hotkey", "{line}");
        }
    });
}

fn start_action_worker(app: AppHandle, rx: Receiver<HotkeyAction>) {
    thread::spawn(move || {
        log::info!(target: "yolo_voice::hotkey", "hotkey action worker started");
        while let Ok(action) = rx.recv() {
            match action {
                HotkeyAction::DictationStart => {
                    let _ = app.emit("hotkey-action", "start");
                }
                HotkeyAction::DictationStop => {
                    let _ = app.emit("hotkey-action", "stop");
                }
                HotkeyAction::DictationCancel => {
                    let _ = app.emit("hotkey-action", "cancel");
                }
                HotkeyAction::StyleSwitch(key_name) => {
                    let _ = app.emit("style-switch", key_name);
                }
                HotkeyAction::CommandStart => {
                    let _ = app.emit("command-hotkey-action", "start");
                }
                HotkeyAction::CommandStop => {
                    let _ = app.emit("command-hotkey-action", "stop");
                }
                HotkeyAction::CommandCancel => {
                    let _ = app.emit("command-hotkey-action", "cancel");
                }
            }
        }
        log::warn!(target: "yolo_voice::hotkey", "hotkey action worker exited");
    });
}

#[cfg(windows)]
fn last_system_input_tick() -> Option<u32> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut info).as_bool() {
            Some(info.dwTime)
        } else {
            None
        }
    }
}

/// True when the OS reports recent input but the hook has gone silent —
/// the usual signature of Windows silently removing a slow WH_KEYBOARD_LL hook.
fn hook_appears_dead(
    runtime: &HotkeyRuntimeState,
    last_seen_input_tick: &mut Option<u32>,
    instance_started: Instant,
) -> bool {
    // No event ever delivered is itself a symptom: the hook installed but is deaf.
    // Measure from listener start so this case is covered rather than skipped.
    let hook_silent_for = runtime
        .last_hook_event_elapsed()
        .unwrap_or_else(|| instance_started.elapsed());

    #[cfg(windows)]
    {
        let Some(input_tick) = last_system_input_tick() else {
            return false;
        };

        let input_changed = match *last_seen_input_tick {
            Some(prev) => input_tick != prev,
            None => {
                *last_seen_input_tick = Some(input_tick);
                return false;
            }
        };
        *last_seen_input_tick = Some(input_tick);

        if !input_changed {
            return false;
        }

        if hook_silent_for.as_millis() <= HOOK_STALE_AFTER_ACTIVITY_MS {
            return false;
        }

        // Give a live hook a moment to deliver the event that moved GetLastInputInfo.
        thread::sleep(Duration::from_millis(HOOK_STALE_CONFIRM_MS));
        runtime
            .last_hook_event_elapsed()
            .unwrap_or_else(|| instance_started.elapsed())
            .as_millis()
            > HOOK_STALE_AFTER_ACTIVITY_MS
    }

    #[cfg(not(windows))]
    {
        let _ = last_seen_input_tick;
        let _ = hook_silent_for;
        false
    }
}

fn run_listener_instance(
    app: AppHandle,
    cache: HotkeyCache,
    runtime: HotkeyRuntimeState,
    tx: SyncSender<HotkeyAction>,
    log_tx: SyncSender<String>,
    generation: u64,
) -> Result<(), rdev::ListenError> {
    let mut state = ListenerState::new(
        runtime
            .reset_generation
            .load(std::sync::atomic::Ordering::SeqCst),
        log_tx,
    );

    let callback = move |event: Event| {
        if runtime.listener_generation() != generation {
            return;
        }

        // Heartbeat for every delivered event (including mouse) proves the hook is alive.
        runtime.mark_hook_event();

        // Never do hotkey work for mouse / non-key events on the hook thread.
        let (is_press, is_release, key) = match event.event_type {
            EventType::KeyPress(key) => (true, false, Some(key)),
            EventType::KeyRelease(key) => (false, true, Some(key)),
            _ => return,
        };
        let key = match key {
            Some(k) => k,
            None => return,
        };

        apply_runtime_reset_if_needed(&app, &runtime, &mut state);

        let key_was_held = is_press && state.held_keys.contains(&key);

        if is_press {
            state.held_keys.insert(key);
        } else if is_release {
            state.held_keys.remove(&key);
        }

        let (dict_key, cmd_chord, voice_activated, style_shortcuts) =
            match cache.0.lock() {
                Ok(inner) => (
                    inner.dict_key,
                    Arc::clone(&inner.cmd_chord),
                    inner.voice_activated,
                    Arc::clone(&inner.style_shortcuts),
                ),
                Err(_) => return,
            };

        let backend_mode = runtime.recording_mode();

        if let Some(target_key) = dict_key {
            if key == target_key {
                // Formatting is cheap; the log *write* is not (mutex + synchronous file
                // I/O + rotation), so hand the line to a worker instead of blocking the
                // WH_KEYBOARD_LL callback on it.
                state.log(format!(
                    "event={:?} repeat={} dict_state={:?} backend_mode={:?} phase={:?} voice_activated={} held_keys={}",
                    event.event_type,
                    key_was_held,
                    state.dict_state,
                    backend_mode,
                    runtime.dictation_phase(),
                    voice_activated,
                    state.held_keys.len(),
                ));
            }
        }


        if let Some(target_key) = dict_key {
            if voice_activated {
                if is_press
                    && key == target_key
                    && !key_was_held
                    && backend_mode != HotkeyRecordingMode::Command
                {
                    match backend_mode {
                        HotkeyRecordingMode::None => {
                            state.reset_dictation();
                            enqueue_action(&tx, &runtime, HotkeyAction::DictationStart);
                        }
                        HotkeyRecordingMode::Dictation => {
                            state.reset_dictation();
                            let action = if runtime.dictation_phase()
                                == DictationRuntimePhase::Listening
                            {
                                HotkeyAction::DictationCancel
                            } else {
                                HotkeyAction::DictationStop
                            };
                            enqueue_action(&tx, &runtime, action);
                        }
                        HotkeyRecordingMode::Command => {}
                    }
                }
            } else if is_press && key == target_key {
                if backend_mode == HotkeyRecordingMode::Command {
                    // Command mode active - ignore dictation.
                } else {
                    match state.dict_state {
                        DictationState::Idle => {
                            if backend_mode == HotkeyRecordingMode::Dictation {
                                stop_dictation(&runtime, &tx, &mut state);
                            } else {
                                start_dictation_press(&runtime, &tx, &mut state);
                            }
                        }
                        DictationState::Pressed => {
                            // Key repeat - ignore.
                        }
                        DictationState::WaitingForDoubleTap => {
                            let in_window = state
                                .dict_release_time
                                .map(|time| time.elapsed().as_millis() < DOUBLE_TAP_WINDOW_MS)
                                .unwrap_or(false);

                            if in_window {
                                cancel_pending_dictation_stop(&runtime);
                                state.dict_state = DictationState::ToggleRecording;
                                state.dict_press_time = None;
                                state.dict_release_time = None;
                            } else if runtime.recording_mode() == HotkeyRecordingMode::Dictation
                                || runtime.dictation_start_pending()
                            {
                                stop_dictation(&runtime, &tx, &mut state);
                            } else {
                                start_dictation_press(&runtime, &tx, &mut state);
                            }
                        }
                        DictationState::ToggleRecording => {
                            stop_dictation(&runtime, &tx, &mut state);
                        }
                    }
                }
            } else if is_press
                && state.dict_state == DictationState::Pressed
                && key != target_key
                && !key_was_held
                && backend_mode == HotkeyRecordingMode::Dictation
            {
                if let Some(key_name) = style_shortcuts.get(&key).cloned() {
                    state.style_key_held = Some(key);
                    if let Ok(mut active_style_key) = app.state::<ActiveStyleKey>().0.lock() {
                        *active_style_key = Some(key_name.clone());
                    }
                    enqueue_action(&tx, &runtime, HotkeyAction::StyleSwitch(key_name));
                }
            } else if is_release
                && state.style_key_held == Some(key)
                && state.dict_state == DictationState::Pressed
                && backend_mode == HotkeyRecordingMode::Dictation
            {
                state.style_key_held = None;
                stop_dictation(&runtime, &tx, &mut state);
            } else if is_release && key == target_key {
                state.style_key_held = None;

                match state.dict_state {
                    DictationState::Pressed => {
                        let held_ms = state
                            .dict_press_time
                            .map(|time| time.elapsed().as_millis())
                            .unwrap_or(0);

                        if held_ms >= HOLD_THRESHOLD_MS {
                            log::info!(
                                target: "yolo_voice::hotkey",
                                "release after hold: {}ms >= {}ms",
                                held_ms, HOLD_THRESHOLD_MS
                            );
                            stop_dictation(&runtime, &tx, &mut state);
                        } else {
                            log::info!(
                                target: "yolo_voice::hotkey",
                                "short tap release: {}ms < {}ms; waiting {}ms for double tap",
                                held_ms, HOLD_THRESHOLD_MS, DOUBLE_TAP_WINDOW_MS
                            );
                            state.dict_state = DictationState::WaitingForDoubleTap;
                            state.dict_release_time = Some(Instant::now());
                            schedule_delayed_dictation_stop(runtime.clone(), tx.clone());
                        }
                    }
                    DictationState::ToggleRecording => {
                        // Release during toggle - ignore.
                    }
                    _ => {}
                }
            }
        }

        if !cmd_chord.is_empty() {
            if is_press && !state.held_keys.is_empty() {
                prune_stale_modifiers(&mut state.held_keys);
            }
            let all_chord_held = cmd_chord.iter().all(|k| state.held_keys.contains(k));

            if is_press
                && !key_was_held
                && all_chord_held
                && state.cmd_state == CommandState::Idle
                && runtime.recording_mode() == HotkeyRecordingMode::None
            {
                state.cmd_state = CommandState::Recording;
                state.cmd_press_time = Some(Instant::now());
                enqueue_action(&tx, &runtime, HotkeyAction::CommandStart);
            } else if is_release
                && state.cmd_state == CommandState::Recording
                && cmd_chord.contains(&key)
            {
                let held_ms = state
                    .cmd_press_time
                    .map(|time| time.elapsed().as_millis())
                    .unwrap_or(0);

                state.reset_command();

                if held_ms >= COMMAND_MIN_HOLD_MS {
                    enqueue_action(&tx, &runtime, HotkeyAction::CommandStop);
                } else {
                    enqueue_action(&tx, &runtime, HotkeyAction::CommandCancel);
                }
            }
        }
    };

    listen(callback)
}

/// Start the global hotkey listener with an action worker and restart supervisor.
pub fn start_hotkey_listener(app_handle: AppHandle, cache: HotkeyCache) {
    let hotkey_runtime = app_handle.state::<HotkeyRuntimeState>().inner().clone();
    if let Ok(inner) = cache.0.lock() {
        log::info!(
            target: "yolo_voice::hotkey",
            "starting global hotkey listener dict_key={:?} cmd_chord={:?} voice_activated={}",
            inner.dict_key,
            inner.cmd_chord,
            inner.voice_activated,
        );
    }

    let (tx, rx) = mpsc::sync_channel::<HotkeyAction>(ACTION_CHANNEL_CAPACITY);
    start_action_worker(app_handle.clone(), rx);

    let (log_tx, log_rx) = mpsc::sync_channel::<String>(HOOK_LOG_CHANNEL_CAPACITY);
    start_hook_log_worker(log_rx);

    thread::spawn(move || {
        let mut restarts: u32 = 0;

        loop {
            // Per-instance: the previous instance's timestamps say nothing about this one.
            let mut last_seen_input_tick: Option<u32> = None;
            let mut consecutive_stale: u32 = 0;
            let instance_started = Instant::now();

            let generation = hotkey_runtime.bump_listener_generation();
            let _ = app_handle.emit("hotkey-listener-status", "starting");

            let (done_tx, done_rx) = mpsc::channel();
            let app = app_handle.clone();
            let cache = cache.clone();
            let runtime = hotkey_runtime.clone();
            let action_tx = tx.clone();
            let instance_log_tx = log_tx.clone();

            thread::spawn(move || {
                let result =
                    run_listener_instance(app, cache, runtime, action_tx, instance_log_tx, generation);
                let _ = done_tx.send(result);
            });

            log::info!(
                target: "yolo_voice::hotkey",
                "hotkey listener instance started generation={generation}"
            );
            let _ = app_handle.emit("hotkey-listener-status", "ok");

            loop {
                match done_rx.recv_timeout(Duration::from_millis(LISTENER_HEALTH_POLL_MS)) {
                    Ok(Ok(())) => {
                        log::warn!(
                            target: "yolo_voice::hotkey",
                            "hotkey listener returned Ok; restarting generation={generation}"
                        );
                        break;
                    }
                    Ok(Err(e)) => {
                        log::error!(
                            target: "yolo_voice::hotkey",
                            "hotkey listener error: {:?}; restarting",
                            e
                        );
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        log::error!(
                            target: "yolo_voice::hotkey",
                            "hotkey listener thread disconnected; restarting"
                        );
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if hotkey_runtime.listener_generation() != generation {
                            break;
                        }
                        if hook_appears_dead(
                            &hotkey_runtime,
                            &mut last_seen_input_tick,
                            instance_started,
                        ) {
                            consecutive_stale += 1;
                            log::warn!(
                                target: "yolo_voice::hotkey",
                                "hotkey hook looks stale ({consecutive_stale}/{HOOK_DEAD_CONSECUTIVE})"
                            );
                        } else {
                            consecutive_stale = 0;
                        }

                        if consecutive_stale >= HOOK_DEAD_CONSECUTIVE {
                            log::error!(
                                target: "yolo_voice::hotkey",
                                "hotkey hook appears dead (OS input without hook events); reinstalling"
                            );
                            let _ = app_handle.emit("hotkey-listener-status", "restarting");
                            // Invalidate the zombie callback before installing a new hook.
                            hotkey_runtime.bump_listener_generation();
                            break;
                        }
                    }
                }
            }

            restarts += 1;
            if restarts > MAX_LISTENER_RESTARTS {
                // rdev's `listen` parks in GetMessageA forever and never unhooks, so each
                // reinstall leaks a thread and an OS hook. Past this point we're doing more
                // harm than good; tell the UI so it can ask the user to restart the app.
                log::error!(
                    target: "yolo_voice::hotkey",
                    "hotkey listener restarted {MAX_LISTENER_RESTARTS} times without recovering; giving up"
                );
                let _ = app_handle.emit("hotkey-listener-status", "dead");
                break;
            }

            thread::sleep(Duration::from_millis(250));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_accepts_letter_names_from_frontend() {
        assert_eq!(parse_key("A"), Some(Key::KeyA));
        assert_eq!(parse_key("z"), Some(Key::KeyZ));
        assert_eq!(parse_key("KeyB"), Some(Key::KeyB));
        assert_eq!(parse_key("keyc"), None); // wrong casing for Key prefix
    }

    #[test]
    fn parse_key_round_trips_frontend_emittable_names() {
        let names = [
            "AltLeft",
            "AltRight",
            "ControlLeft",
            "ShiftLeft",
            "Space",
            "Escape",
            "Return",
            "BackSpace",
            "F5",
            "Digit3",
            "Kp7",
            "A",
            "M",
            "Z",
        ];
        for name in names {
            let key = parse_key(name).unwrap_or_else(|| panic!("failed to parse {name}"));
            let back = key_to_rdev_name(&key);
            assert!(
                parse_key(&back).is_some(),
                "round-trip failed for {name} -> {back}"
            );
        }
    }

    #[test]
    fn parse_chord_uses_letter_fallback() {
        let chord = parse_chord("ControlLeft+A");
        assert_eq!(chord, vec![Key::ControlLeft, Key::KeyA]);
    }
}
