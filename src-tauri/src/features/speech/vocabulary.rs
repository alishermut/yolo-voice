use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

/// Cached compiled regexes for replacement rules, keyed by the "find" pattern.
static REGEX_CACHE: std::sync::LazyLock<Mutex<HashMap<String, Regex>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplacementRule {
    pub find: String,
    pub replace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserDictionary {
    #[serde(default = "default_user_dictionary_version")]
    pub version: u32,
    #[serde(default)]
    pub user_vocabulary: Vec<String>,
    #[serde(default)]
    pub user_normalization_rules: Vec<ReplacementRule>,
}

impl Default for UserDictionary {
    fn default() -> Self {
        Self {
            version: default_user_dictionary_version(),
            user_vocabulary: Vec::new(),
            user_normalization_rules: Vec::new(),
        }
    }
}

fn default_user_dictionary_version() -> u32 {
    2
}

pub struct UserDictionaryState(pub Mutex<UserDictionary>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustryPackInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub vocabulary_count: usize,
    pub replacement_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeDictionary {
    pub vocabulary: Vec<String>,
    pub normalization_rules: Vec<ReplacementRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserDictionaryMigration {
    None,
    LegacyReset { backup_path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadUserDictionaryResult {
    pub dictionary: UserDictionary,
    pub migration: UserDictionaryMigration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LegacyGlobalDictionary {
    #[serde(default)]
    vocabulary: Vec<String>,
    #[serde(default)]
    replacements: Vec<ReplacementRule>,
}

enum DictionaryFileFormat {
    UserV2,
    LegacyV1,
    Unknown,
}

/// Apply normalization rules to text (case-insensitive, word-boundary-aware).
/// Uses a cached regex pool to avoid recompiling on every call.
pub fn apply_normalization_rules(text: &str, rules: &[ReplacementRule]) -> String {
    let mut result = text.to_string();
    let mut cache = REGEX_CACHE.lock().unwrap();

    for rule in rules {
        if rule.find.trim().is_empty() {
            continue;
        }

        let cache_key = rule.find.to_ascii_lowercase();
        let re = cache.entry(cache_key).or_insert_with(|| {
            let escaped = regex::escape(rule.find.trim());
            let pattern = format!(r"(?i)\b{}\b", escaped);
            Regex::new(&pattern).unwrap_or_else(|_| Regex::new("(?:)").unwrap())
        });

        result = re.replace_all(&result, rule.replace.as_str()).to_string();
    }

    result
}

/// Clear the regex cache (called when normalization rules change).
pub fn invalidate_regex_cache() {
    if let Ok(mut cache) = REGEX_CACHE.lock() {
        cache.clear();
    }
}

pub fn load_user_dictionary(app_handle: &AppHandle) -> LoadUserDictionaryResult {
    let path = match dictionary_path(app_handle) {
        Ok(p) => p,
        Err(_) => {
            return LoadUserDictionaryResult {
                dictionary: UserDictionary::default(),
                migration: UserDictionaryMigration::None,
            };
        }
    };

    load_or_migrate_user_dictionary_from_path(&path).unwrap_or(LoadUserDictionaryResult {
        dictionary: UserDictionary::default(),
        migration: UserDictionaryMigration::None,
    })
}

pub fn save_user_dictionary(
    app_handle: &AppHandle,
    dict: &UserDictionary,
) -> Result<(), String> {
    let path = dictionary_path(app_handle)?;
    save_user_dictionary_to_path(&path, dict)
}

pub fn runtime_dictionary_from_user_dictionary(user_dict: &UserDictionary) -> RuntimeDictionary {
    let generated_user_rules = generate_normalization_rules_for_terms(&user_dict.user_vocabulary);

    RuntimeDictionary {
        vocabulary: dedupe_vocabulary(user_dict.user_vocabulary.iter().cloned()),
        normalization_rules: merge_normalization_rule_layers(&[
            &generated_user_rules,
            &user_dict.user_normalization_rules,
        ]),
    }
}

pub fn resolve_runtime_dictionary(
    user_dict: &UserDictionary,
    active_pack: Option<&IndustryPack>,
) -> RuntimeDictionary {
    let pack_vocabulary = active_pack
        .map(|pack| pack.vocabulary.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let pack_rules = active_pack
        .map(|pack| pack.replacements.clone())
        .unwrap_or_default();
    let generated_user_rules = generate_normalization_rules_for_terms(&user_dict.user_vocabulary);

    RuntimeDictionary {
        vocabulary: dedupe_vocabulary(
            user_dict
                .user_vocabulary
                .iter()
                .cloned()
                .chain(pack_vocabulary),
        ),
        normalization_rules: merge_normalization_rule_layers(&[
            &pack_rules,
            &generated_user_rules,
            &user_dict.user_normalization_rules,
        ]),
    }
}

pub fn resolve_runtime_dictionary_for_pack(
    app_handle: &AppHandle,
    user_dict: &UserDictionary,
    active_pack_id: &str,
) -> Result<RuntimeDictionary, String> {
    if active_pack_id.trim().is_empty() || active_pack_id == "general" {
        return Ok(resolve_runtime_dictionary(user_dict, None));
    }

    let pack = load_industry_pack(app_handle, active_pack_id)?;
    Ok(resolve_runtime_dictionary(user_dict, Some(&pack)))
}

pub fn load_or_migrate_user_dictionary_from_path(
    path: &Path,
) -> Result<LoadUserDictionaryResult, String> {
    let Some(contents) = read_dictionary_contents(path)? else {
        return Ok(LoadUserDictionaryResult {
            dictionary: UserDictionary::default(),
            migration: UserDictionaryMigration::None,
        });
    };

    let value: Value = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(_) => {
            return Ok(LoadUserDictionaryResult {
                dictionary: UserDictionary::default(),
                migration: UserDictionaryMigration::None,
            });
        }
    };

    match detect_dictionary_file_format(&value) {
        DictionaryFileFormat::UserV2 => {
            let dict = serde_json::from_value::<UserDictionary>(value).unwrap_or_default();
            Ok(LoadUserDictionaryResult {
                dictionary: sanitize_user_dictionary(dict),
                migration: UserDictionaryMigration::None,
            })
        }
        DictionaryFileFormat::LegacyV1 => {
            let backup_path = next_backup_path(path);
            if let Some(parent) = backup_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            std::fs::write(&backup_path, contents).map_err(|e| e.to_string())?;

            let empty_dict = UserDictionary::default();
            save_user_dictionary_to_path(path, &empty_dict)?;

            Ok(LoadUserDictionaryResult {
                dictionary: empty_dict,
                migration: UserDictionaryMigration::LegacyReset { backup_path },
            })
        }
        DictionaryFileFormat::Unknown => Ok(LoadUserDictionaryResult {
            dictionary: UserDictionary::default(),
            migration: UserDictionaryMigration::None,
        }),
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

fn dictionary_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    Ok(dir.join("global_dictionary.json"))
}

fn save_user_dictionary_to_path(path: &Path, dict: &UserDictionary) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let sanitized = sanitize_user_dictionary(dict.clone());
    let json = serde_json::to_string_pretty(&sanitized).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn sanitize_user_dictionary(mut dict: UserDictionary) -> UserDictionary {
    dict.version = default_user_dictionary_version();
    dict.user_vocabulary = dedupe_vocabulary(dict.user_vocabulary);
    dict.user_normalization_rules = sort_rules_for_application(dict.user_normalization_rules);
    dict
}

fn detect_dictionary_file_format(value: &Value) -> DictionaryFileFormat {
    let Some(obj) = value.as_object() else {
        return DictionaryFileFormat::Unknown;
    };

    if obj
        .get("version")
        .and_then(Value::as_u64)
        .is_some_and(|version| version >= 2)
        || obj.contains_key("user_vocabulary")
        || obj.contains_key("user_normalization_rules")
    {
        return DictionaryFileFormat::UserV2;
    }

    if obj.contains_key("vocabulary") || obj.contains_key("replacements") {
        return DictionaryFileFormat::LegacyV1;
    }

    DictionaryFileFormat::Unknown
}

fn read_dictionary_contents(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn next_backup_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("global_dictionary");
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("json");

    let primary = parent.join(format!("{stem}.v1.backup.{extension}"));
    if !primary.exists() {
        return primary;
    }

    for index in 2..1000 {
        let candidate = parent.join(format!("{stem}.v1.backup.{index}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{stem}.v1.backup.overflow.{extension}"))
}

fn dedupe_vocabulary(words: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for word in words {
        let trimmed = word.trim();
        if trimmed.is_empty() {
            continue;
        }

        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(trimmed.to_string());
        }
    }

    deduped
}

fn merge_normalization_rule_layers(layers: &[&[ReplacementRule]]) -> Vec<ReplacementRule> {
    let mut merged = BTreeMap::<String, ReplacementRule>::new();

    for layer in layers {
        for rule in *layer {
            if let Some(rule) = sanitize_rule(rule) {
                merged.insert(rule.find.to_ascii_lowercase(), rule);
            }
        }
    }

    sort_rules_for_application(merged.into_values().collect())
}

fn generate_normalization_rules_for_terms(terms: &[String]) -> Vec<ReplacementRule> {
    let mut generated = Vec::new();

    for term in terms {
        let canonical = term.trim();
        if canonical.is_empty() {
            continue;
        }

        for alias in generate_alias_variants(canonical) {
            generated.push(ReplacementRule {
                find: alias,
                replace: canonical.to_string(),
            });
        }
    }

    sort_rules_for_application(generated)
}

fn generate_alias_variants(term: &str) -> Vec<String> {
    let canonical = term.trim();
    let mut aliases = BTreeMap::<String, String>::new();

    if canonical.is_empty() {
        return Vec::new();
    }

    aliases.insert(canonical.to_ascii_lowercase(), canonical.to_string());

    let tokens = split_term_tokens(canonical);
    if tokens.len() >= 2 {
        let spaced = tokens.join(" ").to_ascii_lowercase();
        aliases.insert(spaced.to_ascii_lowercase(), spaced);
    }

    let compact = term_to_compact_alnum(canonical);
    if !compact.is_empty() && !eq_ignore_ascii_case_trimmed(&compact, canonical) {
        aliases.insert(compact.to_ascii_lowercase(), compact);
    }

    if is_all_caps_ascii_word(canonical) {
        let spelled = canonical
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .map(|ch| ch.to_ascii_lowercase().to_string())
            .collect::<Vec<_>>()
            .join(" ");

        if !spelled.is_empty() {
            aliases.insert(spelled.to_ascii_lowercase(), spelled);
        }
    }

    aliases.into_values().collect()
}

fn split_term_tokens(term: &str) -> Vec<String> {
    let mut tokens = Vec::new();

    for piece in term.split(|ch: char| !ch.is_alphanumeric()) {
        if piece.is_empty() {
            continue;
        }

        tokens.extend(split_camel_piece(piece));
    }

    tokens
}

fn split_camel_piece(piece: &str) -> Vec<String> {
    let mut boundaries = vec![0usize];
    let indexed_chars: Vec<(usize, char)> = piece.char_indices().collect();

    for index in 1..indexed_chars.len() {
        let (_, previous) = indexed_chars[index - 1];
        let (offset, current) = indexed_chars[index];
        let next = indexed_chars.get(index + 1).map(|(_, ch)| *ch);

        let camel_boundary = previous.is_lowercase() && current.is_uppercase();
        let acronym_boundary = previous.is_uppercase()
            && current.is_uppercase()
            && next.is_some_and(|next_char| next_char.is_lowercase());
        let digit_boundary =
            (previous.is_ascii_digit() && current.is_ascii_alphabetic())
                || (previous.is_ascii_alphabetic() && current.is_ascii_digit());

        if camel_boundary || acronym_boundary || digit_boundary {
            boundaries.push(offset);
        }
    }

    boundaries.push(piece.len());

    boundaries
        .windows(2)
        .filter_map(|window| {
            let token = piece[window[0]..window[1]].trim();
            if token.is_empty() {
                None
            } else {
                Some(token.to_string())
            }
        })
        .collect()
}

fn term_to_compact_alnum(term: &str) -> String {
    term.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_all_caps_ascii_word(term: &str) -> bool {
    let mut has_letter = false;

    for ch in term.chars().filter(|ch| ch.is_ascii_alphanumeric()) {
        if ch.is_ascii_alphabetic() {
            has_letter = true;
            if !ch.is_ascii_uppercase() {
                return false;
            }
        }
    }

    has_letter
}

fn eq_ignore_ascii_case_trimmed(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn sort_rules_for_application(rules: Vec<ReplacementRule>) -> Vec<ReplacementRule> {
    let mut grouped = BTreeMap::<String, Vec<ReplacementRule>>::new();

    for rule in rules {
        if let Some(rule) = sanitize_rule(&rule) {
            grouped
                .entry(rule.replace.to_ascii_lowercase())
                .or_default()
                .push(rule);
        }
    }

    let mut groups: Vec<Vec<ReplacementRule>> = grouped.into_values().collect();
    for group in &mut groups {
        group.sort_by(|a, b| {
            b.find
                .len()
                .cmp(&a.find.len())
                .then_with(|| a.find.to_ascii_lowercase().cmp(&b.find.to_ascii_lowercase()))
        });
    }

    groups.sort_by(|a, b| {
        let a_longest = a.first().map(|rule| rule.find.len()).unwrap_or_default();
        let b_longest = b.first().map(|rule| rule.find.len()).unwrap_or_default();
        b_longest.cmp(&a_longest).then_with(|| {
            let a_key = a
                .first()
                .map(|rule| rule.replace.to_ascii_lowercase())
                .unwrap_or_default();
            let b_key = b
                .first()
                .map(|rule| rule.replace.to_ascii_lowercase())
                .unwrap_or_default();
            a_key.cmp(&b_key)
        })
    });

    groups.into_iter().flatten().collect()
}

fn sanitize_rule(rule: &ReplacementRule) -> Option<ReplacementRule> {
    let find = rule.find.trim();
    let replace = rule.replace.trim();

    if find.is_empty() || replace.is_empty() {
        return None;
    }

    Some(ReplacementRule {
        find: find.to_string(),
        replace: replace.to_string(),
    })
}

fn get_industry_packs_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let candidates = [
            cwd.join("resources/industry_packs"),
            cwd.join("../resources/industry_packs"),
            cwd.join("sidecar/industry_packs"),
            cwd.join("../sidecar/industry_packs"),
        ];
        for dir in &candidates {
            if dir.exists() {
                return Ok(dir.clone());
            }
        }
        Err("Industry packs directory not found".to_string())
    } else {
        let resource_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e| e.to_string())?;
        let candidates = [
            resource_dir.join("industry_packs"),
            resource_dir.join("resources").join("industry_packs"),
            resource_dir.join("sidecar").join("industry_packs"),
        ];
        for dir in &candidates {
            if dir.exists() {
                return Ok(dir.clone());
            }
        }
        Ok(resource_dir.join("industry_packs"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("yolo_voice_vocab_test_{name}_{nanos}"));
        path
    }

    #[test]
    fn migrates_legacy_dictionary_with_backup_and_reset() {
        let dir = temp_path("migration");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("global_dictionary.json");
        std::fs::write(
            &path,
            r#"{
  "vocabulary": ["Supabase", "Vercel"],
  "replacements": [{"find": "super base", "replace": "Supabase"}]
}"#,
        )
        .unwrap();

        let result = load_or_migrate_user_dictionary_from_path(&path).unwrap();
        assert_eq!(result.dictionary, UserDictionary::default());

        let backup_path = match result.migration {
            UserDictionaryMigration::LegacyReset { backup_path } => backup_path,
            UserDictionaryMigration::None => panic!("expected legacy migration"),
        };

        assert!(backup_path.exists());

        let migrated: UserDictionary =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(migrated, UserDictionary::default());

        let backup: LegacyGlobalDictionary =
            serde_json::from_str(&std::fs::read_to_string(&backup_path).unwrap()).unwrap();
        assert_eq!(backup.vocabulary, vec!["Supabase", "Vercel"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_existing_dictionary_loads_empty_without_auto_apply() {
        let dir = temp_path("empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("global_dictionary.json");

        let result = load_or_migrate_user_dictionary_from_path(&path).unwrap();
        assert_eq!(result.dictionary, UserDictionary::default());
        assert_eq!(result.migration, UserDictionaryMigration::None);
        assert!(!path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runtime_resolution_includes_only_active_pack() {
        let user = UserDictionary {
            version: 2,
            user_vocabulary: vec!["AlishTerm".to_string()],
            user_normalization_rules: vec![ReplacementRule {
                find: "super base".to_string(),
                replace: "Supabase".to_string(),
            }],
        };
        let software_pack = IndustryPack {
            id: "software_engineering".to_string(),
            name: "Software Engineering".to_string(),
            description: String::new(),
            vocabulary: vec!["TypeScript".to_string()],
            replacements: vec![ReplacementRule {
                find: "type script".to_string(),
                replace: "TypeScript".to_string(),
            }],
        };
        let medical_pack = IndustryPack {
            id: "medical".to_string(),
            name: "Medical".to_string(),
            description: String::new(),
            vocabulary: vec!["Cardiology".to_string()],
            replacements: vec![ReplacementRule {
                find: "cardio allergy".to_string(),
                replace: "Cardiology".to_string(),
            }],
        };

        let software_runtime = resolve_runtime_dictionary(&user, Some(&software_pack));
        let medical_runtime = resolve_runtime_dictionary(&user, Some(&medical_pack));

        assert!(software_runtime.vocabulary.contains(&"AlishTerm".to_string()));
        assert!(software_runtime.vocabulary.contains(&"TypeScript".to_string()));
        assert!(!software_runtime.vocabulary.contains(&"Cardiology".to_string()));

        assert!(medical_runtime.vocabulary.contains(&"Cardiology".to_string()));
        assert!(!medical_runtime.vocabulary.contains(&"TypeScript".to_string()));
    }

    #[test]
    fn personal_terms_generate_safe_compound_aliases() {
        let user = UserDictionary {
            version: 2,
            user_vocabulary: vec![
                "TypeScript".to_string(),
                "Next.js".to_string(),
                "GitHub".to_string(),
            ],
            user_normalization_rules: vec![],
        };

        let runtime = runtime_dictionary_from_user_dictionary(&user);

        assert_eq!(
            apply_normalization_rules("type script is great", &runtime.normalization_rules),
            "TypeScript is great"
        );
        assert_eq!(
            apply_normalization_rules("deploy it in next js", &runtime.normalization_rules),
            "deploy it in Next.js"
        );
        assert_eq!(
            apply_normalization_rules("check github actions", &runtime.normalization_rules),
            "check GitHub actions"
        );
    }

    #[test]
    fn personal_terms_do_not_guess_phonetic_aliases() {
        let user = UserDictionary {
            version: 2,
            user_vocabulary: vec!["Supabase".to_string()],
            user_normalization_rules: vec![],
        };

        let runtime = runtime_dictionary_from_user_dictionary(&user);

        assert_eq!(
            apply_normalization_rules("super base auth", &runtime.normalization_rules),
            "super base auth"
        );
    }

    #[test]
    fn manual_user_rules_override_generated_personal_term_aliases() {
        let user = UserDictionary {
            version: 2,
            user_vocabulary: vec!["TypeScript".to_string()],
            user_normalization_rules: vec![ReplacementRule {
                find: "type script".to_string(),
                replace: "Type Script Custom".to_string(),
            }],
        };

        let runtime = runtime_dictionary_from_user_dictionary(&user);

        assert_eq!(
            apply_normalization_rules("type script", &runtime.normalization_rules),
            "Type Script Custom"
        );
    }

    #[test]
    fn switching_packs_does_not_leak_previous_pack_rules() {
        let user = UserDictionary::default();
        let software_pack = IndustryPack {
            id: "software_engineering".to_string(),
            name: "Software Engineering".to_string(),
            description: String::new(),
            vocabulary: vec![],
            replacements: vec![ReplacementRule {
                find: "type script".to_string(),
                replace: "TypeScript".to_string(),
            }],
        };
        let medical_pack = IndustryPack {
            id: "medical".to_string(),
            name: "Medical".to_string(),
            description: String::new(),
            vocabulary: vec![],
            replacements: vec![ReplacementRule {
                find: "atrial fib relation".to_string(),
                replace: "atrial fibrillation".to_string(),
            }],
        };

        let software_runtime = resolve_runtime_dictionary(&user, Some(&software_pack));
        let medical_runtime = resolve_runtime_dictionary(&user, Some(&medical_pack));

        assert_eq!(
            apply_normalization_rules("type script", &software_runtime.normalization_rules),
            "TypeScript"
        );
        assert_eq!(
            apply_normalization_rules("type script", &medical_runtime.normalization_rules),
            "type script"
        );
    }

    #[test]
    fn longest_first_alias_rules_win() {
        let rules = vec![
            ReplacementRule {
                find: "script".to_string(),
                replace: "Script".to_string(),
            },
            ReplacementRule {
                find: "type script".to_string(),
                replace: "TypeScript".to_string(),
            },
            ReplacementRule {
                find: "typed script".to_string(),
                replace: "TypeScript".to_string(),
            },
        ];

        let ordered = sort_rules_for_application(rules);
        assert_eq!(ordered[0].find, "typed script");
        assert_eq!(ordered[1].find, "type script");

        assert_eq!(
            apply_normalization_rules("type script is nice", &ordered),
            "TypeScript is nice"
        );
    }

    #[test]
    fn resolving_runtime_dictionary_does_not_mutate_user_storage() {
        let user = UserDictionary {
            version: 2,
            user_vocabulary: vec!["PersonalTerm".to_string()],
            user_normalization_rules: vec![ReplacementRule {
                find: "my repo".to_string(),
                replace: "MyRepo".to_string(),
            }],
        };
        let snapshot = user.clone();
        let pack = IndustryPack {
            id: "software_engineering".to_string(),
            name: "Software Engineering".to_string(),
            description: String::new(),
            vocabulary: vec!["TypeScript".to_string()],
            replacements: vec![ReplacementRule {
                find: "type script".to_string(),
                replace: "TypeScript".to_string(),
            }],
        };

        let _runtime = resolve_runtime_dictionary(&user, Some(&pack));
        assert_eq!(user, snapshot);
    }
}
