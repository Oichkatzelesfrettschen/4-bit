//! ROM fixture helpers.
//!
//! Fixtures are stored as text files with whitespace-separated hex bytes.
//! Lines may contain comments starting with `#`, `//`, or `;`.
//!
//! Bounds: [`parse_hex_bytes`] returns the parsed bytes verbatim and does
//! not enforce a maximum size. Use [`parse_hex_bytes_bounded`] when loading
//! user-supplied ROM images that must fit a fixed-size destination.
//! The 4001 ROM is 256 bytes per bank (16 banks max = 4 KiB total);
//! the 4308 ROM is 1024 bytes.

use std::path::Path;

#[derive(Debug)]
pub enum FixtureError {
    Io(std::io::Error),
    Parse {
        line: usize,
        token: String,
    },
    /// Parsed byte count exceeded the caller-supplied maximum.
    TooLarge {
        parsed: usize,
        max: usize,
    },
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureError::Io(e) => write!(f, "fixture I/O error: {e}"),
            FixtureError::Parse { line, token } => {
                write!(f, "fixture parse error on line {line}: invalid byte {token:?}")
            }
            FixtureError::TooLarge { parsed, max } => {
                write!(f, "fixture too large: {parsed} bytes parsed, max is {max}")
            }
        }
    }
}

impl std::error::Error for FixtureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FixtureError::Io(e) => Some(e),
            FixtureError::Parse { .. } | FixtureError::TooLarge { .. } => None,
        }
    }
}

impl From<std::io::Error> for FixtureError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Load whitespace-separated hex bytes from a text file.
pub fn load_hex_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, FixtureError> {
    let s = std::fs::read_to_string(path)?;
    parse_hex_bytes(&s)
}

/// Load whitespace-separated hex bytes from a file, failing if the byte count exceeds `max`.
pub fn load_hex_bytes_bounded(path: impl AsRef<Path>, max: usize) -> Result<Vec<u8>, FixtureError> {
    let s = std::fs::read_to_string(path)?;
    parse_hex_bytes_bounded(&s, max)
}

/// Parse whitespace-separated hex bytes from a string, failing if more than `max` bytes are found.
///
/// Use this when the destination buffer has a fixed capacity (e.g. 256 bytes for a 4001 ROM bank).
pub fn parse_hex_bytes_bounded(s: &str, max: usize) -> Result<Vec<u8>, FixtureError> {
    let bytes = parse_hex_bytes(s)?;
    if bytes.len() > max {
        return Err(FixtureError::TooLarge {
            parsed: bytes.len(),
            max,
        });
    }
    Ok(bytes)
}

/// Parse whitespace-separated hex bytes from a string.
pub fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, FixtureError> {
    let mut out = Vec::new();

    for (idx, line) in s.lines().enumerate() {
        let line_no = idx + 1;
        let mut s = line;

        if let Some((before, _)) = s.split_once("//") {
            s = before;
        }
        if let Some((before, _)) = s.split_once('#') {
            s = before;
        }
        if let Some((before, _)) = s.split_once(';') {
            s = before;
        }

        for raw in s.split_whitespace() {
            let token = raw.trim_end_matches(',').trim();
            if token.is_empty() {
                continue;
            }

            let token = token.strip_prefix("0x").unwrap_or(token);
            let token = token.strip_prefix("0X").unwrap_or(token);

            if token.len() > 2 || token.is_empty() || !token.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(FixtureError::Parse {
                    line: line_no,
                    token: raw.to_string(),
                });
            }

            let value = u8::from_str_radix(token, 16).map_err(|_| FixtureError::Parse {
                line: line_no,
                token: raw.to_string(),
            })?;
            out.push(value);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_bytes_with_comments() {
        let bytes = parse_hex_bytes(
            r#"
                # comment
                DA E0 00 ; trailing comment
                0x20 0x01, // and inline
            "#,
        )
        .expect("parse");

        assert_eq!(bytes, vec![0xDA, 0xE0, 0x00, 0x20, 0x01]);
    }

    #[test]
    fn bounded_accepts_within_limit() {
        let bytes = parse_hex_bytes_bounded("AA BB CC", 3).expect("should accept 3 bytes within limit 3");
        assert_eq!(bytes, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn bounded_accepts_below_limit() {
        let bytes = parse_hex_bytes_bounded("AA BB", 256).expect("2 bytes well within 256");
        assert_eq!(bytes.len(), 2);
    }

    #[test]
    fn bounded_rejects_oversize() {
        let input = "AA BB CC DD";
        let err = parse_hex_bytes_bounded(input, 3).expect_err("4 bytes should exceed max=3");
        match err {
            FixtureError::TooLarge { parsed, max } => {
                assert_eq!(parsed, 4);
                assert_eq!(max, 3);
            }
            other => panic!("expected TooLarge, got: {other}"),
        }
    }

    #[test]
    fn bounded_zero_max_rejects_any_byte() {
        let err = parse_hex_bytes_bounded("FF", 0).expect_err("1 byte should exceed max=0");
        assert!(matches!(err, FixtureError::TooLarge { parsed: 1, max: 0 }));
    }

    #[test]
    fn bounded_display_message() {
        let err = FixtureError::TooLarge { parsed: 300, max: 256 };
        assert!(err.to_string().contains("300"));
        assert!(err.to_string().contains("256"));
    }
}
