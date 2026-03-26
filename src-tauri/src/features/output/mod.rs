use std::sync::Mutex;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
    VK_SHIFT, VK_V,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

pub struct FocusedWindowState(pub Mutex<isize>);

// Sound registry
const SOUND_CHIME: &[u8] = include_bytes!("../../../sounds/chime.wav");
const SOUND_POP: &[u8] = include_bytes!("../../../sounds/pop.wav");
const SOUND_BELL: &[u8] = include_bytes!("../../../sounds/bell.wav");
const SOUND_DING: &[u8] = include_bytes!("../../../sounds/ding.wav");
const SOUND_CLICK: &[u8] = include_bytes!("../../../sounds/click.wav");
const SOUND_WHOOSH: &[u8] = include_bytes!("../../../sounds/whoosh.wav");
const SOUND_BUBBLE: &[u8] = include_bytes!("../../../sounds/bubble.wav");
const SOUND_TAP: &[u8] = include_bytes!("../../../sounds/tap.wav");
const SOUND_GENTLE: &[u8] = include_bytes!("../../../sounds/gentle.wav");
const SOUND_BRIGHT: &[u8] = include_bytes!("../../../sounds/bright.wav");
const SOUND_CLASSIC_START: &[u8] = include_bytes!("../../../sounds/classic_start.wav");
const SOUND_CLASSIC_DONE: &[u8] = include_bytes!("../../../sounds/classic_done.wav");
const SOUND_RADIO_START: &[u8] = include_bytes!("../../../sounds/radio_start.wav");
const SOUND_RADIO_DONE: &[u8] = include_bytes!("../../../sounds/radio_done.wav");
const SOUND_RETRO_START: &[u8] = include_bytes!("../../../sounds/retro_start.wav");
const SOUND_RETRO_DONE: &[u8] = include_bytes!("../../../sounds/retro_done.wav");

pub const AVAILABLE_SOUNDS: &[&str] = &[
    "chime",
    "pop",
    "bell",
    "ding",
    "click",
    "whoosh",
    "bubble",
    "tap",
    "gentle",
    "bright",
    "classic_start",
    "classic_done",
    "radio_start",
    "radio_done",
    "retro_start",
    "retro_done",
];

fn get_sound_bytes(name: &str) -> &'static [u8] {
    match name {
        "chime" => SOUND_CHIME,
        "pop" => SOUND_POP,
        "bell" => SOUND_BELL,
        "ding" => SOUND_DING,
        "click" => SOUND_CLICK,
        "whoosh" => SOUND_WHOOSH,
        "bubble" => SOUND_BUBBLE,
        "tap" => SOUND_TAP,
        "gentle" => SOUND_GENTLE,
        "bright" => SOUND_BRIGHT,
        "classic_start" => SOUND_CLASSIC_START,
        "classic_done" => SOUND_CLASSIC_DONE,
        "radio_start" => SOUND_RADIO_START,
        "radio_done" => SOUND_RADIO_DONE,
        "retro_start" => SOUND_RETRO_START,
        "retro_done" => SOUND_RETRO_DONE,
        _ => SOUND_CHIME,
    }
}

// ---- Foreground window capture ----

#[cfg(windows)]
pub fn capture_foreground_window() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

#[cfg(target_os = "macos")]
pub fn capture_foreground_window() -> isize {
    // On macOS, we store the PID of the frontmost app
    unsafe {
        let workspace: cocoa::base::id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: cocoa::base::id = msg_send![workspace, frontmostApplication];
        let pid: i32 = msg_send![app, processIdentifier];
        pid as isize
    }
}

// ---- Terminal detection ----

#[cfg(windows)]
pub fn is_terminal_window(hwnd: isize) -> bool {
    let terminal_exes = [
        "windowsterminal.exe",
        "cmd.exe",
        "powershell.exe",
        "pwsh.exe",
        "conemu64.exe",
        "conemuc64.exe",
        "mintty.exe",
        "alacritty.exe",
        "wezterm-gui.exe",
        "hyper.exe",
    ];

    let exe_name = match get_window_exe_name(hwnd) {
        Some(name) => name.to_lowercase(),
        None => return false,
    };

    terminal_exes.iter().any(|t| exe_name == *t)
}

#[cfg(target_os = "macos")]
pub fn is_terminal_window(pid: isize) -> bool {
    let terminal_bundle_ids = [
        "com.apple.terminal",
        "com.googlecode.iterm2",
        "io.alacritty",
        "com.github.wez.wezterm",
        "co.zeit.hyper",
        "net.kovidgoyal.kitty",
    ];

    let bundle_id = match get_bundle_id_for_pid(pid as i32) {
        Some(id) => id.to_lowercase(),
        None => return false,
    };

    terminal_bundle_ids.iter().any(|t| bundle_id.contains(t))
}

#[cfg(target_os = "macos")]
fn get_bundle_id_for_pid(pid: i32) -> Option<String> {
    unsafe {
        let workspace: cocoa::base::id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let apps: cocoa::base::id = msg_send![workspace, runningApplications];
        let count: usize = msg_send![apps, count];
        for i in 0..count {
            let app: cocoa::base::id = msg_send![apps, objectAtIndex: i];
            let app_pid: i32 = msg_send![app, processIdentifier];
            if app_pid == pid {
                let bundle: cocoa::base::id = msg_send![app, bundleIdentifier];
                if bundle != cocoa::base::nil {
                    let cstr: *const std::os::raw::c_char = msg_send![bundle, UTF8String];
                    if !cstr.is_null() {
                        return Some(std::ffi::CStr::from_ptr(cstr).to_string_lossy().to_string());
                    }
                }
                return None;
            }
        }
        None
    }
}

// ---- Window exe name (Windows only) ----

#[cfg(windows)]
fn get_window_exe_name(hwnd: isize) -> Option<String> {
    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = windows::Win32::Foundation::CloseHandle(process);

        if ok.is_err() {
            return None;
        }

        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}

// ---- Text insertion ----

/// Check if a window belongs to our own process.
#[cfg(windows)]
pub fn is_own_window(hwnd: isize) -> bool {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid));
        pid != 0 && pid == std::process::id()
    }
}

#[cfg(target_os = "macos")]
pub fn is_own_window(pid: isize) -> bool {
    pid == std::process::id() as isize
}

pub fn insert_text(text: &str, target_hwnd: isize) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to set clipboard text: {}", e))?;

    // Refocus the target window/app
    #[cfg(windows)]
    unsafe {
        let hwnd = HWND(target_hwnd as *mut _);
        let _ = SetForegroundWindow(hwnd);
    }

    #[cfg(target_os = "macos")]
    unsafe {
        let workspace: cocoa::base::id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let apps: cocoa::base::id = msg_send![workspace, runningApplications];
        let count: usize = msg_send![apps, count];
        for i in 0..count {
            let app: cocoa::base::id = msg_send![apps, objectAtIndex: i];
            let pid: i32 = msg_send![app, processIdentifier];
            if pid == target_hwnd as i32 {
                let _: () = msg_send![app, activateWithOptions: 0x01u64]; // NSApplicationActivateIgnoringOtherApps
                break;
            }
        }
    }

    thread::sleep(Duration::from_millis(50));

    let use_shift = is_terminal_window(target_hwnd);
    send_paste_keystroke(use_shift)
}

// ---- Paste keystroke ----

#[cfg(windows)]
fn send_paste_keystroke(with_shift: bool) -> Result<(), String> {
    let mut inputs: Vec<INPUT> = Vec::new();

    inputs.push(make_key_input(VK_CONTROL, false));
    if with_shift {
        inputs.push(make_key_input(VK_SHIFT, false));
    }
    inputs.push(make_key_input(VK_V, false));
    inputs.push(make_key_input(VK_V, true));
    if with_shift {
        inputs.push(make_key_input(VK_SHIFT, true));
    }
    inputs.push(make_key_input(VK_CONTROL, true));

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(format!(
            "SendInput sent {} of {} events",
            sent,
            inputs.len()
        ));
    }

    Ok(())
}

#[cfg(windows)]
fn make_key_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let mut flags = windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0);
    if key_up {
        flags = KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(target_os = "macos")]
fn send_paste_keystroke(_with_shift: bool) -> Result<(), String> {
    // On macOS, use enigo to send Cmd+V
    use enigo::{Enigo, Keyboard, Settings, Key, Direction};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to create enigo: {}", e))?;

    enigo.key(Key::Meta, Direction::Press)
        .map_err(|e| format!("Enigo key error: {}", e))?;
    enigo.key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Enigo key error: {}", e))?;
    enigo.key(Key::Meta, Direction::Release)
        .map_err(|e| format!("Enigo key error: {}", e))?;

    Ok(())
}

// ---- Sound playback ----

pub fn play_sound(name: &str) {
    let sound_data: &'static [u8] = get_sound_bytes(name);
    thread::spawn(move || {
        play_wav_from_memory(sound_data);
    });
}

pub fn play_start_sound(sound_name: &str) {
    play_sound(sound_name);
}

pub fn play_done_sound(sound_name: &str) {
    play_sound(sound_name);
}

#[cfg(windows)]
fn play_wav_from_memory(data: &[u8]) {
    unsafe {
        let ptr = data.as_ptr() as *const u16;
        let _ = PlaySoundW(PCWSTR(ptr), None, SND_MEMORY | SND_ASYNC);
    }
}

#[cfg(target_os = "macos")]
fn play_wav_from_memory(data: &[u8]) {
    use rodio::{Decoder, OutputStream, Sink};
    use std::io::Cursor;

    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sound] Failed to open audio output: {}", e);
            return;
        }
    };

    let cursor = Cursor::new(data);
    let source = match Decoder::new(cursor) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sound] Failed to decode WAV: {}", e);
            return;
        }
    };

    let sink = match Sink::try_new(&stream_handle) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sound] Failed to create sink: {}", e);
            return;
        }
    };

    sink.append(source);
    sink.sleep_until_end();
}
