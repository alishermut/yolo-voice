use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic suffix so two writers in this process can never pick the same temp path.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `contents` to `path` via temp file + fsync + rename so a crash mid-write
/// cannot leave a truncated JSON file.
///
/// The rename is a single step on purpose: `fs::rename` maps to `MoveFileEx` with
/// `MOVEFILE_REPLACE_EXISTING` on Windows and to `rename(2)` elsewhere, both of which
/// replace the destination atomically. Moving the live file aside first would leave a
/// window where `path` does not exist at all — a crash there reads back as "no config"
/// and silently resets the user's settings.
///
/// Note: the temp file is fsynced, but the parent directory is not. On Windows there is
/// no portable way to fsync a directory, so after a host crash the rename itself may not
/// have reached disk (the file is then either fully old or fully new — never torn).
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let file_name = path
        .file_name()
        .ok_or_else(|| format!("Path has no file name: {}", path.display()))?
        .to_string_lossy();
    // pid + counter keeps concurrent saves in this process on distinct temp paths;
    // without the counter two threads share one temp and can swap each other's bytes.
    let tmp_path = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    {
        let mut file = File::create(&tmp_path)
            .map_err(|e| format!("Failed to create temp file {}: {}", tmp_path.display(), e))?;
        if let Err(e) = file.write_all(contents) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Failed to write temp file {}: {}",
                tmp_path.display(),
                e
            ));
        }
        if let Err(e) = file.sync_all() {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Failed to fsync temp file {}: {}",
                tmp_path.display(),
                e
            ));
        }
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!(
            "Failed to replace {} with temp file: {}",
            path.display(),
            e
        ));
    }

    Ok(())
}

/// Serialize-friendly helper for JSON documents.
pub fn write_json_atomic(path: &Path, json: &str) -> Result<(), String> {
    write_atomic(path, json.as_bytes())
}

/// Copy a corrupt file aside before replacing it with defaults.
/// Returns the backup path when the copy succeeds.
pub fn backup_corrupt_file(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Path has no parent: {}", path.display()))?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = parent.join(format!("{stem}.corrupt-{ts}.json"));
    fs::copy(path, &backup)
        .map_err(|e| format!("Failed to backup corrupt file {}: {}", path.display(), e))?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn write_json_atomic_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "yolo-voice-atomic-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        write_json_atomic(&path, r#"{"hotkey":"A"}"#).unwrap();
        write_json_atomic(&path, r#"{"hotkey":"B"}"#).unwrap();

        let mut contents = String::new();
        File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert_eq!(contents, r#"{"hotkey":"B"}"#);

        let _ = fs::remove_dir_all(&dir);
    }

    /// The target must never be observable as missing: an earlier version moved the live
    /// file aside before renaming the temp in, so a crash in between wiped the config.
    #[test]
    fn write_atomic_leaves_no_temp_or_sidecar_files() {
        let dir = std::env::temp_dir().join(format!(
            "yolo-voice-atomic-clean-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        write_json_atomic(&path, r#"{"a":1}"#).unwrap();
        write_json_atomic(&path, r#"{"a":2}"#).unwrap();

        let entries: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, vec!["config.json".to_string()], "stray files: {entries:?}");

        let _ = fs::remove_dir_all(&dir);
    }

    /// Concurrent writers previously shared one pid-only temp path and could rename each
    /// other's bytes into place. Whoever lands last must win with *their own* content.
    #[test]
    fn concurrent_writes_do_not_interleave_contents() {
        let dir = std::env::temp_dir().join(format!(
            "yolo-voice-atomic-race-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");

        let payloads: Vec<String> = (0..8)
            .map(|i| format!(r#"{{"writer":{i},"pad":"{}"}}"#, "x".repeat(4096)))
            .collect();

        let handles: Vec<_> = payloads
            .iter()
            .cloned()
            .map(|body| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for _ in 0..20 {
                        write_json_atomic(&path, &body).unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let mut contents = String::new();
        File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(
            payloads.contains(&contents),
            "file holds content no single writer wrote"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
