use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;

#[cfg(windows)]
use windows::Win32::Foundation::{GetLastError, SetLastError, HWND, WIN32_ERROR};
#[cfg(windows)]
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
    VK_MEDIA_PLAY_PAUSE, VK_SHIFT, VK_V,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    SetForegroundWindow,
};

pub struct FocusedWindowState(pub Mutex<isize>);

#[derive(Debug, Clone, Serialize)]
pub struct OutputWindowDetails {
    pub hwnd: isize,
    pub pid: Option<u32>,
    pub exe_name: Option<String>,
    pub class_name: Option<String>,
    pub title_length: Option<usize>,
    pub is_terminal: bool,
    pub is_own_window: bool,
    pub is_elevated: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InsertTextReport {
    pub app_pid: u32,
    pub app_is_elevated: Option<bool>,
    pub clipboard_text_len: usize,
    pub used_shift_paste: bool,
    pub target: OutputWindowDetails,
    pub foreground_before: Option<OutputWindowDetails>,
    pub foreground_after_refocus: Option<OutputWindowDetails>,
    pub foreground_after_paste: Option<OutputWindowDetails>,
    pub set_foreground_attempted: bool,
    pub set_foreground_succeeded: Option<bool>,
    pub set_foreground_last_error_code: Option<u32>,
    pub set_foreground_last_error: Option<String>,
    pub send_input_event_count: usize,
    pub send_input_sent: Option<u32>,
    pub send_input_last_error_code: Option<u32>,
    pub send_input_last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InsertTextError {
    pub message: String,
    pub report: InsertTextReport,
}

impl std::fmt::Display for InsertTextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for InsertTextError {}

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
const SOUND_BEGIN: &[u8] = include_bytes!("../../../sounds/begin.mp3");
const SOUND_BACK_002: &[u8] = include_bytes!("../../../sounds/back_002.mp3");
const SOUND_CLICK_SOFT: &[u8] = include_bytes!("../../../sounds/click_soft.mp3");
const SOUND_NOTIFICATION_POP: &[u8] = include_bytes!("../../../sounds/notification_pop.mp3");
const SOUND_SUCCESS_CHIME: &[u8] = include_bytes!("../../../sounds/success_chime.mp3");
const SOUND_ZAP_TWO_TONE: &[u8] = include_bytes!("../../../sounds/zap_two_tone.mp3");
const SOUND_ZAP_THREE_TONE_UP: &[u8] = include_bytes!("../../../sounds/zap_three_tone_up.mp3");
const SOUND_ZAP_THREE_TONE_DOWN: &[u8] = include_bytes!("../../../sounds/zap_three_tone_down.mp3");
const SOUND_CHIME_CANCEL: &[u8] = include_bytes!("../../../sounds/chime_cancel.mp3");
const SOUND_CHIME_CONFIRM: &[u8] = include_bytes!("../../../sounds/chime_confirm.mp3");
const SOUND_CHIME_EXIT: &[u8] = include_bytes!("../../../sounds/chime_exit.mp3");
const SOUND_CHIME_LEVEL_UP: &[u8] = include_bytes!("../../../sounds/chime_level_up.mp3");
const SOUND_CHIME_LOAD: &[u8] = include_bytes!("../../../sounds/chime_load.mp3");
const SOUND_CHIME_SAVE: &[u8] = include_bytes!("../../../sounds/chime_save.mp3");
const SOUND_CHIME_SLEEP: &[u8] = include_bytes!("../../../sounds/chime_sleep.mp3");
const SOUND_UI_BUTTON_1: &[u8] = include_bytes!("../../../sounds/ui_button_1.ogg");
const SOUND_UI_BUTTON_2: &[u8] = include_bytes!("../../../sounds/ui_button_2.ogg");
const SOUND_UI_COMPLETE: &[u8] = include_bytes!("../../../sounds/ui_complete.ogg");
const SOUND_UI_OFF: &[u8] = include_bytes!("../../../sounds/ui_off.ogg");
const SOUND_UI_ON: &[u8] = include_bytes!("../../../sounds/ui_on.ogg");
const SOUND_UI_PACK_0: &[u8] = include_bytes!("../../../sounds/ui_pack_0.wav");
const SOUND_UI_PACK_1: &[u8] = include_bytes!("../../../sounds/ui_pack_1.wav");
const SOUND_UI_PACK_2: &[u8] = include_bytes!("../../../sounds/ui_pack_2.wav");
const SOUND_UI_PACK_3: &[u8] = include_bytes!("../../../sounds/ui_pack_3.wav");
const SOUND_UI_PACK_4: &[u8] = include_bytes!("../../../sounds/ui_pack_4.wav");
const SOUND_UI_PACK_5: &[u8] = include_bytes!("../../../sounds/ui_pack_5.wav");
const SOUND_UI_PACK_6: &[u8] = include_bytes!("../../../sounds/ui_pack_6.wav");
const SOUND_UI_PACK_7: &[u8] = include_bytes!("../../../sounds/ui_pack_7.wav");
const SOUND_UI_PACK_8: &[u8] = include_bytes!("../../../sounds/ui_pack_8.wav");
const SOUND_UI_PACK_9: &[u8] = include_bytes!("../../../sounds/ui_pack_9.wav");
const SOUND_UI_WRONG_ERROR: &[u8] = include_bytes!("../../../sounds/ui_wrong_error.wav");

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
    "begin",
    "back_002",
    "click_soft",
    "notification_pop",
    "success_chime",
    "zap_two_tone",
    "zap_three_tone_up",
    "zap_three_tone_down",
    "chime_cancel",
    "chime_confirm",
    "chime_exit",
    "chime_level_up",
    "chime_load",
    "chime_save",
    "chime_sleep",
    "ui_button_1",
    "ui_button_2",
    "ui_complete",
    "ui_off",
    "ui_on",
    "ui_pack_0",
    "ui_pack_1",
    "ui_pack_2",
    "ui_pack_3",
    "ui_pack_4",
    "ui_pack_5",
    "ui_pack_6",
    "ui_pack_7",
    "ui_pack_8",
    "ui_pack_9",
    "ui_wrong_error",
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
        "begin" => SOUND_BEGIN,
        "back_002" => SOUND_BACK_002,
        "click_soft" => SOUND_CLICK_SOFT,
        "notification_pop" => SOUND_NOTIFICATION_POP,
        "success_chime" => SOUND_SUCCESS_CHIME,
        "zap_two_tone" => SOUND_ZAP_TWO_TONE,
        "zap_three_tone_up" => SOUND_ZAP_THREE_TONE_UP,
        "zap_three_tone_down" => SOUND_ZAP_THREE_TONE_DOWN,
        "chime_cancel" => SOUND_CHIME_CANCEL,
        "chime_confirm" => SOUND_CHIME_CONFIRM,
        "chime_exit" => SOUND_CHIME_EXIT,
        "chime_level_up" => SOUND_CHIME_LEVEL_UP,
        "chime_load" => SOUND_CHIME_LOAD,
        "chime_save" => SOUND_CHIME_SAVE,
        "chime_sleep" => SOUND_CHIME_SLEEP,
        "ui_button_1" => SOUND_UI_BUTTON_1,
        "ui_button_2" => SOUND_UI_BUTTON_2,
        "ui_complete" => SOUND_UI_COMPLETE,
        "ui_off" => SOUND_UI_OFF,
        "ui_on" => SOUND_UI_ON,
        "ui_pack_0" => SOUND_UI_PACK_0,
        "ui_pack_1" => SOUND_UI_PACK_1,
        "ui_pack_2" => SOUND_UI_PACK_2,
        "ui_pack_3" => SOUND_UI_PACK_3,
        "ui_pack_4" => SOUND_UI_PACK_4,
        "ui_pack_5" => SOUND_UI_PACK_5,
        "ui_pack_6" => SOUND_UI_PACK_6,
        "ui_pack_7" => SOUND_UI_PACK_7,
        "ui_pack_8" => SOUND_UI_PACK_8,
        "ui_pack_9" => SOUND_UI_PACK_9,
        "ui_wrong_error" => SOUND_UI_WRONG_ERROR,
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

#[cfg(windows)]
fn get_window_pid(hwnd: isize) -> Option<u32> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid));
        (pid != 0).then_some(pid)
    }
}

#[cfg(windows)]
fn get_process_elevation(pid: u32) -> Option<bool> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut token = Default::default();
        let token_result = OpenProcessToken(process, TOKEN_QUERY, &mut token);
        let _ = windows::Win32::Foundation::CloseHandle(process);
        if token_result.is_err() {
            return None;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let info_result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
        let _ = windows::Win32::Foundation::CloseHandle(token);
        if info_result.is_err() {
            return None;
        }

        Some(elevation.TokenIsElevated != 0)
    }
}

#[cfg(windows)]
fn get_window_class_name(hwnd: isize) -> Option<String> {
    unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buf) as usize;
        if len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

#[cfg(windows)]
fn describe_window(hwnd: isize) -> Option<OutputWindowDetails> {
    if hwnd == 0 {
        return None;
    }

    let pid = get_window_pid(hwnd);
    let title_length = unsafe {
        let hwnd = HWND(hwnd as *mut _);
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf) as usize;
        (len != 0).then_some(len)
    };
    Some(OutputWindowDetails {
        hwnd,
        pid,
        exe_name: get_window_exe_name(hwnd),
        class_name: get_window_class_name(hwnd),
        title_length,
        is_terminal: is_terminal_window(hwnd),
        is_own_window: is_own_window(hwnd),
        is_elevated: pid.and_then(get_process_elevation),
    })
}

#[cfg(target_os = "macos")]
fn describe_window(target: isize) -> Option<OutputWindowDetails> {
    if target == 0 {
        return None;
    }

    Some(OutputWindowDetails {
        hwnd: target,
        pid: Some(target as u32),
        exe_name: None,
        class_name: None,
        title_length: None,
        is_terminal: is_terminal_window(target),
        is_own_window: is_own_window(target),
        is_elevated: None,
    })
}

#[cfg(windows)]
fn win32_error_details(code: u32) -> (Option<u32>, Option<String>) {
    if code == 0 {
        return (None, None);
    }

    let error = windows::core::Error::from_win32();
    (Some(code), Some(error.message().to_string()))
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

pub fn insert_text(text: &str, target_hwnd: isize) -> Result<InsertTextReport, InsertTextError> {
    if text.is_empty() {
        return Ok(InsertTextReport {
            app_pid: std::process::id(),
            app_is_elevated: {
                #[cfg(windows)]
                {
                    get_process_elevation(std::process::id())
                }
                #[cfg(not(windows))]
                {
                    None
                }
            },
            clipboard_text_len: 0,
            used_shift_paste: false,
            target: describe_window(target_hwnd).unwrap_or(OutputWindowDetails {
                hwnd: target_hwnd,
                pid: None,
                exe_name: None,
                class_name: None,
                title_length: None,
                is_terminal: false,
                is_own_window: false,
                is_elevated: None,
            }),
            foreground_before: None,
            foreground_after_refocus: None,
            foreground_after_paste: None,
            set_foreground_attempted: false,
            set_foreground_succeeded: None,
            set_foreground_last_error_code: None,
            set_foreground_last_error: None,
            send_input_event_count: 0,
            send_input_sent: None,
            send_input_last_error_code: None,
            send_input_last_error: None,
        });
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| InsertTextError {
        message: format!("Clipboard error: {}", e),
        report: InsertTextReport {
            app_pid: std::process::id(),
            app_is_elevated: {
                #[cfg(windows)]
                {
                    get_process_elevation(std::process::id())
                }
                #[cfg(not(windows))]
                {
                    None
                }
            },
            clipboard_text_len: text.chars().count(),
            used_shift_paste: false,
            target: describe_window(target_hwnd).unwrap_or(OutputWindowDetails {
                hwnd: target_hwnd,
                pid: None,
                exe_name: None,
                class_name: None,
                title_length: None,
                is_terminal: false,
                is_own_window: false,
                is_elevated: None,
            }),
            foreground_before: None,
            foreground_after_refocus: None,
            foreground_after_paste: None,
            set_foreground_attempted: false,
            set_foreground_succeeded: None,
            set_foreground_last_error_code: None,
            set_foreground_last_error: None,
            send_input_event_count: 0,
            send_input_sent: None,
            send_input_last_error_code: None,
            send_input_last_error: None,
        },
    })?;
    clipboard
        .set_text(text)
        .map_err(|e| InsertTextError {
            message: format!("Failed to set clipboard text: {}", e),
            report: InsertTextReport {
                app_pid: std::process::id(),
                app_is_elevated: {
                    #[cfg(windows)]
                    {
                        get_process_elevation(std::process::id())
                    }
                    #[cfg(not(windows))]
                    {
                        None
                    }
                },
                clipboard_text_len: text.chars().count(),
                used_shift_paste: false,
                target: describe_window(target_hwnd).unwrap_or(OutputWindowDetails {
                hwnd: target_hwnd,
                pid: None,
                exe_name: None,
                class_name: None,
                title_length: None,
                is_terminal: false,
                is_own_window: false,
                    is_elevated: None,
                }),
                foreground_before: None,
                foreground_after_refocus: None,
                foreground_after_paste: None,
                set_foreground_attempted: false,
                set_foreground_succeeded: None,
                set_foreground_last_error_code: None,
                set_foreground_last_error: None,
                send_input_event_count: 0,
                send_input_sent: None,
                send_input_last_error_code: None,
                send_input_last_error: None,
            },
        })?;

    let use_shift = is_terminal_window(target_hwnd);
    let mut report = InsertTextReport {
        app_pid: std::process::id(),
        app_is_elevated: {
            #[cfg(windows)]
            {
                get_process_elevation(std::process::id())
            }
            #[cfg(not(windows))]
            {
                None
            }
        },
        clipboard_text_len: text.chars().count(),
        used_shift_paste: use_shift,
        target: describe_window(target_hwnd).unwrap_or(OutputWindowDetails {
            hwnd: target_hwnd,
            pid: None,
            exe_name: None,
            class_name: None,
            title_length: None,
            is_terminal: use_shift,
            is_own_window: is_own_window(target_hwnd),
            is_elevated: None,
        }),
        foreground_before: describe_window(capture_foreground_window()),
        foreground_after_refocus: None,
        foreground_after_paste: None,
        set_foreground_attempted: false,
        set_foreground_succeeded: None,
        set_foreground_last_error_code: None,
        set_foreground_last_error: None,
        send_input_event_count: 0,
        send_input_sent: None,
        send_input_last_error_code: None,
        send_input_last_error: None,
    };

    // Refocus the target window/app
    #[cfg(windows)]
    unsafe {
        report.set_foreground_attempted = true;
        let hwnd = HWND(target_hwnd as *mut _);
        SetLastError(WIN32_ERROR(0));
        let succeeded = SetForegroundWindow(hwnd).as_bool();
        let last_error = GetLastError().0;
        let (error_code, error_message) = win32_error_details(last_error);
        report.set_foreground_succeeded = Some(succeeded);
        report.set_foreground_last_error_code = error_code;
        report.set_foreground_last_error = error_message;
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
        report.set_foreground_attempted = true;
        report.set_foreground_succeeded = Some(true);
    }

    thread::sleep(Duration::from_millis(50));
    report.foreground_after_refocus = describe_window(capture_foreground_window());

    match send_paste_keystroke(use_shift) {
        Ok((event_count, sent, error_code, error_message)) => {
            report.send_input_event_count = event_count;
            report.send_input_sent = Some(sent);
            report.send_input_last_error_code = error_code;
            report.send_input_last_error = error_message;
            report.foreground_after_paste = describe_window(capture_foreground_window());
            Ok(report)
        }
        Err((message, event_count, sent, error_code, error_message)) => {
            report.send_input_event_count = event_count;
            report.send_input_sent = Some(sent);
            report.send_input_last_error_code = error_code;
            report.send_input_last_error = error_message;
            report.foreground_after_paste = describe_window(capture_foreground_window());
            Err(InsertTextError { message, report })
        }
    }
}

// ---- Paste keystroke ----

#[cfg(windows)]
fn send_paste_keystroke(
    with_shift: bool,
) -> Result<(usize, u32, Option<u32>, Option<String>), (String, usize, u32, Option<u32>, Option<String>)> {
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

    let sent = unsafe {
        SetLastError(WIN32_ERROR(0));
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32)
    };
    let last_error = unsafe { GetLastError().0 };
    let (error_code, error_message) = win32_error_details(last_error);
    if sent != inputs.len() as u32 {
        let message = if let Some(code) = error_code {
            format!(
                "SendInput sent {} of {} events (win32 error {}: {})",
                sent,
                inputs.len(),
                code,
                error_message.clone().unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            format!("SendInput sent {} of {} events", sent, inputs.len())
        };
        return Err((message, inputs.len(), sent, error_code, error_message));
    }

    Ok((inputs.len(), sent, error_code, error_message))
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

/// Send a media play/pause key event to toggle system media playback.
#[cfg(windows)]
pub fn send_media_play_pause() {
    let inputs = vec![
        make_key_input(VK_MEDIA_PLAY_PAUSE, false),
        make_key_input(VK_MEDIA_PLAY_PAUSE, true),
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "macos")]
pub fn send_media_play_pause() {
    // macOS: use osascript to simulate media key
    let _ = std::process::Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to key code 16"])
        .output();
}

#[cfg(target_os = "macos")]
fn send_paste_keystroke(
    _with_shift: bool,
) -> Result<(usize, u32, Option<u32>, Option<String>), (String, usize, u32, Option<u32>, Option<String>)> {
    // On macOS, use enigo to send Cmd+V
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| {
            (
                format!("Failed to create enigo: {}", e),
                0,
                0,
                None,
                None,
            )
        })?;

    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| (format!("Enigo key error: {}", e), 0, 0, None, None))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| (format!("Enigo key error: {}", e), 0, 0, None, None))?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| (format!("Enigo key error: {}", e), 0, 0, None, None))?;

    Ok((3, 3, None, None))
}

// ---- Sound playback ----

pub fn play_sound(name: &str) {
    let sound_data: &'static [u8] = get_sound_bytes(name);
    thread::spawn(move || {
        play_audio_from_memory(sound_data);
    });
}

pub fn play_start_sound(sound_name: &str) {
    play_sound(sound_name);
}

pub fn play_done_sound(sound_name: &str) {
    play_sound(sound_name);
}

fn play_audio_from_memory(data: &[u8]) {
    use rodio::{Decoder, OutputStream, Sink};
    use std::io::Cursor;

    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sound] Failed to open audio output: {}", e);
            return;
        }
    };

    let cursor = Cursor::new(data.to_vec());
    let source = match Decoder::new(cursor) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sound] Failed to decode audio: {}", e);
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
