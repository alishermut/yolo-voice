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

const START_SOUND: &[u8] = include_bytes!("../sounds/start.wav");
const DONE_SOUND: &[u8] = include_bytes!("../sounds/done.wav");

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

pub fn play_start_sound() {
    thread::spawn(|| {
        play_wav_from_memory(START_SOUND);
    });
}

pub fn play_done_sound() {
    thread::spawn(|| {
        play_wav_from_memory(DONE_SOUND);
    });
}

fn play_wav_from_memory(data: &[u8]) {
    unsafe {
        let ptr = data.as_ptr() as *const u16;
        let _ = PlaySoundW(PCWSTR(ptr), None, SND_MEMORY | SND_ASYNC);
    }
}
