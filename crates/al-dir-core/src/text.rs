//! Text normalisation shared by keyword, title and URL matching.

/// Normalises text for "contains" matching: lower-cases, folds Arabic
/// spelling variants (hamza forms of alef, alef maqsura, ta marbuta),
/// drops Arabic diacritics and tatweel, and collapses whitespace.
///
/// Both patterns and inputs go through this, so a rule written as
/// "أفلام" also matches "افلام" or "أَفْلام".
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_space = false;
    for ch in input.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        let ch = match ch {
            // Harakat, Quranic marks, superscript alef, tatweel.
            '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}' => continue,
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            'ى' => 'ي',
            'ة' => 'ه',
            c => c,
        };
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// Decodes `%XX` escapes (invalid UTF-8 is replaced). When `plus_as_space`
/// is set, `+` decodes to a space as in form-encoded query strings.
pub fn percent_decode(input: &str, plus_as_space: bool) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                match (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    (Some(h), Some(l)) => {
                        out.push(h << 4 | l);
                        i += 3;
                        continue;
                    }
                    _ => out.push(b'%'),
                }
            }
            b'+' if plus_as_space => out.push(b' '),
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0xf) as usize] as char);
    }
    s
}

pub(crate) fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .chunks(2)
        .map(|pair| Some(hex_val(pair[0])? << 4 | hex_val(pair[1])?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_space() {
        assert_eq!(normalize("  Hello   WORLD \n"), "hello world");
    }

    #[test]
    fn folds_arabic_variants() {
        assert_eq!(normalize("أَفْلامٌ"), normalize("افلام"));
        assert_eq!(normalize("إسلام"), "اسلام");
        assert_eq!(normalize("مكتبة"), "مكتبه");
        assert_eq!(normalize("مستشفى"), "مستشفي");
        assert_eq!(normalize("كـــتاب"), "كتاب");
    }

    #[test]
    fn decodes_percent() {
        assert_eq!(percent_decode("a%20b+c", true), "a b c");
        assert_eq!(percent_decode("a%20b+c", false), "a b+c");
        assert_eq!(percent_decode("%D8%A7", false), "ا");
        assert_eq!(percent_decode("100%", false), "100%");
        assert_eq!(percent_decode("%zz", false), "%zz");
    }

    #[test]
    fn hex_round_trip() {
        let data = [0u8, 1, 0xab, 0xff];
        assert_eq!(from_hex(&to_hex(&data)).unwrap(), data);
        assert!(from_hex("abc").is_none());
        assert!(from_hex("zz").is_none());
    }
}
