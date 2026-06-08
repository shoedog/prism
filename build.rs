fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let lock = std::fs::read_to_string(format!("{manifest}/Cargo.lock")).unwrap_or_default();
    let mut entries: Vec<String> = Vec::new(); // "name@version" — Vec, not a map, so dup grammars don't collapse (R2-m11)
    let mut cur: Option<String> = None;
    for line in lock.lines() {
        let l = line.trim();
        if let Some(r) = l.strip_prefix("name = \"") {
            cur = r.strip_suffix('"').map(String::from);
        } else if let Some(r) = l.strip_prefix("version = \"") {
            if let Some(n) = &cur {
                if n.starts_with("tree-sitter") {
                    if let Some(v) = r.strip_suffix('"') {
                        entries.push(format!("{n}@{v}"));
                    }
                }
            }
        }
    }
    entries.sort();
    entries.dedup();
    let joined = if entries.is_empty() {
        let version =
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown-version".into());
        println!(
            "cargo:warning=build.rs: no tree-sitter-* crates in Cargo.lock; using fallback grammar fingerprint from CARGO_PKG_VERSION plus no-grammar-lock marker"
        );
        format!("prism@{version};no-grammar-lock")
    } else {
        entries.join(";")
    };
    let mut h: u64 = 0xcbf29ce484222325;
    for b in joined.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    println!("cargo:rustc-env=GRAMMAR_FINGERPRINT={h:016x}");
}
