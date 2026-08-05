use cmudict_fast::{Cmudict, Symbol};
use gtk::prelude::*;
use gtk::TextTag;
use hypher::Lang;
use rphonetic::{DoubleMetaphone, Encoder};
use sourceview5::Buffer as SourceBuffer;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::OnceLock;

/// How long to wait after the last keystroke before recomputing rhyme
/// groups — mirrors the autosave debounce in `text_editor.rs`.
const RHYME_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(400);

const CMUDICT_TXT: &str = include_str!("../../assets/dictionary/cmudict.dict");

/// Background colors cycled across rhyme groups, in the order groups first
/// appear in the document. Darker/more saturated than a foreground palette
/// would be, so the editor's light text (see `assets/styles/rhymr.xml`)
/// stays readable sitting on top of them. 24 evenly-spaced hues so real
/// documents (which can easily have 15-20+ distinct rhyme groups) mostly
/// get a unique color instead of two unrelated groups coincidentally
/// sharing one.
const PALETTE: [&str; 24] = [
    "#a32828", "#a34728", "#a36628", "#a38428", "#a3a328", "#84a328", "#66a328", "#47a328", "#28a328", "#28a347",
    "#28a366", "#28a384", "#28a3a3", "#2884a3", "#2865a3", "#2847a3", "#2828a3", "#4728a3", "#6528a3", "#8428a3",
    "#a328a3", "#a32884", "#a32866", "#a32847",
];

/// Common function/filler words excluded from rhyme matching — nearly every
/// document has *some* other word ending in the same sound as "of" or "is"
/// purely by coincidence of English phonology, which buries genuine rhymes
/// under noise instead of highlighting the writer's actual rhyme scheme.
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "of", "in", "on", "at", "to", "is", "it", "if", "as", "so", "no", "do", "be", "by", "or", "up",
    "we", "he", "she", "i", "my", "me", "you", "your", "am", "are", "was", "were", "been", "being", "and", "but",
    "for", "nor", "yet", "with", "from", "into", "onto", "than", "then", "this", "that", "these", "those", "there",
    "their", "they", "them", "its", "his", "her", "him", "our", "us", "oh", "ah", "uh", "well", "just", "not",
    "all", "any", "can", "could", "would", "should", "will", "shall", "may", "might", "must", "did", "does",
    "done", "had", "has", "have", "let", "get", "got", "go", "goes", "one", "two", "out", "off", "down", "over",
    "under", "again", "also", "too", "very", "much", "some", "such", "same", "own", "each", "every", "both", "few",
    "more", "most", "other", "only", "which", "who", "whom", "what", "when", "where", "why", "how",
];

fn stopwords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| STOPWORDS.iter().copied().collect())
}

fn cmudict() -> &'static Cmudict {
    static DICT: OnceLock<Cmudict> = OnceLock::new();
    DICT.get_or_init(|| Cmudict::from_str(CMUDICT_TXT).expect("bundled cmudict.dict should parse"))
}

fn symbol_base(symbol: &Symbol) -> &'static str {
    use Symbol::*;
    match symbol {
        AA(_) => "AA",
        AE(_) => "AE",
        AH(_) => "AH",
        AO(_) => "AO",
        AW(_) => "AW",
        AY(_) => "AY",
        B => "B",
        CH => "CH",
        D => "D",
        DH => "DH",
        EH(_) => "EH",
        ER(_) => "ER",
        EY(_) => "EY",
        F => "F",
        G => "G",
        HH => "HH",
        IH(_) => "IH",
        IY(_) => "IY",
        JH => "JH",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        NG => "NG",
        OW(_) => "OW",
        OY(_) => "OY",
        P => "P",
        R => "R",
        S => "S",
        SH => "SH",
        T => "T",
        TH => "TH",
        UH(_) => "UH",
        UW(_) => "UW",
        V => "V",
        W => "W",
        Y => "Y",
        Z => "Z",
        ZH => "ZH",
    }
}

/// Split a word's CMU pronunciation into syllables, each consisting of a
/// vowel (nucleus) plus every consonant up to — but not including — the
/// next vowel (its coda). Onset consonants (before the vowel) are
/// deliberately left out of each syllable: they never participate in a
/// rhyme, so excluding them here means the caller never has to worry about
/// stripping them back out of a rhyme key or a highlight span.
fn syllabify(pronunciation: &[Symbol]) -> Vec<Vec<Symbol>> {
    let vowel_positions: Vec<usize> =
        pronunciation.iter().enumerate().filter(|(_, s)| s.is_syllable()).map(|(i, _)| i).collect();

    if vowel_positions.is_empty() {
        return vec![pronunciation.to_vec()];
    }

    vowel_positions
        .iter()
        .enumerate()
        .map(|(i, &vowel_pos)| {
            let end = vowel_positions.get(i + 1).copied().unwrap_or(pronunciation.len());
            pronunciation[vowel_pos..end].to_vec()
        })
        .collect()
}

/// Letter-level syllable spans for `word` from Knuth–Liang hyphenation
/// patterns, used when they agree with `target_count` (the phonetic
/// syllable count). Hyphenation directly encodes English's actual
/// letter-break conventions — "can-dle", not "cand-le" — which the vowel-run
/// heuristic below gets wrong for syllabic-consonant endings like "-le",
/// "-en", "-er" (there's no written vowel to anchor on). Returns `None`
/// when hyphenation disagrees with the phonetic count (it's tuned to avoid
/// ugly typographic breaks, so it sometimes yields fewer pieces than there
/// are phonetic syllables — e.g. "closer" doesn't hyphenate to "clo-ser")
/// or when the word has characters hyphenation patterns aren't meant for
/// (an apostrophe); callers fall back to the vowel-run heuristic then.
fn hyphenation_spans(word: &str, target_count: usize) -> Option<Vec<(usize, usize)>> {
    if !word.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    let pieces: Vec<&str> = hypher::hyphenate(word, Lang::English).collect();
    if pieces.len() != target_count {
        return None;
    }

    let mut spans = Vec::with_capacity(pieces.len());
    let mut cursor = 0;
    for piece in pieces {
        let len = piece.chars().count();
        spans.push((cursor, cursor + len));
        cursor += len;
    }
    Some(spans)
}

/// The mirror image of `syllabify`, applied to spelling instead of
/// phonemes: split `word` into `target_count` character spans, each
/// starting at a run of vowel letters and extending to (not including) the
/// next one. Used as a fallback only when `hyphenation_spans` can't help
/// (see above) — it's cheaper but less accurate, since pairing "i-th
/// vowel-letter run" with "i-th phonetic syllable" doesn't know about
/// syllabic consonants, silent letters, etc. The merge/split logic below at
/// least keeps it from desyncing when a word's letter-vowel-run count
/// doesn't match its syllable count.
fn vowel_run_syllables(word: &str, target_count: usize) -> Vec<(usize, usize)> {
    let is_vowel = |c: char| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
    let char_count = word.chars().count();

    let mut vowel_run_starts = Vec::new();
    let mut prev_vowel = false;
    for (i, c) in word.chars().enumerate() {
        let v = is_vowel(c);
        if v && !prev_vowel {
            vowel_run_starts.push(i);
        }
        prev_vowel = v;
    }

    let mut spans: Vec<(usize, usize)> = if vowel_run_starts.is_empty() {
        vec![(0, char_count)]
    } else {
        vowel_run_starts
            .iter()
            .enumerate()
            .map(|(i, &s)| (s, vowel_run_starts.get(i + 1).copied().unwrap_or(char_count)))
            .collect()
    };

    // More vowel-letter runs than phonetic syllables (e.g. silent e in
    // "like"): fold the extra ones into the syllable before them.
    while spans.len() > target_count && spans.len() > 1 {
        let (_, last_end) = spans.pop().unwrap();
        spans.last_mut().unwrap().1 = last_end;
    }
    // Fewer vowel-letter runs than phonetic syllables (e.g. "our" spelled
    // with one letter-run but pronounced as two syllables): split the
    // trailing run so every syllable still gets a (possibly short) span.
    while spans.len() < target_count {
        let (s, e) = spans.pop().unwrap();
        let mid = s + (e - s) / 2;
        spans.push((s, mid));
        spans.push((mid, e));
    }

    spans
}

/// Letter spans for each of `word`'s `target_count` syllables — preferring
/// `hyphenation_spans`, falling back to `vowel_run_syllables`. Either way,
/// the first syllable's leading onset consonants (e.g. the "c" in "candle")
/// get trimmed off afterward: onset never participates in a rhyme, and
/// `vowel_run_syllables` already excludes it by construction, so this just
/// makes `hyphenation_spans` (which includes it, since a hyphenation piece
/// is a full orthographic syllable, onset and all) consistent with that.
fn orthographic_syllables(word: &str, target_count: usize) -> Vec<(usize, usize)> {
    let target_count = target_count.max(1);
    let mut spans = hyphenation_spans(word, target_count).unwrap_or_else(|| vowel_run_syllables(word, target_count));

    if let Some(first) = spans.first_mut() {
        let is_vowel = |c: char| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
        let onset_len = word.chars().skip(first.0).take(first.1 - first.0).take_while(|&c| !is_vowel(c)).count();
        first.0 += onset_len;
    }

    spans
}

/// One syllable's rhyme key plus the character span (into the word) it
/// covers.
#[derive(Debug)]
struct RhymeUnit {
    key: String,
    start: usize,
    end: usize,
}

/// Every rhymeable syllable in `word`, each independently comparable
/// against every other syllable in the document — so a multi-syllable word
/// can rhyme with different words on different syllables (e.g. "closer"
/// might share its first syllable with one word and its last with
/// another). Prefers the CMU pronouncing dictionary; falls back to a single
/// double-metaphone-keyed unit covering the word's last vowel-letter run
/// onward for words the dictionary doesn't know (proper nouns, slang) —
/// there's no phoneme data to syllabify in that case.
fn rhyme_units(word: &str) -> Vec<RhymeUnit> {
    let lower = word.to_lowercase();
    if !lower.chars().any(|c| c.is_alphabetic()) {
        return Vec::new();
    }

    if let Some(rule) = cmudict().get(&lower).and_then(|rules| rules.first()) {
        let syllables = syllabify(rule.pronunciation());
        let spans = orthographic_syllables(&lower, syllables.len());
        return syllables
            .iter()
            .zip(spans)
            .filter(|(_, (start, end))| end > start)
            .map(|(phonemes, (start, end))| RhymeUnit {
                key: phonemes.iter().map(symbol_base).collect::<Vec<_>>().join("-"),
                start,
                end,
            })
            .collect();
    }

    let is_vowel = |c: char| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
    let mut start = 0;
    let mut prev_vowel = false;
    for (i, c) in lower.chars().enumerate() {
        let v = is_vowel(c);
        if v && !prev_vowel {
            start = i;
        }
        prev_vowel = v;
    }
    let tail: String = lower.chars().skip(start).collect();
    if tail.is_empty() {
        return Vec::new();
    }
    vec![RhymeUnit { key: format!("dm:{}", DoubleMetaphone::default().encode(&tail)), start, end: lower.chars().count() }]
}

struct WordSpan {
    /// Character offsets (not byte offsets — `TextIter` counts characters).
    start: usize,
    end: usize,
    text: String,
}

fn tokenize(text: &str) -> Vec<WordSpan> {
    let mut spans = Vec::new();
    let mut start = None;
    let mut current = String::new();

    for (i, c) in text.chars().enumerate() {
        let is_word_char = c.is_alphabetic() || (c == '\'' && start.is_some());
        if is_word_char {
            if start.is_none() {
                start = Some(i);
            }
            current.push(c);
        } else if let Some(s) = start.take() {
            spans.push(WordSpan { start: s, end: i, text: std::mem::take(&mut current) });
        }
    }
    if let Some(s) = start.take() {
        spans.push(WordSpan { start: s, end: text.chars().count(), text: current });
    }

    spans
}

fn create_tags(buffer: &SourceBuffer) -> Vec<TextTag> {
    PALETTE
        .into_iter()
        .enumerate()
        .map(|(i, color)| {
            buffer
                .create_tag(Some(&format!("rhymr-rhyme-{i}")), &[("background", &color)])
                .expect("tag name is unique per buffer")
        })
        .collect()
}

fn recompute(buffer: &SourceBuffer, tags: &[TextTag]) {
    let start_iter = buffer.start_iter();
    let end_iter = buffer.end_iter();
    for tag in tags {
        buffer.remove_tag(tag, &start_iter, &end_iter);
    }

    let text = buffer.text(&start_iter, &end_iter, false).to_string();

    // Group word spans by rhyme key, preserving the order each key first
    // appears in the document so re-runs assign the same colors to the
    // same groups instead of reshuffling (unlike a plain HashMap, whose
    // iteration order is randomized per-process).
    let mut key_index: HashMap<String, usize> = HashMap::new();
    let mut groups: Vec<Vec<(usize, usize)>> = Vec::new();

    for span in &tokenize(&text) {
        if span.end - span.start < 2 {
            continue;
        }
        if stopwords().contains(span.text.to_lowercase().as_str()) {
            continue;
        }
        for unit in rhyme_units(&span.text) {
            let idx = *key_index.entry(unit.key).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[idx].push((span.start + unit.start, span.start + unit.end));
        }
    }

    let mut color_cursor = 0usize;
    for members in &groups {
        if members.len() < 2 {
            continue;
        }
        let tag = &tags[color_cursor % tags.len()];
        color_cursor += 1;
        for &(s, e) in members {
            let start = buffer.iter_at_offset(s as i32);
            let end = buffer.iter_at_offset(e as i32);
            buffer.apply_tag(tag, &start, &end);
        }
    }
}

/// Recolors words in `buffer` by rhyme group, recomputing (debounced) on
/// every edit.
pub fn attach(buffer: &SourceBuffer) {
    let tags = create_tags(buffer);
    recompute(buffer, &tags);

    let generation = Rc::new(Cell::new(0u64));
    buffer.connect_changed(move |buf| {
        let this_generation = generation.get() + 1;
        generation.set(this_generation);

        let generation_for_timeout = generation.clone();
        let buf_owned = buf.clone();
        let tags_for_timeout = tags.clone();
        glib::timeout_add_local_once(RHYME_DEBOUNCE, move || {
            if generation_for_timeout.get() != this_generation {
                return;
            }
            recompute(&buf_owned, &tags_for_timeout);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhyme_units_never_panics_or_produces_invalid_spans_across_the_dictionary() {
        for word in dictionary_sample() {
            for unit in rhyme_units(word) {
                assert!(unit.start <= unit.end, "{word:?}: unit {unit:?} has start > end");
                assert!(unit.end <= word.chars().count(), "{word:?}: unit {unit:?} runs past the word");
            }
        }
    }

    /// A large, deterministic sample of real dictionary words pulled
    /// straight out of the bundled `cmudict.dict` — cheaper than iterating
    /// all ~135k entries on every test run, but still broad enough to catch
    /// off-by-one bugs in the syllable/span reconciliation logic.
    fn dictionary_sample() -> Vec<&'static str> {
        CMUDICT_TXT.lines().filter_map(|line| line.split_whitespace().next()).step_by(7).collect()
    }

    #[test]
    fn orthographic_syllables_always_returns_target_count_spans_covering_no_more_than_the_word() {
        for word in dictionary_sample() {
            for target in 1..=4 {
                let spans = orthographic_syllables(word, target);
                assert_eq!(spans.len(), target, "{word:?} with target {target}");
                let char_count = word.chars().count();
                for &(s, e) in &spans {
                    assert!(s <= e && e <= char_count, "{word:?} target {target}: span ({s}, {e})");
                }
            }
        }
    }

    #[test]
    fn hyphenation_fixes_syllabic_consonant_endings() {
        // "candle" was the motivating failure case: CMU pronounces it as
        // two syllables (K AE1 N / D AH0 L), and the vowel-run-only
        // heuristic assigned the "l" to the wrong syllable since the
        // second syllable's nucleus is a syllabic /l/ with no written
        // vowel to anchor on — it produced ("andl", "e") instead of
        // ("can", "dle"). Hyphenation patterns encode the real
        // letter-break convention and get this right.
        let units = rhyme_units("candle");
        assert_eq!(units.len(), 2, "{units:?}");
        let word: Vec<char> = "candle".chars().collect();
        let text = |u: &RhymeUnit| word[u.start..u.end].iter().collect::<String>();
        assert_eq!(text(&units[0]), "an", "{units:?}");
        assert_eq!(text(&units[1]), "dle", "{units:?}");
    }

    #[test]
    fn multi_syllable_word_can_produce_multiple_independent_rhyme_units() {
        // "closer" is two syllables (CMU: K L OW1 Z ER0) — confirms a
        // multi-syllable word yields more than one independently-keyed unit
        // instead of collapsing to a single whole-word key.
        let units = rhyme_units("closer");
        assert!(units.len() >= 2, "expected multiple syllable units for \"closer\", got {units:?}");
    }
}
