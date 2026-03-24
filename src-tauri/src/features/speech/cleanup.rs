//! Deterministic text cleanup and segment recombination for dictated speech.
//!
//! The VAD path now uses two cleanup stages:
//! - lightweight per-segment cleanup before joining
//! - stronger final cleanup after full assembly

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

static HARD_FILLER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(uh[\s-]huh|um+|uh+|er+|ah+|huh|mm+|hm+)\b[,]?\s*").unwrap()
});

static LEADING_DISCOURSE_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:you know|i mean)\b,\s*").unwrap());

static MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" {2,}").unwrap());
static SPACE_BEFORE_PUNCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" ([.,!?;:])").unwrap());
static DOUBLE_PUNCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([.,!?;:])\s*([.,!?;:])").unwrap());
static SENTENCE_CAP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([.!?])\s+([a-z])").unwrap());

static RESTART_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "i", "the", "a", "an", "we", "you", "he", "she", "it", "they", "to", "of", "and",
        "but",
    ])
});

static CONTINUATION_FIRST_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "and", "but", "so", "because", "if", "then", "or", "that", "which", "who", "whom",
        "when", "while", "unless", "though", "although", "after", "before", "until", "as",
        "whether", "to", "for", "with", "from", "of", "in", "on", "at", "by", "the", "a",
        "an", "is", "are", "was", "were", "today", "tomorrow", "yesterday", "later", "now",
        "soon",
    ])
});

static TRAILING_INCOMPLETE_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "and",
        "or",
        "but",
        "so",
        "because",
        "if",
        "then",
        "to",
        "for",
        "with",
        "from",
        "of",
        "in",
        "on",
        "at",
        "by",
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "should",
        "could",
        "would",
        "can",
        "will",
        "just",
        "probably",
        "really",
        "very",
        "kind",
        "sort",
        "literally",
        "basically",
        "actually",
        "think",
        "guess",
        "had",
    ])
});

static COMMA_LEAD_INS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["well", "so", "actually", "basically"]));

static AFFIRMATIONS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["yes", "no"]));

/// Lightweight per-segment cleanup. Avoids sentence shaping so segment joins can
/// use more context later.
pub fn clean_segment_text(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut text = input.to_string();
    text = HARD_FILLER.replace_all(&text, "").to_string();
    text = LEADING_DISCOURSE_MARKER.replace(&text, "").to_string();
    text = remove_restart_stutters(&text);
    normalize_spacing_and_punctuation(&text).trim().to_string()
}

/// Stronger final cleanup after segments have been assembled.
pub fn clean_final_text(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut text = input.to_string();
    text = HARD_FILLER.replace_all(&text, "").to_string();
    text = LEADING_DISCOURSE_MARKER.replace(&text, "").to_string();
    text = remove_restart_stutters(&text);
    finalize_text(&text)
}

/// Heuristic segment joining for VAD output.
pub fn join_segments_heuristic(segments: &[String]) -> String {
    let cleaned: Vec<String> = segments
        .iter()
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if cleaned.is_empty() {
        return String::new();
    }
    if cleaned.len() == 1 {
        return cleaned[0].clone();
    }

    let mut result = cleaned[0].clone();

    for segment in &cleaned[1..] {
        let prev = result.trim_end().to_string();
        let curr = segment.trim();
        let first = first_word_lower(curr);
        let last = last_word_lower(&prev);
        let prev_tokens = tokenize(&prev);

        if prev.ends_with(['.', '!', '?']) {
            result = format!("{} {}", prev, curr);
            continue;
        }

        if prev.ends_with([',', ':']) {
            result = format!("{} {}", prev, lowercase_first_word_force(curr));
            continue;
        }

        if is_short_affirmation(&prev_tokens, &first) {
            result = format!("{} {}", prev, lowercase_first_word_force(curr));
            continue;
        }

        if prev_tokens.len() <= 2 && COMMA_LEAD_INS.contains(last.as_str()) {
            result = format!("{} , {}", prev.trim_end_matches(','), lowercase_first_word_force(curr));
            result = result.replace(" , ", ", ");
            continue;
        }

        if should_join_as_continuation(&prev_tokens, &first, &last) {
            result = format!("{} {}", prev, lowercase_first_word_force(curr));
            continue;
        }

        result = format!("{}. {}", prev.trim_end_matches('.'), curr);
    }

    result
}

/// Minimal join path when text cleanup is disabled.
pub fn join_segments_minimal(segments: &[String]) -> String {
    segments
        .iter()
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn should_join_as_continuation(prev_tokens: &[String], first: &str, last: &str) -> bool {
    if CONTINUATION_FIRST_WORDS.contains(first) || TRAILING_INCOMPLETE_WORDS.contains(last) {
        return true;
    }

    if ends_with_meaningful_repetition(prev_tokens) {
        return true;
    }

    false
}

fn is_short_affirmation(prev_tokens: &[String], first: &str) -> bool {
    if prev_tokens.len() != 1 {
        return false;
    }

    let last = prev_tokens
        .last()
        .map(|token| normalize_token(token))
        .unwrap_or_default();
    if !AFFIRMATIONS.contains(last.as_str()) {
        return false;
    }

    !first.is_empty() && first != "can"
}

fn ends_with_meaningful_repetition(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }

    let last = normalize_token(tokens.last().unwrap());
    let prev = normalize_token(&tokens[tokens.len() - 2]);
    !last.is_empty() && last == prev
}

fn remove_restart_stutters(input: &str) -> String {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut result = vec![words[0]];
    for word in &words[1..] {
        let prev_norm = normalize_token(result.last().unwrap());
        let curr_norm = normalize_token(word);

        if !prev_norm.is_empty() && prev_norm == curr_norm && RESTART_WORDS.contains(curr_norm.as_str())
        {
            continue;
        }

        result.push(word);
    }

    result.join(" ")
}

fn normalize_spacing_and_punctuation(input: &str) -> String {
    let mut text = input.to_string();
    text = DOUBLE_PUNCT.replace_all(&text, "$2").to_string();
    text = SPACE_BEFORE_PUNCT.replace_all(&text, "$1").to_string();
    text = MULTI_SPACE.replace_all(&text, " ").to_string();
    text.trim_matches([' ', '-']).to_string()
}

fn finalize_text(input: &str) -> String {
    let mut text = normalize_spacing_and_punctuation(input);

    if !text.is_empty() {
        text = capitalize_first(&text);
        text = SENTENCE_CAP
            .replace_all(&text, |caps: &regex::Captures| {
                format!("{} {}", &caps[1], caps[2].to_uppercase())
            })
            .to_string();
    }

    text
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace().map(|token| token.to_string()).collect()
}

fn normalize_token(token: &str) -> String {
    let trimmed = token
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '\'')
        .to_lowercase();
    trimmed
}

fn first_word_lower(text: &str) -> String {
    tokenize(text)
        .first()
        .map(|token| normalize_token(token))
        .unwrap_or_default()
}

fn last_word_lower(text: &str) -> String {
    tokenize(text)
        .last()
        .map(|token| normalize_token(token))
        .unwrap_or_default()
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn lowercase_first_word_force(text: &str) -> String {
    let Some(first_word) = tokenize(text).first().cloned() else {
        return text.to_string();
    };

    let match_result = Regex::new(r"^(\W*)([A-Z][A-Za-z']*)(.*)$")
        .unwrap()
        .captures(text);

    let Some(caps) = match_result else {
        return text.to_string();
    };

    let prefix = caps.get(1).map(|value| value.as_str()).unwrap_or_default();
    let word = caps.get(2).map(|value| value.as_str()).unwrap_or_default();
    let suffix = caps.get(3).map(|value| value.as_str()).unwrap_or_default();

    if word == "I" || (word.chars().all(|ch| ch.is_ascii_uppercase()) && word.len() > 1) {
        return text.to_string();
    }

    if first_word == word {
        format!("{}{}{}", prefix, word.to_lowercase(), suffix)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_hard_fillers() {
        assert_eq!(clean_final_text("I um think this is good"), "I think this is good");
        assert_eq!(clean_final_text("uh hello there"), "Hello there");
        assert_eq!(clean_final_text("uh-huh yes that's correct"), "Yes that's correct");
    }

    #[test]
    fn removes_only_narrow_discourse_markers() {
        assert_eq!(
            clean_final_text("I mean, we should probably deploy today"),
            "We should probably deploy today"
        );
        assert_eq!(
            clean_final_text("you know, this is the right file"),
            "This is the right file"
        );
        assert_eq!(
            clean_final_text("you know this can fail"),
            "You know this can fail"
        );
        assert_eq!(
            clean_final_text("I mean this is not ideal"),
            "I mean this is not ideal"
        );
    }

    #[test]
    fn preserves_semantic_hedges() {
        assert_eq!(clean_final_text("it's basically done"), "It's basically done");
        assert_eq!(
            clean_final_text("I literally saw it happen"),
            "I literally saw it happen"
        );
        assert_eq!(
            clean_final_text("this is sort of fragile"),
            "This is sort of fragile"
        );
    }

    #[test]
    fn preserves_meaningful_repetition() {
        assert_eq!(
            clean_final_text("this is very very important"),
            "This is very very important"
        );
        assert_eq!(clean_final_text("I had had enough"), "I had had enough");
        assert_eq!(
            clean_final_text("maybe maybe we should wait"),
            "Maybe maybe we should wait"
        );
        assert_eq!(clean_final_text("it felt so so slow"), "It felt so so slow");
    }

    #[test]
    fn removes_restart_stutters() {
        assert_eq!(clean_final_text("I I think we should go"), "I think we should go");
        assert_eq!(
            clean_final_text("the the problem is the API key"),
            "The problem is the API key"
        );
        assert_eq!(
            clean_final_text("we we were going to ship today"),
            "We were going to ship today"
        );
    }

    #[test]
    fn segment_cleanup_stays_lightweight() {
        assert_eq!(clean_segment_text("uh open the settings menu"), "open the settings menu");
        assert_eq!(clean_segment_text("actually"), "actually");
        assert_eq!(clean_segment_text("hello. how are you"), "hello. how are you");
    }

    #[test]
    fn heuristic_join_handles_incomplete_clauses() {
        let segments = vec!["the API endpoint".to_string(), "is down".to_string()];
        assert_eq!(join_segments_heuristic(&segments), "the API endpoint is down");
    }

    #[test]
    fn heuristic_join_handles_comma_continuation() {
        let segments = vec!["For the rollout,".to_string(), "We should notify support".to_string()];
        assert_eq!(
            join_segments_heuristic(&segments),
            "For the rollout, we should notify support"
        );
    }

    #[test]
    fn heuristic_join_handles_short_lead_ins() {
        let segments = vec!["Actually".to_string(), "I think the first version was better".to_string()];
        assert_eq!(
            join_segments_heuristic(&segments),
            "Actually, I think the first version was better"
        );
    }

    #[test]
    fn heuristic_join_handles_affirmations_without_sentence_breaks() {
        let segments = vec!["Yes".to_string(), "That matches my logs".to_string()];
        assert_eq!(join_segments_heuristic(&segments), "Yes that matches my logs");
    }

    #[test]
    fn heuristic_join_handles_time_adverb_continuations() {
        let segments = vec!["we were going to ship".to_string(), "today".to_string()];
        assert_eq!(join_segments_heuristic(&segments), "we were going to ship today");
    }

    #[test]
    fn minimal_join_preserves_rawish_text_when_cleanup_disabled() {
        let segments = vec!["Actually".to_string(), "I Think The First Version Was Better".to_string()];
        assert_eq!(
            join_segments_minimal(&segments),
            "Actually I Think The First Version Was Better"
        );
    }

    #[test]
    fn clean_final_text_capitalizes_sentence_starts() {
        assert_eq!(clean_final_text("hello. how are you"), "Hello. How are you");
    }

    #[test]
    fn handles_empty_input() {
        assert_eq!(clean_segment_text(""), "");
        assert_eq!(clean_final_text("   "), "");
        assert_eq!(join_segments_heuristic(&[]), "");
    }
}
