use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PointerToken {
    Property(String),
    ListItem,
    MapValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerEncodingError;

impl fmt::Display for PointerEncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid pointer segment encoding")
    }
}

impl std::error::Error for PointerEncodingError {}

impl PointerToken {
    pub fn property(value: impl Into<String>) -> Self {
        Self::Property(value.into())
    }

    pub fn decode(segment: &str) -> Result<Self, PointerEncodingError> {
        match segment {
            "[]" => Ok(Self::ListItem),
            "*" => Ok(Self::MapValue),
            property => {
                let name = decode_pointer_segment(&percent_decode(property)?)?;
                let token = Self::Property(name);
                if token.to_string() != segment {
                    return Err(PointerEncodingError);
                }
                Ok(token)
            }
        }
    }
}

impl fmt::Display for PointerToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ListItem => f.write_str("[]"),
            Self::MapValue => f.write_char('*'),
            Self::Property(name) => {
                for byte in name.bytes() {
                    match byte {
                        b'~' => f.write_str("~0")?,
                        b'/' => f.write_str("~1")?,
                        byte if byte.is_ascii_alphanumeric() || b"-._".contains(&byte) => {
                            f.write_char(char::from(byte))?;
                        }
                        byte => write!(f, "%{byte:02X}")?,
                    }
                }
                Ok(())
            }
        }
    }
}

fn percent_decode(text: &str) -> Result<String, PointerEncodingError> {
    let mut input = text.bytes();
    let mut bytes = Vec::with_capacity(text.len());
    while let Some(byte) = input.next() {
        let decoded = if byte == b'%' {
            let hex = |b: u8| char::from(b).to_digit(16).map(|i| i as u8);
            let high = input.next().and_then(hex).ok_or(PointerEncodingError)?;
            let low = input.next().and_then(hex).ok_or(PointerEncodingError)?;
            16 * high + low
        } else {
            byte
        };
        bytes.push(decoded);
    }
    String::from_utf8(bytes).map_err(|_| PointerEncodingError)
}

fn decode_pointer_segment(segment: &str) -> Result<String, PointerEncodingError> {
    let mut output = String::new();
    let mut chars = segment.chars();
    while let Some(ch) = chars.next() {
        output.push(match ch {
            '~' => match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                _ => return Err(PointerEncodingError),
            },
            ch => ch,
        });
    }
    Ok(output)
}

/// Decode a URI-fragment JSON Pointer. An empty fragment names the resource root.
pub fn decode_fragment_pointer(fragment: &str) -> Result<Vec<String>, PointerEncodingError> {
    let fragment = percent_decode(fragment)?;
    if fragment.is_empty() {
        return Ok(Vec::new());
    }
    fragment
        .strip_prefix('/')
        .ok_or(PointerEncodingError)?
        .split('/')
        .map(decode_pointer_segment)
        .collect()
}
