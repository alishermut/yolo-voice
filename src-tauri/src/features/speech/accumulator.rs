//! Segment accumulator — receives speech segments from the VAD processor,
//! transcribes each one via parakeet-rs on a background thread, cleans up
//! the text, and emits `segment-transcribed` events.

use std::sync::mpsc;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::cleanup;
use super::inference::InferenceState;
use super::vad::SpeechSegment;
use crate::app::events::emit_all;

// ── Event payload ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SegmentTranscribed {
    pub index: usize,
    pub text: String,
    pub full_text: String,
}

// ── Segment sender ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SegmentSender {
    tx: mpsc::Sender<AccMsg>,
}

impl SegmentSender {
    pub fn submit(&self, segment: SpeechSegment) {
        let _ = self.tx.send(AccMsg::Segment(segment));
    }
}

// ── Accumulator ──────────────────────────────────────────────────────────────

enum AccMsg {
    Segment(SpeechSegment),
    Flush(mpsc::Sender<String>),
}

pub struct SegmentAccumulator {
    tx: mpsc::Sender<AccMsg>,
}

impl SegmentAccumulator {
    pub fn new(app: AppHandle, text_cleanup_enabled: bool) -> (Self, SegmentSender) {
        let (tx, rx) = mpsc::channel::<AccMsg>();
        let sender = SegmentSender { tx: tx.clone() };

        std::thread::Builder::new()
            .name("vad-transcriber".into())
            .spawn(move || {
                Self::worker(rx, app, text_cleanup_enabled);
            })
            .expect("Failed to spawn vad-transcriber thread");

        (Self { tx }, sender)
    }

    pub fn finalize(self) -> String {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(AccMsg::Flush(reply_tx));
        reply_rx.recv().unwrap_or_default()
    }

    // ── Worker thread ────────────────────────────────────────────────────

    fn worker(rx: mpsc::Receiver<AccMsg>, app: AppHandle, text_cleanup: bool) {
        let mut texts: Vec<String> = Vec::new();

        loop {
            match rx.recv() {
                Ok(AccMsg::Segment(segment)) => {
                    let index = texts.len();
                    let text = Self::transcribe_segment(&app, &segment);

                    match text {
                        Ok(t) if !t.trim().is_empty() => {
                            // Try local LLM cleanup first, fall back to regex
                            let cleaned = Self::apply_cleanup(&t, text_cleanup);

                            if !cleaned.is_empty() {
                                texts.push(cleaned);
                                let full = cleanup::smart_join(&texts);

                                emit_all(
                                    &app,
                                    "segment-transcribed",
                                    SegmentTranscribed {
                                        index,
                                        text: texts.last().cloned().unwrap_or_default(),
                                        full_text: full,
                                    },
                                );
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!(
                                "[accumulator] segment {} failed: {}, skipping",
                                index, e
                            );
                        }
                    }
                }

                Ok(AccMsg::Flush(reply)) => {
                    let full = cleanup::smart_join(&texts);
                    let _ = reply.send(full);
                    break;
                }

                Err(_) => {
                    break;
                }
            }
        }
    }

    /// Apply text cleanup: regex-based filler removal, or pass through.
    fn apply_cleanup(raw_text: &str, regex_cleanup: bool) -> String {
        if regex_cleanup {
            cleanup::clean_text(raw_text)
        } else {
            raw_text.trim().to_string()
        }
    }

    fn transcribe_segment(app: &AppHandle, segment: &SpeechSegment) -> Result<String, String> {
        let inference_state = app.state::<InferenceState>();
        let mut guard = inference_state.0.lock().map_err(|e| e.to_string())?;
        let session = guard
            .as_mut()
            .ok_or_else(|| "Inference engine not initialized".to_string())?;

        session.transcribe(&segment.samples, 16000, 1)
    }
}
