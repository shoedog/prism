//! MEASUREMENT ONLY (#630 slice 6d yield measurement; never merged).
//! `PRISM_6D_MUTE=<comma list>` makes the named escape kinds stop escaping; unset = main.
//! `PRISM_6D_LOG=<file>` appends one TSV record per escape / P1 candidate (default config only).
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

const C1: &[&str] = &["c1call", "c1ctor", "c1brace"];
const C3: &[&str] = &["c3arith", "c3cast"];
const C5: &[&str] = &["c5cmp", "c5arith", "c5shift", "c5other"];

fn set() -> &'static HashSet<String> {
    static S: OnceLock<HashSet<String>> = OnceLock::new();
    S.get_or_init(|| {
        let mut out = HashSet::new();
        let v = std::env::var("PRISM_6D_MUTE").unwrap_or_default();
        for t in v
            .split(',')
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
        {
            match t.as_str() {
                "c1" => out.extend(C1.iter().map(|s| s.to_string())),
                "c3" => out.extend(C3.iter().map(|s| s.to_string())),
                "c5" => out.extend(C5.iter().map(|s| s.to_string())),
                "all" => {
                    for l in [C1, C3, C5, &["c2", "c4"][..]] {
                        out.extend(l.iter().map(|s| s.to_string()));
                    }
                }
                _ => {
                    out.insert(t);
                }
            }
        }
        out
    })
}

pub(super) fn mute(k: &str) -> bool {
    set().contains(k)
}

fn writer() -> &'static Option<Mutex<std::fs::File>> {
    static W: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    W.get_or_init(|| {
        let p = std::env::var("PRISM_6D_LOG").ok()?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .ok()
            .map(Mutex::new)
    })
}

pub(super) fn log_enabled() -> bool {
    writer().is_some()
}

thread_local! {
    static OWNER: RefCell<String> = const { RefCell::new(String::new()) };
    static SITE: RefCell<(usize, String)> = const { RefCell::new((0, String::new())) };
}

pub(super) fn set_owner(file: &str, start: usize, end: usize) {
    if log_enabled() {
        OWNER.with(|o| *o.borrow_mut() = format!("{file}\t{start}\t{end}"));
    }
}

pub(super) fn set_site(line: usize, extra: String) {
    if log_enabled() {
        SITE.with(|s| *s.borrow_mut() = (line, extra));
    }
}

fn raw(rec: &str) {
    if let Some(w) = writer() {
        if let Ok(mut f) = w.lock() {
            let _ = f.write_all(rec.as_bytes());
        }
    }
}

pub(super) fn log_escape(kind: &str, name: &str) {
    if !log_enabled() {
        return;
    }
    let owner = OWNER.with(|o| o.borrow().clone());
    let (line, extra) = SITE.with(|s| s.borrow().clone());
    raw(&format!("E\t{owner}\t{kind}\t{name}\t{line}\t{extra}\n"));
}

pub(super) fn log_p1(file: &str, line: usize, ty: &str, names: &str) {
    raw(&format!("P1\t{file}\t{line}\t{ty}\t{names}\n"));
}

/// Text with tabs/newlines removed, truncated.
pub(super) fn clean(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .take(100)
        .collect()
}

// ---- 6d-2 prototype: PRISM_6D_ALLOW=<file> gates the c2 and c4 lifts (measurement only) ----
// Sections: `[c2]` bare spellings, `[c4-strict]` type names, `[c4-structural]` one line `on`.
// Variable unset: the lifts are unfiltered (as before). Variable set: a lift needs an allowed entry.
struct Allow {
    c2: HashSet<String>,
    c4: HashSet<String>,
    structural: bool,
}

fn allow() -> &'static Option<Allow> {
    static A: OnceLock<Option<Allow>> = OnceLock::new();
    A.get_or_init(|| {
        let p = std::env::var("PRISM_6D_ALLOW").ok()?;
        let text = std::fs::read_to_string(p).expect("PRISM_6D_ALLOW unreadable");
        let mut a = Allow {
            c2: HashSet::new(),
            c4: HashSet::new(),
            structural: false,
        };
        let mut sec = String::new();
        for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if l.starts_with('[') {
                sec = l.to_string();
                continue;
            }
            match sec.as_str() {
                "[c2]" => {
                    a.c2.insert(l.to_string());
                }
                "[c4-strict]" => {
                    a.c4.insert(l.to_string());
                }
                "[c4-structural]" => a.structural = l == "on",
                _ => {}
            }
        }
        Some(a)
    })
}

/// c2 lift for the normalized bare spelling.
pub(super) fn allow_c2(norm: &str) -> bool {
    match allow() {
        None => true,
        Some(a) => a.c2.contains(norm),
    }
}

/// c4 lift for a declaration: `ty` = text of a bare `type_identifier` type (else None),
/// `structural` = the shape cannot be an expression whatever the type names.
pub(super) fn allow_c4(ty: Option<&str>, structural: bool) -> bool {
    match allow() {
        None => true,
        Some(a) => {
            (structural && a.structural) || ty.is_some_and(|t| a.c4.contains(t))
        }
    }
}
