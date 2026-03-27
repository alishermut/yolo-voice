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

static LOWERCASE_FIRST_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\W*)([A-Z][A-Za-z']*)(.*)$").unwrap());

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

    let match_result = LOWERCASE_FIRST_WORD.captures(text);

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

// ---------------------------------------------------------------------------
// Hallucination filtering
// ---------------------------------------------------------------------------

/// Known hallucination phrases that Whisper / Parakeet produce on silence or
/// background noise.  Matched case-insensitively as whole sentences or entire
/// input.  English-only for now.
static HALLUCINATION_PHRASES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let phrases = [
        "thank you for watching",
        "thanks for watching",
        "thank you for listening",
        "thanks for listening",
        "subscribe to my channel",
        "please subscribe",
        "like and subscribe",
        "please like and subscribe",
        "see you in the next",
        "see you in the next video",
        "see you next time",
        "don't forget to subscribe",
        "hit the like button",
        "bye bye",
    ];
    phrases
        .iter()
        .map(|p| {
            // Match the phrase as a standalone sentence (possibly preceded/followed
            // by punctuation and whitespace) or as the entire input.
            let escaped = regex::escape(p);
            Regex::new(&format!(r"(?i)(?:^|\.\s*|\!\s*|\?\s*){}\s*[.!?]*\s*", escaped))
                .unwrap()
        })
        .collect()
});

/// Bracketed / parenthesised audio labels produced by some models.
static HALLUCINATION_LABELS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[\[\(]\s*(?:music|applause|laughter|silence|inaudible|blank audio)\s*[\]\)]\s*")
        .unwrap()
});

/// Remove known hallucination phrases from the text.
pub fn filter_hallucination_phrases(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut text = input.to_string();

    // Remove bracketed labels first
    text = HALLUCINATION_LABELS.replace_all(&text, " ").to_string();

    // Remove known phrases
    for re in HALLUCINATION_PHRASES.iter() {
        text = re.replace_all(&text, " ").to_string();
    }

    let result = MULTI_SPACE.replace_all(text.trim(), " ").to_string();

    // If only punctuation/whitespace remains, treat as fully hallucinated
    if result.trim().chars().all(|c| c.is_ascii_punctuation() || c.is_whitespace()) {
        return String::new();
    }
    result.trim().to_string()
}

/// Detect and collapse repetition loops.
///
/// If a sentence (split on `.!?`) repeats 3+ consecutive times, keep one.
/// If a sub-sentence phrase of 2-5 words repeats 3+ times in a row, keep one.
pub fn filter_hallucination_loops(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // --- Sentence-level dedup ---
    let sentence_re = LazyLock::force(&SENTENCE_SPLIT);
    let sentences: Vec<&str> = sentence_re
        .split(input)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if sentences.len() >= 3 {
        // Count consecutive runs; collapse runs of 3+ to a single occurrence.
        let mut deduped: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < sentences.len() {
            let start = i;
            // Count how many consecutive identical sentences follow
            while i + 1 < sentences.len()
                && sentences[i + 1].eq_ignore_ascii_case(sentences[start])
            {
                i += 1;
            }
            let run_len = i - start + 1;
            if run_len >= 3 {
                // Collapse to single occurrence
                deduped.push(sentences[start]);
            } else {
                // Keep all (1 or 2 occurrences)
                for j in start..=i {
                    deduped.push(sentences[j]);
                }
            }
            i += 1;
        }
        if deduped.len() < sentences.len() {
            let joined = deduped.join(". ");
            // Restore trailing period if original had one
            let result = if input.trim_end().ends_with('.') && !joined.ends_with('.') {
                format!("{}.", joined)
            } else {
                joined
            };
            return filter_word_level_loops(&result);
        }
    }

    filter_word_level_loops(input)
}

/// Split on sentence-ending punctuation while keeping the structure.
static SENTENCE_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]+\s*").unwrap());

/// Collapse word-level repetition: if a contiguous run of identical 2-5 word
/// chunks repeats 3+ times, keep one occurrence.
fn filter_word_level_loops(input: &str) -> String {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.len() < 6 {
        return input.to_string();
    }

    let mut result = words.clone();
    // Try phrase lengths from 5 down to 2
    for phrase_len in (2..=5).rev() {
        let mut cleaned: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < result.len() {
            if i + phrase_len * 3 <= result.len() {
                let phrase: Vec<&str> = result[i..i + phrase_len].to_vec();
                let phrase_lower: Vec<String> =
                    phrase.iter().map(|w| w.to_lowercase()).collect();
                let mut reps = 1usize;
                let mut j = i + phrase_len;
                while j + phrase_len <= result.len() {
                    let next: Vec<String> = result[j..j + phrase_len]
                        .iter()
                        .map(|w| w.to_lowercase())
                        .collect();
                    if next == phrase_lower {
                        reps += 1;
                        j += phrase_len;
                    } else {
                        break;
                    }
                }
                if reps >= 3 {
                    // Keep one occurrence, skip the rest
                    cleaned.extend_from_slice(&result[i..i + phrase_len]);
                    i = j;
                    continue;
                }
            }
            cleaned.push(result[i]);
            i += 1;
        }
        result = cleaned;
    }

    result.join(" ")
}

/// Combined hallucination filter: phrases first, then loops.
pub fn filter_hallucinations(input: &str) -> String {
    let after_phrases = filter_hallucination_phrases(input);
    if after_phrases.is_empty() {
        return String::new();
    }
    filter_hallucination_loops(&after_phrases)
}

// ---------------------------------------------------------------------------
// Spoken punctuation commands
// ---------------------------------------------------------------------------

struct SpokenPunctRule {
    regex: Regex,
    replacement: &'static str,
}

static SPOKEN_PUNCTUATION_RULES: LazyLock<Vec<SpokenPunctRule>> = LazyLock::new(|| {
    let rules: Vec<(&str, &str)> = vec![
        (r"(?i)\bnew paragraph\b", "\n\n"),
        (r"(?i)\bnew line\b", "\n"),
        (r"(?i)\bnewline\b", "\n"),
        (r"(?i)\bfull stop\b", "."),
        (r"(?i)\bquestion mark\b", "?"),
        (r"(?i)\bexclamation mark\b", "!"),
        (r"(?i)\bexclamation point\b", "!"),
        (r"(?i)\bopen parenthesis\b", "("),
        (r"(?i)\bclose parenthesis\b", ")"),
        (r"(?i)\bopen paren\b", "("),
        (r"(?i)\bclose paren\b", ")"),
        (r"(?i)\bopen quote\b", "\""),
        (r"(?i)\bclose quote\b", "\""),
        (r"(?i)\bend quote\b", "\""),
        (r"(?i)\bsemi colon\b", ";"),
        (r"(?i)\bsemicolon\b", ";"),
        (r"(?i)\bperiod\b", "."),
        (r"(?i)\bcomma\b", ","),
        (r"(?i)\bcolon\b", ":"),
        (r"(?i)\bdash\b", " \u{2014} "),
        (r"(?i)\bhyphen\b", "-"),
    ];
    rules
        .into_iter()
        .map(|(pattern, replacement)| SpokenPunctRule {
            regex: Regex::new(pattern).unwrap(),
            replacement,
        })
        .collect()
});

/// Replace spoken punctuation words with their symbol equivalents.
///
/// English-only. Multi-word commands ("new line", "question mark") are matched
/// before single-word ones ("period", "comma") because the rule list is
/// processed in order and multi-word patterns are listed first.
pub fn replace_spoken_punctuation(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut text = input.to_string();
    for rule in SPOKEN_PUNCTUATION_RULES.iter() {
        text = rule.regex.replace_all(&text, rule.replacement).to_string();
    }

    // Normalize spacing but preserve intentional newlines
    let mut result = String::new();
    for line in text.split('\n') {
        let cleaned = normalize_spacing_and_punctuation(line);
        let trimmed = cleaned.trim();
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(trimmed);
    }
    result
}

// ---------------------------------------------------------------------------
// Number-word → digit conversion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum NumberKind {
    Unit,    // 0–9
    Teen,    // 10–19
    Tens,    // 20, 30, …, 90
    Hundred, // 100
    Scale,   // 1 000+
}

fn number_word_info(word: &str) -> Option<(u64, NumberKind)> {
    Some(match word.to_ascii_lowercase().as_str() {
        "zero" => (0, NumberKind::Unit),
        "one" => (1, NumberKind::Unit),
        "two" => (2, NumberKind::Unit),
        "three" => (3, NumberKind::Unit),
        "four" => (4, NumberKind::Unit),
        "five" => (5, NumberKind::Unit),
        "six" => (6, NumberKind::Unit),
        "seven" => (7, NumberKind::Unit),
        "eight" => (8, NumberKind::Unit),
        "nine" => (9, NumberKind::Unit),
        "ten" => (10, NumberKind::Teen),
        "eleven" => (11, NumberKind::Teen),
        "twelve" => (12, NumberKind::Teen),
        "thirteen" => (13, NumberKind::Teen),
        "fourteen" => (14, NumberKind::Teen),
        "fifteen" => (15, NumberKind::Teen),
        "sixteen" => (16, NumberKind::Teen),
        "seventeen" => (17, NumberKind::Teen),
        "eighteen" => (18, NumberKind::Teen),
        "nineteen" => (19, NumberKind::Teen),
        "twenty" => (20, NumberKind::Tens),
        "thirty" => (30, NumberKind::Tens),
        "forty" => (40, NumberKind::Tens),
        "fifty" => (50, NumberKind::Tens),
        "sixty" => (60, NumberKind::Tens),
        "seventy" => (70, NumberKind::Tens),
        "eighty" => (80, NumberKind::Tens),
        "ninety" => (90, NumberKind::Tens),
        "hundred" => (100, NumberKind::Hundred),
        "thousand" => (1_000, NumberKind::Scale),
        "million" => (1_000_000, NumberKind::Scale),
        "billion" => (1_000_000_000, NumberKind::Scale),
        "trillion" => (1_000_000_000_000, NumberKind::Scale),
        _ => return None,
    })
}

/// Split a bare word on `-` if both halves are number words (e.g. "twenty-one").
fn expand_number_token(bare: &str) -> Vec<String> {
    if number_word_info(bare).is_some() {
        return vec![bare.to_string()];
    }
    if let Some((left, right)) = bare.split_once('-') {
        if number_word_info(left).is_some() && number_word_info(right).is_some() {
            return vec![left.to_string(), right.to_string()];
        }
    }
    vec![]
}

/// Split trailing ASCII punctuation from a token: `"five,"` → `("five", ",")`.
fn split_trailing_punct(token: &str) -> (&str, &str) {
    let bare = token.trim_end_matches(|c: char| c.is_ascii_punctuation());
    (&token[..bare.len()], &token[bare.len()..])
}

#[derive(Clone)]
struct NumberAcc {
    result: u64,
    group: u64,
    group_has_content: bool,
}

impl NumberAcc {
    fn new() -> Self {
        Self { result: 0, group: 0, group_has_content: false }
    }

    /// Try to incorporate `value` of `kind`. Returns false if it would start a
    /// new number rather than extending the current one.
    fn try_feed(&mut self, value: u64, kind: NumberKind) -> bool {
        match kind {
            NumberKind::Scale => {
                if self.group == 0 && !self.group_has_content {
                    self.group = 1;
                }
                self.result += self.group * value;
                self.group = 0;
                self.group_has_content = false;
                true
            }
            NumberKind::Hundred => {
                if self.group == 0 && !self.group_has_content {
                    self.group = 1;
                }
                if self.group < 100 {
                    self.group *= 100;
                    true
                } else {
                    false
                }
            }
            NumberKind::Tens | NumberKind::Teen => {
                if self.group % 100 == 0 {
                    self.group += value;
                    self.group_has_content = true;
                    true
                } else {
                    false
                }
            }
            NumberKind::Unit => {
                if value == 0 {
                    // "zero" only valid as a fresh start
                    if !self.group_has_content && self.result == 0 {
                        self.group_has_content = true;
                        true
                    } else {
                        false
                    }
                } else if !self.group_has_content {
                    self.group += value;
                    self.group_has_content = true;
                    true
                } else if self.group % 10 == 0 && self.group >= 20 {
                    // After tens: "twenty" + "five"
                    self.group += value;
                    true
                } else if self.group % 100 == 0 && self.group >= 100 {
                    // After hundreds: "three hundred" + "five"
                    self.group += value;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn finalize(&self) -> u64 {
        self.result + self.group
    }
}

/// Replace English number words with their digit equivalents.
///
/// ```text
/// "twenty three"                     → "23"
/// "one hundred and fifty"            → "150"
/// "I have five apples and ten pears" → "I have 5 apples and 10 pears"
/// ```
pub fn convert_number_words(input: &str) -> String {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }

    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let (bare, suffix) = split_trailing_punct(tokens[i]);
        let expanded = expand_number_token(bare);

        if expanded.is_empty() {
            parts.push(tokens[i].to_string());
            i += 1;
            continue;
        }

        // Start a number sequence
        let mut acc = NumberAcc::new();
        let mut last_suffix = suffix;

        for word in &expanded {
            if let Some((val, kind)) = number_word_info(word) {
                acc.try_feed(val, kind);
            }
        }
        i += 1;

        // Greedily consume more number words
        while i < tokens.len() {
            let (next_bare, next_suffix) = split_trailing_punct(tokens[i]);

            // Handle "and" between number parts
            if next_bare.eq_ignore_ascii_case("and") {
                if next_suffix.is_empty() {
                    if i + 1 < tokens.len() {
                        let (after, _) = split_trailing_punct(tokens[i + 1]);
                        if !expand_number_token(after).is_empty() {
                            i += 1; // skip "and"
                            continue;
                        }
                    }
                }
                break;
            }

            let next_expanded = expand_number_token(next_bare);
            if next_expanded.is_empty() {
                break;
            }

            // Speculatively extend
            let mut temp = acc.clone();
            let mut ok = true;
            for word in &next_expanded {
                if let Some((val, kind)) = number_word_info(word) {
                    if !temp.try_feed(val, kind) {
                        ok = false;
                        break;
                    }
                }
            }

            if ok {
                acc = temp;
                last_suffix = next_suffix;
                i += 1;
            } else {
                break;
            }
        }

        parts.push(format!("{}{}", acc.finalize(), last_suffix));
    }

    parts.join(" ")
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

    // --- Number word conversion tests ---

    #[test]
    fn converts_single_digits() {
        assert_eq!(convert_number_words("five"), "5");
        assert_eq!(convert_number_words("zero"), "0");
        assert_eq!(convert_number_words("nine"), "9");
    }

    #[test]
    fn converts_teens_and_tens() {
        assert_eq!(convert_number_words("thirteen"), "13");
        assert_eq!(convert_number_words("twenty"), "20");
        assert_eq!(convert_number_words("ninety"), "90");
    }

    #[test]
    fn converts_compound_tens() {
        assert_eq!(convert_number_words("twenty three"), "23");
        assert_eq!(convert_number_words("forty five"), "45");
        assert_eq!(convert_number_words("ninety nine"), "99");
    }

    #[test]
    fn converts_hyphenated_compounds() {
        assert_eq!(convert_number_words("twenty-three"), "23");
        assert_eq!(convert_number_words("sixty-one"), "61");
    }

    #[test]
    fn converts_hundreds() {
        assert_eq!(convert_number_words("three hundred"), "300");
        assert_eq!(convert_number_words("three hundred twenty three"), "323");
        assert_eq!(convert_number_words("one hundred and fifty"), "150");
    }

    #[test]
    fn converts_thousands() {
        assert_eq!(convert_number_words("five thousand"), "5000");
        assert_eq!(
            convert_number_words("two thousand five hundred and thirty four"),
            "2534"
        );
    }

    #[test]
    fn converts_numbers_in_sentence() {
        assert_eq!(
            convert_number_words("I have twenty three apples"),
            "I have 23 apples"
        );
        assert_eq!(
            convert_number_words("chapter five section twelve"),
            "chapter 5 section 12"
        );
    }

    #[test]
    fn preserves_trailing_punctuation() {
        assert_eq!(convert_number_words("five."), "5.");
        assert_eq!(convert_number_words("twenty three,"), "23,");
        assert_eq!(convert_number_words("I need five."), "I need 5.");
    }

    #[test]
    fn keeps_separate_numbers_separate() {
        assert_eq!(convert_number_words("five five"), "5 5");
        assert_eq!(convert_number_words("thirteen five"), "13 5");
    }

    #[test]
    fn standalone_hundred_thousand() {
        assert_eq!(convert_number_words("hundred"), "100");
        assert_eq!(convert_number_words("thousand"), "1000");
    }

    #[test]
    fn no_numbers_passthrough() {
        assert_eq!(convert_number_words("hello world"), "hello world");
        assert_eq!(convert_number_words(""), "");
    }

    // --- Hallucination filtering tests ---

    #[test]
    fn filters_known_hallucination_phrases() {
        assert_eq!(filter_hallucinations("Thank you for watching"), "");
        assert_eq!(filter_hallucinations("thanks for listening"), "");
        assert_eq!(filter_hallucinations("Please subscribe"), "");
        assert_eq!(filter_hallucinations("Like and subscribe"), "");
        assert_eq!(filter_hallucinations("[Music]"), "");
        assert_eq!(filter_hallucinations("(applause)"), "");
        assert_eq!(filter_hallucinations("(Silence)"), "");
    }

    #[test]
    fn preserves_normal_text_with_similar_words() {
        // "thank you" alone is NOT a hallucination phrase
        assert_eq!(filter_hallucinations("thank you"), "thank you");
        assert_eq!(
            filter_hallucinations("I want to subscribe to the newsletter"),
            "I want to subscribe to the newsletter"
        );
    }

    #[test]
    fn removes_hallucination_mixed_with_real_text() {
        let input = "Hello world. Thank you for watching.";
        let result = filter_hallucinations(input);
        assert!(result.contains("Hello world"));
        assert!(!result.contains("Thank you for watching"));
    }

    #[test]
    fn collapses_sentence_repetition_loops() {
        // 4 repetitions → collapsed to 1
        assert_eq!(
            filter_hallucinations("Thank you. Thank you. Thank you. Thank you."),
            "Thank you."
        );
        // Two repetitions are preserved (not a loop of 3+)
        assert_eq!(
            filter_hallucinations("OK. OK."),
            "OK. OK."
        );
        // 3 repetitions → collapsed to 1
        assert_eq!(
            filter_hallucination_loops("Stop. Stop. Stop."),
            "Stop."
        );
    }

    #[test]
    fn collapses_word_level_loops() {
        // 3+ repetitions of a multi-word phrase
        let input = "the end the end the end the end";
        let result = filter_hallucination_loops(input);
        assert_eq!(result, "the end");
    }

    #[test]
    fn preserves_intentional_double_repetition() {
        // "very very" is only 2 reps, not 3 — preserved
        assert_eq!(
            filter_hallucination_loops("this is very very important"),
            "this is very very important"
        );
    }

    #[test]
    fn empty_after_all_filtering() {
        assert_eq!(filter_hallucinations(""), "");
        assert_eq!(filter_hallucinations("   "), "");
        assert_eq!(filter_hallucinations("..."), "");
    }

    // --- Spoken punctuation tests ---

    #[test]
    fn replaces_basic_spoken_punctuation() {
        assert_eq!(
            replace_spoken_punctuation("hello period how are you"),
            "hello. how are you"
        );
        assert_eq!(
            replace_spoken_punctuation("yes comma I agree"),
            "yes, I agree"
        );
    }

    #[test]
    fn replaces_question_and_exclamation() {
        assert_eq!(
            replace_spoken_punctuation("is that right question mark"),
            "is that right?"
        );
        assert_eq!(
            replace_spoken_punctuation("wow exclamation mark"),
            "wow!"
        );
        assert_eq!(
            replace_spoken_punctuation("really exclamation point"),
            "really!"
        );
    }

    #[test]
    fn replaces_new_line_and_paragraph() {
        assert_eq!(
            replace_spoken_punctuation("first line new line second line"),
            "first line\nsecond line"
        );
        assert_eq!(
            replace_spoken_punctuation("intro new paragraph body"),
            "intro\n\nbody"
        );
    }

    #[test]
    fn replaces_multi_word_before_single_word() {
        // "full stop" should be matched before "stop" could cause issues
        assert_eq!(
            replace_spoken_punctuation("end of sentence full stop"),
            "end of sentence."
        );
        // "semi colon" (two words) should work
        assert_eq!(
            replace_spoken_punctuation("item one semi colon item two"),
            "item one; item two"
        );
    }

    #[test]
    fn spoken_punctuation_passthrough() {
        assert_eq!(
            replace_spoken_punctuation("hello world"),
            "hello world"
        );
        assert_eq!(replace_spoken_punctuation(""), "");
    }
}
