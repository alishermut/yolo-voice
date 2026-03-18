use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_READ, REG_SZ, RegQueryValueExW,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "YOLOVoice";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Enable or disable launch on Windows startup via the registry.
pub fn set_launch_on_startup(enable: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();

    unsafe {
        let mut key = HKEY::default();
        let subkey = to_wide(RUN_KEY);

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        );

        if result.is_err() {
            return Err(format!("Failed to open registry key: {:?}", result));
        }

        let value_name = to_wide(VALUE_NAME);

        if enable {
            let value_data = to_wide(&format!("\"{}\"", exe_str));
            let data_bytes = std::slice::from_raw_parts(
                value_data.as_ptr() as *const u8,
                value_data.len() * 2,
            );

            let result = RegSetValueExW(
                key,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(data_bytes),
            );

            let _ = RegCloseKey(key);
            if result.is_err() {
                return Err(format!("Failed to set registry value: {:?}", result));
            }
        } else {
            let result = RegDeleteValueW(key, PCWSTR(value_name.as_ptr()));
            let _ = RegCloseKey(key);
            // Ignore "not found" errors when disabling
            if result.is_err() {
                eprintln!("Registry delete (may not exist): {:?}", result);
            }
        }
    }

    Ok(())
}

/// Check if launch-on-startup is currently enabled in the registry.
pub fn is_launch_on_startup() -> bool {
    unsafe {
        let mut key = HKEY::default();
        let subkey = to_wide(RUN_KEY);

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut key,
        );

        if result.is_err() {
            return false;
        }

        let value_name = to_wide(VALUE_NAME);
        let mut data_size: u32 = 0;

        let result = RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            None,
            Some(&mut data_size),
        );

        let _ = RegCloseKey(key);
        result.is_ok() && data_size > 0
    }
}
