//! A small lexical scan of Go source for top-level function names. No AST: it
//! skips line and block comments, interpreted and raw strings and rune
//! literals, then recognizes `func`, whitespace, a name, optional whitespace,
//! and `(` or `[`. A method (`func (r R) m(`) has no name at that position and
//! is not listed. Used only to name the file that declares an unqualified or
//! package-qualified callee, from text never parsed.
use std::collections::BTreeSet;

pub(super) fn declared_funcs(source: &str) -> BTreeSet<String> {
    let bytes = source.as_bytes();
    let mut names = BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                    i += 1;
                }
                i += 2;
            }
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote && bytes[i] != b'\n' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'`' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'`' {
                    i += 1;
                }
                i += 1;
            }
            b'f' if is_func_keyword(bytes, i) => {
                i += 4;
                i = skip_space(bytes, i);
                let start = i;
                while i < bytes.len() && is_ident_byte(bytes[i]) {
                    i += 1;
                }
                if i > start && !bytes[start].is_ascii_digit() {
                    let name_end = i;
                    i = skip_space(bytes, i);
                    if matches!(bytes.get(i), Some(b'(' | b'[')) {
                        names.insert(source[start..name_end].to_string());
                    }
                }
            }
            _ => i += 1,
        }
    }
    names
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn skip_space(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
        i += 1;
    }
    i
}

/// `func` as a whole word followed by whitespace.
fn is_func_keyword(bytes: &[u8], i: usize) -> bool {
    bytes[i..].starts_with(b"func")
        && (i == 0 || !is_ident_byte(bytes[i - 1]))
        && bytes
            .get(i + 4)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
}

#[cfg(test)]
mod tests {
    use super::declared_funcs;

    fn names(source: &str) -> Vec<String> {
        declared_funcs(source).into_iter().collect()
    }

    #[test]
    fn finds_functions_generics_and_odd_spacing_but_not_methods() {
        let src = "package p\n\nfunc a() {}\nfunc  b (x int) {}\nfunc\tc[T any]() {}\nfunc (r R) d() {}\nvar f = func() {}\n";
        assert_eq!(names(src), ["a", "b", "c"]);
    }

    #[test]
    fn ignores_comments_strings_and_runes() {
        let src = "package p\n// func a() {}\n/* func b() {}\n func c() {} */\nvar s = \"func d() {}\"\nvar r = `func e() {}`\nvar q = '\"'\nfunc real() {}\nvar t = \"esc \\\" func f() {}\"\n";
        assert_eq!(names(src), ["real"]);
    }

    #[test]
    fn requires_a_whole_func_keyword() {
        assert_eq!(
            names("package p\nvar myfunc helper()\nfuncs x()\nfunc ok() {}\n"),
            ["ok"]
        );
    }
}
