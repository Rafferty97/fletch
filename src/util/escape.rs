use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("{kind}")]
pub struct UnescapeError {
    pub position: usize,
    pub kind: UnescapeErrorKind,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum UnescapeErrorKind {
    /// Invalid escape sequence
    #[error("Invalid escape sequence '{sequence}'")]
    InvalidEscape { sequence: String },
    /// Incomplete escape sequence at end of string
    #[error("Incomplete escape sequence")]
    IncompleteEscape,
    /// Invalid unicode escape sequence
    #[error("Invalid unicode codepoint U+{value:X}")]
    InvalidUnicode { value: u32 },
    /// Unexpected end of string
    #[error("Unexpected end of string")]
    UnexpectedEnd,
}

impl UnescapeError {
    fn new(position: usize, kind: UnescapeErrorKind) -> Self {
        Self { position, kind }
    }
}

/// Unescapes a string, resolving escape sequences like "\n", "\t", "\u{...}", etc.
pub fn unescape(raw: &str) -> Result<String, UnescapeError> {
    let bytes = raw.as_bytes();
    let mut result = String::with_capacity(raw.len());
    let mut i = 0;

    while i < bytes.len() {
        // Find the next backslash and copy everything up to it
        if let Some(backslash_pos) = bytes[i..].iter().position(|&b| b == b'\\') {
            let absolute_pos = i + backslash_pos;
            // Copy all characters from current position up to the backslash
            if backslash_pos > 0 {
                result.push_str(
                    std::str::from_utf8(&bytes[i..absolute_pos])
                        .expect("input should be valid UTF-8"),
                );
            }
            i = absolute_pos;
        } else {
            // No more backslashes, copy the rest of the string
            result.push_str(std::str::from_utf8(&bytes[i..]).expect("input should be valid UTF-8"));
            break;
        }

        // We found a backslash
        i += 1;
        if i >= bytes.len() {
            return Err(UnescapeError::new(
                i - 1,
                UnescapeErrorKind::IncompleteEscape,
            ));
        }

        match bytes[i] {
            b'0' => {
                result.push('\0');
                i += 1;
            }
            b'n' => {
                result.push('\n');
                i += 1;
            }
            b'r' => {
                result.push('\r');
                i += 1;
            }
            b't' => {
                result.push('\t');
                i += 1;
            }
            b'\\' | b'\'' | b'"' => {
                result.push(bytes[i] as char);
                i += 1;
            }
            b'x' => {
                // \x## - ASCII escape (2 hex digits)
                i += 1;
                if i + 1 >= bytes.len() {
                    return Err(UnescapeError::new(
                        i - 2,
                        UnescapeErrorKind::IncompleteEscape,
                    ));
                }

                let hex_str = std::str::from_utf8(&bytes[i..i + 2]).map_err(|_| {
                    UnescapeError::new(
                        i - 2,
                        UnescapeErrorKind::InvalidEscape {
                            sequence: format!(
                                "\\x{}",
                                String::from_utf8_lossy(&bytes[i..i.min(i + 2)])
                            ),
                        },
                    )
                })?;

                let value = u8::from_str_radix(hex_str, 16).map_err(|_| {
                    UnescapeError::new(
                        i - 2,
                        UnescapeErrorKind::InvalidEscape { sequence: format!("\\x{}", hex_str) },
                    )
                })?;

                result.push(value as char);
                i += 2;
            }
            b'u' => {
                // \u{######} - Unicode escape (1-6 hex digits)
                i += 1;
                if i >= bytes.len() || bytes[i] != b'{' {
                    return Err(UnescapeError::new(
                        i - 2,
                        UnescapeErrorKind::InvalidEscape { sequence: "\\u".to_string() },
                    ));
                }
                i += 1; // skip '{'

                let start = i;
                while i < bytes.len() && bytes[i] != b'}' {
                    if !bytes[i].is_ascii_hexdigit() {
                        return Err(UnescapeError::new(
                            start - 3,
                            UnescapeErrorKind::InvalidEscape {
                                sequence: format!(
                                    "\\u{{{}}}",
                                    String::from_utf8_lossy(&bytes[start..i + 1])
                                ),
                            },
                        ));
                    }
                    i += 1;
                }

                if i >= bytes.len() {
                    return Err(UnescapeError::new(
                        start - 3,
                        UnescapeErrorKind::UnexpectedEnd,
                    ));
                }

                let hex_str = std::str::from_utf8(&bytes[start..i]).map_err(|_| {
                    UnescapeError::new(
                        start - 3,
                        UnescapeErrorKind::InvalidEscape {
                            sequence: format!(
                                "\\u{{{}}}",
                                String::from_utf8_lossy(&bytes[start..i])
                            ),
                        },
                    )
                })?;

                if hex_str.is_empty() || hex_str.len() > 6 {
                    return Err(UnescapeError::new(
                        start - 3,
                        UnescapeErrorKind::InvalidEscape {
                            sequence: format!("\\u{{{}}}", hex_str),
                        },
                    ));
                }

                let value = u32::from_str_radix(hex_str, 16).map_err(|_| {
                    UnescapeError::new(
                        start - 3,
                        UnescapeErrorKind::InvalidEscape {
                            sequence: format!("\\u{{{}}}", hex_str),
                        },
                    )
                })?;

                let ch = char::from_u32(value).ok_or(UnescapeError::new(
                    start - 3,
                    UnescapeErrorKind::InvalidUnicode { value },
                ))?;

                result.push(ch);
                i += 1; // skip '}'
            }
            _ => {
                return Err(UnescapeError::new(
                    i - 1,
                    UnescapeErrorKind::InvalidEscape {
                        sequence: format!("\\{}", bytes[i] as char),
                    },
                ));
            }
        }
    }

    Ok(result)
}

/// Escapes a string, converting special characters to escape sequences like "\n", "\t", etc.
/// This is the inverse operation of `unescape`.
#[allow(unused)]
pub fn escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for ch in s.chars() {
        match ch {
            '\n' => result.push_str(r"\n"),
            '\r' => result.push_str(r"\r"),
            '\t' => result.push_str(r"\t"),
            '\\' => result.push_str(r"\\"),
            '"' => result.push_str(r#"\""#),
            '\'' => result.push_str(r"\'"),
            '\0' => result.push_str(r"\0"),
            // For control characters and non-printable ASCII, use hex escape
            ch if ch.is_ascii_control() && ch != '\n' && ch != '\r' && ch != '\t' && ch != '\0' => {
                result.push_str(&format!(r"\x{:02X}", ch as u8));
            }
            // For non-ASCII characters that might need escaping, use unicode escape
            ch if !ch.is_ascii() && (ch as u32) > 0x7F => {
                result.push_str(&format!(r"\u{{{:X}}}", ch as u32));
            }
            // Regular printable characters
            ch => result.push(ch),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_escapes() {
        assert_eq!(unescape("hello world").unwrap(), "hello world");
        assert_eq!(unescape("").unwrap(), "");
    }

    #[test]
    fn test_simple_escapes() {
        assert_eq!(unescape(r"hello\nworld").unwrap(), "hello\nworld");
        assert_eq!(unescape(r"tab\there").unwrap(), "tab\there");
        assert_eq!(unescape(r#"quote\"here"#).unwrap(), "quote\"here");
        assert_eq!(unescape(r"back\\slash").unwrap(), "back\\slash");
        assert_eq!(unescape(r#"null\0char"#).unwrap(), "null\0char");
    }

    #[test]
    fn test_hex_escapes() {
        assert_eq!(unescape(r"\x41").unwrap(), "A");
        assert_eq!(unescape(r"\x20").unwrap(), " ");
        assert_eq!(unescape(r"hello\x20world").unwrap(), "hello world");
    }

    #[test]
    fn test_unicode_escapes() {
        assert_eq!(unescape(r"\u{41}").unwrap(), "A");
        assert_eq!(unescape(r"\u{1F600}").unwrap(), "\u{1F600}");
        assert_eq!(unescape(r"\u{03B1}").unwrap(), "\u{03B1}");
        assert_eq!(unescape(r"hello\u{20}world").unwrap(), "hello world");
    }

    #[test]
    fn test_multiple_escapes() {
        assert_eq!(unescape(r"a\nb\tc\rd").unwrap(), "a\nb\tc\rd");
        assert_eq!(unescape(r#"\"\'\\\0"#).unwrap(), "\"\'\\\0");
    }

    #[test]
    fn test_incomplete_escape() {
        assert!(matches!(
            unescape(r"hello\"),
            Err(UnescapeError { kind: UnescapeErrorKind::IncompleteEscape, .. })
        ));
        assert!(matches!(
            unescape(r"hello\x"),
            Err(UnescapeError { kind: UnescapeErrorKind::IncompleteEscape, .. })
        ));
        assert!(matches!(
            unescape(r"hello\x4"),
            Err(UnescapeError { kind: UnescapeErrorKind::IncompleteEscape, .. })
        ));
    }

    #[test]
    fn test_invalid_escape() {
        assert!(matches!(
            unescape(r"\q"),
            Err(UnescapeError { kind: UnescapeErrorKind::InvalidEscape { .. }, .. })
        ));
        assert!(matches!(
            unescape(r"\xGG"),
            Err(UnescapeError { kind: UnescapeErrorKind::InvalidEscape { .. }, .. })
        ));
    }

    #[test]
    fn test_invalid_unicode() {
        assert!(matches!(
            unescape(r"\u{D800}"),
            Err(UnescapeError { kind: UnescapeErrorKind::InvalidUnicode { .. }, .. })
        ));
        assert!(matches!(
            unescape(r"\u{110000}"),
            Err(UnescapeError { kind: UnescapeErrorKind::InvalidUnicode { .. }, .. })
        ));
    }

    #[test]
    fn test_malformed_unicode() {
        assert!(matches!(
            unescape(r"\u{"),
            Err(UnescapeError { kind: UnescapeErrorKind::UnexpectedEnd, .. })
        ));
        assert!(matches!(
            unescape(r"\u{}"),
            Err(UnescapeError { kind: UnescapeErrorKind::InvalidEscape { .. }, .. })
        ));
        assert!(matches!(
            unescape(r"\u{1234567}"),
            Err(UnescapeError { kind: UnescapeErrorKind::InvalidEscape { .. }, .. })
        ));
    }

    #[test]
    fn test_escape_simple() {
        assert_eq!(escape("hello world"), "hello world");
        assert_eq!(escape(""), "");
    }

    #[test]
    fn test_escape_special_chars() {
        assert_eq!(escape("hello\nworld"), r"hello\nworld");
        assert_eq!(escape("tab\there"), r"tab\there");
        assert_eq!(escape("quote\"here"), r#"quote\"here"#);
        assert_eq!(escape("back\\slash"), r"back\\slash");
        assert_eq!(escape("null\0char"), r"null\0char");
        assert_eq!(escape("carriage\rreturn"), r"carriage\rreturn");
    }

    #[test]
    fn test_escape_roundtrip() {
        let test_cases = vec![
            "hello world",
            "hello\nworld",
            "tab\there",
            r#"quote"here"#,
            "back\\slash",
            "null\0char",
            "multiple\n\t\r\\escapes",
        ];

        for test in test_cases {
            let escaped = escape(test);
            let unescaped = unescape(&escaped).unwrap();
            assert_eq!(test, unescaped, "Roundtrip failed for: {:?}", test);
        }
    }

    #[test]
    fn test_escape_control_chars() {
        // Test that control characters are properly escaped
        let s = "hello\x01\x02world";
        let escaped = escape(s);
        assert!(escaped.contains(r"\x"));
        assert_eq!(unescape(&escaped).unwrap(), s);
    }

    #[test]
    fn test_escape_unicode() {
        // Test unicode characters
        let s = "hello α world 😀";
        let escaped = escape(s);
        // Should contain unicode escapes for non-ASCII
        assert!(escaped.contains(r"\u{"));
        assert_eq!(unescape(&escaped).unwrap(), s);
    }
}
