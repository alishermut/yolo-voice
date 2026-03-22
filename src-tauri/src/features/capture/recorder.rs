use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::AppHandle;

use crate::features::speech::accumulator::{SegmentAccumulator, SegmentSender};
use crate::features::speech::vad::VadProcessor;

// ── Types ────────────────────────────────────────────────────────────────────

pub struct RecordingStream {
    _stream: cpal::Stream,
    stop_flag: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    /// Present when VAD mode is active.
    accumulator: Option<SegmentAccumulator>,
}

unsafe impl Send for RecordingStream {}

impl Drop for RecordingStream {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

pub struct RecordingState(pub Mutex<Option<RecordingStream>>);

/// Configuration for VAD-enabled recording.
pub struct VadConfig {
    pub silence_threshold_ms: u32,
    pub model_path: PathBuf,
    pub text_cleanup_enabled: bool,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Start recording from the given device.
///
/// If `vad_config` is `Some`, VAD segmentation is enabled: audio is streamed
/// through Silero VAD and each detected speech segment is transcribed in the
/// background via the `SegmentAccumulator`.
pub fn start_recording(
    device_index: usize,
    app_handle: AppHandle,
    vad_config: Option<VadConfig>,
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

    // ── VAD setup (optional) ─────────────────────────────────────────────
    let (vad_audio_tx, accumulator) = if let Some(cfg) = vad_config {
        let (accumulator, segment_sender) = SegmentAccumulator::new(
            app_handle.clone(),
            cfg.text_cleanup_enabled,
        );

        // Channel for raw audio: callback → VAD thread
        let (audio_tx, audio_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        let vad_stop = stop_flag.clone();
        let model_path = cfg.model_path;
        let silence_ms = cfg.silence_threshold_ms;
        let dev_rate = sample_rate;
        let dev_ch = channels;

        std::thread::Builder::new()
            .name("vad-processor".into())
            .spawn(move || {
                vad_thread(audio_rx, vad_stop, &model_path, silence_ms, segment_sender, dev_rate, dev_ch);
            })
            .map_err(|e| format!("Failed to spawn VAD thread: {e}"))?;

        (Some(audio_tx), Some(accumulator))
    } else {
        (None, None)
    };

    let vad_tx = vad_audio_tx;

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let vad_tx = vad_tx.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = samples_writer.lock() {
                        buf.extend_from_slice(data);
                    }
                    let sum: f32 = data.iter().map(|s| s * s).sum();
                    let rms_val = (sum / data.len() as f32).sqrt();
                    rms_writer.store(rms_val.to_bits(), Ordering::Relaxed);

                    if let Some(tx) = &vad_tx {
                        let _ = tx.send(data.to_vec());
                    }
                },
                |err| eprintln!("Recording stream error: {}", err),
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let vad_tx = vad_tx.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let floats: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    if let Ok(mut buf) = samples_writer.lock() {
                        buf.extend_from_slice(&floats);
                    }
                    let sum: f32 = floats.iter().map(|s| s * s).sum();
                    let rms_val = (sum / floats.len() as f32).sqrt();
                    rms_writer.store(rms_val.to_bits(), Ordering::Relaxed);

                    if let Some(tx) = &vad_tx {
                        let _ = tx.send(floats);
                    }
                },
                |err| eprintln!("Recording stream error: {}", err),
                None,
            )
        }
        _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
    }
    .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    // ── RMS emission thread (~30fps) ─────────────────────────────────────
    let rms_reader = rms.clone();
    std::thread::spawn(move || {
        let mut smoothed: f32 = 0.0;
        while !stop_reader.load(Ordering::Relaxed) {
            let raw_rms = f32::from_bits(rms_reader.load(Ordering::Relaxed));
            let gated = if raw_rms < 0.005 { 0.0 } else { raw_rms };
            let alpha = if gated > smoothed { 0.5 } else { 0.35 };
            smoothed = smoothed + alpha * (gated - smoothed);
            let normalized = (smoothed * 1100.0).min(100.0);
            crate::app::events::emit_all(&app_handle, "recording-level", normalized);
            std::thread::sleep(Duration::from_millis(33));
        }
    });

    Ok(RecordingStream {
        _stream: stream,
        stop_flag,
        samples,
        sample_rate,
        channels,
        accumulator,
    })
}

/// Stop recording and save audio to a WAV file (for cloud APIs).
pub fn stop_and_save(recording: RecordingStream) -> Result<PathBuf, String> {
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

/// Stop recording and return raw audio samples with metadata (no encoding).
pub fn stop_and_get_raw_samples(
    recording: RecordingStream,
) -> Result<(Vec<f32>, u32, u16), String> {
    recording.stop_flag.store(true, Ordering::Relaxed);

    let samples = recording
        .samples
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    Ok((samples, recording.sample_rate, recording.channels))
}

/// Stop a VAD-enabled recording and return assembled text.
pub fn stop_vad_recording(mut recording: RecordingStream) -> Result<String, String> {
    recording.stop_flag.store(true, Ordering::Relaxed);

    // Give the VAD thread time to process remaining audio
    std::thread::sleep(Duration::from_millis(150));

    let accumulator = recording
        .accumulator
        .take()
        .ok_or_else(|| "No VAD accumulator — not a VAD recording".to_string())?;

    let full_text = accumulator.finalize();
    Ok(full_text)
}

/// Check if this recording has VAD active.
pub fn has_vad(recording: &RecordingStream) -> bool {
    recording.accumulator.is_some()
}

/// Stop recording and return WAV bytes in memory (no disk I/O).
#[allow(dead_code)]
pub fn stop_and_get_wav_bytes(recording: RecordingStream) -> Result<Vec<u8>, String> {
    recording.stop_flag.store(true, Ordering::Relaxed);

    let samples = recording
        .samples
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let num_channels = recording.channels as u32;
    let sample_rate = recording.sample_rate;
    let bits_per_sample: u32 = 32;
    let byte_rate = sample_rate * num_channels * bits_per_sample / 8;
    let block_align = (num_channels * bits_per_sample / 8) as u16;
    let data_size = (samples.len() * 4) as u32;
    let file_size = 36 + data_size;

    let mut buf: Vec<u8> = Vec::with_capacity(44 + samples.len() * 4);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&3u16.to_le_bytes());
    buf.extend_from_slice(&(num_channels as u16).to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&(bits_per_sample as u16).to_le_bytes());

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for sample in &samples {
        buf.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(buf)
}

// ── VAD thread ───────────────────────────────────────────────────────────────

fn vad_thread(
    rx: std::sync::mpsc::Receiver<Vec<f32>>,
    stop_flag: Arc<AtomicBool>,
    model_path: &std::path::Path,
    silence_ms: u32,
    segment_sender: SegmentSender,
    device_sample_rate: u32,
    device_channels: u16,
) {
    let mut vad = match VadProcessor::new(model_path, silence_ms) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[vad-thread] Failed to initialize VAD: {e}");
            return;
        }
    };

    let ch = device_channels as usize;
    let needs_resample = device_sample_rate != 16000;
    let ratio = 16000.0 / device_sample_rate as f64;

    // Buffer for accumulating resampled samples until we have enough
    // for VAD processing (needs 512-sample chunks at 16kHz).
    // Audio callbacks send small chunks (e.g. 480 @ 48kHz = 160 @ 16kHz).
    let mut vad_buf: Vec<f32> = Vec::with_capacity(4096);

    while let Ok(raw_chunk) = rx.recv() {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        // Convert to mono
        let mono: Vec<f32> = if ch > 1 {
            let num_frames = raw_chunk.len() / ch;
            (0..num_frames)
                .map(|i| {
                    let mut sum = 0.0f32;
                    for c in 0..ch {
                        sum += raw_chunk[i * ch + c];
                    }
                    sum / ch as f32
                })
                .collect()
        } else {
            raw_chunk
        };

        // Resample to 16 kHz (linear interpolation — adequate for VAD)
        if needs_resample {
            let out_len = (mono.len() as f64 * ratio) as usize;
            for i in 0..out_len {
                let src_idx = i as f64 / ratio;
                let idx0 = src_idx as usize;
                let frac = (src_idx - idx0 as f64) as f32;
                let s0 = mono.get(idx0).copied().unwrap_or(0.0);
                let s1 = mono.get(idx0 + 1).copied().unwrap_or(s0);
                vad_buf.push(s0 + (s1 - s0) * frac);
            }
        } else {
            vad_buf.extend_from_slice(&mono);
        };

        // Feed buffered audio to VAD (it processes in 512-sample chunks internally)
        if vad_buf.len() >= 512 {
            let segments = vad.process(&vad_buf);
            // Keep leftover samples that didn't fill a complete chunk
            let consumed = (vad_buf.len() / 512) * 512;
            let leftover = vad_buf[consumed..].to_vec();
            vad_buf = leftover;

            for seg in segments {
                segment_sender.submit(seg);
            }
        }
    }

    // Process any remaining buffered audio
    if vad_buf.len() >= 512 {
        let segments = vad.process(&vad_buf);
        for seg in segments {
            segment_sender.submit(seg);
        }
    }

    if let Some(seg) = vad.flush() {
        segment_sender.submit(seg);
    }

}
