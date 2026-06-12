//! The glyph alphabet for the digital rain.
//!
//! The film's rain is mostly mirrored half-width katakana mixed with Latin
//! letters, mirrored numerals, and the odd kanji. Terminals can't mirror
//! glyphs, so for the numerals we substitute Unicode lookalikes where a
//! convincing one exists (Ƨ Ɛ Г for 2 3 7) and keep the rest upright.
//!
//! Kanji are double-width in terminals — they paint two columns. The pool
//! is laid out narrow-first so callers can exclude wide glyphs where a
//! second column isn't available (see `alphabet`'s returned split point).

use rand::Rng;

/// A few kanji for flavor, kept rare. All double-width.
const KANJI: &[char] = &['日', '月', '木', '中', '口'];

/// Build the pool of glyphs we draw from.
///
/// Returns the pool and the number of leading *narrow* (single-column)
/// glyphs: `pool[..narrow]` is safe anywhere, `pool[narrow..]` is the
/// double-width kanji tail.
pub fn alphabet() -> (Vec<char>, usize) {
    let mut v = Vec::new();

    // Half-width katakana (U+FF66 ..= U+FF9D): the signature Matrix glyphs.
    for c in 0xFF66u32..=0xFF9D {
        if let Some(ch) = char::from_u32(c) {
            v.push(ch);
        }
    }

    // Latin capitals.
    v.extend('A'..='Z');

    // "Mirrored" Arabic numerals — lookalike codepoints where Unicode has
    // one, upright otherwise.
    v.extend("01ƧƐ456Г89".chars());

    let narrow = v.len();
    v.extend(KANJI);

    (v, narrow)
}

/// Whether a glyph paints two terminal columns (CJK ideographs).
#[inline]
pub fn is_wide(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF)
}

/// Pick a random glyph from the pool.
#[inline]
pub fn random_glyph<R: Rng>(rng: &mut R, pool: &[char]) -> char {
    pool[rng.gen_range(0..pool.len())]
}
