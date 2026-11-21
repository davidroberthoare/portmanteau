//! Turning file bytes into text.
//!
//! Scripts arrive from Windows tools as UTF-8 with a BOM, UTF-16 or plain
//! Windows-1252. Guessing wrong turns every curly quote and dash into
//! mojibake, so decode explicitly and say when we fell back.

/// How the bytes were read, for the import warnings.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decoded {
    /// UTF-8 (with or without a BOM) or UTF-16 with a BOM: no caveat.
    Clean,
    /// Not valid UTF-8, so read as Windows-1252.
    Windows1252,
}

/// Decode `bytes` and normalise line endings to `\n`.
pub(crate) fn decode(bytes: &[u8]) -> (String, Decoded) {
    let (text, how) = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        (String::from_utf8_lossy(rest).into_owned(), Decoded::Clean)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        (utf16(rest, u16::from_le_bytes), Decoded::Clean)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        (utf16(rest, u16::from_be_bytes), Decoded::Clean)
    } else {
        match std::str::from_utf8(bytes) {
            Ok(text) => (text.to_owned(), Decoded::Clean),
            Err(_) => (bytes.iter().map(|&b| windows_1252(b)).collect(), Decoded::Windows1252),
        }
    };
    let text = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{feff}', "");
    (text, how)
}

fn utf16(bytes: &[u8], read: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(|pair| read([pair[0], pair[1]])).collect();
    String::from_utf16_lossy(&units)
}

/// Windows-1252 differs from Latin-1 only in 0x80..=0x9F.
fn windows_1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ',
        '\u{9d}', 'ž', 'Ÿ',
    ];
    match byte {
        0x80..=0x9F => HIGH[(byte - 0x80) as usize],
        other => char::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_with_and_without_a_bom_is_clean() {
        assert_eq!(decode("caf\u{e9}".as_bytes()), ("caf\u{e9}".into(), Decoded::Clean));
        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.extend("caf\u{e9}".as_bytes());
        assert_eq!(decode(&with_bom).0, "caf\u{e9}");
    }

    #[test]
    fn utf16_is_read_in_either_byte_order() {
        let mut le = vec![0xFF, 0xFE];
        le.extend("Hi “there”".encode_utf16().flat_map(|u| u.to_le_bytes()));
        assert_eq!(decode(&le).0, "Hi “there”");
        let mut be = vec![0xFE, 0xFF];
        be.extend("Hi “there”".encode_utf16().flat_map(|u| u.to_be_bytes()));
        assert_eq!(decode(&be).0, "Hi “there”");
    }

    #[test]
    fn windows_1252_smart_punctuation_is_recovered_and_reported() {
        // 0x93/0x94 are curly quotes, 0x97 an em dash, 0xE9 an e-acute.
        let (text, how) = decode(&[0x93, b'c', b'a', b'f', 0xE9, 0x94, b' ', 0x97]);
        assert_eq!(text, "“café” —");
        assert_eq!(how, Decoded::Windows1252);
    }

    #[test]
    fn line_endings_are_normalised() {
        assert_eq!(decode(b"a\r\nb\rc\nd").0, "a\nb\nc\nd");
    }
}
