//! A small lexical scan of Go source for top-level function names. No AST: it
//! skips line and block comments, interpreted and raw strings and rune
//! literals, then recognizes `func`, a name and `(` or `[`, with whitespace and
//! comments allowed between the tokens (but no newline after the name). A
//! method (`func (r R) m(`) has no name at that position and is not listed.
//! Used only to name the file that declares an unqualified or
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
                // Comments between declaration tokens are trivia, as is a
                // newline before the name.
                (i, _) = skip_trivia(bytes, i);
                let start = i;
                while i < bytes.len() && is_ident_byte(bytes[i]) {
                    i += 1;
                }
                if i > start && !bytes[start].is_ascii_digit() {
                    let name_end = i;
                    // A newline after the name inserts a semicolon: no declaration.
                    let (next, crossed_newline) = skip_trivia(bytes, i);
                    i = next;
                    if !crossed_newline && matches!(bytes.get(i), Some(b'(' | b'[')) {
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

/// Skip whitespace and comments from `i`. The flag is true when a newline was
/// crossed: directly, ending a line comment, or inside a block comment.
fn skip_trivia(bytes: &[u8], mut i: usize) -> (usize, bool) {
    let mut newline = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                newline = true;
                i += 1;
            }
            b' ' | b'\t' | b'\r' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < bytes.len() && !(bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/')) {
                    newline |= bytes[i] == b'\n';
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            }
            _ => break,
        }
    }
    (i, newline)
}

/// `func` as a whole word (not a prefix of a longer identifier).
fn is_func_keyword(bytes: &[u8], i: usize) -> bool {
    bytes[i..].starts_with(b"func")
        && (i == 0 || !is_ident_byte(bytes[i - 1]))
        && !bytes.get(i + 4).is_some_and(|b| is_ident_byte(*b))
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
            names("package p\nvar myfunc helper()\nfuncs x()\nvar _ = funcs(1)\nfunc ok() {}\n"),
            ["ok"]
        );
    }

    #[test]
    fn comments_between_declaration_tokens_are_trivia() {
        let src = "package p\nfunc/* c */a() {}\nfunc b /* c */(x int) {}\nfunc /* c */ c /* d */ [T any]() {}\nfunc /* func fake() {} */ real() {}\nfunc // line\n d() {}\n";
        assert_eq!(names(src), ["a", "b", "c", "d", "real"]);
    }

    #[test]
    fn a_newline_after_the_name_ends_the_declaration() {
        // A semicolon is inserted after the identifier: `func a;` is no
        // declaration, in a line comment or a multi-line block comment alike.
        let src =
            "package p\nfunc a // c\n() {}\nfunc b /* x\ny */ () {}\nfunc c\n() {}\nfunc ok() {}\n";
        assert_eq!(names(src), ["ok"]);
    }
}
