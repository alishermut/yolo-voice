use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::HWND;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
    VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};
use windows::core::PCWSTR;

pub struct FocusedWindowState(pub Mutex<isize>);

// Sound registry: all available notification sounds
const SOUND_CHIME: &[u8] = include_bytes!("../sounds/chime.wav");
const SOUND_POP: &[u8] = include_bytes!("../sounds/pop.wav");
const SOUND_BELL: &[u8] = include_bytes!("../sounds/bell.wav");
const SOUND_DING: &[u8] = include_bytes!("../sounds/ding.wav");
const SOUND_CLICK: &[u8] = include_bytes!("../sounds/click.wav");
const SOUND_WHOOSH: &[u8] = include_bytes!("../sounds/whoosh.wav");
const SOUND_BUBBLE: &[u8] = include_bytes!("../sounds/bubble.wav");
const SOUND_TAP: &[u8] = include_bytes!("../sounds/tap.wav");
const SOUND_GENTLE: &[u8] = include_bytes!("../sounds/gentle.wav");
const SOUND_BRIGHT: &[u8] = include_bytes!("../sounds/bright.wav");
const SOUND_CLASSIC_START: &[u8] = include_bytes!("../sounds/classic_start.wav");
const SOUND_CLASSIC_DONE: &[u8] = include_bytes!("../sounds/classic_done.wav");

/// All available sound names for the frontend.
pub const AVAILABLE_SOUNDS: &[&str] = &[
    "chime", "pop", "bell", "ding", "click",
    "whoosh", "bubble", "tap", "gentle", "bright",
    "classic_start", "classic_done",
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
        _ => SOUND_CHIME, // fallback
    }
}

pub fn capture_foreground_window() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

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
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = windows::Win32::Foundation::CloseHandle(process);

        if ok.is_err() {
            return None;
        }

        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}

pub fn insert_text(text: &str, target_hwnd: isize) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    // Write to clipboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to set clipboard text: {}", e))?;

    // Restore focus to the target window
    unsafe {
        let hwnd = HWND(target_hwnd as *mut _);
        let _ = SetForegroundWindow(hwnd);
    }
    thread::sleep(Duration::from_millis(50));

    // Determine paste keystroke
    let use_shift = is_terminal_window(target_hwnd);

    // Build SendInput sequence
    send_paste_keystroke(use_shift)
}

fn send_paste_keystroke(with_shift: bool) -> Result<(), String> {
    let mut inputs: Vec<INPUT> = Vec::new();

    // Key down: Ctrl
    inputs.push(make_key_input(VK_CONTROL, false));
    // Key down: Shift (if terminal)
    if with_shift {
        inputs.push(make_key_input(VK_SHIFT, false));
    }
    // Key down: V
    inputs.push(make_key_input(VK_V, false));
    // Key up: V
    inputs.push(make_key_input(VK_V, true));
    // Key up: Shift (if terminal)
    if with_shift {
        inputs.push(make_key_input(VK_SHIFT, true));
    }
    // Key up: Ctrl
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

pub fn play_sound(name: &str) {
    let sound_data = get_sound_bytes(name);
    // We need to copy the pointer since the data is 'static
    let ptr = sound_data.as_ptr() as usize;
    let len = sound_data.len();
    thread::spawn(move || {
        // SAFETY: sound data is 'static, pointer is valid for program lifetime
        let data = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        play_wav_from_memory(data);
    });
}

pub fn play_start_sound(sound_name: &str) {
    play_sound(sound_name);
}

pub fn play_done_sound(sound_name: &str) {
    play_sound(sound_name);
}

fn play_wav_from_memory(data: &[u8]) {
    unsafe {
        let ptr = data.as_ptr() as *const u16;
        let _ = PlaySoundW(PCWSTR(ptr), None, SND_MEMORY | SND_ASYNC);
    }
}
