//! General helpers for validating inputs.

use unicode_segmentation::UnicodeSegmentation;

/// Truncate to `n` unicode code points.
///
/// A normal string truncation may panic if the byte length splits a code point,
/// so this avoid the pitfall.
pub fn trunc_to_char(s: &mut String, n: usize) {
    let upto = s.char_indices().map(|(i, _)| i).nth(n).unwrap_or(s.len());
    s.truncate(upto);
}

/// Truncate to a given grapheme length.
///
/// Sometimes there is no "variation selector" after an emoji, so to prevent
/// putting too many emoji, we have to truncate just on graphemes
pub fn trunc_to_grapheme(s: &mut String, n: usize) {
    let upto = s
        .grapheme_indices(true)
        .map(|(i, _)| i)
        .nth(n)
        .unwrap_or(s.len());
    s.truncate(upto);
}
