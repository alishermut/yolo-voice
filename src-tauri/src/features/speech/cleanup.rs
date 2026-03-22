//! Text cleanup for dictated speech.
//!
//! Runs automatically after transcription — removes filler words, stutters,
//! normalizes whitespace and punctuation. Zero latency, no external deps.

use std::sync::LazyLock;

use regex::Regex;

// ── Compiled regexes (compiled once, reused) ─────────────────────────────────

/// Single filler words: um, uh, uhh, umm, mmm, hmm, er, ah, huh, uh-huh
static FILLER_SINGLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(um+|uh+|er+|ah+|huh|uh[\s-]huh|mm+|hm+)\b[,]?\s*").unwrap()
});

/// Filler phrases: "you know", "I mean", "sort of", "kind of", etc.
/// Matches with optional trailing comma and space.
static FILLER_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(you know|I mean|sort of|kind of|basically|actually|literally)\b[,]?\s*")
        .unwrap()
});

// Stutter/repeat removal is done programmatically (regex crate doesn't support backreferences)

/// Multiple spaces → single space
static MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r" {2,}").unwrap()
});

/// Space before punctuation: " ." → ".", " ," → ","
static SPACE_BEFORE_PUNCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r" ([.,!?;:])").unwrap()
});

/// Orphaned/doubled punctuation from filler removal: ",," → ","  ", ." → "."
static DOUBLE_PUNCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([.,!?;:])\s*([.,!?;:])").unwrap()
});

/// Sentence boundary: period/exclamation/question followed by space and lowercase
static SENTENCE_CAP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([.!?])\s+([a-z])").unwrap()
});

// ── Public API ───────────────────────────────────────────────────────────────

/// Clean up dictated speech text.
///
/// Removes filler words, stutters, normalizes whitespace and punctuation,
/// and capitalizes sentence starts. Designed to be always-on with zero
/// perceptible latency.
pub fn clean_text(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut text = input.to_string();

    // 1. Remove filler words and phrases
    text = FILLER_SINGLE.replace_all(&text, "").to_string();
    text = FILLER_PHRASE.replace_all(&text, "").to_string();

    // 2. Remove stutters/repeats: "I I think" → "I think"
    text = remove_stutters(&text);

    // 3. Clean up punctuation artifacts
    text = DOUBLE_PUNCT.replace_all(&text, "$2").to_string();
    text = SPACE_BEFORE_PUNCT.replace_all(&text, "$1").to_string();

    // 4. Normalize whitespace
    text = MULTI_SPACE.replace_all(&text, " ").to_string();
    text = text.trim().to_string();

    // 5. Capitalize sentence starts
    if !text.is_empty() {
        // Capitalize first character
        let mut chars = text.chars();
        if let Some(first) = chars.next() {
            text = first.to_uppercase().to_string() + chars.as_str();
        }

        // Capitalize after sentence-ending punctuation
        text = SENTENCE_CAP
            .replace_all(&text, |caps: &regex::Captures| {
                format!(
                    "{} {}",
                    &caps[1],
                    caps[2].to_uppercase()
                )
            })
            .to_string();
    }

    text
}

/// Join text segments with smart punctuation.
///
/// If the previous segment doesn't end with sentence-ending punctuation,
/// adds a period before joining. Capitalizes the start of each new sentence.
pub fn smart_join(segments: &[String]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    if segments.len() == 1 {
        return segments[0].clone();
    }

    let mut result = segments[0].clone();

    for seg in &segments[1..] {
        if seg.trim().is_empty() {
            continue;
        }

        let trimmed_prev = result.trim_end();
        let needs_period = !trimmed_prev.ends_with('.')
            && !trimmed_prev.ends_with('!')
            && !trimmed_prev.ends_with('?')
            && !trimmed_prev.ends_with(',');

        if needs_period {
            // Add period to end the previous sentence
            result = trimmed_prev.to_string();
            result.push('.');
        }

        // Capitalize the first letter of the new segment
        let capitalized = capitalize_first(seg.trim());
        result.push(' ');
        result.push_str(&capitalized);
    }

    result
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Remove consecutive duplicate words: "I I think" → "I think"
fn remove_stutters(input: &str) -> String {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut result = vec![words[0]];
    for word in &words[1..] {
        let prev = result.last().unwrap();
        // Case-insensitive comparison for duplicate detection
        if !prev.eq_ignore_ascii_case(word) {
            result.push(word);
        }
    }

    result.join(" ")
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_filler_words() {
        assert_eq!(clean_text("I um think this is good"), "I think this is good");
        assert_eq!(clean_text("uh hello there"), "Hello there");
        assert_eq!(clean_text("so um uh yeah"), "So yeah");
    }

    #[test]
    fn removes_filler_phrases() {
        assert_eq!(
            clean_text("I think, you know, we should go"),
            "I think, we should go"
        );
        assert_eq!(
            clean_text("it's basically done"),
            "It's done"
        );
    }

    #[test]
    fn removes_stutters() {
        assert_eq!(clean_text("I I think so"), "I think so");
        assert_eq!(clean_text("the the the cat"), "The cat");
    }

    #[test]
    fn capitalizes_sentences() {
        assert_eq!(
            clean_text("hello. how are you"),
            "Hello. How are you"
        );
    }

    #[test]
    fn handles_empty_input() {
        assert_eq!(clean_text(""), "");
        assert_eq!(clean_text("   "), "");
    }

    #[test]
    fn smart_join_adds_periods() {
        let segments = vec![
            "Hello there".to_string(),
            "how are you".to_string(),
        ];
        assert_eq!(smart_join(&segments), "Hello there. How are you");
    }

    #[test]
    fn smart_join_preserves_existing_punctuation() {
        let segments = vec![
            "Hello there.".to_string(),
            "How are you?".to_string(),
        ];
        assert_eq!(smart_join(&segments), "Hello there. How are you?");
    }
}
