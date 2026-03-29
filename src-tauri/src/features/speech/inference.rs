use std::path::Path;
use std::sync::Mutex;

use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, Transcriber};

pub struct InferenceSession {
    parakeet: ParakeetTDT,
    gpu_available: bool,
}

// ParakeetTDT contains ONNX session which is Send but not Sync by default.
// We guard access with a Mutex via InferenceState, so this is safe.
unsafe impl Send for InferenceSession {}

pub struct InferenceState(pub Mutex<Option<InferenceSession>>);

impl InferenceSession {
    /// Create a new inference session from the model directory.
    /// Tries DirectML first, falls back to CPU.
    pub fn new(model_dir: &Path) -> Result<Self, String> {
        Self::with_gpu(model_dir, true)
    }

    /// Create a new inference session, optionally using GPU.
    pub fn with_gpu(model_dir: &Path, use_gpu: bool) -> Result<Self, String> {
        #[cfg(windows)]
        if use_gpu {
            match Self::try_create(model_dir, ExecutionProvider::DirectML) {
                Ok(p) => {
                    return Ok(InferenceSession {
                        parakeet: p,
                        gpu_available: true,
                    })
                }
                Err(e) => {
                    eprintln!("[inference] DirectML failed ({}), falling back to CPU", e);
                }
            }
        }

        #[cfg(not(windows))]
        if use_gpu {
            eprintln!("[inference] GPU acceleration not available on this platform, using CPU");
        }

        let p = Self::try_create(model_dir, ExecutionProvider::Cpu)
            .map_err(|e| format!("Failed to initialize inference on CPU: {}", e))?;
        Ok(InferenceSession {
            parakeet: p,
            gpu_available: false,
        })
    }

    fn try_create(model_dir: &Path, provider: ExecutionProvider) -> Result<ParakeetTDT, String> {
        let config = ExecutionConfig::new().with_execution_provider(provider);
        ParakeetTDT::from_pretrained(model_dir, Some(config)).map_err(|e| format!("{}", e))
    }

    /// Transcribe raw audio samples.
    /// Handles stereo→mono conversion and resampling to 16kHz internally.
    pub fn transcribe(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<String, String> {
        if samples.is_empty() {
            return Ok(String::new());
        }

        let audio = prepare_audio(samples, sample_rate, channels)?;
        if audio.is_empty() {
            return Ok(String::new());
        }

        let result = self
            .parakeet
            .transcribe_samples(audio, 16000, 1, None)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        Ok(result.text)
    }

    pub fn is_gpu(&self) -> bool {
        self.gpu_available
    }
}

/// Convert audio to 16kHz mono f32 suitable for parakeet-rs.
fn prepare_audio(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<f32>, String> {
    let mono = if channels > 1 {
        // Average channels to mono
        let ch = channels as usize;
        let num_frames = samples.len() / ch;
        let mut mono = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            let mut sum = 0.0f32;
            for c in 0..ch {
                sum += samples[i * ch + c];
            }
            mono.push(sum / ch as f32);
        }
        mono
    } else {
        samples.to_vec()
    };

    if sample_rate == 16000 {
        return Ok(mono);
    }

    if mono.is_empty() {
        return Ok(mono);
    }

    // Resample to 16kHz using rubato
    use rubato::{FftFixedIn, Resampler};

    let mut resampler = FftFixedIn::<f32>::new(
        sample_rate as usize,
        16000,
        mono.len().min(1024).max(1), // chunk size — must be ≥ 1
        1,                           // sub chunks
        1,                           // channels
    )
    .map_err(|e| format!("Resampler init failed: {}", e))?;

    let mut output = Vec::new();
    let chunk_size = resampler.input_frames_max();
    if chunk_size == 0 {
        return Err("Resampler returned zero chunk size".to_string());
    }

    for chunk_start in (0..mono.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(mono.len());
        let mut chunk = mono[chunk_start..chunk_end].to_vec();

        // Pad last chunk if needed
        if chunk.len() < resampler.input_frames_next() {
            chunk.resize(resampler.input_frames_next(), 0.0);
        }

        let resampled = resampler
            .process(&[chunk], None)
            .map_err(|e| format!("Resampling failed: {}", e))?;

        if let Some(channel) = resampled.first() {
            output.extend_from_slice(channel);
        }
    }

    Ok(output)
}
