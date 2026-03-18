use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use windows::Win32::Media::Audio::{
    eCapture, IMMDeviceEnumerator, MMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::System::Com::STGM;

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub index: usize,
}

/// Get full device names from Windows Core Audio API (IMMDeviceEnumerator).
/// Returns names like "Microphone (Realtek(R) Audio)", "Microphone (2- Trust GXT 232 Microphone)".
fn get_windows_audio_device_names() -> Vec<String> {
    let mut names = Vec::new();

    unsafe {
        // COM might already be initialized, ignore errors
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(_) => return names,
            };

        let collection = match enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) {
            Ok(c) => c,
            Err(_) => return names,
        };

        let count = match collection.GetCount() {
            Ok(c) => c,
            Err(_) => return names,
        };

        for i in 0..count {
            let device = match collection.Item(i) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let store = match device.OpenPropertyStore(STGM(0)) { // STGM_READ = 0
                Ok(s) => s,
                Err(_) => continue,
            };

            let prop = match store.GetValue(&PKEY_Device_FriendlyName) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let name = prop.to_string();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }

    names
}

pub fn list_input_devices() -> Vec<DeviceInfo> {
    // Get full friendly names from Windows API
    let win_names = get_windows_audio_device_names();
    eprintln!("[audio] Windows API found {} capture devices:", win_names.len());
    for (i, name) in win_names.iter().enumerate() {
        eprintln!("[audio]   Win[{}]: \"{}\"", i, name);
    }

    // Get cpal devices — both lists use WASAPI so indices should match
    let host = cpal::default_host();
    let result: Vec<DeviceInfo> = host.input_devices()
        .map(|devices| {
            devices
                .enumerate()
                .map(|(i, d)| {
                    #[allow(deprecated)]
                    let cpal_name = d.name().unwrap_or_else(|_| "Unknown".to_string());
                    eprintln!("[audio]   cpal[{}]: \"{}\"", i, cpal_name);

                    // Use Windows name at same index if available (both lists are WASAPI-ordered)
                    let full_name = if i < win_names.len() {
                        win_names[i].clone()
                    } else {
                        cpal_name
                    };

                    DeviceInfo { name: full_name, index: i }
                })
                .collect()
        })
        .unwrap_or_default();

    eprintln!("[audio] Final device list:");
    for dev in &result {
        eprintln!("[audio]   [{}] \"{}\"", dev.index, dev.name);
    }

    result
}

pub struct AudioStream {
    _stream: cpal::Stream,
    stop_flag: Arc<AtomicBool>,
}

// cpal::Stream is not Send by default on some platforms,
// but on Windows WASAPI it is safe to move between threads.
unsafe impl Send for AudioStream {}

impl Drop for AudioStream {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

pub fn start_level_monitor(
    device_index: usize,
    app_handle: AppHandle,
) -> Result<AudioStream, String> {
    let host = cpal::default_host();
    let device = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .nth(device_index)
        .ok_or_else(|| "Device not found".to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| e.to_string())?;

    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let rms = Arc::new(AtomicU32::new(0));
    let rms_writer = rms.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_reader = stop_flag.clone();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let sum: f32 = data.iter().map(|s| s * s).sum();
                let rms_val = (sum / data.len() as f32).sqrt();
                rms_writer.store(rms_val.to_bits(), Ordering::Relaxed);
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let sum: f32 = data
                    .iter()
                    .map(|&s| {
                        let f = s as f32 / i16::MAX as f32;
                        f * f
                    })
                    .sum();
                let rms_val = (sum / data.len() as f32).sqrt();
                rms_writer.store(rms_val.to_bits(), Ordering::Relaxed);
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ),
        _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    // Polling thread: emits audio-level events at ~30fps
    let rms_reader = rms.clone();
    std::thread::spawn(move || {
        while !stop_reader.load(Ordering::Relaxed) {
            let level = f32::from_bits(rms_reader.load(Ordering::Relaxed));
            let _ = app_handle.emit("audio-level", level);
            std::thread::sleep(Duration::from_millis(33));
        }
    });

    Ok(AudioStream {
        _stream: stream,
        stop_flag,
    })
}
