//! Keeps the pill overlay window strictly on top by re-asserting HWND_TOPMOST
//! whenever another window steals z-order. Uses SetWinEventHook so the OS
//! notifies us — no polling required.
//!
//! Technique: same approach used by PowerToys "Always on Top" (microsoft/PowerToys).

use std::sync::atomic::{AtomicUsize, Ordering};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WS_EX_TOPMOST,
};

// WinEvent codes (Win32 accessibility constants)
const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const EVENT_SYSTEM_MINIMIZEEND: u32 = 0x0017;
const EVENT_OBJECT_FOCUS: u32 = 0x8005;

/// Raw HWND of the pill window stored as usize. 0 = not installed.
static PILL_HWND: AtomicUsize = AtomicUsize::new(0);

/// WinEvent callback — runs on the main thread via the Win32 message pump.
/// Re-asserts HWND_TOPMOST only when the pill's WS_EX_TOPMOST flag was
/// actually cleared by another app (e.g. Electron, Discord, Spotify).
unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    let raw = PILL_HWND.load(Ordering::Relaxed);
    if raw == 0 {
        return;
    }
    let pill = HWND(raw as *mut core::ffi::c_void);

    // Guard: only re-assert if WS_EX_TOPMOST was actually stripped.
    // GetWindowLongW is a fast kernel call — negligible overhead per event.
    let ex_style = GetWindowLongW(pill, GWL_EXSTYLE);
    if (ex_style as u32 & WS_EX_TOPMOST.0) == 0 {
        let _ = SetWindowPos(
            pill,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Register WinEvent hooks that keep the pill window always on top.
///
/// Must be called from the main thread — `WINEVENT_OUTOFCONTEXT` delivers
/// callbacks via `DispatchMessage` on the registering thread, which in Tauri
/// is the main Win32 message loop thread.
///
/// Hook handles are intentionally not stored — they live for the process
/// lifetime and are cleaned up when the process exits.
/// # Safety
/// `hwnd` must be a valid HWND for the pill window, obtained from Tauri's
/// `WebviewWindow::hwnd()`. It is stored and used only within this process.
pub fn install(hwnd: *mut core::ffi::c_void) {
    PILL_HWND.store(hwnd as usize, Ordering::Relaxed);

    let events = [
        EVENT_OBJECT_FOCUS,       // keyboard/mouse focus changed to any element
        EVENT_SYSTEM_FOREGROUND,  // foreground window changed (most common trigger)
        EVENT_SYSTEM_MINIMIZEEND, // window restored from minimize (can clear topmost)
    ];

    unsafe {
        for event in events {
            SetWinEventHook(
                event,
                event,
                None, // no DLL — OUTOFCONTEXT runs in our process
                Some(win_event_proc),
                0,
                0, // monitor all processes and threads
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
        }
    }
}
