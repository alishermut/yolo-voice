use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use super::TextAction;

const CLEAN_UP_ID: &str = "clean_up";
const SHORTEN_ID: &str = "shorten";
const PROFESSIONAL_ID: &str = "professional";
const FRIENDLY_ID: &str = "friendly";
const FREEFORM_COMMAND_ID: &str = "freeform_command";
const MIGRATED_COMMAND_ID: &str = "migrated_command_prompt";

pub fn default_text_action_id() -> &'static str {
    CLEAN_UP_ID
}

pub fn freeform_command_action_id() -> &'static str {
    FREEFORM_COMMAND_ID
}

pub fn get_text_actions_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    let text_actions_dir = dir.join("text_actions");
    std::fs::create_dir_all(&text_actions_dir).map_err(|e| e.to_string())?;
    Ok(text_actions_dir)
}

pub fn builtin_text_actions() -> Vec<TextAction> {
    vec![
        TextAction {
            id: CLEAN_UP_ID.to_string(),
            name: "Clean up".to_string(),
            prompt: "Clean up the source text for readability. Fix grammar, punctuation, capitalization, and obvious dictation artifacts while preserving the original meaning and voice. Return only the final text."
                .to_string(),
            builtin: true,
        },
        TextAction {
            id: SHORTEN_ID.to_string(),
            name: "Shorten".to_string(),
            prompt: "Rewrite the source text so it is shorter and clearer while preserving the key meaning. Return only the final text."
                .to_string(),
            builtin: true,
        },
        TextAction {
            id: PROFESSIONAL_ID.to_string(),
            name: "Professional".to_string(),
            prompt: "Rewrite the source text in a professional, polished tone while preserving the original meaning. Return only the final text."
                .to_string(),
            builtin: true,
        },
        TextAction {
            id: FRIENDLY_ID.to_string(),
            name: "Friendly".to_string(),
            prompt: "Rewrite the source text in a warm, friendly tone while preserving the original meaning. Return only the final text."
                .to_string(),
            builtin: true,
        },
        TextAction {
            id: FREEFORM_COMMAND_ID.to_string(),
            name: "Freeform command".to_string(),
            prompt: crate::features::settings::legacy_default_command_system_prompt().to_string(),
            builtin: true,
        },
    ]
}

pub fn ensure_text_actions_ready(
    app_handle: &AppHandle,
    config: &mut crate::features::settings::AppConfig,
) -> Result<bool, String> {
    let dir = get_text_actions_dir(app_handle)?;
    let mut changed = ensure_builtin_text_actions(&dir)?;
    let actions = list_text_actions(&dir)?;

    let current_prompt = normalize_prompt(&config.command_system_prompt);
    let default_prompt =
        normalize_prompt(crate::features::settings::legacy_default_command_system_prompt());
    let command_prompt_is_custom = !current_prompt.is_empty() && current_prompt != default_prompt;

    if command_prompt_is_custom {
        let migrated_id = find_action_with_prompt(&actions, &current_prompt)
            .unwrap_or_else(|| MIGRATED_COMMAND_ID.to_string());

        if !actions.iter().any(|action| action.id == migrated_id) {
            save_text_action(
                &dir,
                &TextAction {
                    id: migrated_id.clone(),
                    name: "Migrated command".to_string(),
                    prompt: config.command_system_prompt.clone(),
                    builtin: false,
                },
            )?;
            changed = true;
        }

        if config.default_text_action_id != migrated_id {
            config.default_text_action_id = migrated_id;
            changed = true;
        }
    } else if config.default_text_action_id.trim().is_empty()
        || !actions
            .iter()
            .any(|action| action.id == config.default_text_action_id)
    {
        config.default_text_action_id = default_text_action_id().to_string();
        changed = true;
    }

    Ok(changed)
}

pub fn list_text_actions(text_actions_dir: &Path) -> Result<Vec<TextAction>, String> {
    let mut actions = Vec::new();
    let entries = std::fs::read_dir(text_actions_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|ext| ext == "json").unwrap_or(false) {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<TextAction>(&contents) {
                    Ok(action) => actions.push(action),
                    Err(e) => {
                        eprintln!("[text-actions] Failed to parse {}: {}", path.display(), e)
                    }
                },
                Err(e) => eprintln!("[text-actions] Failed to read {}: {}", path.display(), e),
            }
        }
    }

    actions.sort_by(|a, b| match (a.builtin, b.builtin) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(actions)
}

pub fn get_text_action(text_actions_dir: &Path, id: &str) -> Result<TextAction, String> {
    let sanitized = sanitize_id(id)?;
    let path = text_actions_dir.join(format!("{}.json", sanitized));
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read text action '{}': {}", sanitized, e))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse text action '{}': {}", sanitized, e))
}

pub fn save_text_action(text_actions_dir: &Path, action: &TextAction) -> Result<(), String> {
    sanitize_id(&action.id)?;
    std::fs::create_dir_all(text_actions_dir).map_err(|e| e.to_string())?;

    let path = text_actions_dir.join(format!("{}.json", action.id));
    let json = serde_json::to_string_pretty(action)
        .map_err(|e| format!("Failed to serialize text action: {}", e))?;
    crate::infra::fs_util::write_json_atomic(&path, &json)
        .map_err(|e| format!("Failed to write text action: {}", e))?;
    Ok(())
}

pub fn delete_text_action(text_actions_dir: &Path, id: &str) -> Result<(), String> {
    let action = get_text_action(text_actions_dir, id)?;
    if action.builtin {
        return Err("Built-in text actions cannot be deleted.".to_string());
    }

    let path = text_actions_dir.join(format!("{}.json", sanitize_id(id)?));
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete text action: {}", e))?;
    }
    Ok(())
}

pub fn reset_text_action_to_default(text_actions_dir: &Path, id: &str) -> Result<(), String> {
    let builtin = builtin_text_actions()
        .into_iter()
        .find(|action| action.id == id)
        .ok_or_else(|| format!("No built-in text action with id '{}'", id))?;

    save_text_action(text_actions_dir, &builtin)
}

fn ensure_builtin_text_actions(text_actions_dir: &Path) -> Result<bool, String> {
    std::fs::create_dir_all(text_actions_dir).map_err(|e| e.to_string())?;
    let mut changed = false;

    for builtin in builtin_text_actions() {
        let path = text_actions_dir.join(format!("{}.json", builtin.id));
        if !path.exists() {
            save_text_action(text_actions_dir, &builtin)?;
            changed = true;
        }
    }

    Ok(changed)
}

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

fn normalize_prompt(prompt: &str) -> String {
    prompt
        .replace("\r\n", "\n")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn find_action_with_prompt(actions: &[TextAction], prompt: &str) -> Option<String> {
    actions
        .iter()
        .find(|action| !action.builtin && normalize_prompt(&action.prompt) == prompt)
        .map(|action| action.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::settings::AppConfig;

    fn temp_actions_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "yolo_voice_text_actions_test_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ensure_builtin_actions_seeds_empty_directory() {
        let dir = temp_actions_dir("seed");
        let changed = ensure_builtin_text_actions(&dir).expect("should seed built-ins");
        let actions = list_text_actions(&dir).expect("should list actions");

        assert!(changed);
        assert!(actions.iter().any(|action| action.id == CLEAN_UP_ID));
        assert!(actions
            .iter()
            .any(|action| action.id == FREEFORM_COMMAND_ID));
    }

    #[test]
    fn custom_command_prompt_migrates_to_custom_text_action() {
        let dir = temp_actions_dir("migrate_custom");
        ensure_builtin_text_actions(&dir).expect("should seed built-ins");
        let mut config = AppConfig::default();
        config.command_system_prompt =
            "Rewrite the source text into a concise Slack update.".to_string();
        config.default_text_action_id.clear();

        let current_prompt = normalize_prompt(&config.command_system_prompt);
        let existing = find_action_with_prompt(&list_text_actions(&dir).unwrap(), &current_prompt);
        assert!(existing.is_none());

        let migrated = TextAction {
            id: MIGRATED_COMMAND_ID.to_string(),
            name: "Migrated command".to_string(),
            prompt: config.command_system_prompt.clone(),
            builtin: false,
        };
        save_text_action(&dir, &migrated).unwrap();
        let actions = list_text_actions(&dir).unwrap();

        assert!(actions
            .iter()
            .any(|action| action.id == MIGRATED_COMMAND_ID));
        assert!(actions
            .iter()
            .any(|action| action.prompt == "Rewrite the source text into a concise Slack update."));
    }

    #[test]
    fn delete_text_action_blocks_builtins() {
        let dir = temp_actions_dir("delete_builtin");
        ensure_builtin_text_actions(&dir).expect("should seed built-ins");

        let err = delete_text_action(&dir, CLEAN_UP_ID).expect_err("built-ins cannot be deleted");
        assert!(err.contains("Built-in"));
    }

    #[test]
    fn reset_text_action_restores_builtin_prompt() {
        let dir = temp_actions_dir("reset");
        ensure_builtin_text_actions(&dir).expect("should seed built-ins");

        let edited = TextAction {
            id: CLEAN_UP_ID.to_string(),
            name: "Clean up".to_string(),
            prompt: "Changed prompt".to_string(),
            builtin: true,
        };
        save_text_action(&dir, &edited).unwrap();
        reset_text_action_to_default(&dir, CLEAN_UP_ID).unwrap();

        let action = get_text_action(&dir, CLEAN_UP_ID).unwrap();
        assert_ne!(action.prompt, "Changed prompt");
    }
}
