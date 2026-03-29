use std::path::Path;

use super::Profile;
use tauri::{AppHandle, Manager};

/// Get the profiles directory path.
pub fn get_profiles_dir(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    let profiles_dir = dir.join("profiles");
    std::fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    Ok(profiles_dir)
}

/// Ensure profiles directory exists and seed defaults if empty.
pub fn ensure_profiles_dir(profiles_dir: &Path, app_handle: &AppHandle) -> Result<(), String> {
    std::fs::create_dir_all(profiles_dir).map_err(|e| e.to_string())?;

    // Check if any profiles exist
    let has_profiles = std::fs::read_dir(profiles_dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !has_profiles {
        seed_default_profiles(profiles_dir, app_handle)?;
    }

    Ok(())
}

/// Seed default profiles from bundled resource file.
fn seed_default_profiles(profiles_dir: &Path, app_handle: &AppHandle) -> Result<(), String> {
    // Try to find default_profiles.json in resources
    let defaults_path = find_default_profiles(app_handle)?;

    let contents = std::fs::read_to_string(&defaults_path)
        .map_err(|e| format!("Failed to read default profiles: {}", e))?;

    let defaults: Vec<Profile> = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse default profiles: {}", e))?;

    for profile in &defaults {
        let path = profiles_dir.join(format!("{}.json", profile.id));
        let json = serde_json::to_string_pretty(profile)
            .map_err(|e| format!("Failed to serialize profile: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write profile {}: {}", profile.id, e))?;
    }

    Ok(())
}

fn find_default_profiles(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    // Dev: check project root
    if cfg!(debug_assertions) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let candidates = [
            cwd.join("resources/default_profiles.json"),
            cwd.join("../sidecar/default_profiles.json"), // fallback during transition
        ];
        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }
    }

    // Production: check Tauri resource directory
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?;

    let candidates = [
        resource_dir.join("default_profiles.json"),
        resource_dir.join("resources/default_profiles.json"),
        // Legacy sidecar location
        resource_dir.join("sidecar/default_profiles.json"),
    ];
    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    Err("Default profiles file not found".to_string())
}

/// List all profiles from the profiles directory.
pub fn list_profiles(profiles_dir: &Path) -> Result<Vec<Profile>, String> {
    let mut profiles = Vec::new();

    let entries = std::fs::read_dir(profiles_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|ext| ext == "json").unwrap_or(false) {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<Profile>(&contents) {
                    Ok(profile) => profiles.push(profile),
                    Err(e) => eprintln!("[profiles] Failed to parse {}: {}", path.display(), e),
                },
                Err(e) => eprintln!("[profiles] Failed to read {}: {}", path.display(), e),
            }
        }
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

/// Sanitize an ID for use in file paths — reject path traversal attempts.
fn sanitize_id(id: &str) -> Result<&str, String> {
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || id.starts_with('.')
    {
        return Err(format!("Invalid ID: {}", id));
    }
    Ok(id)
}

/// Save a profile to disk.
pub fn save_profile(profiles_dir: &Path, profile: &Profile) -> Result<(), String> {
    sanitize_id(&profile.id)?;
    std::fs::create_dir_all(profiles_dir).map_err(|e| e.to_string())?;

    let path = profiles_dir.join(format!("{}.json", profile.id));
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write profile: {}", e))?;
    // Ensure the write is flushed to disk (prevents data loss on crash/close)
    if let Ok(file) = std::fs::File::open(&path) {
        let _ = file.sync_all();
    }

    Ok(())
}

/// Reset a built-in profile to its default version from bundled resources.
pub fn reset_profile_to_default(
    profiles_dir: &Path,
    id: &str,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let defaults_path = find_default_profiles(app_handle)?;
    let contents = std::fs::read_to_string(&defaults_path)
        .map_err(|e| format!("Failed to read default profiles: {}", e))?;
    let defaults: Vec<Profile> = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse default profiles: {}", e))?;

    let default_profile = defaults
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("No default profile with id '{}'", id))?;

    save_profile(profiles_dir, default_profile)?;
    Ok(())
}

/// Delete a profile from disk.
pub fn delete_profile(profiles_dir: &Path, id: &str) -> Result<(), String> {
    sanitize_id(id)?;
    let path = profiles_dir.join(format!("{}.json", id));
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete profile: {}", e))?;
    }
    Ok(())
}
