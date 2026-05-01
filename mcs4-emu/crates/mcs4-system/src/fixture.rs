//! ROM fixture helpers.
//!
//! Fixtures are stored as text files with whitespace-separated hex bytes.
//! Lines may contain comments starting with `#`, `//`, or `;`.
//!
//! Bounds: [`parse_hex_bytes`] returns the parsed bytes verbatim and does
//! not enforce a maximum size. Callers that load the result into a
//! fixed-size ROM bank (e.g. `Mcs4System::load_rom`) are responsible for
//! verifying the slice fits the destination. The 4001 ROM is 256 bytes
//! per bank and 16 banks max (4 KiB total addressable); the 4308 ROM is
//! 1024 bytes. `load_rom` truncates silently rather than panicking, so
//! oversize fixtures degrade to "first N bytes loaded" behavior. This is
//! intentional for the fixture/REPL use-case but should NOT be relied on
//! by future callers loading user-supplied ROM images; expose a
//! `parse_hex_bytes_bounded(s, max)` variant before doing so. (Tracked
//! under debt phase D10.3.3.)

use std::path::Path;

#[derive(Debug)]
pub enum FixtureError {
    Io(std::io::Error),
    Parse { line: usize, token: String },
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureError::Io(e) => write!(f, "fixture I/O error: {e}"),
            FixtureError::Parse { line, token } => {
                write!(f, "fixture parse error on line {line}: invalid byte {token:?}")
            }
        }
    }
}

impl std::error::Error for FixtureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FixtureError::Io(e) => Some(e),
            FixtureError::Parse { .. } => None,
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
}
