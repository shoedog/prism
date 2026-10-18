//! The bundled agent skills (`skills/*/SKILL.md`) keep within the Agent Skills frontmatter limits,
//! so loaders that enforce them neither reject nor truncate a skill: `name` and a `description` of
//! at most 1024 characters (Codex truncates a longer one).

use std::path::Path;

/// The folded (`>-`) or single-line `description` value from a SKILL.md frontmatter.
fn description(skill: &str) -> String {
    let frontmatter = skill
        .strip_prefix("---\n")
        .and_then(|rest| rest.split("\n---").next())
        .expect("a frontmatter block");
    let mut lines = frontmatter
        .lines()
        .skip_while(|l| !l.starts_with("description:"));
    let first = lines.next().expect("a description field");
    let inline = first["description:".len()..].trim();
    if inline != ">-" && inline != ">" {
        return inline.to_string();
    }
    lines
        .take_while(|l| l.starts_with(' '))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_bundled_skill_description_fits_the_limit() {
    let skills = Path::new(env!("CARGO_MANIFEST_DIR")).join("skills");
    let mut checked = 0;
    for entry in std::fs::read_dir(&skills).expect("skills/") {
        let path = entry.expect("an entry").path().join("SKILL.md");
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read SKILL.md");
        let description = description(&text);
        assert!(!description.is_empty(), "{path:?}: empty description");
        assert!(
            description.chars().count() <= 1024,
            "{path:?}: description is {} characters (limit 1024)",
            description.chars().count()
        );
        checked += 1;
    }
    assert!(checked >= 2, "expected the bundled skills, found {checked}");
}
