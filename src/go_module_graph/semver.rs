pub(super) fn semver_is_valid(version: &str) -> bool {
    let bytes = version.as_bytes();
    if bytes.first() != Some(&b'v') {
        return false;
    }

    let mut index = 1;
    if !parse_integer(bytes, &mut index) {
        return false;
    }
    if index == bytes.len() {
        return true;
    }
    if !take_byte(bytes, &mut index, b'.') || !parse_integer(bytes, &mut index) {
        return false;
    }
    if index == bytes.len() {
        return true;
    }
    if !take_byte(bytes, &mut index, b'.') || !parse_integer(bytes, &mut index) {
        return false;
    }
    if take_byte(bytes, &mut index, b'-') && !parse_identifiers(bytes, &mut index, true, true) {
        return false;
    }
    if take_byte(bytes, &mut index, b'+') && !parse_identifiers(bytes, &mut index, false, false) {
        return false;
    }
    index == bytes.len()
}

fn parse_integer(bytes: &[u8], index: &mut usize) -> bool {
    let start = *index;
    while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
        *index += 1;
    }
    start != *index && (*index - start == 1 || bytes[start] != b'0')
}

fn parse_identifiers(
    bytes: &[u8],
    index: &mut usize,
    stop_at_build: bool,
    reject_leading_zero_numeric: bool,
) -> bool {
    let mut start = *index;
    loop {
        let at_end = *index == bytes.len();
        let at_build = stop_at_build && bytes.get(*index) == Some(&b'+');
        if at_end || at_build {
            return valid_identifier(bytes, start, *index, reject_leading_zero_numeric);
        }
        match bytes[*index] {
            b'.' => {
                if !valid_identifier(bytes, start, *index, reject_leading_zero_numeric) {
                    return false;
                }
                *index += 1;
                start = *index;
            }
            byte if is_identifier_char(byte) => *index += 1,
            _ => return false,
        }
    }
}

fn valid_identifier(bytes: &[u8], start: usize, end: usize, reject_leading_zero: bool) -> bool {
    if start == end {
        return false;
    }
    let identifier = &bytes[start..end];
    !(reject_leading_zero
        && identifier.len() > 1
        && identifier[0] == b'0'
        && identifier.iter().all(u8::is_ascii_digit))
}

fn is_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

fn take_byte(bytes: &[u8], index: &mut usize, expected: u8) -> bool {
    if bytes.get(*index) != Some(&expected) {
        return false;
    }
    *index += 1;
    true
}
