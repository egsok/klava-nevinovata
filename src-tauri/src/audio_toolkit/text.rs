use natural::phonetics::soundex;
use once_cell::sync::Lazy;
use regex::Regex;
use strsim::levenshtein;

/// Builds an n-gram string by cleaning and concatenating words
///
/// Strips punctuation from each word, lowercases, and joins without spaces.
/// This allows matching "Charge B" against "ChargeBee".
fn build_ngram(words: &[&str]) -> String {
    words
        .iter()
        .map(|w| build_match_key(w))
        .collect::<Vec<_>>()
        .concat()
}

fn build_match_key(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

struct CustomWordMatchKey {
    word_index: usize,
    key: String,
}

fn build_custom_word_match_keys(word: &str, word_index: usize) -> Vec<CustomWordMatchKey> {
    let primary_key = build_match_key(word);
    let mut keys = Vec::with_capacity(2);

    // The fallback matcher is intentionally limited to ASCII terms. Its
    // whitespace tokenization and Soundex scoring are not suitable for CJK
    // scripts. Unicode custom words remain available to models that accept
    // them as native decode prompts; they are simply skipped by this fallback.
    if is_supported_fuzzy_key(&primary_key) {
        keys.push(CustomWordMatchKey {
            word_index,
            key: primary_key.clone(),
        });
    }

    if word.contains('&') {
        let expanded_key = build_match_key(&word.replace('&', " and "));
        if is_supported_fuzzy_key(&expanded_key) && expanded_key != primary_key {
            keys.push(CustomWordMatchKey {
                word_index,
                key: expanded_key,
            });
        }
    }

    keys
}

fn is_supported_fuzzy_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric())
}

fn supports_soundex(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_alphabetic())
}

/// Finds the best matching custom word for a candidate string
///
/// Uses Levenshtein distance and Soundex phonetic matching to find
/// the best match above the given threshold.
///
/// # Arguments
/// * `candidate` - The cleaned/lowercased candidate string to match
/// * `custom_words` - Original custom words (for returning the replacement)
/// * `custom_word_match_keys` - Normalized custom-word keys for comparison
/// * `threshold` - Maximum similarity score to accept
///
/// # Returns
/// The best matching custom word and its score, if any match was found
fn find_best_match<'a>(
    candidate: &str,
    custom_words: &'a [String],
    custom_word_match_keys: &[CustomWordMatchKey],
    threshold: f64,
) -> Option<(&'a String, f64)> {
    if !is_supported_fuzzy_key(candidate) || candidate.chars().count() > 50 {
        return None;
    }

    let mut best_match: Option<&String> = None;
    let mut best_score = f64::MAX;

    for custom_word_key in custom_word_match_keys {
        // Skip if lengths are too different (optimization + prevents over-matching)
        // Use percentage-based check: max 25% length difference (prevents n-grams from
        // matching significantly shorter custom words, e.g., "openaigpt" vs "openai")
        let candidate_len = candidate.chars().count();
        let custom_word_len = custom_word_key.key.chars().count();
        let len_diff = candidate_len.abs_diff(custom_word_len) as f64;
        let max_len = candidate_len.max(custom_word_len) as f64;
        let max_allowed_diff = (max_len * 0.25).max(2.0); // At least 2 chars difference allowed
        if len_diff > max_allowed_diff {
            continue;
        }

        // Calculate Levenshtein distance (normalized by length)
        let levenshtein_dist = levenshtein(candidate, &custom_word_key.key);
        let levenshtein_score = if max_len > 0.0 {
            levenshtein_dist as f64 / max_len
        } else {
            1.0
        };

        // Soundex is an English/ASCII phonetic algorithm. Numeric terms can
        // still use edit distance, but must not receive a phonetic boost.
        let phonetic_match = supports_soundex(candidate)
            && supports_soundex(&custom_word_key.key)
            && soundex(candidate, &custom_word_key.key);

        // Combine scores: favor phonetic matches, but also consider string similarity
        let combined_score = if phonetic_match {
            levenshtein_score * 0.3 // Give significant boost to phonetic matches
        } else {
            levenshtein_score
        };

        // Accept if the score is good enough (configurable threshold)
        if combined_score < threshold && combined_score < best_score {
            best_match = Some(&custom_words[custom_word_key.word_index]);
            best_score = combined_score;
        }
    }

    best_match.map(|m| (m, best_score))
}

/// Applies custom word corrections to transcribed text using fuzzy matching
///
/// This function corrects words in the input text by finding the best matches
/// from a list of custom words using a combination of:
/// - Levenshtein distance for string similarity
/// - Soundex phonetic matching for pronunciation similarity
/// - N-gram matching for multi-word speech artifacts (e.g., "Charge B" -> "ChargeBee")
///
/// # Arguments
/// * `text` - The input text to correct
/// * `custom_words` - List of custom words to match against
/// * `threshold` - Maximum similarity score to accept (0.0 = exact match, 1.0 = any match)
///
/// # Returns
/// The corrected text with custom words applied
pub fn apply_custom_words(text: &str, custom_words: &[String], threshold: f64) -> String {
    if custom_words.is_empty() {
        return text.to_string();
    }

    // Pre-compute normalized comparison keys to avoid repeated allocations.
    let custom_word_match_keys: Vec<CustomWordMatchKey> = custom_words
        .iter()
        .enumerate()
        .flat_map(|(index, word)| build_custom_word_match_keys(word, index))
        .collect();

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut best_match: Option<(usize, &String, f64)> = None;

        // Consider n-grams up to three words and choose the closest match. A
        // longest-first match can consume a following ordinary word when both
        // candidates happen to share a Soundex code (for example,
        // "Charge B, che" matching "ChargeBee").
        for n in (1..=3).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            // Do not consume across a punctuation boundary. In
            // "Charge B, che", the comma closes the candidate at "B,".
            if ngram_words[..n.saturating_sub(1)]
                .iter()
                .any(|word| !extract_punctuation(word).1.is_empty())
            {
                continue;
            }
            let ngram = build_ngram(ngram_words);

            if let Some((replacement, score)) =
                find_best_match(&ngram, custom_words, &custom_word_match_keys, threshold)
            {
                let is_better = best_match
                    .as_ref()
                    .is_none_or(|(_, _, best_score)| score < *best_score);
                if is_better {
                    best_match = Some((n, replacement, score));
                }
            }
        }

        if let Some((n, replacement, _)) = best_match {
            let ngram_words = &words[i..i + n];
            // Extract punctuation from first and last words of the n-gram.
            let (prefix, _) = extract_punctuation(ngram_words[0]);
            let (_, suffix) = extract_punctuation(ngram_words[n - 1]);

            // Preserve case from first word.
            let corrected = preserve_case_pattern(ngram_words[0], replacement);

            result.push(format!("{}{}{}", prefix, corrected, suffix));
            i += n;
        } else {
            result.push(words[i].to_string());
            i += 1;
        }
    }

    result.join(" ")
}

/// Preserves the case pattern of the original word when applying a replacement
fn preserve_case_pattern(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars: Vec<char> = replacement.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect()
    } else {
        replacement.to_string()
    }
}

/// Extracts punctuation prefix and suffix from a word
fn extract_punctuation(word: &str) -> (&str, &str) {
    // String slices use byte offsets. Derive both boundaries from char_indices
    // so multibyte punctuation such as `。` and `「」` can never be split.
    let prefix_end = word
        .char_indices()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(index, _)| index)
        .unwrap_or(word.len());
    let suffix_start = word
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(index, c)| index + c.len_utf8())
        .unwrap_or(0);

    let prefix = if prefix_end > 0 {
        &word[..prefix_end]
    } else {
        ""
    };

    let suffix = if suffix_start < word.len() {
        &word[suffix_start..]
    } else {
        ""
    };

    (prefix, suffix)
}

/// Evidence for the language of the text being cleaned.
///
/// This intentionally describes the transcription output, not Handy's UI
/// language. Unknown output languages fail closed: built-in filler removal is
/// skipped rather than applying a language profile speculatively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputLanguageEvidence {
    UserSelected(String),
    ModelConstrained(String),
    /// The transcription model itself identified the language (audio-based
    /// LID, e.g. Whisper in auto mode).
    ModelDetected(String),
    /// Detected from the transcribed text with high confidence, constrained to
    /// the model's supported languages. Weakest accepted evidence.
    TextDetected(String),
    TranslatedToEnglish,
    Unknown,
}

impl OutputLanguageEvidence {
    pub(crate) fn language(&self) -> Option<&str> {
        match self {
            Self::UserSelected(language)
            | Self::ModelConstrained(language)
            | Self::ModelDetected(language)
            | Self::TextDetected(language) => Some(language),
            Self::TranslatedToEnglish => Some("en"),
            Self::Unknown => None,
        }
    }
}

/// Filler tokens that are not lexical words in any language Handy's models can
/// output, so removing them cannot corrupt text regardless of the (possibly
/// unknown) output language. Kept deliberately conservative: anything that is a
/// real word somewhere ("um" pt/de, "ha" es, "ah"/"eh" interjections, "mm"
/// millimetres) belongs in the language-gated lists instead.
const UNIVERSAL_FILLER_WORDS: &[&str] = &[
    "uh", "uhm", "umm", "uhh", "uhhh", "ehh", "ehm", "ahm", "hmm", "hm", "mmm", "хм", "ммм",
];

/// Filler words that are only safe to remove with evidence for the output
/// language, because the same token is a real word elsewhere (e.g. Portuguese
/// "um" = "a/an", German "um" = "at/around", Spanish "ha" = "has").
fn gated_filler_words_for_language(lang: &str) -> &'static [&'static str] {
    let base_lang = lang.split(&['-', '_'][..]).next().unwrap_or(lang);

    match base_lang {
        "en" => &["um", "ah", "eh"],
        "de" => &["äh", "ähm"],
        "fr" => &["euh"],
        _ => &[],
    }
}

static MULTI_SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

// Sentence-boundary glue patterns produced by code-switch-trained Whisper
// models (notably Breeze ASR) on Cyrillic / mixed Cyrillic-Latin output.
// Order matters: period-glue patterns run before case-glue patterns so the
// inserted space doesn't trigger downstream matches on adjacent characters.
//
// Latin↔Latin glue is intentionally NOT covered — `iPhone`, `camelCase`,
// `app.NET` and similar legitimate constructs would be broken.

// "слово.Следующее" / "слово.Word" — Cyrillic lowercase, '.', then upper letter
// (any script) followed by a lowercase letter (any script). The trailing-
// lowercase guard rejects ".PDF", ".NET", ".A1".
static GLUE_PERIOD_FROM_CYR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([а-яё])\.([А-ЯЁA-Z][а-яёa-z])").unwrap());

// "предложение.Я" — Cyrillic lowercase, '.', then a single uppercase letter
// (Cyrillic or Latin) followed by a word boundary. Catches one-letter
// sentence-starters like Я, И, А, О, В, С, У, К in Russian (or English "A",
// "I" in code-switch). The `\b` requirement is what keeps file extensions
// like `.PDF` / `.NET` safe — there `\b` does NOT match between two letters.
static GLUE_PERIOD_FROM_CYR_SHORT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([а-яё])\.([А-ЯЁA-Z])\b").unwrap());

// "word.Привет" / "STT.Можем" — Latin letter (any case), '.', then Cyrillic
// upper+lower. Right side restricted to Cyrillic so all-Latin domains/versions
// stay intact ("app.NET", "PDF.NET", "API.SDK", "v1.2"). Uppercase left covers
// sentence-ending acronyms like "STT.Можем" / "API.Это" — common in code-switch
// ASR output.
static GLUE_PERIOD_FROM_LAT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([a-zA-Z])\.([А-ЯЁ][а-яё])").unwrap());

// "done.А" / "API.Я" — Latin letter (any case), '.', then a single Cyrillic
// uppercase followed by word boundary. Right side stays Cyrillic-only (single
// Latin uppercase after Latin period — like `app.A` — would create too many
// false positives in version strings / acronyms).
static GLUE_PERIOD_FROM_LAT_SHORT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([a-zA-Z])\.([А-ЯЁ])\b").unwrap());

// "словоСледующее" / "словоWord" — Cyrillic lowercase glued to any-script
// uppercase. Catches "словоUI" / "словоAPI" too, which is desired (these
// never appear glued in legitimate text).
static GLUE_CASE_FROM_CYR: Lazy<Regex> = Lazy::new(|| Regex::new(r"([а-яё])([А-ЯЁA-Z])").unwrap());

// "wordПривет" — Latin lowercase glued to Cyrillic uppercase. Right side
// restricted to Cyrillic so "iPhone", "camelCase", "OpenAI" stay intact.
static GLUE_CASE_FROM_LAT: Lazy<Regex> = Lazy::new(|| Regex::new(r"([a-z])([А-ЯЁ])").unwrap());

/// Insert spaces at sentence boundaries lost by code-switch-trained Whisper
/// models. Conservative: only fires on Cyrillic-involved patterns. Designed
/// for Breeze ASR Russian / mixed output.
pub fn fix_word_boundary_glue(text: &str) -> String {
    let s = GLUE_PERIOD_FROM_CYR.replace_all(text, "$1. $2");
    let s = GLUE_PERIOD_FROM_CYR_SHORT.replace_all(&s, "$1. $2");
    let s = GLUE_PERIOD_FROM_LAT.replace_all(&s, "$1. $2");
    let s = GLUE_PERIOD_FROM_LAT_SHORT.replace_all(&s, "$1. $2");
    let s = GLUE_CASE_FROM_CYR.replace_all(&s, "$1 $2");
    GLUE_CASE_FROM_LAT.replace_all(&s, "$1 $2").into_owned()
}

/// Collapses repeated words (3+ repetitions) to a single instance.
/// E.g., "wh wh wh wh" -> "wh", "I I I I" -> "I"
fn collapse_stutters(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut result: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let word_lower = word.to_lowercase();

        if word_lower.chars().all(|c| c.is_alphabetic()) {
            // Count consecutive repetitions (case-insensitive)
            let mut count = 1;
            while i + count < words.len() && words[i + count].to_lowercase() == word_lower {
                count += 1;
            }

            // If 3+ repetitions, collapse to single instance
            if count >= 3 {
                result.push(word);
                i += count;
            } else {
                result.push(word);
                i += 1;
            }
        } else {
            result.push(word);
            i += 1;
        }
    }

    result.join(" ")
}

/// Whether a word appended to `kept` would open a sentence: nothing but
/// whitespace so far, or the last visible character ends a sentence.
fn opens_sentence(kept: &str) -> bool {
    kept.trim_end()
        .chars()
        .next_back()
        .is_none_or(|c| matches!(c, '.' | '!' | '?' | '…'))
}

/// Appends `segment` to `kept`. While `capital_owed` is set, the first
/// alphanumeric character of `segment` is uppercased and the debt is settled.
fn push_restoring_capital(kept: &mut String, segment: &str, capital_owed: &mut bool) {
    if *capital_owed {
        if let Some((index, first)) = segment.char_indices().find(|(_, c)| c.is_alphanumeric()) {
            *capital_owed = false;
            kept.push_str(&segment[..index]);
            kept.extend(first.to_uppercase());
            kept.push_str(&segment[index + first.len_utf8()..]);
            return;
        }
    }
    kept.push_str(segment);
}

/// Deletes every match of one filler pattern. A capitalized filler that opened
/// a sentence hands its capital to the word that takes its place, so
/// "Um, so I think" becomes "So I think" rather than "so I think".
fn remove_filler_matches(text: &str, pattern: &Regex) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut resume = 0;
    let mut capital_owed = false;

    for filler in pattern.find_iter(text) {
        push_restoring_capital(&mut kept, &text[resume..filler.start()], &mut capital_owed);
        let capitalized = filler.as_str().starts_with(char::is_uppercase);
        capital_owed |= capitalized && opens_sentence(&kept);
        resume = filler.end();
    }
    push_restoring_capital(&mut kept, &text[resume..], &mut capital_owed);

    kept
}

/// Removes filler words from transcription output when enabled.
///
/// Built-in removal is two-tiered: [`UNIVERSAL_FILLER_WORDS`] apply regardless
/// of language evidence, while [`gated_filler_words_for_language`] tokens are
/// only removed when the output language is known. A custom list is an
/// explicit user override and replaces both tiers without requiring language
/// evidence. `Some(empty vec)` disables removal, preserving the legacy
/// power-user setting. The master toggle takes precedence over both built-in
/// and custom lists.
///
/// # Arguments
/// * `text` - The raw transcription text to filter
/// * `language` - Evidence for the language of the transcription output
/// * `custom_filler_words` - Optional user-provided filler word list. `Some(vec)` overrides
///   language defaults; `Some(empty vec)` disables filtering; `None` uses language defaults.
/// * `enabled` - Whether filler-word removal is enabled
///
/// # Returns
/// The text with configured filler words removed
pub fn remove_filler_words(
    text: &str,
    language: &OutputLanguageEvidence,
    custom_filler_words: &Option<Vec<String>>,
    enabled: bool,
) -> String {
    if !enabled {
        return text.to_string();
    }

    // Build filler patterns from custom list or the built-in tiers
    let patterns: Vec<Regex> = match custom_filler_words {
        Some(words) => words
            .iter()
            .filter_map(|word| Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).ok())
            .collect(),
        None => UNIVERSAL_FILLER_WORDS
            .iter()
            .chain(
                language
                    .language()
                    .map(gated_filler_words_for_language)
                    .unwrap_or_default(),
            )
            .map(|word| Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).unwrap())
            .collect(),
    };

    // Remove filler words
    let mut filtered = text.to_string();
    for pattern in &patterns {
        filtered = remove_filler_matches(&filtered, pattern);
    }

    filtered
}

/// Applies non-filler transcription cleanup.
///
/// Kept separate from [`remove_filler_words`] so disabling filler deletion
/// does not also disable the existing repeated-word and whitespace cleanup.
pub fn normalize_transcription_output(text: &str) -> String {
    let mut normalized = collapse_stutters(text);

    // Clean up multiple spaces to single space
    normalized = MULTI_SPACE_PATTERN
        .replace_all(&normalized, " ")
        .to_string();

    // Trim leading/trailing whitespace
    normalized.trim().to_string()
}

/// Splits text into sentence segments. Each segment is a raw slice of the
/// input (leading whitespace and the full run of trailing sentence-ending
/// punctuation included), so `segments.concat() == text`.
fn split_sentences(text: &str) -> Vec<&str> {
    const ENDERS: [char; 7] = ['.', '!', '?', '…', '。', '！', '？'];
    let mut segments = Vec::new();
    let mut last = 0;
    let mut in_ender_run = false;
    // Previous two chars, to spot initials ("А.Синецкая", "J.Doe"): a period
    // after a single uppercase letter is an abbreviation dot, not a sentence
    // boundary.
    let mut prev: Option<char> = None;
    let mut prev2: Option<char> = None;
    for (i, c) in text.char_indices() {
        let after_initial = c == '.'
            && prev.is_some_and(|p| p.is_alphabetic() && p.is_uppercase())
            && !prev2.is_some_and(|p| p.is_alphanumeric());
        prev2 = prev;
        prev = Some(c);
        if after_initial {
            continue;
        }
        if ENDERS.contains(&c) {
            in_ender_run = true;
        } else if in_ender_run {
            // First char after a punctuation run: close the segment there.
            if !text[last..i].trim().is_empty() {
                segments.push(&text[last..i]);
            }
            last = i;
            in_ender_run = false;
        }
    }
    if last < text.len() && !text[last..].trim().is_empty() {
        segments.push(&text[last..]);
    }
    segments
}

/// Normalizes a sentence for duplicate/blocklist comparison: trims, strips
/// leading/trailing punctuation (brackets kept — they are meaningful for
/// non-speech annotations), lowercases.
fn normalize_sentence(seg: &str) -> String {
    seg.trim()
        .trim_matches(|c: char| {
            matches!(
                c,
                '.' | '!' | '?' | '…' | '。' | '！' | '？' | ',' | '"' | '«' | '»' | '\''
            )
        })
        .trim()
        .to_lowercase()
}

/// Collapses whisper repetition-loop hallucinations: runs of identical
/// consecutive sentences. A run of 3+ collapses at any length (nobody dictates
/// the same sentence three times in a row); a run of exactly 2 collapses only
/// for long sentences (>= 12 chars normalized), so deliberate short doubles
/// like "Да. Да." survive.
pub fn remove_repeated_sentences(text: &str) -> String {
    let segments = split_sentences(text);
    if segments.len() < 2 {
        return text.to_string();
    }

    let normalized: Vec<String> = segments.iter().map(|s| normalize_sentence(s)).collect();
    let mut kept: Vec<&str> = Vec::with_capacity(segments.len());
    let mut i = 0;
    while i < segments.len() {
        let mut run = 1;
        while i + run < segments.len()
            && !normalized[i].is_empty()
            && normalized[i + run] == normalized[i]
        {
            run += 1;
        }
        let collapse = run >= 3 || (run == 2 && normalized[i].chars().count() >= 12);
        if collapse {
            kept.push(segments[i]);
            i += run;
        } else {
            for seg in &segments[i..i + run] {
                kept.push(seg);
            }
            i += run;
        }
    }
    kept.concat().trim().to_string()
}

// Known whisper hallucinations emitted on silence/noise — phrases the model
// memorized from its subtitle-heavy training data (large-v3 / turbo are the
// worst offenders in Russian). Matched against a full normalized sentence,
// never a substring, so real speech containing these words is safe.
static HALLUCINATED_SENTENCES: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"^продолжение следует$",
        r"^субтитры (сделал|делал|создал|создавал|сделаны|подготовил|подготовила) .{0,50}$",
        r"^редактор субтитров .{0,60}$",
        r"^корректор [а-яё]\.? ?[а-яё]{1,20}$",
        r"^спасибо за просмотр$",
        r"^thanks for watching$",
        r"^subtitles by .{0,50}$",
        // Bracketed non-speech annotations: "[музыка]", "(смех)", "[applause]".
        r"^[\[(].{0,30}[\])]$",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

/// Drops sentences that are known whisper silence-hallucinations
/// ("Продолжение следует...", subtitle credits, bracketed non-speech tags).
/// Whole-sentence match only.
pub fn remove_hallucinated_sentences(text: &str) -> String {
    let segments = split_sentences(text);
    if segments.is_empty() {
        return text.to_string();
    }
    let kept: Vec<&str> = segments
        .iter()
        .filter(|seg| {
            let norm = normalize_sentence(seg);
            norm.is_empty() || !HALLUCINATED_SENTENCES.iter().any(|re| re.is_match(&norm))
        })
        .copied()
        .collect();
    if kept.len() == segments.len() {
        return text.to_string();
    }
    kept.concat().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercise the complete cleanup sequence with an explicitly selected
    /// language. Individual tests below predate the split between filler
    /// removal and non-filler normalization.
    fn filter_transcription_output(
        text: &str,
        language: &str,
        custom_filler_words: &Option<Vec<String>>,
    ) -> String {
        let language = OutputLanguageEvidence::UserSelected(language.to_string());
        let filtered = remove_filler_words(text, &language, custom_filler_words, true);
        normalize_transcription_output(&filtered)
    }

    #[test]
    fn test_repeated_sentences_loop_collapses() {
        // Classic turbo repetition loop, short sentence, many repeats.
        let text = "Покажу. Покажу. Покажу. Покажу. Покажу.";
        assert_eq!(remove_repeated_sentences(text), "Покажу.");
    }

    #[test]
    fn test_repeated_sentences_long_pair_collapses() {
        let text = "Если нужно, сделаем. Если нужно, сделаем.";
        assert_eq!(remove_repeated_sentences(text), "Если нужно, сделаем.");
    }

    #[test]
    fn test_repeated_sentences_short_double_survives() {
        // Deliberate short double must NOT be collapsed.
        let text = "Да. Да.";
        assert_eq!(remove_repeated_sentences(text), "Да. Да.");
    }

    #[test]
    fn test_repeated_sentences_normal_text_untouched() {
        let text = "Первое предложение. Второе предложение. Третье.";
        assert_eq!(remove_repeated_sentences(text), text);
    }

    #[test]
    fn test_repeated_sentences_loop_mid_text() {
        let text = "Начало диктовки. Так. Так. Так. Так. И конец.";
        assert_eq!(
            remove_repeated_sentences(text),
            "Начало диктовки. Так. И конец."
        );
    }

    #[test]
    fn test_hallucinated_prodolzhenie_sleduet() {
        let text = "Это реальный текст. Продолжение следует...";
        assert_eq!(remove_hallucinated_sentences(text), "Это реальный текст.");
    }

    #[test]
    fn test_hallucinated_subtitle_credits() {
        let text = "Субтитры сделал DimaTorzok. Редактор субтитров А.Синецкая Корректор А.Егорова.";
        assert_eq!(remove_hallucinated_sentences(text), "");
    }

    #[test]
    fn test_hallucinated_only_full_sentence_matches() {
        // The phrase inside a longer real sentence must survive.
        let text = "Я думаю, что продолжение следует выпустить в марте.";
        assert_eq!(remove_hallucinated_sentences(text), text);
    }

    #[test]
    fn test_hallucinated_bracketed_annotation() {
        let text = "Реальная речь. [музыка]";
        assert_eq!(remove_hallucinated_sentences(text), "Реальная речь.");
    }

    #[test]
    fn test_hallucinated_pure_silence_transcript() {
        assert_eq!(remove_hallucinated_sentences("Спасибо за просмотр!"), "");
    }

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec!["hello".to_string(), "world".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_preserve_case_pattern() {
        assert_eq!(preserve_case_pattern("HELLO", "world"), "WORLD");
        assert_eq!(preserve_case_pattern("Hello", "world"), "World");
        assert_eq!(preserve_case_pattern("hello", "WORLD"), "WORLD");
    }

    #[test]
    fn test_extract_punctuation() {
        assert_eq!(extract_punctuation("hello"), ("", ""));
        assert_eq!(extract_punctuation("!hello?"), ("!", "?"));
        assert_eq!(extract_punctuation("...hello..."), ("...", "..."));
    }

    #[test]
    fn test_extract_punctuation_uses_unicode_boundaries() {
        assert_eq!(extract_punctuation("你好。"), ("", "。"));
        assert_eq!(extract_punctuation("「你好」"), ("「", "」"));
        assert_eq!(extract_punctuation("你好！"), ("", "！"));
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_filter_filler_words() {
        let text = "So uhm I was thinking uh about this";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "So I was thinking about this");
    }

    #[test]
    fn test_filter_filler_words_case_insensitive() {
        let text = "UHM this is UH a test";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "This is a test");
    }

    #[test]
    fn test_filter_filler_words_with_punctuation() {
        let text = "Well, uhm, I think, uh. that's right";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Well, I think, that's right");
    }

    #[test]
    fn test_filter_cleans_whitespace() {
        let text = "Hello    world   test";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Hello world test");
    }

    #[test]
    fn test_filter_trims() {
        let text = "  Hello world  ";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_filter_combined() {
        let text = "  Uhm, so I was, uh, thinking about this  ";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "So I was, thinking about this");
    }

    #[test]
    fn test_filter_leading_filler_keeps_sentence_capital() {
        let result = filter_transcription_output("Um, so I think we should ship it.", "en", &None);
        assert_eq!(result, "So I think we should ship it.");

        let result = filter_transcription_output("That works. Um, let me check.", "en", &None);
        assert_eq!(result, "That works. Let me check.");

        // Mid-sentence there is no capital to hand over.
        let result = filter_transcription_output("He said, Um, not today.", "en", &None);
        assert_eq!(result, "He said, not today.");
    }

    #[test]
    fn test_filter_preserves_valid_text() {
        let text = "This is a completely normal sentence.";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "This is a completely normal sentence.");
    }

    #[test]
    fn test_filter_stutter_collapse() {
        let text = "w wh wh wh wh wh wh wh wh wh why";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "w wh why");
    }

    #[test]
    fn test_filter_stutter_short_words() {
        let text = "I I I I think so so so so";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "I think so");
    }

    #[test]
    fn test_filter_stutter_longer_words() {
        let text = "Check data doc doc doc doc documentation.";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Check data doc documentation.");
    }

    #[test]
    fn test_filter_stutter_mixed_case() {
        let text = "No NO no NO no";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "No");
    }

    #[test]
    fn test_filter_stutter_preserves_two_repetitions() {
        let text = "no no is fine";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "no no is fine");
    }

    #[test]
    fn test_filter_english_removes_um() {
        let text = "um I think um this is good";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "I think this is good");
    }

    #[test]
    fn test_filter_portuguese_preserves_um() {
        // "um" means "a/an" in Portuguese
        let text = "um gato bonito";
        let result = filter_transcription_output(text, "pt", &None);
        assert_eq!(result, "um gato bonito");
    }

    #[test]
    fn test_filter_spanish_preserves_ha() {
        // "ha" means "has" in Spanish
        let text = "ha sido un buen día";
        let result = filter_transcription_output(text, "es", &None);
        assert_eq!(result, "ha sido un buen día");
    }

    #[test]
    fn test_filter_language_code_with_region() {
        // "pt-BR" should normalize to "pt"
        let text = "um gato bonito";
        let result = filter_transcription_output(text, "pt-BR", &None);
        assert_eq!(result, "um gato bonito");
    }

    #[test]
    fn test_filter_custom_filler_words_override() {
        let custom = Some(vec!["okay".to_string(), "right".to_string()]);
        let text = "okay so I think right this works";
        let result = filter_transcription_output(text, "en", &custom);
        assert_eq!(result, "so I think this works");
    }

    #[test]
    fn test_filter_custom_filler_words_empty_disables() {
        let custom = Some(vec![]);
        let text = "So uhm I was thinking uh about this";
        let result = filter_transcription_output(text, "en", &custom);
        // No filler words removed since custom list is empty
        assert_eq!(result, "So uhm I was thinking uh about this");
    }

    #[test]
    fn test_filter_unknown_language_still_removes_universal_fillers() {
        let text = "uh I think uhm this works";
        let result = filter_transcription_output(text, "xx", &None);
        assert_eq!(result, "I think this works");
    }

    #[test]
    fn test_filter_unknown_language_does_not_remove_um() {
        let text = "um I think this works";
        let result = filter_transcription_output(text, "xx", &None);
        assert_eq!(result, "um I think this works");
    }

    #[test]
    fn test_filter_unknown_evidence_removes_universal_keeps_gated() {
        let filtered = remove_filler_words(
            "uhh bueno hmm creo que um ha llegado",
            &OutputLanguageEvidence::Unknown,
            &None,
            true,
        );
        assert_eq!(
            normalize_transcription_output(&filtered),
            "bueno creo que um ha llegado"
        );

        let cyrillic = remove_filler_words(
            "хм я думаю ммм это работает",
            &OutputLanguageEvidence::Unknown,
            &None,
            true,
        );
        assert_eq!(
            normalize_transcription_output(&cyrillic),
            "я думаю это работает"
        );
    }

    #[test]
    fn test_filter_german_gated_fillers_require_evidence() {
        let text = "äh ich glaube ähm das passt";

        let unknown = remove_filler_words(text, &OutputLanguageEvidence::Unknown, &None, true);
        assert_eq!(normalize_transcription_output(&unknown), text);

        let result = filter_transcription_output(text, "de", &None);
        assert_eq!(result, "ich glaube das passt");
    }

    #[test]
    fn test_filter_preserves_millimetre_unit() {
        // "mm" was removed from the filler lists because it eats units.
        let text = "the screw is 5 mm long";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "the screw is 5 mm long");
    }

    #[test]
    fn test_filter_detected_evidence_unlocks_gated_fillers() {
        let model = remove_filler_words(
            "um I think this works",
            &OutputLanguageEvidence::ModelDetected("en".to_string()),
            &None,
            true,
        );
        assert_eq!(normalize_transcription_output(&model), "I think this works");

        let text = remove_filler_words(
            "euh je pense que ça marche",
            &OutputLanguageEvidence::TextDetected("fr".to_string()),
            &None,
            true,
        );
        assert_eq!(
            normalize_transcription_output(&text),
            "je pense que ça marche"
        );
    }

    #[test]
    fn test_filter_master_toggle_disables_custom_and_builtin_removal() {
        let text = "um customword I think";
        let language = OutputLanguageEvidence::UserSelected("en".to_string());
        let custom = Some(vec!["customword".to_string()]);

        let result = remove_filler_words(text, &language, &custom, false);

        assert_eq!(result, text);
    }

    #[test]
    fn test_filter_custom_words_apply_without_language_evidence() {
        let custom = Some(vec!["customword".to_string()]);
        let text = "customword should be removed but um should remain";

        let filtered = remove_filler_words(text, &OutputLanguageEvidence::Unknown, &custom, true);
        let result = normalize_transcription_output(&filtered);

        assert_eq!(result, "should be removed but um should remain");
    }

    #[test]
    fn test_apply_custom_words_ngram_two_words() {
        let text = "il cui nome è Charge B, che permette";
        let custom_words = vec!["ChargeBee".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("ChargeBee,"), "unexpected result: {result}");
        assert!(!result.contains("Charge B"));
    }

    #[test]
    fn test_apply_custom_words_ngram_three_words() {
        let text = "use Chat G P T for this";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("ChatGPT"));
    }

    #[test]
    fn test_apply_custom_words_prefers_longer_ngram() {
        let text = "Open AI GPT model";
        let custom_words = vec!["OpenAI".to_string(), "GPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "OpenAI GPT model");
    }

    #[test]
    fn test_apply_custom_words_ngram_preserves_case() {
        let text = "CHARGE B is great";
        let custom_words = vec!["ChargeBee".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("CHARGEBEE"));
    }

    #[test]
    fn test_apply_custom_words_ngram_with_spaces_in_custom() {
        // Custom word with space should also match against split words
        let text = "using Mac Book Pro";
        let custom_words = vec!["MacBook Pro".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "using MacBook Pro");
    }

    #[test]
    fn glue_cyr_period_cyr() {
        assert_eq!(fix_word_boundary_glue("привет.Мир и"), "привет. Мир и");
    }

    #[test]
    fn glue_cyr_period_lat() {
        assert_eq!(
            fix_word_boundary_glue("слово.Hello world"),
            "слово. Hello world"
        );
    }

    #[test]
    fn glue_lat_period_cyr() {
        assert_eq!(
            fix_word_boundary_glue("test.Привет всем"),
            "test. Привет всем"
        );
    }

    #[test]
    fn glue_cyr_case_cyr() {
        assert_eq!(fix_word_boundary_glue("приветМир"), "привет Мир");
    }

    #[test]
    fn glue_cyr_case_lat() {
        assert_eq!(fix_word_boundary_glue("словоWord"), "слово Word");
    }

    #[test]
    fn glue_lat_case_cyr() {
        assert_eq!(fix_word_boundary_glue("wordПривет"), "word Привет");
    }

    #[test]
    fn glue_preserves_file_extensions() {
        assert_eq!(fix_word_boundary_glue("файл.PDF готов"), "файл.PDF готов");
        assert_eq!(fix_word_boundary_glue("стек.NET"), "стек.NET");
    }

    #[test]
    fn glue_preserves_lat_camel_case() {
        assert_eq!(
            fix_word_boundary_glue("на iPhone и iPad"),
            "на iPhone и iPad"
        );
        assert_eq!(
            fix_word_boundary_glue("OpenAI и Anthropic"),
            "OpenAI и Anthropic"
        );
    }

    #[test]
    fn glue_preserves_lat_period_lat() {
        assert_eq!(fix_word_boundary_glue("версия app.NET"), "версия app.NET");
    }

    #[test]
    fn glue_preserves_lowercase_domains() {
        assert_eq!(
            fix_word_boundary_glue("сайтмосквы.рф открыт"),
            "сайтмосквы.рф открыт"
        );
    }

    #[test]
    fn glue_preserves_cyrillic_initials() {
        assert_eq!(
            fix_word_boundary_glue("И.О.Петров вошёл"),
            "И.О.Петров вошёл"
        );
    }

    #[test]
    fn glue_idempotent_on_clean_text() {
        let clean = "Это нормальный текст. Без проблем.";
        assert_eq!(fix_word_boundary_glue(clean), clean);
    }

    #[test]
    fn glue_cyr_period_single_letter_word() {
        // Regression: "предложение.Я думал" — single-letter Cyrillic word
        // after the period was previously missed because pattern 1a required
        // a trailing lowercase letter. Pattern 1b (\b) catches this.
        assert_eq!(
            fix_word_boundary_glue("предложение.Я думал"),
            "предложение. Я думал"
        );
        assert_eq!(
            fix_word_boundary_glue("работает.И завтра"),
            "работает. И завтра"
        );
        assert_eq!(fix_word_boundary_glue("вышло.А вот"), "вышло. А вот");
    }

    #[test]
    fn glue_cyr_period_single_letter_at_eol() {
        assert_eq!(fix_word_boundary_glue("конец.Я."), "конец. Я.");
    }

    #[test]
    fn glue_lat_period_single_cyr_letter() {
        assert_eq!(fix_word_boundary_glue("done.А завтра"), "done. А завтра");
    }

    #[test]
    fn glue_short_does_not_break_extensions() {
        // PDF/NET have multi-letter all-uppercase, so \b doesn't fire after
        // the first letter. Already covered by glue_preserves_file_extensions
        // but re-asserted here under the SHORT-pattern regression scope.
        assert_eq!(fix_word_boundary_glue("файл.PDF"), "файл.PDF");
        assert_eq!(fix_word_boundary_glue("стек.NET"), "стек.NET");
    }

    #[test]
    fn glue_uppercase_lat_period_cyr() {
        // Sentence ends with an uppercase-Latin acronym, next sentence starts
        // with a Cyrillic capital. Common in code-switch ASR output.
        assert_eq!(
            fix_word_boundary_glue("Google Chirp 3 STT.Можем ли"),
            "Google Chirp 3 STT. Можем ли"
        );
        assert_eq!(
            fix_word_boundary_glue("использую API.Это инструмент"),
            "использую API. Это инструмент"
        );
    }

    #[test]
    fn glue_uppercase_lat_period_single_cyr_letter() {
        // Same boundary, single-letter Cyrillic word right after the period.
        assert_eq!(
            fix_word_boundary_glue("через API.Я думал"),
            "через API. Я думал"
        );
    }

    #[test]
    fn glue_uppercase_lat_period_cyr_does_not_break_lat_lat() {
        // Right side is Latin → rule must not fire. Regression guard for the
        // [a-z]→[a-zA-Z] widening of GLUE_PERIOD_FROM_LAT{,_SHORT}.
        assert_eq!(fix_word_boundary_glue("PDF.NET"), "PDF.NET");
        assert_eq!(fix_word_boundary_glue("API.SDK"), "API.SDK");
    }

    #[test]
    fn test_apply_custom_words_trailing_number_not_doubled() {
        // Verify that trailing non-alpha chars (like numbers) aren't double-counted
        // between build_ngram stripping them and extract_punctuation capturing them
        let text = "use GPT4 for this";
        let custom_words = vec!["GPT-4".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // Should NOT produce "GPT-44" (double-counting the trailing 4)
        assert!(
            !result.contains("GPT-44"),
            "got double-counted result: {}",
            result
        );
    }

    #[test]
    fn test_apply_custom_words_matches_ampersand_word() {
        let text = "send it to RD for review";
        let custom_words = vec!["R&D".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "send it to R&D for review");
    }

    #[test]
    fn test_apply_custom_words_matches_spoken_ampersand_word() {
        let text = "send it to R and D for review";
        let custom_words = vec!["R&D".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "send it to R&D for review");
    }

    #[test]
    fn test_apply_custom_words_preserves_ampersand_word() {
        let text = "send it to R&D for review";
        let custom_words = vec!["R&D".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.18);
        assert_eq!(result, "send it to R&D for review");
    }

    #[test]
    fn test_apply_custom_words_handles_unicode_punctuation() {
        let text = "「Handee。」";
        let custom_words = vec!["Handy".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "「Handy。」");
    }

    #[test]
    fn test_apply_custom_words_skips_cjk_fuzzy_matching() {
        let text = "你好。";
        let custom_words = vec!["你号".to_string()];
        let result = apply_custom_words(text, &custom_words, 1.0);
        assert_eq!(result, text);
    }
}
