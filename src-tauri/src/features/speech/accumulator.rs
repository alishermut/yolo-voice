//! Segment accumulator for the VAD path.
//!
//! Each segment is transcribed independently, lightly cleaned when enabled,
//! and then joined into a progressive preview. Stronger cleanup happens only
//! after final assembly in capture/mod.rs.

use std::sync::mpsc;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::cleanup;
use super::inference::InferenceState;
use super::vad::SpeechSegment;
use crate::app::events::emit_all;

#[derive(Debug, Clone, Serialize)]
pub struct SegmentTranscribed {
    pub index: usize,
    pub text: String,
    pub full_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedSegments {
    pub raw_segments: Vec<String>,
    pub joined_text: String,
}

#[derive(Clone)]
pub struct SegmentSender {
    tx: mpsc::Sender<AccMsg>,
}

impl SegmentSender {
    pub fn submit(&self, segment: SpeechSegment) {
        let _ = self.tx.send(AccMsg::Segment(segment));
    }
}

enum AccMsg {
    Segment(SpeechSegment),
    Flush(mpsc::Sender<FinalizedSegments>),
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

    pub fn finalize(self) -> FinalizedSegments {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(AccMsg::Flush(reply_tx));
        reply_rx.recv().unwrap_or_else(|_| FinalizedSegments {
            raw_segments: Vec::new(),
            joined_text: String::new(),
        })
    }

    fn worker(rx: mpsc::Receiver<AccMsg>, app: AppHandle, text_cleanup_enabled: bool) {
        let mut raw_segments: Vec<String> = Vec::new();
        let mut texts: Vec<String> = Vec::new();

        loop {
            match rx.recv() {
                Ok(AccMsg::Segment(segment)) => {
                    let index = texts.len();
                    let text = Self::transcribe_segment(&app, &segment);

                    match text {
                        Ok(raw_text) if !raw_text.trim().is_empty() => {
                            raw_segments.push(raw_text.trim().to_string());
                            let cleaned = Self::clean_segment(&raw_text, text_cleanup_enabled);

                            if !cleaned.is_empty() {
                                texts.push(cleaned);
                                let full = Self::assemble_preview(&texts, text_cleanup_enabled);

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
                        Err(err) => {
                            eprintln!("[accumulator] segment {} failed: {}, skipping", index, err);
                        }
                    }
                }
                Ok(AccMsg::Flush(reply)) => {
                    let _ = reply.send(FinalizedSegments {
                        raw_segments,
                        joined_text: Self::assemble_preview(&texts, text_cleanup_enabled),
                    });
                    break;
                }
                Err(_) => break,
            }
        }
    }

    fn clean_segment(raw_text: &str, text_cleanup_enabled: bool) -> String {
        if text_cleanup_enabled {
            cleanup::clean_segment_text(raw_text)
        } else {
            raw_text.trim().to_string()
        }
    }

    fn assemble_preview(texts: &[String], text_cleanup_enabled: bool) -> String {
        if text_cleanup_enabled {
            cleanup::join_segments_heuristic(texts)
        } else {
            cleanup::join_segments_minimal(texts)
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
