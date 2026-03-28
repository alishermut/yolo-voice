//! Voice Activity Detection using Silero VAD v5 ONNX model.
//!
//! Uses the same `ort` runtime already pulled in by parakeet-rs, so no extra
//! native dependency is needed. The model file (`silero_vad_v5.onnx`) lives in
//! `resources/` and is resolved at runtime via Tauri's resource dir.
//!
//! The processor works in streaming mode: feed it 512-sample chunks of 16 kHz
//! mono audio and it returns a speech probability for each chunk.

use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;

// ── Constants ────────────────────────────────────────────────────────────────

/// Silero VAD v5 requires 16 kHz sample rate.
const VAD_SAMPLE_RATE: i64 = 16000;

/// Chunk size in samples for 16 kHz (fixed by the model architecture).
const CHUNK_SIZE: usize = 512;

/// Context size in samples prepended to each chunk (16 kHz model).
const CONTEXT_SIZE: usize = 64;

/// Default speech probability threshold.
const DEFAULT_THRESHOLD: f32 = 0.5;

/// Minimum speech duration in ms — ignore blips shorter than this.
const MIN_SPEECH_MS: u32 = 350;

/// Padding added after speech end to avoid cutting off trailing sounds (ms).
const SPEECH_PAD_MS: u32 = 120;

// ── Public types ─────────────────────────────────────────────────────────────

/// A completed speech segment extracted from the audio stream.
pub struct SpeechSegment {
    pub samples: Vec<f32>,
}

/// Streaming VAD processor. Feed it raw 16 kHz mono f32 chunks.
pub struct VadProcessor {
    session: Session,
    /// RNN hidden state — flat [2 * 1 * 128] = 256 floats.
    state: Vec<f32>,
    /// Context carried over from the previous chunk.
    context: Vec<f32>,

    // ── Segmentation state ───────────────────────────────────────────────
    in_speech: bool,
    speech_buf: Vec<f32>,
    silence_chunks: u32,
    silence_limit: u32,
    min_speech_chunks: u32,
    speech_chunk_count: u32,
    pad_chunks: u32,
    remaining_pad: u32,
    threshold: f32,
}

impl VadProcessor {
    /// Create a new VAD processor from the ONNX model path.
    pub fn new(model_path: &Path, silence_threshold_ms: u32) -> Result<Self, String> {
        let session = Session::builder()
            .map_err(|e| format!("VAD session builder error: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| format!("VAD thread config error: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("VAD model load error: {e}"))?;

        let chunk_duration_ms = (CHUNK_SIZE as f64 / VAD_SAMPLE_RATE as f64 * 1000.0) as u32;
        let silence_limit = silence_threshold_ms / chunk_duration_ms.max(1);
        let min_speech_chunks = MIN_SPEECH_MS / chunk_duration_ms.max(1);
        let pad_chunks = SPEECH_PAD_MS / chunk_duration_ms.max(1);

        Ok(Self {
            session,
            state: vec![0.0f32; 2 * 1 * 128],
            context: vec![0.0f32; CONTEXT_SIZE],
            in_speech: false,
            speech_buf: Vec::new(),
            silence_chunks: 0,
            silence_limit,
            min_speech_chunks,
            speech_chunk_count: 0,
            pad_chunks,
            remaining_pad: 0,
            threshold: DEFAULT_THRESHOLD,
        })
    }

    /// Process a buffer of 16 kHz mono f32 samples. Returns completed segments.
    pub fn process(&mut self, samples: &[f32]) -> Vec<SpeechSegment> {
        let mut segments = Vec::new();

        let mut offset = 0;
        while offset + CHUNK_SIZE <= samples.len() {
            let chunk = &samples[offset..offset + CHUNK_SIZE];
            let prob = self.forward(chunk);

            if let Some(seg) = self.update_segmentation(chunk, prob) {
                segments.push(seg);
            }

            offset += CHUNK_SIZE;
        }

        segments
    }

    /// Flush any in-progress speech segment.
    pub fn flush(&mut self) -> Option<SpeechSegment> {
        if self.in_speech && self.speech_chunk_count >= self.min_speech_chunks {
            let seg = SpeechSegment {
                samples: std::mem::take(&mut self.speech_buf),
            };
            self.in_speech = false;
            self.speech_chunk_count = 0;
            self.silence_chunks = 0;
            self.remaining_pad = 0;
            Some(seg)
        } else {
            self.speech_buf.clear();
            self.in_speech = false;
            self.speech_chunk_count = 0;
            None
        }
    }

    /// Reset all internal state.
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.state = vec![0.0f32; 2 * 1 * 128];
        self.context = vec![0.0f32; CONTEXT_SIZE];
        self.in_speech = false;
        self.speech_buf.clear();
        self.silence_chunks = 0;
        self.speech_chunk_count = 0;
        self.remaining_pad = 0;
    }

    // ── Private ──────────────────────────────────────────────────────────

    /// Run one forward pass of the Silero VAD model.
    fn forward(&mut self, chunk: &[f32]) -> f32 {
        // Build input: context + chunk → shape [1, CONTEXT_SIZE + CHUNK_SIZE]
        let mut input_vec = Vec::with_capacity(CONTEXT_SIZE + CHUNK_SIZE);
        input_vec.extend_from_slice(&self.context);
        input_vec.extend_from_slice(chunk);

        let input_tensor = match Tensor::from_array(([1usize, input_vec.len()], input_vec.clone())) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[vad] input tensor error: {e}");
                return 0.0;
            }
        };

        // State: shape [2, 1, 128]
        let state_tensor = match Tensor::from_array(([2usize, 1, 128], self.state.clone())) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[vad] state tensor error: {e}");
                return 0.0;
            }
        };

        // Sample rate: shape [1]
        let sr_tensor = match Tensor::from_array(([1usize], vec![VAD_SAMPLE_RATE])) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[vad] sr tensor error: {e}");
                return 0.0;
            }
        };

        // Run inference
        let outputs = match self.session.run(ort::inputs![input_tensor, state_tensor, sr_tensor]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[vad] inference error: {e}");
                return 0.0;
            }
        };

        // Extract speech probability from output[0]
        let prob = match outputs[0].try_extract_tensor::<f32>() {
            Ok((_shape, data)) => data.first().copied().unwrap_or(0.0),
            Err(e) => {
                eprintln!("[vad] output extraction error: {e}");
                0.0
            }
        };

        // Extract new state from output[1] — shape [2, 1, 128]
        if let Ok((_shape, data)) = outputs[1].try_extract_tensor::<f32>() {
            if data.len() == 256 {
                self.state.copy_from_slice(data);
            }
        }

        // Update context for next chunk
        let chunk_len = chunk.len();
        if chunk_len >= CONTEXT_SIZE {
            self.context.copy_from_slice(&chunk[chunk_len - CONTEXT_SIZE..]);
        }

        prob
    }

    /// Update the speech/silence state machine.
    fn update_segmentation(&mut self, chunk: &[f32], prob: f32) -> Option<SpeechSegment> {
        let is_speech = prob >= self.threshold;

        if self.remaining_pad > 0 {
            self.speech_buf.extend_from_slice(chunk);
            self.remaining_pad -= 1;

            if self.remaining_pad == 0 {
                if self.speech_chunk_count >= self.min_speech_chunks {
                    let seg = SpeechSegment {
                        samples: std::mem::take(&mut self.speech_buf),
                    };
                    self.in_speech = false;
                    self.speech_chunk_count = 0;
                    self.silence_chunks = 0;
                    return Some(seg);
                } else {
                    self.speech_buf.clear();
                    self.in_speech = false;
                    self.speech_chunk_count = 0;
                    self.silence_chunks = 0;
                }
            } else if is_speech {
                // Speech resumed during padding — cancel the end
                self.remaining_pad = 0;
                self.silence_chunks = 0;
                self.speech_chunk_count += 1;
            }

            return None;
        }

        if is_speech {
            if !self.in_speech {
                self.in_speech = true;
                self.speech_chunk_count = 0;
                self.silence_chunks = 0;
            }
            self.speech_buf.extend_from_slice(chunk);
            self.speech_chunk_count += 1;
            self.silence_chunks = 0;
        } else if self.in_speech {
            self.speech_buf.extend_from_slice(chunk);
            self.silence_chunks += 1;

            if self.silence_chunks >= self.silence_limit {
                if self.pad_chunks > 0 {
                    self.remaining_pad = self.pad_chunks;
                } else {
                    if self.speech_chunk_count >= self.min_speech_chunks {
                        let seg = SpeechSegment {
                            samples: std::mem::take(&mut self.speech_buf),
                        };
                        self.in_speech = false;
                        self.speech_chunk_count = 0;
                        self.silence_chunks = 0;
                        return Some(seg);
                    } else {
                        self.speech_buf.clear();
                        self.in_speech = false;
                        self.speech_chunk_count = 0;
                        self.silence_chunks = 0;
                    }
                }
            }
        }

        None
    }
}
