//! `say(text)`: a voice that reads a text, in English or in French.
//!
//! No recording and no model: the text becomes phonemes by spelling rules
//! (the NRL rules for English, after Elovitz et al., 1976, and rules of the
//! same kind for French, with a short list of exceptions for each), then the
//! phonemes become sound by formant synthesis, the way the first speaking
//! machines did it: a glottal pulse, or noise, through resonators set to the
//! resonances of the vocal tract, gliding from one sound to the next.
//!
//! The phonemes are written in a SAMPA-like notation, which a text can use
//! directly between slashes: `say("/h@'loU/")`.

use std::f64::consts::PI;
use std::sync::Arc;

use crate::sound::SR;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Fr,
}

impl Lang {
    pub fn parse(s: &str) -> Option<Lang> {
        match s {
            "en" | "english" => Some(Lang::En),
            "fr" | "french" | "français" | "francais" => Some(Lang::Fr),
            _ => None,
        }
    }
}

// ───────────────────────────── the phonemes ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Vowel,
    Glide,
    Liquid,
    Nasal,
    Fric,
    Stop,
    Affric,
    Aspirate,
    Pause,
}

#[derive(Clone, Copy, Debug)]
struct Ph {
    name: &'static str,
    class: Class,
    /// the first three formants (Hz), for an adult voice
    f: [f64; 3],
    /// where a diphthong glides to
    to: Option<[f64; 3]>,
    voiced: bool,
    /// noise: centre, width, loudness (fricatives, bursts)
    noise: (f64, f64, f64),
    nasal: bool,
    /// length (ms), before stress and speed
    ms: f64,
}

const fn v(name: &'static str, f: [f64; 3], ms: f64) -> Ph {
    Ph {
        name,
        class: Class::Vowel,
        f,
        to: None,
        voiced: true,
        noise: (0.0, 0.0, 0.0),
        nasal: false,
        ms,
    }
}
const fn dip(name: &'static str, f: [f64; 3], to: [f64; 3]) -> Ph {
    Ph {
        name,
        class: Class::Vowel,
        f,
        to: Some(to),
        voiced: true,
        noise: (0.0, 0.0, 0.0),
        nasal: false,
        ms: 170.0,
    }
}
const fn nv(name: &'static str, f: [f64; 3]) -> Ph {
    Ph {
        name,
        class: Class::Vowel,
        f,
        to: None,
        voiced: true,
        noise: (0.0, 0.0, 0.0),
        nasal: true,
        ms: 135.0,
    }
}
const fn son(name: &'static str, class: Class, f: [f64; 3], ms: f64) -> Ph {
    Ph {
        name,
        class,
        f,
        to: None,
        voiced: true,
        noise: (0.0, 0.0, 0.0),
        nasal: matches!(class, Class::Nasal),
        ms,
    }
}
const fn fr(name: &'static str, f: [f64; 3], voiced: bool, noise: (f64, f64, f64), ms: f64) -> Ph {
    Ph {
        name,
        class: Class::Fric,
        f,
        to: None,
        voiced,
        noise,
        nasal: false,
        ms,
    }
}
const fn st(name: &'static str, f: [f64; 3], voiced: bool, noise: (f64, f64, f64)) -> Ph {
    Ph {
        name,
        class: Class::Stop,
        f,
        to: None,
        voiced,
        noise,
        nasal: false,
        ms: 70.0,
    }
}

const PHONEMES: &[Ph] = &[
    // vowels
    v("i", [280.0, 2250.0, 2950.0], 120.0),
    v("I", [400.0, 1950.0, 2550.0], 85.0),
    v("e", [380.0, 2150.0, 2750.0], 110.0),
    v("E", [540.0, 1800.0, 2500.0], 100.0),
    v("&", [680.0, 1700.0, 2400.0], 125.0),
    v("a", [720.0, 1350.0, 2450.0], 115.0),
    v("A", [720.0, 1100.0, 2450.0], 130.0),
    v("O", [560.0, 900.0, 2400.0], 125.0),
    v("o", [400.0, 800.0, 2350.0], 115.0),
    v("U", [450.0, 1050.0, 2250.0], 85.0),
    v("u", [310.0, 900.0, 2250.0], 120.0),
    v("V", [620.0, 1200.0, 2450.0], 90.0),
    v("@", [500.0, 1450.0, 2450.0], 55.0),
    v("3", [480.0, 1300.0, 1650.0], 130.0),
    v("y", [280.0, 1800.0, 2250.0], 115.0),
    v("2", [380.0, 1450.0, 2250.0], 120.0),
    v("9", [520.0, 1450.0, 2400.0], 115.0),
    nv("a~", [680.0, 1100.0, 2500.0]),
    nv("e~", [600.0, 1600.0, 2500.0]),
    nv("o~", [460.0, 850.0, 2400.0]),
    nv("9~", [560.0, 1500.0, 2450.0]),
    dip("aI", [720.0, 1250.0, 2450.0], [380.0, 2050.0, 2600.0]),
    dip("aU", [720.0, 1200.0, 2450.0], [420.0, 900.0, 2300.0]),
    dip("OI", [560.0, 900.0, 2400.0], [380.0, 2050.0, 2600.0]),
    dip("eI", [480.0, 1950.0, 2600.0], [330.0, 2200.0, 2850.0]),
    dip("oU", [500.0, 950.0, 2400.0], [360.0, 850.0, 2300.0]),
    // glides and liquids
    son("w", Class::Glide, [300.0, 650.0, 2200.0], 60.0),
    son("j", Class::Glide, [270.0, 2100.0, 3000.0], 55.0),
    son("H", Class::Glide, [270.0, 1750.0, 2250.0], 55.0),
    son("l", Class::Liquid, [330.0, 1050.0, 2800.0], 65.0),
    son("r", Class::Liquid, [330.0, 1100.0, 1450.0], 65.0),
    // nasals: the murmur, low and dull
    son("m", Class::Nasal, [280.0, 1000.0, 2200.0], 70.0),
    son("n", Class::Nasal, [280.0, 1600.0, 2550.0], 65.0),
    son("N", Class::Nasal, [280.0, 1900.0, 2700.0], 75.0),
    son("J", Class::Nasal, [280.0, 2100.0, 2800.0], 75.0),
    // fricatives
    fr(
        "f",
        [320.0, 1000.0, 2300.0],
        false,
        (7000.0, 5000.0, 0.025),
        95.0,
    ),
    fr(
        "v",
        [320.0, 1000.0, 2300.0],
        true,
        (7000.0, 5000.0, 0.012),
        65.0,
    ),
    fr(
        "T",
        [320.0, 1500.0, 2600.0],
        false,
        (6500.0, 4000.0, 0.02),
        95.0,
    ),
    fr(
        "D",
        [320.0, 1500.0, 2600.0],
        true,
        (6500.0, 4000.0, 0.01),
        50.0,
    ),
    fr(
        "s",
        [320.0, 1600.0, 2600.0],
        false,
        (5800.0, 1800.0, 0.1),
        110.0,
    ),
    fr(
        "z",
        [320.0, 1600.0, 2600.0],
        true,
        (5800.0, 1800.0, 0.05),
        80.0,
    ),
    fr(
        "S",
        [320.0, 1900.0, 2600.0],
        false,
        (3000.0, 1500.0, 0.22),
        115.0,
    ),
    fr(
        "Z",
        [320.0, 1900.0, 2600.0],
        true,
        (3000.0, 1500.0, 0.11),
        80.0,
    ),
    fr(
        "R",
        [520.0, 1250.0, 2400.0],
        true,
        (1500.0, 1200.0, 0.03),
        60.0,
    ),
    Ph {
        name: "h",
        class: Class::Aspirate,
        f: [500.0, 1500.0, 2500.0],
        to: None,
        voiced: false,
        noise: (0.0, 0.0, 0.0),
        nasal: false,
        ms: 65.0,
    },
    // stops: the formants are where the vowels glide from
    st("p", [250.0, 800.0, 2200.0], false, (1200.0, 1600.0, 0.25)),
    st("b", [250.0, 800.0, 2200.0], true, (1200.0, 1600.0, 0.17)),
    st("t", [250.0, 1750.0, 2650.0], false, (4200.0, 2400.0, 0.3)),
    st("d", [250.0, 1750.0, 2650.0], true, (4200.0, 2400.0, 0.2)),
    st("k", [250.0, 1900.0, 2400.0], false, (2200.0, 1200.0, 0.3)),
    st("g", [250.0, 1900.0, 2400.0], true, (2200.0, 1200.0, 0.2)),
    Ph {
        name: "tS",
        class: Class::Affric,
        f: [300.0, 1900.0, 2600.0],
        to: None,
        voiced: false,
        noise: (3000.0, 1500.0, 0.22),
        nasal: false,
        ms: 120.0,
    },
    Ph {
        name: "dZ",
        class: Class::Affric,
        f: [300.0, 1900.0, 2600.0],
        to: None,
        voiced: true,
        noise: (3000.0, 1500.0, 0.11),
        nasal: false,
        ms: 100.0,
    },
    Ph {
        name: "_",
        class: Class::Pause,
        f: [500.0, 1500.0, 2500.0],
        to: None,
        voiced: false,
        noise: (0.0, 0.0, 0.0),
        nasal: false,
        ms: 0.0,
    },
];

fn ph(name: &str) -> Option<usize> {
    PHONEMES.iter().position(|p| p.name == name)
}

/// Phonemes written in the notation, possibly with `'` (the next vowel is
/// stressed) and `,` (a lighter stress). Unknown characters are skipped.
fn parse_phonemes(s: &str) -> Vec<(usize, u8)> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut stress = 0u8;
    let mut k = 0;
    while k < chars.len() {
        match chars[k] {
            '\'' | 'ˈ' => {
                stress = 2;
                k += 1;
                continue;
            }
            ',' | 'ˌ' => {
                stress = 1;
                k += 1;
                continue;
            }
            _ => {}
        }
        let mut found = None;
        for len in (1..=2).rev() {
            if k + len <= chars.len() {
                let tok: String = chars[k..k + len].iter().collect();
                if let Some(p) = ph(&tok) {
                    found = Some((p, len));
                    break;
                }
            }
        }
        match found {
            Some((p, len)) => {
                let s = if PHONEMES[p].class == Class::Vowel {
                    std::mem::take(&mut stress)
                } else {
                    0
                };
                out.push((p, s));
                k += len;
            }
            None => k += 1,
        }
    }
    out
}

// ───────────────────────────── spelling rules ─────────────────────────────

/// One rule: in `left` and before `right`, `matched` sounds as `out`.
type Rule = (&'static str, &'static str, &'static str, &'static str);

trait Letters {
    fn vowel(c: char) -> bool;
}

struct EnLetters;
impl Letters for EnLetters {
    fn vowel(c: char) -> bool {
        matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
    }
}
struct FrLetters;
impl Letters for FrLetters {
    fn vowel(c: char) -> bool {
        matches!(
            c,
            'a' | 'e'
                | 'i'
                | 'o'
                | 'u'
                | 'y'
                | 'é'
                | 'è'
                | 'ê'
                | 'ë'
                | 'à'
                | 'â'
                | 'î'
                | 'ï'
                | 'ô'
                | 'û'
                | 'ù'
                | 'ü'
                | 'œ'
                | 'ÿ'
        )
    }
}

fn letter(c: char) -> bool {
    c.is_alphabetic()
}
fn consonant<L: Letters>(c: char) -> bool {
    letter(c) && !L::vowel(c)
}

/// Does the context `pat` hold to the right of `pos` (forward = true) or to
/// the left of it (forward = false, `pos` being the first letter before)?
fn context<L: Letters>(text: &[char], mut pos: isize, pat: &str, forward: bool) -> bool {
    let pat: Vec<char> = if forward {
        pat.chars().collect()
    } else {
        pat.chars().rev().collect()
    };
    let step: isize = if forward { 1 } else { -1 };
    let at = |p: isize| -> char {
        if p < 0 || p as usize >= text.len() {
            ' '
        } else {
            text[p as usize]
        }
    };
    for &c in &pat {
        match c {
            ' ' => {
                if letter(at(pos)) {
                    return false;
                }
                pos += step;
            }
            '#' => {
                if !L::vowel(at(pos)) {
                    return false;
                }
                while L::vowel(at(pos)) {
                    pos += step;
                }
            }
            ':' => {
                while consonant::<L>(at(pos)) {
                    pos += step;
                }
            }
            '*' => {
                if !consonant::<L>(at(pos)) {
                    return false;
                }
                while consonant::<L>(at(pos)) {
                    pos += step;
                }
            }
            '^' => {
                if !consonant::<L>(at(pos)) {
                    return false;
                }
                pos += step;
            }
            '.' => {
                if !matches!(
                    at(pos),
                    'b' | 'd' | 'v' | 'g' | 'j' | 'l' | 'm' | 'n' | 'r' | 'w' | 'z'
                ) {
                    return false;
                }
                pos += step;
            }
            '+' => {
                if !matches!(at(pos), 'e' | 'i' | 'y' | 'é' | 'è' | 'ê') {
                    return false;
                }
                pos += step;
            }
            // a nasal vowel's end: a consonant other than n and m, or the end
            '~' => {
                let c = at(pos);
                if L::vowel(c) || c == 'n' || c == 'm' || c == 'h' {
                    return false;
                }
                pos += step;
            }
            '%' => {
                // a suffix (right only): er, e, es, ed, ing, ely
                let s = |p: isize, w: &str| {
                    w.chars()
                        .enumerate()
                        .all(|(k, ch)| at(p + k as isize) == ch)
                };
                if s(pos, "ing") {
                    pos += 3;
                } else if s(pos, "ely") {
                    pos += 3;
                } else if s(pos, "er") || s(pos, "es") || s(pos, "ed") {
                    pos += 2;
                } else if at(pos) == 'e' {
                    pos += 1;
                } else {
                    return false;
                }
            }
            '&' => {
                let c = at(pos);
                if matches!(c, 's' | 'c' | 'g' | 'z' | 'x' | 'j') {
                    // ch, sh read backwards or forwards
                    pos += step;
                } else {
                    return false;
                }
            }
            '@' => {
                if !matches!(at(pos), 't' | 's' | 'r' | 'd' | 'l' | 'z' | 'n' | 'j') {
                    return false;
                }
                pos += step;
            }
            lit => {
                if at(pos) != lit {
                    return false;
                }
                pos += step;
            }
        }
    }
    true
}

/// Applies the rules to a word (lowercase letters, and `'`): the phonemes, in
/// the rules' own notation.
fn apply<L: Letters>(word: &str, rules: &[Rule]) -> String {
    let text: Vec<char> = word.chars().collect();
    let mut out = String::new();
    let mut k = 0;
    while k < text.len() {
        let mut done = false;
        for &(left, m, right, o) in rules {
            let mc: Vec<char> = m.chars().collect();
            if k + mc.len() > text.len() || text[k..k + mc.len()] != mc[..] {
                continue;
            }
            if !context::<L>(&text, k as isize - 1, left, false)
                || !context::<L>(&text, (k + mc.len()) as isize, right, true)
            {
                continue;
            }
            out.push_str(o);
            out.push(' ');
            k += mc.len();
            done = true;
            break;
        }
        if !done {
            k += 1;
        }
    }
    out
}

// The NRL rules (Elovitz, Johnson, McHugh and Shore, 1976), in their own
// notation: IY IH EY EH AE AA AO OW UH UW ER AX AH AY AW OY, and the
// consonants p b t d k g f v TH DH s z SH ZH HH m n NG l w y r CH j WH.
// " " is the edge of the word, "" anything.
#[rustfmt::skip]
const EN_RULES: &[Rule] = &[
    // A
    ("", "a", " ", "AX"), (" ", "are", " ", "AA r"), (" ", "ar", "o", "AX r"), ("", "ar", "#", "EH r"),
    ("^", "as", "#", "EY s"), ("", "a", "wa", "AX"), ("", "aw", "", "AO"), (" :", "any", "", "EH n IY"),
    ("", "a", "^+#", "EY"), ("#:", "ally", "", "AX l IY"), (" ", "al", "#", "AX l"), ("", "again", "", "AX g EH n"),
    ("#:", "ag", "e", "IH j"), ("", "a", "^+:#", "AE"), (" :", "a", "^+ ", "EY"), ("", "a", "^%", "EY"),
    (" ", "arr", "", "AX r"), ("", "arr", "", "AE r"), (" :", "ar", " ", "AA r"), ("", "ar", " ", "ER"),
    ("", "ar", "", "AA r"), ("", "air", "", "EH r"), ("", "ai", "", "EY"), ("", "ay", "", "EY"), ("", "au", "", "AO"),
    ("#:", "al", " ", "AX l"), ("#:", "als", " ", "AX l z"), ("", "alk", "", "AO k"), ("", "al", "^", "AO l"),
    (" :", "able", "", "EY b AX l"), ("", "able", "", "AX b AX l"), ("", "ang", "+", "EY n j"), ("", "a", "", "AE"),
    // B
    (" ", "be", "^#", "b IH"), ("", "being", "", "b IY IH NG"), (" ", "both", " ", "b OW TH"), (" ", "bus", "#", "b IH z"),
    ("", "buil", "", "b IH l"), ("", "bb", "", "b"), ("", "b", "", "b"),
    // C
    (" ", "ch", "^", "k"), ("^e", "ch", "", "k"), ("", "ch", "", "CH"), (" s", "ci", "#", "s AY"), ("", "ci", "a", "SH"),
    ("", "ci", "o", "SH"), ("", "ci", "en", "SH"), ("", "c", "+", "s"), ("", "ck", "", "k"), ("", "cc", "+", "k s"), ("", "cc", "", "k"), ("", "com", "%", "k AH m"),
    ("", "c", "", "k"),
    // D
    ("#:", "ded", " ", "d IH d"), (".e", "d", " ", "d"), ("#:^e", "d", " ", "t"), (" ", "de", "^#", "d IH"),
    (" ", "do", " ", "d UW"), (" ", "does", "", "d AH z"), (" ", "doing", "", "d UW IH NG"), (" ", "dow", "", "d AW"),
    ("", "du", "a", "j UW"), ("", "dg", "", "j"), ("", "dd", "", "d"), ("", "d", "", "d"),
    // E
    ("#:", "e", " ", ""), ("'^:", "e", " ", ""), (" :", "e", " ", "IY"), ("#", "ed", " ", "d"), ("y", "ed", " ", "d"), ("#:", "e", "d ", ""),
    ("", "ev", "er", "EH v"), ("", "e", "^%", "IY"), ("", "eri", "#", "IY r IY"), ("", "eri", "", "EH r IH"),
    ("#:", "er", "#", "ER"), ("", "er", "#", "EH r"), ("", "er", "", "ER"), (" ", "even", "", "IY v EH n"),
    ("#:", "e", "w", ""), ("t", "ew", "", "UW"), ("s", "ew", "", "UW"), ("r", "ew", "", "UW"), ("d", "ew", "", "UW"),
    ("l", "ew", "", "UW"), ("z", "ew", "", "UW"), ("n", "ew", "", "UW"), ("j", "ew", "", "UW"), ("th", "ew", "", "UW"),
    ("ch", "ew", "", "UW"), ("sh", "ew", "", "UW"), ("", "ew", "", "y UW"), ("", "e", "o", "IY"),
    ("#:s", "es", " ", "IH z"), ("#:c", "es", " ", "IH z"), ("#:g", "es", " ", "IH z"), ("#:z", "es", " ", "IH z"),
    ("#:x", "es", " ", "IH z"), ("#:j", "es", " ", "IH z"), ("#:ch", "es", " ", "IH z"), ("#:sh", "es", " ", "IH z"),
    ("#:", "e", "s ", ""), ("#:", "ely", " ", "l IY"), ("#:", "ement", "", "m EH n t"), ("", "eful", "", "f UH l"),
    ("", "ee", "", "IY"), ("", "earn", "", "ER n"), (" ", "ear", "^", "ER"), ("", "ead", "", "EH d"), ("#:", "ea", " ", "IY AX"),
    ("", "ea", "su", "EH"), ("", "ea", "", "IY"), ("", "eigh", "", "EY"), ("", "ei", "", "IY"), (" ", "eye", "", "AY"),
    ("", "ey", "", "IY"), ("", "eu", "", "y UW"), ("", "e", "", "EH"),
    // F
    ("", "ful", "", "f UH l"), ("", "ff", "", "f"), ("", "f", "", "f"),
    // G
    ("", "giv", "", "g IH v"), (" ", "g", "i^", "g"), ("", "ge", "t", "g EH"), ("su", "gges", "", "g j EH s"), ("", "gg", "", "g"),
    (" b#", "g", "", "g"), ("", "g", "+", "j"), ("", "great", "", "g r EY t"), ("#", "gh", "", ""), ("", "g", "", "g"),
    // H
    (" ", "hav", "", "HH AE v"), (" ", "here", "", "HH IY r"), (" ", "hour", "", "AW ER"), ("", "how", "", "HH AW"),
    ("", "h", "#", "HH"), ("", "h", "", ""),
    // I
    (" ", "in", "", "IH n"), (" ", "i", " ", "AY"), ("", "in", "d", "AY n"), ("", "ier", "", "IY ER"), ("#:r", "ied", "", "IY d"),
    ("", "ied", " ", "AY d"), ("", "ien", "", "IY EH n"), ("", "ie", "t", "AY EH"), ("", "ie", "^", "IY"), (" :", "i", "%", "AY"), ("", "i", "%", "IY"),
    ("", "ie", "", "IY"), ("", "i", "^+:#", "IH"), ("", "ir", "#", "AY r"), ("", "iz", "%", "AY z"), ("", "is", "%", "AY z"),
    ("", "i", "d%", "AY"), ("+^", "i", "^+", "IH"), ("", "i", "t%", "AY"), ("#:^", "i", "^+", "IH"), ("", "i", "^+", "AY"),
    ("", "ir", "", "ER"), ("", "igh", "", "AY"), ("", "ild", "", "AY l d"), ("", "ign", " ", "AY n"), ("", "ign", "^", "AY n"),
    ("", "ign", "%", "AY n"), ("", "ique", "", "IY k"), ("", "i", "", "IH"),
    // J K L M
    ("", "j", "", "j"), (" ", "k", "n", ""), ("", "k", "", "k"),
    ("", "lo", "c#", "l OW"), ("l", "l", "", ""), ("#:^", "l", "%", "AX l"), ("", "lead", "", "l IY d"), ("", "l", "", "l"),
    ("", "mov", "", "m UW v"), ("", "mm", "", "m"), ("", "m", "", "m"),
    // N
    ("e", "ng", "+", "n j"), ("", "ng", "r", "NG g"), ("", "ng", "#", "NG g"), ("", "ngl", "%", "NG g AX l"), ("", "ng", "", "NG"),
    ("", "nk", "", "NG k"), (" ", "now", " ", "n AW"), ("", "nn", "", "n"), ("", "n", "", "n"),
    // O
    ("", "of", " ", "AX v"), ("", "orough", "", "ER OW"), ("#:", "or", " ", "ER"), ("#:", "ors", " ", "ER z"), ("", "or", "", "AO r"),
    (" ", "one", "", "w AH n"), ("", "ow", "", "OW"), (" ", "over", "", "OW v ER"), ("", "ov", "", "AH v"), ("", "o", "^%", "OW"),
    ("", "o", "^en", "OW"), ("", "o", "^i#", "OW"), ("", "ol", "d", "OW l"), ("", "ought", "", "AO t"), ("", "ough", "", "AH f"),
    (" ", "ou", "", "AW"), ("h", "ou", "s#", "AW"), ("", "ous", "", "AX s"), ("", "our", "", "AO r"), ("", "ould", "", "UH d"),
    ("^", "ou", "^l", "AH"), ("", "oup", "", "UW p"), ("", "ou", "", "AW"), ("", "oy", "", "OY"), ("", "oing", "", "OW IH NG"),
    ("", "oi", "", "OY"), ("", "oor", "", "AO r"), ("", "ook", "", "UH k"), ("", "ood", "", "UH d"), ("", "oo", "", "UW"),
    ("", "o", "e", "OW"), ("", "o", " ", "OW"), ("", "oa", "", "OW"), (" ", "only", "", "OW n l IY"), (" ", "once", "", "w AH n s"),
    ("", "on't", "", "OW n t"), ("c", "o", "n", "AA"), ("", "o", "ng", "AO"), (" :^", "o", "n", "AH"), ("i", "on", "", "AX n"),
    ("#:", "on", " ", "AX n"), ("#^", "on", "", "AX n"), ("", "o", "st ", "OW"), ("", "off", "", "AO f"), ("", "of", "^", "AO f"), ("", "other", "", "AH DH ER"),
    ("", "oss", " ", "AO s"), ("#:^", "om", "", "AH m"), ("", "o", "", "AA"),
    // P Q R
    ("", "ph", "", "f"), ("", "peop", "", "p IY p"), ("", "pow", "", "p AW"), ("", "put", " ", "p UH t"), ("", "pp", "", "p"), ("", "p", "", "p"),
    ("", "quar", "", "k w AO r"), ("", "qu", "", "k w"), ("", "q", "", "k"),
    (" ", "re", "^#", "r IY"), ("", "rr", "", "r"), ("", "r", "", "r"),
    // S
    ("", "sh", "", "SH"), ("#", "sion", "", "ZH AX n"), ("", "some", "", "s AH m"), ("#", "sur", "#", "ZH ER"), ("", "sur", "#", "SH ER"),
    ("#", "su", "#", "ZH UW"), ("#", "ssu", "#", "SH UW"), ("#", "sed", " ", "z d"), ("#", "s", "#", "z"), ("", "said", "", "s EH d"),
    ("^", "sion", "", "SH AX n"), ("", "s", "s", ""), (".", "s", " ", "z"), ("#:.e", "s", " ", "z"), ("#:^##", "s", " ", "z"),
    ("#e", "s", " ", "z"), ("y", "s", " ", "z"), ("#:^#", "s", " ", "s"), ("u", "s", " ", "s"), (" :#", "s", " ", "z"), (" ", "sch", "", "s k"), ("", "s", "c+", ""),
    ("#", "sm", "", "z m"), ("#", "sn", "'", "z AX n"), ("", "s", "", "s"),
    // T
    (" ", "the", " ", "DH AX"), ("", "to", " ", "t UW"), ("", "that", " ", "DH AE t"), (" ", "this", " ", "DH IH s"),
    (" ", "they", "", "DH EY"), (" ", "there", "", "DH EH r"), ("", "ther", "", "DH ER"), ("", "their", "", "DH EH r"),
    (" ", "than", " ", "DH AE n"), (" ", "them", " ", "DH EH m"), ("", "these", " ", "DH IY z"), (" ", "then", "", "DH EH n"),
    ("", "through", "", "TH r UW"), ("", "those", "", "DH OW z"), ("", "though", " ", "DH OW"), (" ", "thus", "", "DH AH s"),
    ("", "th", "", "TH"), ("#:", "ted", " ", "t IH d"), ("s", "ti", "#n", "CH"), ("", "ti", "o", "SH"), ("", "ti", "a", "SH"),
    ("", "tien", "", "SH AX n"), ("", "tch", "", "CH"), ("", "tt", "", "t"), ("", "tur", "#", "CH ER"), ("", "tu", "a", "CH UW"), (" ", "two", "", "t UW"), ("", "t", "", "t"),
    // U
    (" ", "un", "i", "y UW n"), (" ", "un", "", "AH n"), (" ", "upon", "", "AX p AO n"), ("@", "ur", "#", "UH r"),
    ("", "ur", "#", "y UH r"), ("", "ur", "", "ER"), ("", "u", "^ ", "AH"), ("", "u", "^^", "AH"), ("", "uy", "", "AY"),
    (" g", "u", "#", ""), ("g", "u", "%", ""), ("g", "u", "#", "w"), ("#n", "u", "", "y UW"), ("@", "u", "", "UW"), ("", "u", "", "y UW"),
    // V W X Y Z
    ("", "view", "", "v y UW"), ("", "v", "", "v"),
    (" ", "were", "", "w ER"), ("", "wa", "s", "w AA"), ("", "wa", "t", "w AA"), ("", "where", "", "WH EH r"), ("", "what", "", "WH AA t"),
    ("", "whol", "", "HH OW l"), ("", "who", "", "HH UW"), ("", "wh", "", "WH"), ("", "war", "", "w AO r"), ("", "wor", "^", "w ER"),
    ("", "wr", "", "r"), ("", "w", "", "w"),
    ("", "x", "", "k s"), ("", "zz", "", "z"),
    ("a", "y", "", ""), ("o", "y", "", ""), ("", "young", "", "y AH NG"), (" ", "you", "", "y UW"), (" ", "yes", "", "y EH s"), (" ", "y", "", "y"), ("#:^", "y", " ", "IY"),
    ("#:^", "y", "i", "IY"), (" :", "y", " ", "AY"), (" :", "y", "#", "AY"), (" :", "y", "^+:#", "IH"), (" :", "y", "^#", "AY"), ("", "y", "", "IH"),
    ("", "z", "", "z"),
];

/// The NRL notation to ours.
fn from_nrl(s: &str) -> Vec<usize> {
    s.split_whitespace()
        .filter_map(|t| {
            let p = match t {
                "IY" => "i",
                "IH" => "I",
                "EY" => "eI",
                "EH" => "E",
                "AE" => "&",
                "AA" => "A",
                "AO" => "O",
                "OW" => "oU",
                "UH" => "U",
                "UW" => "u",
                "ER" => "3",
                "AX" => "@",
                "AH" => "V",
                "AY" => "aI",
                "AW" => "aU",
                "OY" => "OI",
                "TH" => "T",
                "DH" => "D",
                "SH" => "S",
                "ZH" => "Z",
                "HH" => "h",
                "NG" => "N",
                "CH" => "tS",
                "WH" => "w",
                "j" => "dZ",
                "y" => "j",
                other => other,
            };
            ph(p)
        })
        .collect()
}

/// English words the rules get wrong, and the short words that carry no stress.
#[rustfmt::skip]
const EN_WORDS: &[(&str, &str)] = &[
    ("a", "@"), ("an", "@n"), ("the", "D@"), ("of", "@v"), ("to", "tu"), ("and", "&nd"), ("in", "In"), ("is", "Iz"),
    ("it", "It"), ("as", "&z"), ("at", "&t"), ("be", "bi"), ("by", "baI"), ("for", "fOr"), ("from", "frVm"), ("or", "Or"),
    ("on", "An"), ("we", "wi"), ("he", "hi"), ("she", "Si"), ("me", "mi"), ("my", "maI"), ("your", "jOr"), ("you", "ju"),
    ("are", "Ar"), ("was", "wVz"), ("were", "w3"), ("have", "h&v"), ("has", "h&z"), ("do", "du"), ("does", "dVz"),
    ("what", "wVt"), ("who", "hu"), ("one", "'wVn"), ("two", "'tu"), ("once", "'wVns"), ("again", "@'gEn"),
    ("said", "'sEd"), ("says", "'sEz"), ("give", "'gIv"), ("live", "'lIv"), ("love", "'lVv"), ("come", "'kVm"),
    ("some", "'sVm"), ("done", "'dVn"), ("none", "'nVn"), ("gone", "'gOn"), ("only", "'oUnli"), ("every", "'Evri"),
    ("never", "'nEv3"), ("ever", "'Ev3"), ("very", "'vEri"), ("any", "'Eni"), ("many", "'mEni"), ("thing", "'TIN"),
    ("things", "'TINz"), ("same", "'seIm"), ("make", "'meIk"), ("made", "'meId"), ("time", "'taIm"), ("twice", "'twaIs"),
    ("language", "'l&NgwIdZ"), ("art", "'Art"), ("artwork", "'Artw3k"), ("artworks", "'Artw3ks"), ("code", "'koUd"),
    ("random", "'r&nd@m"), ("chance", "'tS&ns"), ("seed", "'sid"), ("edition", "I'dIS@n"), ("editions", "I'dIS@nz"),
    ("alea", "'Ali@"), ("generative", "'dZEn@r@tIv"), ("kernel", "'k3n@l"), ("linux", "'lIn@ks"), ("sound", "'saUnd"),
    ("music", "'mjuzIk"), ("image", "'ImIdZ"), ("images", "'ImIdZIz"), ("video", "'vIdioU"), ("picture", "'pIktS3"),
    ("write", "'raIt"), ("written", "'rIt@n"), ("run", "'rVn"), ("runs", "'rVnz"), ("draw", "'drO"), ("drawn", "'drOn"),
    ("die", "'daI"), ("dice", "'daIs"), ("throw", "'TroU"), ("listen", "'lIs@n"), ("hello", "h@'loU"), ("world", "'w3ld"),
    ("voice", "'vOIs"), ("read", "'rid"), ("reads", "'ridz"), ("text", "'tEkst"), ("synthesis", "'sInT@sIs"),
    ("computer", "k@m'pjut3"), ("machine", "m@'Sin"), ("program", "'proUgr&m"), ("different", "'dIfr@nt"),
    ("because", "bI'kOz"), ("before", "bI'fOr"), ("between", "bI'twin"), ("idea", "aI'di@"), ("create", "kri'eIt"),
    ("there", "DEr"), ("their", "DEr"), ("where", "'wEr"), ("here", "'hir"), ("eye", "'aI"), ("eyes", "'aIz"),
    ("people", "'pip@l"), ("would", "wUd"), ("could", "kUd"), ("should", "SUd"), ("through", "'Tru"), ("though", "'DoU"),
    ("knows", "'noUz"), ("know", "'noU"), ("new", "'nju"), ("now", "'naU"), ("how", "'haU"), ("they", "DeI"),
    ("this", "DIs"), ("that", "D&t"), ("with", "wID"), ("not", "nAt"), ("but", "bVt"), ("can", "k&n"), ("will", "wIl"),
    ("i", "'aI"), ("i'm", "'aIm"), ("it's", "Its"), ("don't", "'doUnt"), ("can't", "'k&nt"), ("am", "&m"), ("no", "'noU"),
    ("yes", "'jEs"), ("go", "'goU"), ("so", "'soU"), ("all", "'Ol"), ("call", "'kOl"), ("calls", "'kOlz"), ("system", "'sIst@m"),
    ("interrupt", "'Int@rVpt"), ("process", "'prAsEs"), ("memory", "'mEm@ri"), ("shell", "'SEl"), ("user", "'juz3"),
    ("hardware", "'hArdwEr"), ("software", "'sOftwEr"), ("noise", "'nOIz"), ("number", "'nVmb3"), ("open", "'oUp@n"),
    ("heart", "'hArt"), ("choir", "'kwaI3"), ("window", "'wIndoU"), ("echo", "'EkoU"), ("echoes", "'EkoUz"),
    ("timeline", "'taImlaIn"), ("midi", "'mIdi"), ("field", "'fild"), ("planet", "'pl&nIt"), ("friend", "'frEnd"),
    ("rhythm", "'rID@m"), ("chord", "'kOrd"), ("chords", "'kOrdz"), ("words", "'w3dz"), ("word", "'w3d"),
    ("into", "'Intu"), ("onto", "'Antu"), ("our", "'aU3"), ("out", "'aUt"), ("about", "@'baUt"),
];

const EN_UNSTRESSED: &[&str] = &[
    "a", "an", "the", "of", "to", "and", "in", "is", "it", "as", "at", "be", "by", "for", "from",
    "or", "on", "we", "he", "she", "me", "my", "your", "you", "are", "was", "were", "have", "has",
    "do", "does", "there", "their", "they", "this", "that", "with", "not", "but", "can", "will",
    "would", "could", "should", "am", "its", "it's", "his", "her", "them", "than", "then",
];

#[rustfmt::skip]
const EN_DIGITS: [&str; 20] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"];
const EN_TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

fn en_number(n: u64) -> String {
    match n {
        0..=19 => EN_DIGITS[n as usize].to_string(),
        20..=99 => {
            let t = EN_TENS[(n / 10) as usize];
            if n % 10 == 0 {
                t.to_string()
            } else {
                format!("{t} {}", EN_DIGITS[(n % 10) as usize])
            }
        }
        100..=999 => {
            let r = n % 100;
            format!(
                "{} hundred{}",
                EN_DIGITS[(n / 100) as usize],
                if r > 0 {
                    format!(" {}", en_number(r))
                } else {
                    String::new()
                }
            )
        }
        1000..=999_999 => {
            let r = n % 1000;
            format!(
                "{} thousand{}",
                en_number(n / 1000),
                if r > 0 {
                    format!(" {}", en_number(r))
                } else {
                    String::new()
                }
            )
        }
        _ => {
            let r = n % 1_000_000;
            format!(
                "{} million{}",
                en_number(n / 1_000_000),
                if r > 0 {
                    format!(" {}", en_number(r))
                } else {
                    String::new()
                }
            )
        }
    }
}

#[rustfmt::skip]
const FR_DIGITS: [&str; 17] = ["zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix",
    "onze", "douze", "treize", "quatorze", "quinze", "seize"];

fn fr_number(n: u64) -> String {
    let tens = [
        "",
        "dix",
        "vingt",
        "trente",
        "quarante",
        "cinquante",
        "soixante",
    ];
    match n {
        0..=16 => FR_DIGITS[n as usize].to_string(),
        17..=19 => format!("dix {}", FR_DIGITS[(n - 10) as usize]),
        20..=69 => {
            let (t, u) = (n / 10, n % 10);
            match u {
                0 => tens[t as usize].to_string(),
                1 => format!("{} et un", tens[t as usize]),
                _ => format!("{} {}", tens[t as usize], FR_DIGITS[u as usize]),
            }
        }
        70..=79 => {
            if n == 71 {
                "soixante et onze".into()
            } else {
                format!("soixante {}", fr_number(n - 60))
            }
        }
        80..=99 => {
            if n == 80 {
                "quatre vingts".into()
            } else {
                format!("quatre vingt {}", fr_number(n - 80))
            }
        }
        100..=999 => {
            let (h, r) = (n / 100, n % 100);
            let head = if h == 1 {
                "cent".to_string()
            } else {
                format!("{} cent", FR_DIGITS[h as usize])
            };
            if r > 0 {
                format!("{head} {}", fr_number(r))
            } else {
                head
            }
        }
        1000..=999_999 => {
            let (th, r) = (n / 1000, n % 1000);
            let head = if th == 1 {
                "mille".to_string()
            } else {
                format!("{} mille", fr_number(th))
            };
            if r > 0 {
                format!("{head} {}", fr_number(r))
            } else {
                head
            }
        }
        _ => {
            let (m, r) = (n / 1_000_000, n % 1_000_000);
            let head = format!("{} million", fr_number(m));
            if r > 0 {
                format!("{head} {}", fr_number(r))
            } else {
                head
            }
        }
    }
}

const EN_LETTERS: [&str; 26] = [
    "eI", "bi", "si", "di", "i", "Ef", "dZi", "eItS", "aI", "dZeI", "keI", "El", "Em", "En", "oU",
    "pi", "kju", "Ar", "Es", "ti", "ju", "vi", "dVb@lju", "Eks", "waI", "zi",
];
const FR_LETTERS: [&str; 26] = [
    "a", "be", "se", "de", "@", "Ef", "Ze", "aS", "i", "Zi", "ka", "El", "Em", "En", "o", "pe",
    "ky", "ER", "Es", "te", "y", "ve", "dubl@ve", "iks", "igREk", "zEd",
];

// French: the same kind of rules, in our notation directly.
#[rustfmt::skip]
const FR_RULES: &[Rule] = &[
    // vowels
    ("", "aient", " ", "E"), ("", "ais", " ", "E"), ("", "ait", " ", "E"), ("", "aim", "~", "e~"), ("", "ain", "~", "e~"),
    ("", "ail", " ", "aj"), ("", "aill", "", "aj"), ("", "ai", "", "E"), ("", "ay", "#", "Ej"), ("", "ay", "", "E"), ("", "au", "", "o"),
    ("", "am", "~", "a~"), ("", "an", "~", "a~"), ("", "a", "", "a"), ("", "à", "", "a"), ("", "â", "", "a"),
    ("", "eaux", " ", "o"), ("", "eau", "", "o"), ("", "ein", "~", "e~"), ("", "eim", "~", "e~"), ("", "eil", " ", "Ej"), ("", "eill", "", "Ej"),
    ("", "ei", "", "E"), ("", "euill", "", "9j"), ("", "euil", "", "9j"), ("", "eux", " ", "2"), ("", "eu", "r", "9"), ("", "eu", "f", "9"),
    ("", "eu", "l", "9"), ("", "eu", "", "2"), ("", "ey", "", "E"), (" ", "ex", "#", "Egz"), ("", "enn", "", "En"), ("", "emm", "", "am"),
    ("", "ment", " ", "ma~"), ("#:", "ent", " ", ""), ("", "em", "~", "a~"), ("", "en", "~", "a~"),
    ("#:", "er", " ", "e"), ("#:", "ez", " ", "e"), ("", "et", " ", "E"), ("#:", "es", " ", ""), (" :", "es", " ", "e"),
    ("#:", "e", " ", ""), (" :", "e", " ", "@"), ("", "é", "", "e"), ("", "è", "", "E"), ("", "ê", "", "E"), ("", "ë", "", "E"),
    ("", "e", "x", "E"), ("", "e", "^^", "E"), ("", "e", "^ ", "E"), ("", "e", "", "@"),
    ("", "ien", "~", "je~"), ("", "ies", " ", "i"), ("", "ie", " ", "i"), ("", "ient", " ", "i"), ("", "im", "~", "e~"), ("", "in", "~", "e~"), ("", "ill", "", "ij"), ("", "i", "#", "j"), ("", "î", "", "i"),
    ("", "ï", "", "i"), ("", "i", "", "i"),
    ("", "oin", "~", "we~"), ("", "oi", "", "wa"), ("", "oy", "#", "waj"), ("", "ouill", "", "uj"), ("", "ou", "#", "w"), ("", "où", "", "u"),
    ("", "oû", "", "u"), ("", "ou", "", "u"), ("", "om", "~", "o~"), ("", "on", "~", "o~"), ("", "œu", "", "2"), ("", "œ", "", "2"),
    ("", "ô", "", "o"), ("", "o", " ", "o"), ("", "o", "s ", "o"), ("", "o", "t ", "o"), ("", "o", "x ", "o"), ("", "o", "se", "o"),
    ("", "o", "", "O"),
    ("", "um", " ", "Om"), ("", "un", "~", "e~"), ("", "um", "~", "e~"), ("", "ui", "", "Hi"), ("", "u", "#", "H"), ("", "û", "", "y"),
    ("", "ù", "", "y"), ("", "ü", "", "y"), ("", "u", "", "y"),
    ("", "y", "#", "j"), ("", "y", "", "i"), ("", "ÿ", "", "i"),
    // consonants
    ("", "bb", "", "b"), ("", "b", "", "b"),
    ("", "ch", "", "S"), ("", "ck", "", "k"), ("", "cc", "+", "ks"), ("", "cc", "", "k"), ("", "c", "+", "s"), ("", "ç", "", "s"), ("", "c", "", "k"),
    ("", "d", " ", ""), ("", "ds", " ", ""), ("", "dd", "", "d"), ("", "d", "", "d"),
    ("", "ff", "", "f"), ("", "f", "", "f"),
    ("", "gn", "", "J"), ("", "gu", "+", "g"), ("", "ge", "#", "Z"), ("", "gg", "", "g"), ("", "g", "+", "Z"), ("#", "g", " ", ""), ("", "g", "", "g"),
    ("", "h", "", ""), ("", "j", "", "Z"), ("", "k", "", "k"), ("", "ll", "", "l"), ("", "l", "", "l"), ("", "mm", "", "m"), ("", "m", "", "m"),
    ("", "nn", "", "n"), ("", "n", "", "n"),
    ("", "ph", "", "f"), ("", "pp", "", "p"), ("#", "p", " ", ""), ("", "ps", " ", ""), ("", "p", "", "p"),
    ("", "qu", "", "k"), ("", "q", "", "k"),
    ("", "rr", "", "R"), ("", "r", "", "R"),
    ("", "sch", "", "S"), ("", "ss", "", "s"), ("", "sc", "+", "s"), ("", "s", " ", ""), ("#", "s", "#", "z"), ("", "s", "", "s"),
    ("", "th", "", "t"), ("", "tt", "", "t"), ("s", "tion", "", "tjo~"), ("", "tion", "", "sjo~"), ("", "t", " ", ""), ("", "ts", " ", ""), ("", "t", "", "t"),
    ("", "v", "", "v"), ("", "w", "", "w"), ("#", "x", " ", ""), ("", "x", "", "ks"), ("#", "z", " ", ""), ("", "z", "", "z"),
];

#[rustfmt::skip]
const FR_WORDS: &[(&str, &str)] = &[
    ("le", "l@"), ("la", "la"), ("les", "le"), ("de", "d@"), ("des", "de"), ("du", "dy"), ("un", "e~"), ("une", "yn"), ("et", "e"),
    ("est", "E"), ("je", "Z@"), ("tu", "ty"), ("il", "il"), ("elle", "El"), ("nous", "nu"), ("vous", "vu"), ("ils", "il"),
    ("elles", "El"), ("on", "o~"), ("ne", "n@"), ("pas", "pa"), ("que", "k@"), ("qui", "ki"), ("quoi", "kwa"), ("ce", "s@"),
    ("ces", "se"), ("cet", "sEt"), ("cette", "sEt"), ("se", "s@"), ("sa", "sa"), ("son", "so~"), ("ses", "se"), ("mon", "mo~"),
    ("ma", "ma"), ("mes", "me"), ("me", "m@"), ("te", "t@"), ("ton", "to~"), ("ta", "ta"), ("tes", "te"), ("en", "a~"),
    ("dans", "da~"), ("avec", "avEk"), ("pour", "puR"), ("par", "paR"), ("sur", "syR"), ("mais", "mE"), ("ou", "u"), ("où", "u"),
    ("plus", "ply"), ("tout", "tu"), ("tous", "tu"), ("toutes", "tut"), ("fils", "fis"), ("femme", "fam"), ("monsieur", "m@sj2"),
    ("oui", "wi"), ("non", "no~"), ("sept", "sEt"), ("huit", "Hit"), ("six", "sis"), ("dix", "dis"), ("neuf", "n9f"), ("cinq", "se~k"),
    ("vingt", "ve~"), ("vingts", "ve~"), ("cent", "sa~"), ("mille", "mil"), ("ville", "vil"), ("second", "s@go~"), ("seconde", "s@go~d"),
    ("eu", "y"), ("eux", "2"), ("hier", "jER"), ("mer", "mER"), ("fer", "fER"), ("cher", "SER"), ("fier", "fjER"), ("hiver", "ivER"),
    ("aléa", "alea"), ("alea", "alea"), ("aléas", "alea"), ("souvent", "suva~"), ("vent", "va~"), ("chaque", "Sak"), ("noyau", "nwajo"),
    ("algorithme", "algORitm"), ("hasard", "azaR"), ("aux", "o"), ("au", "o"), ("ai", "e"), ("a", "a"), ("y", "i"), ("à", "a"),
    ("deux", "d2"), ("trois", "tRwa"), ("quatre", "katR"), ("zéro", "zeRo"), ("onze", "o~z"), ("douze", "duz"), ("seize", "sEz"),
    ("quinze", "ke~z"), ("trente", "tRa~t"), ("quarante", "kaRa~t"), ("cinquante", "se~ka~t"), ("soixante", "swasa~t"),
    ("million", "miljo~"), ("monde", "mo~d"), ("bonjour", "bo~ZuR"), ("pays", "pei"), ("jamais", "ZamE"), ("parfois", "paRfwa"),
    ("chose", "Soz"), ("choses", "Soz"), ("fois", "fwa"), ("rien", "Rje~"), ("bien", "bje~"), ("son", "so~"), ("sons", "so~"),
    ("image", "imaZ"), ("images", "imaZ"), ("code", "kOd"), ("voix", "vwa"), ("langage", "la~gaZ"), ("graine", "gREn"),
    ("œuvre", "9vR"), ("œuvres", "9vR"), ("lit", "li"), ("dit", "di"), ("fait", "fE"), ("font", "fo~"), ("sont", "so~"), ("ont", "o~"),
    ("c'est", "sE"), ("t", "t"), ("bus", "bys"), ("sens", "sa~s"), ("ours", "uRs"), ("os", "Os"), ("ouest", "wEst"), ("test", "tEst"),
    ("net", "nEt"), ("but", "byt"), ("août", "ut"), ("tennis", "tenis"), ("virus", "viRys"), ("cactus", "kaktys"), ("hélas", "elas"), ("moi", "mwa"), ("toi", "twa"), ("lui", "lHi"), ("nuit", "nHi"), ("noyaux", "nwajo"), ("système", "sistEm"),
    ("mémoire", "memwaR"), ("matériel", "mateRjEl"), ("utilisateur", "ytilizat9R"), ("processus", "pROsesys"),
];

const FR_UNSTRESSED: &[&str] = &[
    "le", "la", "les", "de", "des", "du", "un", "une", "et", "je", "tu", "il", "elle", "nous",
    "vous", "ils", "on", "ne", "que", "ce", "se", "sa", "son", "ses", "mon", "ma", "mes", "me",
    "te", "en", "dans", "pour", "par", "sur", "au", "aux", "à", "est",
];

// ───────────────────────────── from text to phonemes ─────────────────────────────

/// One word, as phonemes with their stress (0, 1 or 2).
fn word_phonemes(w: &str, lang: Lang) -> Vec<(usize, u8)> {
    let lower = w.to_lowercase();
    // short words in capitals are spelled out: CPU, IRQ
    let caps = w.chars().count() >= 2
        && w.chars().count() <= 4
        && w.chars().all(|c| c.is_ascii_uppercase());
    let dict = match lang {
        Lang::En => EN_WORDS,
        Lang::Fr => FR_WORDS,
    };
    if let Some((_, p)) = dict.iter().find(|(k, _)| *k == lower) {
        let mut out = parse_phonemes(p);
        if !out.iter().any(|&(_, s)| s > 0) {
            let unstressed = match lang {
                Lang::En => EN_UNSTRESSED,
                Lang::Fr => FR_UNSTRESSED,
            };
            if !unstressed.contains(&lower.as_str()) {
                stress_first(&mut out, lang);
            }
        }
        return out;
    }
    if caps {
        let names = match lang {
            Lang::En => &EN_LETTERS,
            Lang::Fr => &FR_LETTERS,
        };
        let mut out = Vec::new();
        for c in lower.chars() {
            let mut l = parse_phonemes(names[(c as u8 - b'a') as usize]);
            // the last letter carries the stress
            if let Some(v) = l
                .iter_mut()
                .rev()
                .find(|(p, _)| PHONEMES[*p].class == Class::Vowel)
            {
                v.1 = 1;
            }
            out.extend(l);
        }
        if let Some(v) = out
            .iter_mut()
            .rev()
            .find(|(p, _)| PHONEMES[*p].class == Class::Vowel)
        {
            v.1 = 2;
        }
        return out;
    }
    match lang {
        Lang::En => {
            let mut out: Vec<(usize, u8)> = from_nrl(&apply::<EnLetters>(&lower, EN_RULES))
                .into_iter()
                .map(|p| (p, 0))
                .collect();
            // the stress: on the first full vowel, after a light prefix
            let vowels: Vec<usize> = (0..out.len())
                .filter(|&k| PHONEMES[out[k].0].class == Class::Vowel)
                .collect();
            let prefix = ["be", "de", "re", "pre", "a", "ex", "con", "com", "pro"]
                .iter()
                .any(|p| lower.starts_with(p));
            let skip = usize::from(
                prefix && vowels.len() >= 2 && !lower.starts_with("ar") && !lower.starts_with("al"),
            );
            let full = vowels
                .iter()
                .skip(skip)
                .find(|&&k| PHONEMES[out[k].0].name != "@")
                .or(vowels.first());
            if let Some(&k) = full {
                out[k].1 = 2;
            }
            out
        }
        Lang::Fr => {
            let mut out = parse_phonemes(&apply::<FrLetters>(&lower, FR_RULES));
            if !FR_UNSTRESSED.contains(&lower.as_str()) {
                stress_first(&mut out, lang);
            }
            out
        }
    }
}

fn stress_first(out: &mut [(usize, u8)], lang: Lang) {
    let vowels = out
        .iter()
        .enumerate()
        .filter(|(_, (p, _))| PHONEMES[*p].class == Class::Vowel)
        .map(|(k, _)| k);
    let k = match lang {
        Lang::En => vowels
            .clone()
            .find(|&k| PHONEMES[out[k].0].name != "@")
            .or(vowels.clone().next()),
        Lang::Fr => vowels.last(),
    };
    if let Some(k) = k {
        out[k].1 = 2;
    }
}

/// A piece of the text: a word, or a pause after punctuation.
#[derive(Debug, Clone)]
enum Token {
    Word(Vec<(usize, u8)>),
    /// `,` `;` `:` (a short pause, the voice stays up), `.` `!` (a long one,
    /// it falls), `?` (it rises)
    Break(char),
}

fn tokens(text: &str, lang: Lang) -> Vec<Token> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut k = 0;
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut Vec<Token>| {
        if word.is_empty() {
            return;
        }
        let w = std::mem::take(word);
        if w.chars().all(|c| c.is_ascii_digit()) {
            let n: u64 = w.parse().unwrap_or(0);
            let spoken = match lang {
                Lang::En => en_number(n),
                Lang::Fr => fr_number(n),
            };
            for s in spoken.split_whitespace() {
                out.push(Token::Word(word_phonemes(s, lang)));
            }
            return;
        }
        // French elision: l'homme, c'est, qu'il
        if lang == Lang::Fr {
            if let Some(i) = w.find(['\'', '’']) {
                let apostrophe = w[i..].chars().next().map_or(1, char::len_utf8);
                let (head, rest) = (&w[..i], &w[i + apostrophe..]);
                let lower = w.to_lowercase();
                if FR_WORDS.iter().any(|(k, _)| *k == lower) {
                    out.push(Token::Word(word_phonemes(&lower, lang)));
                    return;
                }
                let lead = match head.to_lowercase().as_str() {
                    "l" => "l",
                    "d" => "d",
                    "j" => "Z",
                    "c" => "s",
                    "n" => "n",
                    "s" => "s",
                    "m" => "m",
                    "t" => "t",
                    "qu" | "lorsqu" | "jusqu" | "puisqu" => "k",
                    _ => "",
                };
                let mut ph = parse_phonemes(lead);
                ph.extend(word_phonemes(rest, lang));
                out.push(Token::Word(ph));
                return;
            }
        }
        out.push(Token::Word(word_phonemes(&w.replace('’', "'"), lang)));
    };
    while k < chars.len() {
        let c = chars[k];
        if c == '/' {
            flush(&mut word, &mut out);
            let end = chars[k + 1..]
                .iter()
                .position(|&c| c == '/')
                .map_or(chars.len(), |p| k + 1 + p);
            let inner: String = chars[k + 1..end].iter().collect();
            for w in inner.split_whitespace() {
                out.push(Token::Word(parse_phonemes(w)));
            }
            k = end + 1;
            continue;
        }
        if c.is_alphanumeric() || ((c == '\'' || c == '’') && !word.is_empty()) {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            match c {
                ',' | ';' | ':' | '—' | '–' | '(' | ')' => out.push(Token::Break(',')),
                '.' | '!' | '\n' | '…' => out.push(Token::Break('.')),
                '?' => out.push(Token::Break('?')),
                '-' if lang == Lang::En => {}
                _ => {}
            }
        }
        k += 1;
    }
    flush(&mut word, &mut out);
    out
}

// ───────────────────────────── timing and melody ─────────────────────────────

/// One sound of the utterance, placed in time.
#[derive(Debug, Clone, Copy)]
pub struct Seg {
    ph: usize,
    start: f64,
    dur: f64,
    stress: u8,
    /// for a stop: aspirated before the next vowel
    aspirate: bool,
}

/// What `say` will say: its sounds in time, its words and its syllables.
#[derive(Debug)]
pub struct Utterance {
    segs: Vec<Seg>,
    /// the pitch along the utterance, as (time, ratio to the voice's own pitch)
    f0: Vec<(f64, f64)>,
    /// the start and the end of each word, in seconds from the start
    pub words: Vec<(f64, f64)>,
    /// the start of each vowel
    pub syllables: Vec<f64>,
    pub duration: f64,
}

/// The lengths in the table, to the pace of a voice at `speed` 1.
const TEMPO: f64 = 0.9;

/// Places the sounds of a text in time and draws its melody: `speed` 1 is an
/// unhurried voice; `melody`, if given, is sung, a semitone value per
/// syllable (in a loop), instead of the melody of speech.
pub fn plan(text: &str, lang: Lang, speed: f64, melody: &[f64]) -> Utterance {
    let toks = tokens(text, lang);
    let speed = speed.clamp(0.2, 5.0);
    let mut segs: Vec<Seg> = Vec::new();
    let mut words = Vec::new();
    let mut f0 = Vec::new();
    let mut syllables = Vec::new();
    let mut t = 0.03;
    let sing = !melody.is_empty();
    let mut sung = 0usize;
    // phrase by phrase: a run of words up to a break
    let mut k = 0;
    while k < toks.len() {
        let start = k;
        while k < toks.len() && matches!(toks[k], Token::Word(_)) {
            k += 1;
        }
        let end_mark = match toks.get(k) {
            Some(Token::Break(c)) => *c,
            _ => '.',
        };
        let phrase: Vec<&Vec<(usize, u8)>> = toks[start..k]
            .iter()
            .filter_map(|t| {
                if let Token::Word(w) = t {
                    Some(w)
                } else {
                    None
                }
            })
            .collect();
        if !phrase.is_empty() {
            let n_words = phrase.len();
            let mut points: Vec<(f64, f64)> = Vec::new();
            let phrase_t0 = t;
            for (wi, w) in phrase.iter().enumerate() {
                let w_start = t;
                let last_word = wi == n_words - 1;
                let vowel_ix: Vec<usize> = (0..w.len())
                    .filter(|&j| PHONEMES[w[j].0].class == Class::Vowel)
                    .collect();
                let last_vowel = vowel_ix.last().copied();
                for (j, &(p, stress)) in w.iter().enumerate() {
                    let pd = PHONEMES[p];
                    let mut ms = pd.ms;
                    let is_vowel = pd.class == Class::Vowel;
                    if is_vowel {
                        ms *= match (lang, stress) {
                            (Lang::En, 2) => 1.3,
                            (Lang::En, 1) => 1.05,
                            (Lang::En, _) => 0.85,
                            (Lang::Fr, _) => 0.9,
                        };
                        // short enough to be lost: no vowel under 65 ms
                        ms = ms.max(65.0);
                        // the end of a phrase is drawn out, in French the end of each word group
                        if Some(j) == last_vowel && (last_word || (lang == Lang::Fr && stress > 0))
                        {
                            ms *= 1.45;
                        }
                        if sing {
                            ms = ms.max(150.0) * 1.6;
                        }
                    } else if last_word && last_vowel.is_some_and(|lv| j > lv) {
                        ms *= 1.25;
                    }
                    // a stop or a fricative at the start of a word is a touch longer: it is heard
                    if j == 0 && matches!(pd.class, Class::Stop | Class::Fric | Class::Affric) {
                        ms *= 1.15;
                    }
                    // a voiceless stop before a stressed vowel breathes out (English)
                    let next = w.get(j + 1).copied();
                    let aspirate = lang == Lang::En
                        && pd.class == Class::Stop
                        && !pd.voiced
                        && next.is_some_and(|(q, s)| {
                            PHONEMES[q].class == Class::Vowel && (s > 0 || j == 0)
                        });
                    if aspirate {
                        ms += 40.0;
                    }
                    let dur = ms / 1000.0 / speed * TEMPO;
                    if is_vowel {
                        syllables.push(t);
                        let mid = t + dur * 0.5;
                        if sing {
                            let m = 2f64.powf(melody[sung % melody.len()] / 12.0);
                            sung += 1;
                            points.push((t + dur * 0.15, m));
                            points.push((t + dur * 0.95, m));
                        } else {
                            // declination from the start of the phrase; stressed vowels rise above it
                            let accent = match (lang, stress) {
                                (Lang::En, 2) => 0.16,
                                (Lang::En, 1) => 0.06,
                                _ => 0.0,
                            };
                            points.push((mid, accent));
                        }
                    }
                    segs.push(Seg {
                        ph: p,
                        start: t,
                        dur,
                        stress,
                        aspirate,
                    });
                    t += dur;
                }
                words.push((w_start, t));
                // a hair of silence between words: each one stands on its own
                if !last_word {
                    let gap = 0.028 / speed;
                    segs.push(Seg { ph: ph("_").expect("pause"), start: t, dur: gap, stress: 0, aspirate: false });
                    t += gap;
                }
                // French: each word group ends rising (but the last)
                if lang == Lang::Fr && !sing && !last_word && w.len() > 2 && last_vowel.is_some() {
                    if let Some(p) = points.last_mut() {
                        p.1 += 0.07;
                    }
                }
            }
            if !sing {
                // the melody of the phrase: a line falling from 1.06 to 0.9,
                // accents above it, and the end by its punctuation
                let span = (t - phrase_t0).max(1e-3);
                let n = points.len();
                for (q, &(pt, acc)) in points.iter().enumerate() {
                    let base = 1.06 - 0.16 * (pt - phrase_t0) / span;
                    let last = q + 1 == n;
                    let r = if last {
                        match end_mark {
                            '?' => 1.35,
                            ',' => 1.08,
                            _ => base - 0.08,
                        }
                    } else {
                        base + acc
                    };
                    f0.push((pt, r));
                    if last && end_mark == '.' {
                        f0.push((t, 0.74));
                    }
                    if last && end_mark == '?' {
                        f0.push((t, 1.45));
                    }
                }
            } else {
                f0.extend(points);
            }
        }
        // the pause
        let pause = match toks.get(k) {
            Some(Token::Break(',')) => 0.15,
            Some(Token::Break(_)) => 0.36,
            _ => 0.0,
        } / speed;
        if pause > 0.0 {
            segs.push(Seg {
                ph: ph("_").expect("pause"),
                start: t,
                dur: pause,
                stress: 0,
                aspirate: false,
            });
            t += pause;
        }
        k += 1;
    }
    if f0.is_empty() {
        f0.push((0.0, 1.0));
    }
    // a voice that ends without a pause still takes a breath
    t += 0.05;
    Utterance {
        segs,
        f0,
        words,
        syllables,
        duration: t,
    }
}

// ───────────────────────────── the voice ─────────────────────────────

/// A resonator (Klatt): one formant.
#[derive(Default, Clone, Copy)]
struct Reso {
    y1: f64,
    y2: f64,
}

impl Reso {
    fn tick(&mut self, x: f64, f: f64, bw: f64) -> f64 {
        let r = libm::exp(-PI * bw / SR);
        let c = -r * r;
        let b = 2.0 * r * libm::cos(2.0 * PI * f.min(SR * 0.45) / SR);
        let a = 1.0 - b - c;
        let y = a * x + b * self.y1 + c * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// What the tract is doing at a moment: formants, the voice, breath, noise.
#[derive(Clone, Copy, Debug, Default)]
struct Target {
    f: [f64; 3],
    voice: f64,
    asp: f64,
    noise: f64,
    nf: f64,
    nbw: f64,
    nasal: f64,
}

impl Target {
    fn lerp(&self, o: &Target, a: f64) -> Target {
        let l = |x: f64, y: f64| x + (y - x) * a;
        Target {
            f: [l(self.f[0], o.f[0]), l(self.f[1], o.f[1]), l(self.f[2], o.f[2])],
            voice: l(self.voice, o.voice),
            asp: l(self.asp, o.asp),
            noise: l(self.noise, o.noise),
            // the colour of the noise jumps with the louder of the two
            nf: if self.noise >= o.noise { self.nf } else { o.nf },
            nbw: if self.noise >= o.noise { self.nbw } else { o.nbw },
            nasal: l(self.nasal, o.nasal),
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }
}

/// The voice's character: its pitch (Hz), the size of its tract (1 an adult
/// man, smaller is higher, a child near 0.7), roughness and breath (0–1).
#[derive(Clone, Copy, Debug)]
pub struct Timbre {
    pub pitch: f64,
    pub size: f64,
    pub rough: f64,
    pub breath: f64,
}

/// A moment the tract is heading for: at `t`, it is at `target`; between two
/// keys it moves in a straight line. A phase of a sound holds its target
/// between two keys, and the stretch to the next sound's first key is the
/// transition (the formants' glide from a stop into its vowel, say).
struct Key {
    t: f64,
    target: Target,
}

/// The keys of an utterance: its sounds as phases, each held, with the
/// transitions between them. This is where articulation happens.
fn keys(u: &Utterance) -> Vec<Key> {
    let mut keys: Vec<Key> = Vec::new();
    // a phase held from `a` to `b`, reached over `tin` and left over `tout`
    fn hold(keys: &mut Vec<Key>, a: f64, b: f64, tin: f64, tout: f64, target: Target) {
        let d = b - a;
        let tin = tin.min(d * 0.4);
        let tout = tout.min(d * 0.4);
        keys.push(Key { t: a + tin, target });
        keys.push(Key { t: b - tout, target });
    }
    let n = u.segs.len();
    for (k, s) in u.segs.iter().enumerate() {
        let p = PHONEMES[s.ph];
        let (a, b) = (s.start, s.start + s.dur);
        let next = u.segs.get(k + 1).map(|n| PHONEMES[n.ph]);
        let next_vowel = next.filter(|q| matches!(q.class, Class::Vowel | Class::Glide | Class::Liquid));
        let prev_sonorant = k > 0 && matches!(PHONEMES[u.segs[k - 1].ph].class, Class::Vowel | Class::Glide | Class::Liquid | Class::Nasal);
        // the formants the stop's burst and breath open into: the vowel's
        let after = next_vowel.map_or(p.f, |q| q.f);
        let tr: f64 = 0.035; // a formant transition
        match p.class {
            Class::Vowel => {
                let loud = if s.stress > 0 { 1.0 } else { 0.85 };
                let t = Target { f: p.f, voice: loud, nasal: f64::from(u8::from(p.nasal)), ..Default::default() };
                match p.to {
                    // a diphthong moves through its whole length
                    Some(to) => {
                        keys.push(Key { t: a + tr.min(s.dur * 0.3), target: t });
                        keys.push(Key { t: b - (tr * 0.5).min(s.dur * 0.2), target: Target { f: to, ..t } });
                    }
                    None => hold(&mut keys, a, b, tr, tr, t),
                }
            }
            Class::Glide => hold(&mut keys, a, b, tr, tr, Target { f: p.f, voice: 0.7, ..Default::default() }),
            Class::Liquid => hold(&mut keys, a, b, tr * 0.7, tr * 0.7, Target { f: p.f, voice: 0.65, ..Default::default() }),
            Class::Nasal => hold(&mut keys, a, b, 0.012, 0.012, Target { f: p.f, voice: 0.45, nasal: 1.0, ..Default::default() }),
            Class::Fric => {
                let voice = if p.voiced { if p.name == "R" { 0.55 } else { 0.35 } } else { 0.0 };
                hold(&mut keys, a, b, 0.02, 0.02, Target { f: p.f, voice, noise: p.noise.2, nf: p.noise.0, nbw: p.noise.1, ..Default::default() })
            }
            Class::Aspirate => hold(&mut keys, a, b, 0.015, 0.015, Target { f: after, asp: 0.5, ..Default::default() }),
            Class::Stop => {
                // closure: silence (a voiced stop keeps a faint voice bar);
                // the burst: a few ms of noise at the place of the stop;
                // the release: breath into the vowel, the voice starting
                // late after a voiceless stop (the voice onset time)
                let vot = if p.voiced { 0.008 } else if s.aspirate { 0.055 } else { 0.025 };
                let burst = 0.012;
                let close_end = (b - vot - burst).max(a + 0.02);
                let bar = Target { f: [180.0, p.f[1], p.f[2]], voice: if p.voiced { 0.15 } else { 0.0 }, ..Default::default() };
                // the formants glide to the stop's place during the vowel before it
                hold(&mut keys, a, close_end, if prev_sonorant { 0.004 } else { 0.001 }, 0.001, bar);
                let bt = Target { f: after, noise: p.noise.2 * 1.8, nf: p.noise.0, nbw: p.noise.1, voice: if p.voiced { 0.3 } else { 0.0 }, ..Default::default() };
                hold(&mut keys, close_end, close_end + burst, 0.001, 0.001, bt);
                let rel = Target { f: after, asp: if p.voiced { 0.05 } else { 0.4 }, voice: if p.voiced { 0.7 } else { 0.0 }, ..Default::default() };
                hold(&mut keys, close_end + burst, b, 0.002, 0.004, rel);
            }
            Class::Affric => {
                let close_end = a + s.dur * 0.4;
                let bar = Target { f: [180.0, p.f[1], p.f[2]], voice: if p.voiced { 0.15 } else { 0.0 }, ..Default::default() };
                hold(&mut keys, a, close_end, 0.004, 0.001, bar);
                let fr = Target { f: p.f, noise: p.noise.2, nf: p.noise.0, nbw: p.noise.1, voice: if p.voiced { 0.3 } else { 0.0 }, ..Default::default() };
                hold(&mut keys, close_end, b, 0.003, 0.01, fr);
            }
            Class::Pause => {
                // silence; the formants rest where they were
                let f = keys.last().map_or([500.0, 1500.0, 2500.0], |k| k.target.f);
                hold(&mut keys, a, b, 0.03, 0.02, Target { f, ..Default::default() });
            }
        }
        if k + 1 == n {
            let f = keys.last().map_or([500.0, 1500.0, 2500.0], |k| k.target.f);
            keys.push(Key { t: b + 0.03, target: Target { f, ..Default::default() } });
        }
    }
    // keys in order, at least a hair apart
    let mut t = -1.0;
    for k in keys.iter_mut() {
        if k.t <= t {
            k.t = t + 1e-4;
        }
        t = k.t;
    }
    keys
}

/// The utterance, spoken: a mono sound at the rate of alea's sound.
pub fn speak(u: &Utterance, tb: Timbre, seed: u64) -> Vec<f64> {
    let n = (u.duration * SR) as usize + 1;
    let mut out = vec![0.0; n];
    let scale = 1.06 / tb.size.clamp(0.4, 2.0);
    let mut rng = Rng(seed ^ 0x0005_a70f_70ce);
    let keys = keys(u);

    // the pitch
    let f0_at = |time: f64| -> f64 {
        let pts = &u.f0;
        if time <= pts[0].0 {
            return pts[0].1;
        }
        for w in pts.windows(2) {
            if time <= w[1].0 {
                let a = (time - w[0].0) / (w[1].0 - w[0].0).max(1e-6);
                return w[0].1 + (w[1].1 - w[0].1) * a;
            }
        }
        pts[pts.len() - 1].1
    };

    let mut ki = 0usize;
    let mut cur = Target { f: [500.0, 1500.0, 2500.0], ..Default::default() };
    let mut res = [Reso::default(); 5];
    let mut nres = [Reso::default(); 2];
    let mut nasal_pole = Reso::default();
    let mut phase = 0.0f64;
    let mut period_jit = 1.0;
    let mut period_amp = 1.0;
    let mut tilt = 0.0;
    let mut wander = 0.0;
    // amplitudes move fast but not instantly: no clicks
    let k_amp = 1.0 - libm::exp(-1.0 / (0.0025 * SR));
    let mut hp = (0.0, 0.0);
    for (i, o) in out.iter_mut().enumerate() {
        let time = i as f64 / SR;
        while ki + 1 < keys.len() && time >= keys[ki + 1].t {
            ki += 1;
        }
        let tg = if ki + 1 < keys.len() && time >= keys[ki].t {
            let (a, b) = (&keys[ki], &keys[ki + 1]);
            a.target.lerp(&b.target, (time - a.t) / (b.t - a.t).max(1e-6))
        } else if ki + 1 < keys.len() {
            Target { f: keys[0].target.f, ..Default::default() }
        } else {
            Target { f: keys[ki].target.f, ..Default::default() }
        };
        cur.f = tg.f;
        cur.nasal = tg.nasal;
        cur.voice += (tg.voice - cur.voice) * k_amp;
        cur.asp += (tg.asp - cur.asp) * k_amp;
        cur.noise += (tg.noise - cur.noise) * k_amp;
        if tg.noise > 1e-4 {
            cur.nf = tg.nf;
            cur.nbw = tg.nbw;
        }

        // the glottis: a pulse per period, its shape after Klatt's KLGLOTT88
        wander += (rng.next() * 0.02 - wander) * 0.0005;
        let f0 = tb.pitch * f0_at(time) * (1.0 + wander) * period_jit;
        phase += f0 / SR;
        if phase >= 1.0 {
            phase -= 1.0;
            period_jit = 1.0 + tb.rough * 0.04 * rng.next();
            period_amp = 1.0 + tb.rough * 0.35 * rng.next();
        }
        let oq = 0.6;
        let pulse = if phase < oq {
            let x = phase / oq;
            2.0 * x - 3.0 * x * x
        } else {
            0.0
        };
        tilt += (pulse - tilt) * 0.8;
        let open = if phase < oq { 1.0 } else { 0.35 };
        let noise = rng.next();
        let src = tilt * cur.voice * period_amp + noise * (cur.asp + tb.breath * 0.12 * cur.voice) * open;

        // the tract: five formants in a row
        let bw1 = 70.0 + 90.0 * cur.nasal;
        let mut x = res[0].tick(src, cur.f[0] * scale, bw1);
        x = res[1].tick(x, cur.f[1] * scale, 90.0);
        x = res[2].tick(x, cur.f[2] * scale, 150.0);
        x = res[3].tick(x, 3500.0 * scale, 250.0);
        x = res[4].tick(x, 4500.0 * scale, 320.0);
        // nasal: a low resonance, the rest damped
        let nas = nasal_pole.tick(src, 260.0 * scale, 100.0);
        let tract = x * (1.0 - 0.45 * cur.nasal) + nas * 0.8 * cur.nasal;

        // the fricatives and the bursts: noise through their own resonance
        let fr = if cur.noise > 1e-4 {
            let a = nres[0].tick(noise, cur.nf.min(SR * 0.42), cur.nbw);
            let b = nres[1].tick(a, cur.nf.min(SR * 0.42), cur.nbw * 1.4);
            b * cur.noise * (cur.nbw / cur.nf.max(1.0)).min(1.0) * 1.1
        } else {
            0.0
        };
        let y = tract + fr;
        // brighter (the lips radiate the highs), and no rumble
        let hpf = y - 0.9 * hp.0 + 0.1 * hp.1;
        hp = (y, hpf);
        *o = hpf;
    }
    // to a common level
    let peak = out.iter().fold(0.0f64, |m, x| m.max(x.abs())).max(1e-9);
    for x in out.iter_mut() {
        *x = libm::tanh(1.2 * *x / peak) / libm::tanh(1.2);
    }
    out
}

/// A text said, with what makes its voice: ready for the mix.
#[derive(Debug)]
pub struct Speech {
    pub utterance: Arc<Utterance>,
    pub at: f64,
    pub timbre: Timbre,
    pub pan: f64,
    pub seed: u64,
    /// A voice from outside (`engine:`), already said: its sound, mono, at
    /// alea's rate. The utterance's times are scaled onto it.
    pub external: Option<Arc<Vec<f64>>>,
}

impl Speech {
    pub fn sound(&self) -> Vec<f64> {
        match &self.external {
            Some(v) => v.as_ref().clone(),
            None => speak(&self.utterance, self.timbre, self.seed),
        }
    }
}

// ───────────────────────────── a voice from outside ─────────────────────────────

/// What an engine is given: the text on its standard input, and these in its
/// environment. It writes a WAV to its standard output.
pub struct Ask<'a> {
    pub program: &'a std::path::Path,
    pub text: &'a str,
    pub lang: Lang,
    pub voice: &'a str,
    pub speed: f64,
    pub pitch: f64,
    pub size: f64,
}

/// Where the sounds of outside voices are kept, so that a text is said once:
/// `$ALEA_VOICES`, or `voices/` in the user's cache directory.
fn cache_dir() -> Option<std::path::PathBuf> {
    if let Ok(d) = std::env::var("ALEA_VOICES") {
        return Some(std::path::PathBuf::from(d));
    }
    let home = std::env::var("XDG_CACHE_HOME").map(std::path::PathBuf::from).ok().or_else(|| {
        std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".cache"))
    })?;
    Some(home.join("alea").join("voices"))
}

/// Runs the engine (or finds what it said last time), and gives the sound,
/// mono, at alea's rate, with silence at both ends trimmed to a few
/// milliseconds; and the voiced stretch's length.
#[cfg(not(target_arch = "wasm32"))]
pub fn ask(a: &Ask) -> Result<Vec<f64>, String> {
    use std::io::Write;
    let program = std::fs::canonicalize(a.program).map_err(|e| format!("cannot find the engine `{}`: {e}", a.program.display()))?;
    let lang = match a.lang {
        Lang::En => "en",
        Lang::Fr => "fr",
    };
    let key = {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in format!("{}\0{}\0{}\0{}\0{:.3}\0{:.1}\0{:.3}", program.display(), a.text, lang, a.voice, a.speed, a.pitch, a.size).bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{h:016x}")
    };
    let cached = cache_dir().map(|d| d.join(format!("{key}.wav")));
    let bytes = match cached.as_ref().and_then(|p| std::fs::read(p).ok()) {
        Some(b) => b,
        None => {
            let mut child = std::process::Command::new(&program)
                .env("ALEA_LANG", lang)
                .env("ALEA_VOICE", a.voice)
                .env("ALEA_SPEED", format!("{:.3}", a.speed))
                .env("ALEA_PITCH", format!("{:.1}", a.pitch))
                .env("ALEA_SIZE", format!("{:.3}", a.size))
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("cannot run the engine `{}`: {e}", a.program.display()))?;
            let text = a.text.to_string();
            let mut stdin = child.stdin.take().expect("piped");
            let writer = std::thread::spawn(move || {
                let _ = stdin.write_all(text.as_bytes());
            });
            let out = child.wait_with_output().map_err(|e| format!("the engine `{}` failed: {e}", a.program.display()))?;
            let _ = writer.join();
            if !out.status.success() || out.stdout.is_empty() {
                let err = String::from_utf8_lossy(&out.stderr);
                let err = err.trim().lines().last().unwrap_or("").to_string();
                return Err(format!("the engine `{}` gave no sound{}{}", a.program.display(), if err.is_empty() { "" } else { ": " }, err));
            }
            if let Some(p) = &cached {
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(p, &out.stdout);
            }
            out.stdout
        }
    };
    let input = crate::input::decode("voice", &bytes).map_err(|e| format!("the engine `{}` did not write a WAV: {e}", a.program.display()))?;
    let crate::input::Kind::Sound(clip) = &input.kind else {
        return Err(format!("the engine `{}` did not write a WAV", a.program.display()));
    };
    let n = (clip.duration() * SR) as usize;
    let st = clip.stereo(SR, n);
    let mut mono: Vec<f64> = st[0].iter().zip(&st[1]).map(|(l, r)| (l + r) * 0.5).collect();
    // trim the silence at both ends (under -45 dB), keeping 20 ms
    let thr = 10f64.powf(-45.0 / 20.0) * mono.iter().fold(0.0f64, |m, x| m.max(x.abs())).max(1e-9);
    let first = mono.iter().position(|x| x.abs() > thr).unwrap_or(0);
    let last = mono.iter().rposition(|x| x.abs() > thr).unwrap_or(mono.len());
    let pad = (0.02 * SR) as usize;
    let (a0, b0) = (first.saturating_sub(pad), (last + pad).min(mono.len()));
    mono.drain(b0..);
    mono.drain(..a0);
    Ok(mono)
}

#[cfg(target_arch = "wasm32")]
pub fn ask(a: &Ask) -> Result<Vec<f64>, String> {
    Err(format!("`engine: \"{}\"` needs the command line: in the browser, only alea's own voice speaks", a.program.display()))
}

impl Utterance {
    /// The same words and syllables, scaled to fit a sound of `duration`
    /// seconds (an outside voice's).
    pub fn scaled_to(&self, duration: f64) -> Utterance {
        let own = self.duration.max(1e-6);
        let k = duration / own;
        Utterance {
            segs: self.segs.iter().map(|s| Seg { start: s.start * k, dur: s.dur * k, ..*s }).collect(),
            f0: self.f0.iter().map(|&(t, r)| (t * k, r)).collect(),
            words: self.words.iter().map(|&(a, b)| (a * k, b * k)).collect(),
            syllables: self.syllables.iter().map(|&t| t * k).collect(),
            duration,
        }
    }
}

/// The phonemes of a text, written out (for tests and for `alea` itself).
pub fn transcribe(text: &str, lang: Lang) -> String {
    tokens(text, lang)
        .iter()
        .map(|t| match t {
            Token::Word(w) => w
                .iter()
                .map(|&(p, s)| {
                    format!(
                        "{}{}",
                        if s == 2 {
                            "'"
                        } else if s == 1 {
                            ","
                        } else {
                            ""
                        },
                        PHONEMES[p].name
                    )
                })
                .collect::<String>(),
            Token::Break(c) => c.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_words_by_rule_and_by_list() {
        // the stress mark is written before its vowel
        assert_eq!(transcribe("the cat", Lang::En), "D@ k'&t");
        assert_eq!(transcribe("hello world", Lang::En), "h@l'oU w'3ld");
        assert_eq!(transcribe("ship", Lang::En), "S'Ip");
        assert_eq!(transcribe("CPU", Lang::En), "s,ip,ij'u");
        assert_eq!(transcribe("42", Lang::En), "f'Orti t'u");
        assert_eq!(transcribe("Hello, world!", Lang::En), "h@l'oU , w'3ld .");
    }

    #[test]
    fn french_words() {
        assert_eq!(transcribe("bonjour", Lang::Fr), "bo~Z'uR");
        assert_eq!(transcribe("le chat", Lang::Fr), "l@ S'a");
        assert_eq!(transcribe("l'image", Lang::Fr), "lim'aZ");
        assert_eq!(transcribe("maison", Lang::Fr), "mEz'o~");
        assert_eq!(transcribe("parler", Lang::Fr), "paRl'e");
        assert_eq!(transcribe("les enfants", Lang::Fr), "le a~f'a~");
        assert_eq!(transcribe("42", Lang::Fr), "kaR'a~t d'2");
    }

    /// `ALEA_G2P=words.txt ALEA_LANG=en cargo test g2p_dump -- --ignored`: one transcription per line
    #[test]
    #[ignore]
    fn g2p_dump() {
        let path = std::env::var("ALEA_G2P").expect("ALEA_G2P");
        let lang = Lang::parse(&std::env::var("ALEA_LANG").unwrap_or("en".into())).expect("lang");
        let out: Vec<String> = std::fs::read_to_string(&path)
            .expect("read")
            .lines()
            .map(|w| transcribe(w, lang))
            .collect();
        std::fs::write(format!("{path}.out"), out.join("\n")).expect("write");
    }

    /// `ALEA_PLAN=lines.txt cargo test plan_dump -- --ignored --nocapture`: the duration of each line
    #[test]
    #[ignore]
    fn plan_dump() {
        let path = std::env::var("ALEA_PLAN").expect("ALEA_PLAN");
        let lang = Lang::parse(&std::env::var("ALEA_LANG").unwrap_or("en".into())).expect("lang");
        for line in std::fs::read_to_string(&path).expect("read").lines() {
            let u = plan(line, lang, 1.0, &[]);
            println!("{:.2}\t{line}", u.duration);
        }
    }

    #[test]
    fn phonemes_between_slashes() {
        assert_eq!(transcribe("/h@'loU/", Lang::En), "h@l'oU");
    }

    #[test]
    fn a_voice_with_its_words_in_time() {
        let u = plan("Hello world. Again?", Lang::En, 1.0, &[]);
        assert_eq!(u.words.len(), 3);
        assert!(u.words.windows(2).all(|w| w[0].1 <= w[1].0));
        assert!(u.duration > 0.8 && u.duration < 3.0, "{}", u.duration);
        let s = speak(
            &u,
            Timbre {
                pitch: 110.0,
                size: 1.0,
                rough: 0.0,
                breath: 0.1,
            },
            1,
        );
        assert_eq!(s.len(), (u.duration * SR) as usize + 1);
        assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        // it speaks: loud in the words, quiet in the pause
        let rms = |a: f64, b: f64| {
            let (a, b) = ((a * SR) as usize, (b * SR) as usize);
            (s[a..b].iter().map(|x| x * x).sum::<f64>() / (b - a) as f64).sqrt()
        };
        let pause = (u.words[1].1 + 0.1, u.words[2].0 - 0.05);
        assert!(rms(u.words[0].0, u.words[0].1) > 4.0 * rms(pause.0, pause.1));
    }

    #[test]
    fn the_same_text_the_same_voice() {
        let u = plan("alea", Lang::Fr, 1.0, &[0.0, 7.0]);
        let tb = Timbre {
            pitch: 140.0,
            size: 0.9,
            rough: 0.3,
            breath: 0.2,
        };
        assert_eq!(speak(&u, tb, 3), speak(&u, tb, 3));
    }
}
