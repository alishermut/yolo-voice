#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageFamily {
    Latin,
    Cyrillic,
    Unknown,
}

impl LanguageFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            LanguageFamily::Latin => "latin",
            LanguageFamily::Cyrillic => "cyrillic",
            LanguageFamily::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageLockConfidence {
    None,
    High,
}

impl LanguageLockConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            LanguageLockConfidence::None => "none",
            LanguageLockConfidence::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptCounts {
    pub latin: usize,
    pub cyrillic: usize,
}

impl ScriptCounts {
    pub fn total(self) -> usize {
        self.latin + self.cyrillic
    }

    pub fn mixed(self) -> bool {
        self.latin >= 2 && self.cyrillic >= 2
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewLanguageAnalysis {
    pub family: LanguageFamily,
    pub confidence: LanguageLockConfidence,
    pub confident_lock_reached_by_second_segment: bool,
    pub mixed_script_detected: bool,
    pub unstable: bool,
    pub non_empty_segments: usize,
    pub alphabetic_chars: usize,
}

pub fn script_counts(text: &str) -> ScriptCounts {
    let mut counts = ScriptCounts::default();
    for ch in text.chars() {
        if is_latin(ch) {
            counts.latin += 1;
        } else if is_cyrillic(ch) {
            counts.cyrillic += 1;
        }
    }
    counts
}

pub fn dominant_family_from_counts(counts: ScriptCounts) -> LanguageFamily {
    let total = counts.total();
    if total == 0 {
        return LanguageFamily::Unknown;
    }

    if counts.latin * 100 >= total * 80 {
        LanguageFamily::Latin
    } else if counts.cyrillic * 100 >= total * 80 {
        LanguageFamily::Cyrillic
    } else {
        LanguageFamily::Unknown
    }
}

pub fn detect_language_family(text: &str) -> LanguageFamily {
    dominant_family_from_counts(script_counts(text))
}

pub fn analyze_preview_segments(segments: &[String]) -> PreviewLanguageAnalysis {
    let mut cumulative = ScriptCounts::default();
    let mut non_empty_segments = 0usize;
    let mut confident_lock_reached_by_second_segment = false;
    let mut confidence = LanguageLockConfidence::None;
    let mut family = LanguageFamily::Unknown;
    let mut mixed_script_detected = false;
    let mut unstable = false;

    for segment in segments {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }

        non_empty_segments += 1;
        let segment_counts = script_counts(trimmed);
        cumulative.latin += segment_counts.latin;
        cumulative.cyrillic += segment_counts.cyrillic;
        mixed_script_detected |= cumulative.mixed() || segment_counts.mixed();

        let alpha = cumulative.total();
        let eligible_to_lock = non_empty_segments >= 2 || alpha >= 12;
        let cumulative_family = dominant_family_from_counts(cumulative);

        if confidence == LanguageLockConfidence::None && eligible_to_lock && cumulative_family != LanguageFamily::Unknown
        {
            confidence = LanguageLockConfidence::High;
            family = cumulative_family;
            if non_empty_segments <= 2 {
                confident_lock_reached_by_second_segment = true;
            }
        }

        if confidence == LanguageLockConfidence::High {
            let opposite = match family {
                LanguageFamily::Latin => segment_counts.cyrillic,
                LanguageFamily::Cyrillic => segment_counts.latin,
                LanguageFamily::Unknown => 0,
            };
            let segment_total = segment_counts.total();
            if segment_total >= 4 && opposite * 100 >= segment_total * 80 {
                unstable = true;
                mixed_script_detected = true;
            }
        }
    }

    if confidence == LanguageLockConfidence::None {
        family = dominant_family_from_counts(cumulative);
    }

    PreviewLanguageAnalysis {
        family,
        confidence,
        confident_lock_reached_by_second_segment,
        mixed_script_detected,
        unstable,
        non_empty_segments,
        alphabetic_chars: cumulative.total(),
    }
}

fn is_latin(ch: char) -> bool {
    ch.is_ascii_alphabetic() || matches!(ch as u32, 0x00C0..=0x024F | 0x1E00..=0x1EFF)
}

fn is_cyrillic(ch: char) -> bool {
    matches!(ch as u32, 0x0400..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F | 0x1C80..=0x1C8F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dominant_latin_and_cyrillic_scripts() {
        assert_eq!(detect_language_family("hello deploy today"), LanguageFamily::Latin);
        assert_eq!(detect_language_family("привет как дела сегодня"), LanguageFamily::Cyrillic);
    }

    #[test]
    fn mixed_script_text_stays_unknown() {
        assert_eq!(detect_language_family("hello привет"), LanguageFamily::Unknown);
    }

    #[test]
    fn analysis_tracks_lock_and_instability() {
        let analysis = analyze_preview_segments(&[
            "hello world".to_string(),
            "deploy now".to_string(),
            "привет мир".to_string(),
        ]);

        assert_eq!(analysis.family, LanguageFamily::Latin);
        assert_eq!(analysis.confidence, LanguageLockConfidence::High);
        assert!(analysis.confident_lock_reached_by_second_segment);
        assert!(analysis.mixed_script_detected);
        assert!(analysis.unstable);
    }
}
