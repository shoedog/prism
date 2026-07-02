use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

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

    // Build identity (Tier-A spec §2.3). IDENTITY GRAMMAR — single owner is this
    // comment; consumers MUST cite it: tests/cli/version_test.rs and the eval
    // harness's parse_version (eval/tier_a/sut.py):
    //     version line = "slicing <semver> (<identity>)"
    //     identity     = <sha:[0-9a-f]{12,40}>["-dirty"]  |  "unknown"
    // "unknown" = gitless build (source archive, no git in PATH); consumers treat it
    // as not-verifiable. Staleness model: rerun triggers cover (a) source edits
    // within a commit (src/ watch — the binary's actual inputs), (b) commits/branch
    // moves and (c) gc/pack-refs (resolved gitdir HEAD + loose ref + packed-refs;
    // worktree-safe via `git rev-parse --git-dir`). The eval harness ALSO checks the
    // repo's HEAD and dirtiness directly at run time — the embedded identity is the
    // binary's claim about what it was built from, not the sole source of truth.
    println!("cargo:rerun-if-changed={manifest}/src");
    println!("cargo:rerun-if-changed={manifest}/build.rs");
    println!("cargo:rerun-if-changed={manifest}/Cargo.toml");
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&manifest)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if let Some(gd) = git(&["rev-parse", "--git-dir"]) {
        let gd = if std::path::Path::new(&gd).is_absolute() {
            gd
        } else {
            format!("{manifest}/{gd}")
        };
        println!("cargo:rerun-if-changed={gd}/HEAD");
        println!("cargo:rerun-if-changed={gd}/packed-refs");
        if let Ok(head) = std::fs::read_to_string(format!("{gd}/HEAD")) {
            if let Some(r) = head.trim().strip_prefix("ref: ") {
                println!("cargo:rerun-if-changed={gd}/{r}");
            }
        }
    }
    let sha = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain", "-uno"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    println!(
        "cargo:rustc-env=GIT_SHA={sha}{}",
        if sha != "unknown" && dirty {
            "-dirty"
        } else {
            ""
        }
    );

    let manifest_path = Path::new(&manifest);
    let binary_inputs = binary_input_paths(manifest_path);
    let binary_input_dirty = git_binary_input_dirty(&manifest).unwrap_or(false);
    println!(
        "cargo:rustc-env=PRISM_BINARY_INPUT_DIRTY={}",
        if binary_input_dirty { "1" } else { "0" }
    );
    println!(
        "cargo:rustc-env=PRISM_CACHE_BUILD_IDENTITY={}",
        binary_input_fingerprint(manifest_path, &binary_inputs)
    );
}

fn binary_input_paths(manifest: &Path) -> Vec<String> {
    let mut paths = BTreeSet::new();
    if let Some(listed) = git(
        &manifest.to_string_lossy(),
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "src",
            "build.rs",
            "Cargo.toml",
            "Cargo.lock",
        ],
    ) {
        for line in listed.lines() {
            let path = line.trim();
            if !path.is_empty() {
                paths.insert(path.to_string());
            }
        }
    }

    if paths.is_empty() {
        collect_fallback_inputs(manifest, &mut paths);
    }

    paths.into_iter().collect()
}

fn collect_fallback_inputs(root: &Path, paths: &mut BTreeSet<String>) {
    for name in ["build.rs", "Cargo.toml", "Cargo.lock"] {
        if root.join(name).is_file() {
            paths.insert(name.to_string());
        }
    }
    let src = root.join("src");
    if !src.exists() {
        return;
    }
    collect_src_tree(root, &src, paths);
}

fn collect_src_tree(root: &Path, dir: &Path, paths: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_src_tree(root, &path, paths);
        } else if path.is_file() {
            insert_relative(root, &path, paths);
        }
    }
}

fn insert_relative(root: &Path, path: &Path, paths: &mut BTreeSet<String>) {
    if let Ok(rel) = path.strip_prefix(root) {
        paths.insert(rel.to_string_lossy().replace('\\', "/"));
    }
}

fn git_binary_input_dirty(manifest: &str) -> Option<bool> {
    git(
        manifest,
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--",
            "src",
            "build.rs",
            "Cargo.toml",
            "Cargo.lock",
        ],
    )
    .map(|s| !s.is_empty())
}

fn binary_input_fingerprint(manifest: &Path, paths: &[String]) -> String {
    let mut hasher = Sha256::new();
    for rel in paths {
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        let path = manifest.join(rel);
        match std::fs::read(&path) {
            Ok(bytes) => hasher.update(bytes),
            Err(_) => hasher.update(b"<missing>"),
        }
        hasher.update([0xff]);
    }
    format!("{:x}", hasher.finalize())
}

fn git(manifest: &str, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(manifest)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
