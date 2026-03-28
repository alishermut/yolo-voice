use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::{AppHandle, Manager};

pub const TRANSCRIPT_DIAGNOSTICS_MAX_SAMPLES: u64 = 1000;

#[derive(Debug, Clone)]
pub struct TranscriptDiagnosticsStore {
    db_path: PathBuf,
    session_id: String,
    utterance_counter: Arc<AtomicU64>,
    tx: mpsc::Sender<DiagnosticsMsg>,
}

pub struct TranscriptDiagnosticsState(pub TranscriptDiagnosticsStore);

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TranscriptDiagnosticsStatus {
    pub enabled: bool,
    pub sample_count: u64,
    pub max_samples: u64,
    pub db_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptSample {
    pub created_at: i64,
    pub app_version: String,
    pub session_id: String,
    pub utterance_id: String,
    /// "dictation" or "command"
    pub pipeline_mode: String,
    pub transcription_mode: String,
    pub stt_provider: String,
    pub active_industry_pack: String,
    pub active_profile_id: String,
    pub cleanup_enabled: bool,
    pub post_processing_enabled: bool,
    pub vad_silence_threshold_ms: u32,
    pub utterance_duration_ms: u64,
    pub preview_segment_count: u32,
    pub final_pass_used: bool,
    pub final_pass_reason: String,
    pub final_pass_latency_ms: Option<i64>,
    pub language_family: String,
    pub language_lock_confidence: String,
    pub mixed_script_detected: bool,
    pub raw_segments_json: String,
    pub joined_text: Option<String>,
    pub normalized_text: Option<String>,
    pub cleaned_text: Option<String>,
    pub post_processed_text: Option<String>,
    pub final_text: Option<String>,
    pub inserted_text: Option<String>,
    pub insert_success: bool,
}

enum DiagnosticsMsg {
    Write(TranscriptSample),
    Flush(mpsc::Sender<()>),
}

impl TranscriptDiagnosticsStore {
    pub fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let db_path = diagnostics_db_path(app_handle)?;
        Self::from_db_path(db_path)
    }

    pub fn from_db_path(db_path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        bootstrap_database(&db_path)?;

        let (tx, rx) = mpsc::channel::<DiagnosticsMsg>();
        let writer_path = db_path.clone();

        std::thread::Builder::new()
            .name("transcript-diagnostics-writer".into())
            .spawn(move || writer_loop(writer_path, rx))
            .map_err(|e| format!("Failed to spawn transcript diagnostics writer: {e}"))?;

        Ok(Self {
            db_path,
            session_id: generate_session_id(),
            utterance_counter: Arc::new(AtomicU64::new(1)),
            tx,
        })
    }

    pub fn next_utterance_id(&self) -> String {
        let ordinal = self.utterance_counter.fetch_add(1, Ordering::Relaxed);
        format!("{}-{}", self.session_id, ordinal)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn status(&self, enabled: bool) -> Result<TranscriptDiagnosticsStatus, String> {
        Ok(TranscriptDiagnosticsStatus {
            enabled,
            sample_count: count_samples_at_path(&self.db_path)?,
            max_samples: TRANSCRIPT_DIAGNOSTICS_MAX_SAMPLES,
            db_path: self.db_path.to_string_lossy().to_string(),
        })
    }

    pub fn clear(&self, enabled: bool) -> Result<TranscriptDiagnosticsStatus, String> {
        self.flush_writer();
        let conn = open_connection(&self.db_path)?;
        clear_samples(&conn)?;
        self.status(enabled)
    }

    pub fn log_sample(&self, sample: TranscriptSample) {
        let _ = self.tx.send(DiagnosticsMsg::Write(sample));
    }

    fn flush_writer(&self) {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(DiagnosticsMsg::Flush(reply_tx));
        let _ = reply_rx.recv();
    }

    #[cfg(test)]
    pub fn flush_for_tests(&self) {
        self.flush_writer();
    }
}

pub fn diagnostics_db_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("diagnostics").join("transcript_samples.sqlite3"))
}

fn writer_loop(db_path: PathBuf, rx: mpsc::Receiver<DiagnosticsMsg>) {
    let conn = match open_connection(&db_path) {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!(
                "[diagnostics] Failed to open transcript diagnostics database: {}",
                err
            );
            return;
        }
    };

    while let Ok(msg) = rx.recv() {
        match msg {
            DiagnosticsMsg::Write(sample) => {
                if let Err(err) = insert_sample(&conn, &sample) {
                    eprintln!("[diagnostics] Failed to write transcript sample: {}", err);
                }
            }
            DiagnosticsMsg::Flush(reply) => {
                let _ = reply.send(());
            }
        }
    }
}

fn bootstrap_database(db_path: &Path) -> Result<(), String> {
    let _ = open_connection(db_path)?;
    Ok(())
}

fn open_connection(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        CREATE TABLE IF NOT EXISTS transcript_samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            app_version TEXT NOT NULL,
            session_id TEXT NOT NULL,
            utterance_id TEXT NOT NULL,
            pipeline_mode TEXT NOT NULL DEFAULT 'dictation',
            transcription_mode TEXT NOT NULL,
            stt_provider TEXT NOT NULL,
            active_industry_pack TEXT NOT NULL,
            active_profile_id TEXT NOT NULL,
            cleanup_enabled INTEGER NOT NULL,
            post_processing_enabled INTEGER NOT NULL,
            vad_silence_threshold_ms INTEGER NOT NULL,
            utterance_duration_ms INTEGER NOT NULL DEFAULT 0,
            preview_segment_count INTEGER NOT NULL DEFAULT 0,
            final_pass_used INTEGER NOT NULL DEFAULT 0,
            final_pass_reason TEXT NOT NULL DEFAULT 'single-pass',
            final_pass_latency_ms INTEGER,
            language_family TEXT NOT NULL DEFAULT 'unknown',
            language_lock_confidence TEXT NOT NULL DEFAULT 'none',
            mixed_script_detected INTEGER NOT NULL DEFAULT 0,
            raw_segments_json TEXT NOT NULL,
            joined_text TEXT,
            normalized_text TEXT,
            cleaned_text TEXT,
            post_processed_text TEXT,
            final_text TEXT,
            inserted_text TEXT,
            insert_success INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_transcript_samples_created_at
            ON transcript_samples(created_at DESC, id DESC);
        ",
    )
    .map_err(|e| e.to_string())?;

    // Migration: add pipeline_mode column to existing databases (ignored if already present)
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN pipeline_mode TEXT NOT NULL DEFAULT 'dictation';"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN utterance_duration_ms INTEGER NOT NULL DEFAULT 0;"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN preview_segment_count INTEGER NOT NULL DEFAULT 0;"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN final_pass_used INTEGER NOT NULL DEFAULT 0;"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN final_pass_reason TEXT NOT NULL DEFAULT 'single-pass';"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN final_pass_latency_ms INTEGER;"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN language_family TEXT NOT NULL DEFAULT 'unknown';"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN language_lock_confidence TEXT NOT NULL DEFAULT 'none';"
    );
    let _ = conn.execute_batch(
        "ALTER TABLE transcript_samples ADD COLUMN mixed_script_detected INTEGER NOT NULL DEFAULT 0;"
    );

    Ok(conn)
}

fn insert_sample(conn: &Connection, sample: &TranscriptSample) -> Result<(), String> {
    conn.execute(
        "
        INSERT INTO transcript_samples (
            created_at,
            app_version,
            session_id,
            utterance_id,
            pipeline_mode,
            transcription_mode,
            stt_provider,
            active_industry_pack,
            active_profile_id,
            cleanup_enabled,
            post_processing_enabled,
            vad_silence_threshold_ms,
            utterance_duration_ms,
            preview_segment_count,
            final_pass_used,
            final_pass_reason,
            final_pass_latency_ms,
            language_family,
            language_lock_confidence,
            mixed_script_detected,
            raw_segments_json,
            joined_text,
            normalized_text,
            cleaned_text,
            post_processed_text,
            final_text,
            inserted_text,
            insert_success
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)
        ",
        params![
            sample.created_at,
            sample.app_version,
            sample.session_id,
            sample.utterance_id,
            sample.pipeline_mode,
            sample.transcription_mode,
            sample.stt_provider,
            sample.active_industry_pack,
            sample.active_profile_id,
            sample.cleanup_enabled,
            sample.post_processing_enabled,
            sample.vad_silence_threshold_ms,
            sample.utterance_duration_ms,
            sample.preview_segment_count,
            sample.final_pass_used,
            sample.final_pass_reason,
            sample.final_pass_latency_ms,
            sample.language_family,
            sample.language_lock_confidence,
            sample.mixed_script_detected,
            sample.raw_segments_json,
            sample.joined_text,
            sample.normalized_text,
            sample.cleaned_text,
            sample.post_processed_text,
            sample.final_text,
            sample.inserted_text,
            sample.insert_success,
        ],
    )
    .map_err(|e| e.to_string())?;

    prune_old_samples(conn, TRANSCRIPT_DIAGNOSTICS_MAX_SAMPLES)?;
    Ok(())
}

fn prune_old_samples(conn: &Connection, max_samples: u64) -> Result<(), String> {
    conn.execute(
        "
        DELETE FROM transcript_samples
        WHERE id NOT IN (
            SELECT id
            FROM transcript_samples
            ORDER BY created_at DESC, id DESC
            LIMIT ?1
        )
        ",
        params![max_samples as i64],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn count_samples_at_path(db_path: &Path) -> Result<u64, String> {
    let conn = open_connection(db_path)?;
    count_samples(&conn)
}

fn count_samples(conn: &Connection) -> Result<u64, String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transcript_samples", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count.max(0) as u64)
}

fn clear_samples(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM transcript_samples", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// A lightweight transcript history entry for the user-facing history UI.
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptHistoryEntry {
    pub id: i64,
    pub created_at: i64,
    pub final_text: Option<String>,
    pub inserted_text: Option<String>,
    pub transcription_mode: String,
    pub stt_provider: String,
    pub pipeline_mode: String,
    pub insert_success: bool,
}

impl TranscriptDiagnosticsStore {
    /// Query paginated transcript history, newest first.
    pub fn list_history(
        &self,
        limit: u32,
        offset: u32,
        search: Option<&str>,
    ) -> Result<Vec<TranscriptHistoryEntry>, String> {
        self.flush_writer();
        let conn = open_connection(&self.db_path)?;

        let (query, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(term) = search {
            if term.trim().is_empty() {
                (
                    "SELECT id, created_at, final_text, inserted_text, transcription_mode, stt_provider, pipeline_mode, insert_success \
                     FROM transcript_samples ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2".to_string(),
                    vec![Box::new(limit as i64), Box::new(offset as i64)],
                )
            } else {
                let like = format!("%{}%", term.replace('%', "\\%").replace('_', "\\_"));
                (
                    "SELECT id, created_at, final_text, inserted_text, transcription_mode, stt_provider, pipeline_mode, insert_success \
                     FROM transcript_samples WHERE final_text LIKE ?1 ESCAPE '\\' \
                     ORDER BY created_at DESC, id DESC LIMIT ?2 OFFSET ?3".to_string(),
                    vec![Box::new(like), Box::new(limit as i64), Box::new(offset as i64)],
                )
            }
        } else {
            (
                "SELECT id, created_at, final_text, inserted_text, transcription_mode, stt_provider, pipeline_mode, insert_success \
                 FROM transcript_samples ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2".to_string(),
                vec![Box::new(limit as i64), Box::new(offset as i64)],
            )
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let entries = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(TranscriptHistoryEntry {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    final_text: row.get(2)?,
                    inserted_text: row.get(3)?,
                    transcription_mode: row.get(4)?,
                    stt_provider: row.get(5)?,
                    pipeline_mode: row.get(6)?,
                    insert_success: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Delete a single transcript history entry by id.
    pub fn delete_entry(&self, id: i64) -> Result<(), String> {
        self.flush_writer();
        let conn = open_connection(&self.db_path)?;
        conn.execute("DELETE FROM transcript_samples WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Extract unique words from a transcript's final_text (for auto-learn dictionary).
    pub fn get_entry_words(&self, id: i64) -> Result<Vec<String>, String> {
        self.flush_writer();
        let conn = open_connection(&self.db_path)?;
        let text: Option<String> = conn
            .query_row(
                "SELECT final_text FROM transcript_samples WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        let words = text
            .unwrap_or_default()
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()).to_string())
            .filter(|w| !w.is_empty() && w.len() > 1)
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();

        Ok(words)
    }
}

pub fn current_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn generate_session_id() -> String {
    format!(
        "session-{}-{}",
        std::process::id(),
        current_timestamp_ms()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let unique = format!(
            "{}-{}-{}.sqlite3",
            label,
            std::process::id(),
            nanos
        );
        std::env::temp_dir().join(unique)
    }

    fn sample_with_id(session_id: &str, utterance_id: &str, created_at: i64) -> TranscriptSample {
        TranscriptSample {
            created_at,
            app_version: "0.6.0-test".to_string(),
            session_id: session_id.to_string(),
            utterance_id: utterance_id.to_string(),
            transcription_mode: "offline".to_string(),
            stt_provider: "parakeet-tdt".to_string(),
            active_industry_pack: "general".to_string(),
            active_profile_id: "general".to_string(),
            cleanup_enabled: true,
            post_processing_enabled: false,
            vad_silence_threshold_ms: 500,
            utterance_duration_ms: 2_500,
            preview_segment_count: 1,
            final_pass_used: false,
            final_pass_reason: "single-pass".to_string(),
            final_pass_latency_ms: None,
            language_family: "latin".to_string(),
            language_lock_confidence: "high".to_string(),
            mixed_script_detected: false,
            raw_segments_json: "[\"hello world\"]".to_string(),
            joined_text: Some("hello world".to_string()),
            normalized_text: Some("Hello world".to_string()),
            cleaned_text: Some("Hello world.".to_string()),
            post_processed_text: None,
            final_text: Some("Hello world.".to_string()),
            inserted_text: Some("Hello world. ".to_string()),
            insert_success: true,
            pipeline_mode: "dictation".to_string(),
        }
    }

    #[test]
    fn bootstrap_creates_schema_and_empty_status() {
        let db_path = temp_db_path("diagnostics-bootstrap");
        let store = TranscriptDiagnosticsStore::from_db_path(db_path.clone()).unwrap();

        let status = store.status(false).unwrap();
        assert!(!status.enabled);
        assert_eq!(status.sample_count, 0);
        assert_eq!(status.max_samples, TRANSCRIPT_DIAGNOSTICS_MAX_SAMPLES);
        assert_eq!(PathBuf::from(status.db_path), db_path);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn enabled_logging_writes_rows() {
        let db_path = temp_db_path("diagnostics-insert");
        let store = TranscriptDiagnosticsStore::from_db_path(db_path.clone()).unwrap();

        store.log_sample(sample_with_id("session-a", "utt-1", 1));
        store.flush_for_tests();

        let status = store.status(true).unwrap();
        assert_eq!(status.sample_count, 1);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn retention_keeps_latest_rows_only() {
        let db_path = temp_db_path("diagnostics-retention");
        let store = TranscriptDiagnosticsStore::from_db_path(db_path.clone()).unwrap();

        for index in 0..(TRANSCRIPT_DIAGNOSTICS_MAX_SAMPLES + 5) {
            store.log_sample(sample_with_id(
                "session-b",
                &format!("utt-{index}"),
                index as i64,
            ));
        }
        store.flush_for_tests();

        let status = store.status(true).unwrap();
        assert_eq!(status.sample_count, TRANSCRIPT_DIAGNOSTICS_MAX_SAMPLES);

        let conn = open_connection(&db_path).unwrap();
        let oldest_kept: String = conn
            .query_row(
                "SELECT utterance_id FROM transcript_samples ORDER BY created_at ASC, id ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(oldest_kept, "utt-5");

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn clear_removes_all_rows() {
        let db_path = temp_db_path("diagnostics-clear");
        let store = TranscriptDiagnosticsStore::from_db_path(db_path.clone()).unwrap();

        store.log_sample(sample_with_id("session-c", "utt-1", 1));
        store.flush_for_tests();

        let status = store.clear(true).unwrap();
        assert_eq!(status.sample_count, 0);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn bootstrap_migrates_new_telemetry_columns() {
        let db_path = temp_db_path("diagnostics-migration");
        let store = TranscriptDiagnosticsStore::from_db_path(db_path.clone()).unwrap();
        store.flush_for_tests();

        let conn = open_connection(&db_path).unwrap();
        let mut stmt = conn
            .prepare("PRAGMA table_info(transcript_samples)")
            .unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|row| row.ok())
            .collect();

        assert!(columns.contains(&"utterance_duration_ms".to_string()));
        assert!(columns.contains(&"preview_segment_count".to_string()));
        assert!(columns.contains(&"final_pass_used".to_string()));
        assert!(columns.contains(&"final_pass_reason".to_string()));
        assert!(columns.contains(&"final_pass_latency_ms".to_string()));
        assert!(columns.contains(&"language_family".to_string()));
        assert!(columns.contains(&"language_lock_confidence".to_string()));
        assert!(columns.contains(&"mixed_script_detected".to_string()));

        let _ = fs::remove_file(db_path);
    }
}
