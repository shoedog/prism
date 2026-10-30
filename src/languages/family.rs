use super::Language;

/// Families supported by free-function lookup: C/C++ and JS/TS/TSX interoperate.
/// Every other language only shares a family with itself.
pub fn same_language_family(left: Language, right: Language) -> bool {
    use Language::*;
    left == right
        || matches!((left, right), (C | Cpp, C | Cpp))
        || matches!(
            (left, right),
            (JavaScript | TypeScript | Tsx, JavaScript | TypeScript | Tsx)
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ordered_language_pair() {
        use Language::*;
        let families: &[&[Language]] = &[
            &[C, Cpp],
            &[JavaScript, TypeScript, Tsx],
            &[Python],
            &[Go],
            &[Java],
            &[Rust],
            &[Lua],
            &[Terraform],
            &[Bash],
        ];
        for left in Language::all() {
            for right in Language::all() {
                let expected = families
                    .iter()
                    .any(|f| f.contains(&left) && f.contains(&right));
                assert_eq!(
                    same_language_family(left, right),
                    expected,
                    "{left:?} -> {right:?}"
                );
            }
        }
        assert_eq!(
            families.iter().map(|f| f.len()).sum::<usize>(),
            Language::all().len()
        );
    }
}
