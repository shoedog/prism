//! #585: the CPG build resolves calls through the memoized twin of the full resolver
//! (`GoSatisfierMemo::resolve_call_site_full`); method rungs must not bind across language
//! families there either.
use std::collections::BTreeMap;

fn parse(
    sources: &[(&str, &str, prism::languages::Language)],
) -> BTreeMap<String, prism::ast::ParsedFile> {
    sources
        .iter()
        .map(|(path, src, lang)| {
            (
                path.to_string(),
                prism::ast::ParsedFile::parse(path, src, *lang).unwrap(),
            )
        })
        .collect()
}

#[test]
fn cpg_has_no_cross_family_call_edge_for_qualified_method_calls() {
    // #585: the CPG build resolves through the memoized twin of the full resolver. A Python
    // `Config.load()` must not bind a Rust `impl Config`, while a same-family class still does.
    use prism::cpg::CodePropertyGraph;
    use prism::languages::Language::{Python, Rust};
    let callees = |sources: &[(&str, &str, prism::languages::Language)]| {
        let files = parse(sources);
        CodePropertyGraph::build(&files)
            .callees_of("run", "py/app.py", 1)
            .into_iter()
            .map(|(f, _)| (f.file, f.name))
            .collect::<Vec<_>>()
    };
    let app = ("py/app.py", "def run():\n    Config.load()\n", Python);
    let rust = (
        "rs/src/lib.rs",
        "pub struct Config;\nimpl Config {\n    pub fn load() {}\n}\n",
        Rust,
    );
    let models = (
        "py/models.py",
        "class Config:\n    def load():\n        pass\n",
        Python,
    );
    assert_eq!(
        callees(&[models, app]),
        vec![("py/models.py".to_string(), "load".to_string())],
        "control: a same-family class still produces the CPG call edge"
    );
    assert_eq!(
        callees(&[app, rust]),
        Vec::<(String, String)>::new(),
        "Python caller must not get a CPG call edge to the Rust impl"
    );
}
