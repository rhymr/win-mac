use fancy_regex::Regex;
use std::sync::OnceLock;

/// Port of the CodeMirror web version's `countTotalSyllables` (see
/// `syllableCounter.js`) — a large set of regex heuristics layered on top of
/// each other rather than a real phonetic model. Kept as a faithful port
/// (including its quirks) so syllable counts match the web version.
struct SyllableRegexes {
    word_boundary: Regex,
    c_endings: Regex,
    c_beginnings: Regex,
    esylp: Regex,
    esylm: Regex,
    isylp: Regex,
    osylp: Regex,
    osylm: Regex,
    asylp: Regex,
    asylm: Regex,
    usylp: Regex,
    usylm: Regex,
    ysylp: Regex,
    ysylm: Regex,
    essuffix: Regex,
    edsuffix: Regex,
    csylp: Regex,
    e_vowels: Regex,
    nt_suffix: Regex,
    ent_suffix: Regex,
}

fn regexes() -> &'static SyllableRegexes {
    static REGEXES: OnceLock<SyllableRegexes> = OnceLock::new();
    REGEXES.get_or_init(|| SyllableRegexes {
        word_boundary: Regex::new(r"(?:\w-\w|[\w\u{C0}-\u{FF}'\u{2019}])+").expect("valid regex"),
        c_endings: Regex::new(r"(?mi)(?<=\w{3})(side|\wess|(?<!ed)ly|ment|ship|board|ground|(?<![^u]de)ville|port|ful(ly)?|berry|box|nesse?|such|m[ae]n|wom[ae]n|anne)s?$").expect("valid regex"),
        c_beginnings: Regex::new(r"(?mi)^(ware|side(?![sd]$)|p?re(?!ach|agan|al|au)|[rf]ace(?!([sd]|tte)$)|place[^nsd])").expect("valid regex"),
        esylp: Regex::new(r"(?mi)ie($|l|t|rg)|([cb]|tt|pp)le$|phe$|kle(s|$)|[^n]scien|sue|aybe$|[^aeiou]shed|[^lsoai]les$|([^e]r|g)ge$|(gg|ck|yw|etch)ed$|(sc|o)he$|seer|^re[eiuy]").expect("valid regex"),
        esylm: Regex::new(r"(?mi)every|some([^aeiouyr]|$)|[^trb]ere(?!d|$|o|r|t|a[^v]|n|s|x)|[^g]eous|niet").expect("valid regex"),
        isylp: Regex::new(r"(?mi)rie[^sndfvtl]|(?<=^|[^tcs]|st)ia|siai|[^ct]ious|quie|[lk]ier|settli|[^cn]ien[^d]|[aeio]ing$|dei[tf]|isms?$").expect("valid regex"),
        osylp: Regex::new(r"(?mi)nyo|osm(s$|$)|oinc|ored(?!$)|(^|[^ts])io|oale|[aeiou]yoe|^m[ia]cro([aiouy]|e)|roe(v|$)|ouel|^proa|oolog").expect("valid regex"),
        osylm: Regex::new(r"(?mi)[^f]ore(?!$|[vcaot]|d$|tte)|fore|llio").expect("valid regex"),
        asylp: Regex::new(r"(?mi)asm(s$|$)|ausea|oa$|anti[aeiou]|raor|intra[ou]|iae|ahe$|dais|(?<!p)ea(l(?!m)|$)|(?<!j)ean|(?<!il)eage").expect("valid regex"),
        asylm: Regex::new(r"(?mi)aste(?!$|ful|s$|r)|[^r]ared$").expect("valid regex"),
        usylp: Regex::new(r"(?mi)uo[^y]|[^gq]ua(?!r)|uen|[^g]iu|uis(?![aeiou]|se)|ou(et|ille)|eu(ing|er)|uye[dh]|nuine|ucle[aeiuy]").expect("valid regex"),
        usylm: Regex::new(r"(?mi)geous|busi|logu(?![ei])").expect("valid regex"),
        ysylp: Regex::new(r"(?mi)[ibcmrluhp]ya|nyac|[^e]yo|[aiou]y[aiou]|[aoruhm]ye(tt|l|n|v|z)|pye|dy[ae]|oye[exu]|lye[nlrs]|olye|aye(k|r|$|u[xr]|da)|saye\w|iye|wy[ae]|[^aiou]ying").expect("valid regex"),
        ysylm: Regex::new(r"(?mi)arley|key|ney$").expect("valid regex"),
        essuffix: Regex::new(r"(?mi)(?<!c[hrl]|sh|[iszxgej]|[niauery]c|do)es$").expect("valid regex"),
        edsuffix: Regex::new(r"(?mi)([aeiouy][^aeiouyrdt]|[^aeiouy][^laeiouyrdtbm]|ll|bb|ield|[ou]rb)ed$|[^cbda]red$").expect("valid regex"),
        csylp: Regex::new(r"(?mi)chn[^eai]|mc|thm").expect("valid regex"),
        e_vowels: Regex::new(r"(?mi)[aiouy](?![aeiouy])|ee|e(?!$|-|[iua])").expect("valid regex"),
        nt_suffix: Regex::new(r"(?i)[^aeiou]n['\u{2019}]t$").expect("valid regex"),
        ent_suffix: Regex::new(r"(?i)en['\u{2019}]t$").expect("valid regex"),
    })
}

fn count_matches(re: &Regex, text: &str) -> i64 {
    re.find_iter(text).filter(|m| m.is_ok()).count() as i64
}

/// Count syllables across a whole string (may contain several words/lines).
pub fn count_syllables(text: &str) -> u32 {
    let re = regexes();
    let with_words = convert_numbers_to_words(text.trim());
    let mut total: i64 = 0;

    for m in re.word_boundary.find_iter(&with_words) {
        let Ok(m) = m else { continue };
        let raw = m.as_str();
        if raw == "'" || raw == "\u{2019}" {
            continue;
        }
        if raw.chars().count() <= 2 {
            total += 1;
            continue;
        }

        let mut word = raw.to_string();
        let mut syllables: i64 = 0;

        if let Ok(Some(m)) = re.c_endings.find(&word) {
            let matched = m.as_str().to_string();
            word = word.replacen(&matched, &format!("\n{matched}"), 1);
        }
        if let Ok(Some(m)) = re.c_beginnings.find(&word) {
            let matched = m.as_str().to_string();
            word = word.replacen(&matched, "", 1);
            syllables += 1;
        }

        syllables += count_matches(&re.esylp, &word);
        syllables -= count_matches(&re.esylm, &word);
        syllables += count_matches(&re.isylp, &word);
        syllables += count_matches(&re.osylp, &word);
        syllables -= count_matches(&re.osylm, &word);
        syllables += count_matches(&re.asylp, &word);
        syllables -= count_matches(&re.asylm, &word);
        syllables += count_matches(&re.usylp, &word);
        syllables -= count_matches(&re.usylm, &word);
        syllables += count_matches(&re.ysylp, &word);
        syllables -= count_matches(&re.ysylm, &word);
        if re.essuffix.is_match(&word).unwrap_or(false) {
            syllables -= 1;
        }
        if re.edsuffix.is_match(&word).unwrap_or(false) {
            syllables -= 1;
        }
        syllables += count_matches(&re.csylp, &word);
        syllables += count_matches(&re.e_vowels, &word);

        if syllables <= 0 {
            syllables = 1;
        }
        if re.nt_suffix.is_match(&word).unwrap_or(false) {
            syllables += 1;
        }
        if re.ent_suffix.is_match(&word).unwrap_or(false) {
            syllables -= 1;
        }

        total += syllables;
    }

    total.max(0) as u32
}

/// Port of `wordsCount.js`'s `countTotalDocumentWords`: counts runs of word
/// characters (`\w`), i.e. transitions from non-word to word.
pub fn count_words(text: &str) -> u32 {
    let mut count = 0u32;
    let mut in_word = false;
    for c in text.chars() {
        let is_word_char = c.is_alphanumeric() || c == '_';
        if is_word_char && !in_word {
            count += 1;
        }
        in_word = is_word_char;
    }
    count
}

const ONES: [&str; 10] = [
    "", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];
const TENS: [&str; 10] = [
    "", "ten", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];
const TEENS: [&str; 9] = [
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

fn number_to_words(num: i64) -> String {
    if num == 0 {
        return "zero".to_string();
    }

    let mut num = num;
    let mut words = String::new();

    if num >= 1000 {
        words.push_str(&number_to_words(num / 1000));
        words.push_str(" thousand ");
        num %= 1000;
    }

    if num >= 100 {
        words.push_str(ONES[(num / 100) as usize]);
        words.push_str(" hundred ");
        num %= 100;
    }

    if num >= 20 {
        words.push_str(TENS[(num / 10) as usize]);
        words.push(' ');
        num %= 10;
    } else if num >= 11 {
        words.push_str(TEENS[(num - 11) as usize]);
        words.push(' ');
        return words;
    } else if num == 10 {
        words.push_str("ten ");
        return words;
    }

    words.push_str(ONES[num as usize]);
    words.push(' ');
    words
}

fn convert_numbers_to_words(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut digits = String::new();
            digits.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() {
                    digits.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            match digits.parse::<i64>() {
                Ok(n) => result.push_str(number_to_words(n).trim()),
                Err(_) => result.push_str(&digits),
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_to_words_never_panics_or_indexes_out_of_bounds() {
        for n in 0..100_000 {
            number_to_words(n);
        }
    }

    #[test]
    fn count_syllables_handles_three_digit_numbers_in_text() {
        // Regression test: `number_to_words` used to be missing hundreds-place
        // handling, so any 100-999 run of digits (e.g. a track number in a
        // lyric line) crashed the syllable gutter mid-render.
        assert!(count_syllables("100 Topics for BANNED IN AUSTRALIA") > 0);
        assert!(count_syllables("973") > 0);
    }
}
