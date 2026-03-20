use std::collections::HashMap;
use std::sync::Mutex;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Cached compiled regexes for replacement rules, keyed by the "find" pattern.
static REGEX_CACHE: std::sync::LazyLock<Mutex<HashMap<String, Regex>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementRule {
    pub find: String,
    pub replace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalDictionary {
    #[serde(default)]
    pub vocabulary: Vec<String>,
    #[serde(default)]
    pub replacements: Vec<ReplacementRule>,
}

pub struct GlobalDictionaryState(pub Mutex<GlobalDictionary>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryPack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub vocabulary: Vec<String>,
    #[serde(default)]
    pub replacements: Vec<ReplacementRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryPackInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vocabulary_count: usize,
    pub replacement_count: usize,
}

/// Apply replacement rules to text (case-insensitive, word-boundary-aware).
/// Uses a cached regex pool to avoid recompiling on every call.
pub fn apply_replacements(text: &str, rules: &[ReplacementRule]) -> String {
    let mut result = text.to_string();
    let mut cache = REGEX_CACHE.lock().unwrap();
    for rule in rules {
        if rule.find.is_empty() {
            continue;
        }
        let re = cache.entry(rule.find.clone()).or_insert_with(|| {
            let escaped = regex::escape(&rule.find);
            let pattern = format!(r"(?i)\b{}\b", escaped);
            Regex::new(&pattern).unwrap_or_else(|_| Regex::new("(?:)").unwrap())
        });
        result = re.replace_all(&result, rule.replace.as_str()).to_string();
    }
    result
}

/// Clear the regex cache (called when replacement rules change).
pub fn invalidate_regex_cache() {
    if let Ok(mut cache) = REGEX_CACHE.lock() {
        cache.clear();
    }
}

fn dictionary_path(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    Ok(dir.join("global_dictionary.json"))
}

pub fn load_global_dictionary(app_handle: &AppHandle) -> GlobalDictionary {
    let path = match dictionary_path(app_handle) {
        Ok(p) => p,
        Err(_) => return GlobalDictionary::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => GlobalDictionary::default(),
    }
}

pub fn save_global_dictionary(
    app_handle: &AppHandle,
    dict: &GlobalDictionary,
) -> Result<(), String> {
    let path = dictionary_path(app_handle)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(dict).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Merge an industry pack into an existing dictionary (deduplicates).
pub fn merge_pack_into_dictionary(dict: &mut GlobalDictionary, pack: &IndustryPack) {
    for word in &pack.vocabulary {
        if !dict
            .vocabulary
            .iter()
            .any(|w| w.eq_ignore_ascii_case(word))
        {
            dict.vocabulary.push(word.clone());
        }
    }
    for rule in &pack.replacements {
        if !dict
            .replacements
            .iter()
            .any(|r| r.find.eq_ignore_ascii_case(&rule.find))
        {
            dict.replacements.push(rule.clone());
        }
    }
}

/// Auto-apply all industry packs on first install (when dictionary is empty).
pub fn auto_apply_all_packs(app_handle: &AppHandle, dict: &mut GlobalDictionary) {
    if !dict.vocabulary.is_empty() || !dict.replacements.is_empty() {
        return;
    }

    eprintln!("[app] First install detected: auto-applying all industry packs...");

    let packs = match list_industry_packs(app_handle) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[app] Failed to list industry packs: {}", e);
            return;
        }
    };

    for pack_info in &packs {
        match load_industry_pack(app_handle, &pack_info.id) {
            Ok(pack) => {
                eprintln!(
                    "[app] Applying pack '{}': {} vocab, {} replacements",
                    pack_info.name, pack_info.vocabulary_count, pack_info.replacement_count
                );
                merge_pack_into_dictionary(dict, &pack);
            }
            Err(e) => {
                eprintln!("[app] Failed to load pack '{}': {}", pack_info.id, e);
            }
        }
    }

    if let Err(e) = save_global_dictionary(app_handle, dict) {
        eprintln!("[app] Failed to save auto-applied dictionary: {}", e);
    } else {
        eprintln!(
            "[app] Auto-applied {} packs: {} vocab terms, {} replacement rules",
            packs.len(),
            dict.vocabulary.len(),
            dict.replacements.len()
        );
    }
}

/// List available industry packs from the sidecar/industry_packs directory.
pub fn list_industry_packs(app_handle: &AppHandle) -> Result<Vec<IndustryPackInfo>, String> {
    let packs_dir = get_industry_packs_dir(app_handle)?;
    let mut packs = Vec::new();

    if !packs_dir.exists() {
        return Ok(packs);
    }

    for entry in std::fs::read_dir(&packs_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(pack) = serde_json::from_str::<IndustryPack>(&contents) {
                packs.push(IndustryPackInfo {
                    id: pack.id,
                    name: pack.name,
                    description: pack.description,
                    vocabulary_count: pack.vocabulary.len(),
                    replacement_count: pack.replacements.len(),
                });
            }
        }
    }

    packs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packs)
}

/// Load a specific industry pack by id.
pub fn load_industry_pack(app_handle: &AppHandle, id: &str) -> Result<IndustryPack, String> {
    let packs_dir = get_industry_packs_dir(app_handle)?;
    let path = packs_dir.join(format!("{}.json", id));
    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Pack '{}' not found: {}", id, e))?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

fn get_industry_packs_dir(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
    if cfg!(debug_assertions) {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let dir = if cwd.join("sidecar/industry_packs").exists() {
            cwd.join("sidecar/industry_packs")
        } else if cwd.join("../sidecar/industry_packs").exists() {
            cwd.join("../sidecar/industry_packs")
        } else {
            return Err("Industry packs directory not found".to_string());
        };
        Ok(dir)
    } else {
        let resource_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e| e.to_string())?;
        let nested_dir = resource_dir.join("sidecar").join("industry_packs");
        if nested_dir.exists() {
            Ok(nested_dir)
        } else {
            Ok(resource_dir.join("industry_packs"))
        }
    }
}
