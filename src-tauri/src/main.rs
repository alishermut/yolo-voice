// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // In release builds, redirect stderr to a log file so we can diagnose issues
    if !cfg!(debug_assertions) {
        if let Some(app_data) = dirs_next::data_dir() {
            let log_dir = app_data.join("com.alish.yolo-voice");
            let _ = std::fs::create_dir_all(&log_dir);
            let log_path = log_dir.join("yolo-voice.log");
            // Truncate to last 50KB if too large (keeps recent logs only)
            if let Ok(meta) = std::fs::metadata(&log_path) {
                if meta.len() > 500_000 {
                    if let Ok(content) = std::fs::read_to_string(&log_path) {
                        let keep = &content[content.len().saturating_sub(50_000)..];
                        let _ = std::fs::write(&log_path, keep);
                    }
                }
            }
            if let Ok(file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                use std::os::windows::io::IntoRawHandle;
                let handle = file.into_raw_handle();
                unsafe {
                    use windows_sys::Win32::System::Console::SetStdHandle;
                    use windows_sys::Win32::System::Console::STD_ERROR_HANDLE;
                    SetStdHandle(STD_ERROR_HANDLE, handle as _);
                }
                // Write session header
                eprintln!(
                    "\n=== YOLO Voice session started at {:?} ===",
                    std::time::SystemTime::now()
                );
            }
        }
    }

    yolo_voice_lib::run()
}
