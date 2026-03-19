use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::{AppHandle, Emitter, Manager};
use crate::commands::PillUiState;

pub struct RecordingStream {
    _stream: cpal::Stream,
    stop_flag: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

unsafe impl Send for RecordingStream {}

impl Drop for RecordingStream {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

pub struct RecordingState(pub Mutex<Option<RecordingStream>>);

pub fn start_recording(
    device_index: usize,
    app_handle: AppHandle,
) -> Result<RecordingStream, String> {
    let host = cpal::default_host();
    let device = host
        .input_devices()
        .map_err(|e| e.to_string())?
        .nth(device_index)
        .ok_or_else(|| "Device not found".to_string())?;

    let config = device.default_input_config().map_err(|e| e.to_string())?;

    let sample_format = config.sample_format();
    let sample_rate = config.sample_rate();
    let channels = config.channels();
    let stream_config: cpal::StreamConfig = config.into();

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_writer = samples.clone();

    let rms = Arc::new(AtomicU32::new(0));
    let rms_writer = rms.clone();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_reader = stop_flag.clone();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut buf) = samples_writer.lock() {
                    buf.extend_from_slice(data);
                }
                let sum: f32 = data.iter().map(|s| s * s).sum();
                let rms_val = (sum / data.len() as f32).sqrt();
                rms_writer.store(rms_val.to_bits(), Ordering::Relaxed);
            },
            |err| eprintln!("Recording stream error: {}", err),
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let floats: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                if let Ok(mut buf) = samples_writer.lock() {
                    buf.extend_from_slice(&floats);
                }
                let sum: f32 = floats.iter().map(|s| s * s).sum();
                let rms_val = (sum / floats.len() as f32).sqrt();
                rms_writer.store(rms_val.to_bits(), Ordering::Relaxed);
            },
            |err| eprintln!("Recording stream error: {}", err),
            None,
        ),
        _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    // Polling thread: emits recording-level events at ~30fps + updates shared state
    let rms_reader = rms.clone();
    std::thread::spawn(move || {
        let mut frame_count = 0u32;
        while !stop_reader.load(Ordering::Relaxed) {
            let raw_rms = f32::from_bits(rms_reader.load(Ordering::Relaxed));
            // Normalize: raw RMS is typically 0.0001-0.1 for speech.
            // Convert to 0-100 scale with aggressive amplification.
            let normalized = (raw_rms * 1500.0).min(100.0);
            let _ = app_handle.emit("recording-level", normalized);
            // Update shared PillUiState for polling
            if let Ok(mut lv) = app_handle.state::<PillUiState>().audio_level.lock() {
                *lv = normalized;
            }
            // Log every ~1 second so we can debug
            frame_count += 1;
            if frame_count % 30 == 0 {
                eprintln!("[recorder] raw_rms={:.5} normalized={:.1}", raw_rms, normalized);
            }
            std::thread::sleep(Duration::from_millis(33));
        }
    });

    Ok(RecordingStream {
        _stream: stream,
        stop_flag,
        samples,
        sample_rate,
        channels,
    })
}

pub fn stop_and_save(recording: RecordingStream) -> Result<PathBuf, String> {
    // Signal stop
    recording.stop_flag.store(true, Ordering::Relaxed);

    let samples = recording
        .samples
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let path = std::env::temp_dir().join("yolo_voice_recording.wav");

    let spec = hound::WavSpec {
        channels: recording.channels,
        sample_rate: recording.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(&path, spec).map_err(|e| e.to_string())?;

    for sample in &samples {
        writer.write_sample(*sample).map_err(|e| e.to_string())?;
    }

    writer.finalize().map_err(|e| e.to_string())?;

    Ok(path)
}

/// Stop recording and return WAV bytes in memory (no disk I/O).
pub fn stop_and_get_wav_bytes(recording: RecordingStream) -> Result<Vec<u8>, String> {
    // Signal stop
    recording.stop_flag.store(true, Ordering::Relaxed);

    let samples = recording
        .samples
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    // Build WAV in memory manually (WAV format is simple for PCM float32)
    let num_channels = recording.channels as u32;
    let sample_rate = recording.sample_rate;
    let bits_per_sample: u32 = 32;
    let byte_rate = sample_rate * num_channels * bits_per_sample / 8;
    let block_align = (num_channels * bits_per_sample / 8) as u16;
    let data_size = (samples.len() * 4) as u32;
    let file_size = 36 + data_size; // 36 = header size minus 8 bytes for RIFF chunk header

    let mut buf: Vec<u8> = Vec::with_capacity(44 + samples.len() * 4);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt sub-chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size
    buf.extend_from_slice(&3u16.to_le_bytes()); // audio format: 3 = IEEE float
    buf.extend_from_slice(&(num_channels as u16).to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&(bits_per_sample as u16).to_le_bytes());

    // data sub-chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for sample in &samples {
        buf.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(buf)
}
