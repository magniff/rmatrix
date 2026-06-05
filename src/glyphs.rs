//! The glyph alphabet for the digital rain.
//!
//! The film's rain is mostly mirrored half-width katakana mixed with western
//! digits and a handful of symbols. We don't mirror (terminals can't), but the
//! half-width katakana block gives the authentic look that plain ASCII can't.

use rand::Rng;

/// Build the pool of glyphs we draw from.
pub fn alphabet() -> Vec<char> {
    let mut v = Vec::new();

    // Half-width katakana (U+FF66 ..= U+FF9D): the signature Matrix glyphs.
    for c in 0xFF66u32..=0xFF9D {
        if let Some(ch) = char::from_u32(c) {
            v.push(ch);
        }
    }

    // Latin digits and a few symbols for variety / that "code" feel.
    v.extend("0123456789".chars());
    v.extend("｜<>=*+-/\\:.\"".chars());

    v
}

/// Pick a random glyph from the pool.
#[inline]
pub fn random_glyph<R: Rng>(rng: &mut R, pool: &[char]) -> char {
    pool[rng.gen_range(0..pool.len())]
}
