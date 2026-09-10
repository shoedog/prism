fn main() {
    let root_dir = std::path::Path::new(".");
    let typescript_dir = root_dir.join("typescript").join("src");
    let tsx_dir = root_dir.join("tsx").join("src");
    let common_dir = root_dir.join("common");

    // Watch headers as well as C sources; same-version vendored header changes
    // must rebuild the native library, not just Prism's cache identity.
    for dir in [&typescript_dir, &tsx_dir, &common_dir] {
        println!("cargo:rerun-if-changed={}", dir.display());
    }

    let mut config = cc::Build::new();
    config.include(&typescript_dir);
    config
        .flag_if_supported("-std=c11")
        .flag_if_supported("-Wno-unused-parameter");

    for path in &[
        typescript_dir.join("parser.c"),
        typescript_dir.join("scanner.c"),
        tsx_dir.join("parser.c"),
        tsx_dir.join("scanner.c"),
    ] {
        config.file(path);
        println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
    }

    println!(
        "cargo:rerun-if-changed={}",
        common_dir.join("scanner.h").to_str().unwrap()
    );

    config.compile("tree-sitter-typescript");
}
