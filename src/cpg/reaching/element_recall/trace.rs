//! `PRISM_ELEMENT_RECALL_TRACE=1`: versioned stderr records of the I0 inventory and its physical
//! joins, Python only. Unset or any other value, no analysis runs and nothing is written. The
//! trace never changes the graph, the statistics or the review output.
//!
//! Line: `element-recall\tv1\t<json>`. `kind` is `owner` (one per owner holding a candidate store),
//! `write` (every indexed store candidate, refused ones included) or `node` (every occurrence of a
//! store's root). `join` is the endpoint verdict (`joined` or the refusal), null for a candidate
//! the syntax refused. `binding` is the root name; its declaration proof is a later stage's.
use super::join::{join_owner, Buckets, Endpoint, Originals};
use super::*;
use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::OnceLock;

pub(crate) const PREFIX: &str = "element-recall\tv1\t";

/// Whether the environment asked for the trace (read once per process).
fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("PRISM_ELEMENT_RECALL_TRACE").is_ok_and(|v| v == "1"))
}

/// Unit tests collect the records of the files they register instead of reading the environment.
#[cfg(test)]
pub(super) static CAPTURE: std::sync::Mutex<
    Option<std::collections::BTreeMap<String, Vec<String>>>,
> = std::sync::Mutex::new(None);

#[cfg(test)]
fn captured(file: &str) -> bool {
    CAPTURE
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|m| m.contains_key(file))
}
#[cfg(not(test))]
fn captured(_: &str) -> bool {
    false
}

/// The ordinary occurrences of every owner pass of one file, kept until the buckets are final.
pub(crate) struct FileTrace {
    owners: Vec<Originals>,
}

fn occurrences(spans: &[PathSpan]) -> Vec<join::Occurrence> {
    let row = |s: &PathSpan| (s.path.clone(), s.line, s.start_byte, s.end_byte);
    spans.iter().map(row).collect()
}

fn verdict(e: Option<Endpoint>) -> Value {
    match e {
        None => Value::Null,
        Some(Ok(())) => json!("joined"),
        Some(Err(why)) => json!(format!("{why:?}").to_lowercase()),
    }
}

/// `Assign` for an admitted candidate, `refused:Slice` for a refused one.
fn tag<T: std::fmt::Debug, E: std::fmt::Debug>(r: Result<T, E>) -> String {
    match r {
        Ok(v) => format!("{v:?}"),
        Err(e) => format!("refused:{e:?}"),
    }
}

fn path(p: &StaticPath, brackets: bool) -> String {
    let fields: String = p.fields.iter().map(|f| format!(".{f}")).collect();
    let root = p.root.as_deref().unwrap_or("");
    let tail = if brackets {
        "[]".repeat(p.brackets)
    } else {
        String::new()
    };
    format!("{root}{fields}{tail}")
}

fn bytes(s: Span) -> Value {
    json!([s.start, s.end])
}

impl FileTrace {
    /// `Some` only for a Python file while the trace is on.
    pub(crate) fn begin(file: &str, parsed: &ParsedFile) -> Option<Self> {
        let on = parsed.language == Language::Python && (enabled() || captured(file));
        on.then(|| FileTrace { owners: Vec::new() })
    }

    /// Record the ordinary occurrences one owner pass produced, before materialization.
    pub(crate) fn owner(
        &mut self,
        name: &str,
        start_line: usize,
        defs: &[PathSpan],
        uses: &[PathSpan],
    ) {
        self.owners.push(Originals {
            name: name.to_string(),
            start_line,
            defs: occurrences(defs),
            uses: occurrences(uses),
        });
    }

    /// Join the inventory to the final buckets and write the records.
    pub(crate) fn finish(self, file: &str, parsed: &ParsedFile, defs: &Buckets, uses: &Buckets) {
        let lines = self.records(file, parsed, defs, uses);
        #[cfg(test)]
        if let Some(sink) = CAPTURE
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|m| m.get_mut(file))
        {
            sink.extend(lines);
            return;
        }
        if lines.is_empty() {
            return;
        }
        let mut out = lines.join("\n");
        out.push('\n');
        let _ = std::io::stderr().lock().write_all(out.as_bytes());
    }

    fn records(
        &self,
        file: &str,
        parsed: &ParsedFile,
        defs: &Buckets,
        uses: &Buckets,
    ) -> Vec<String> {
        let scans = python::scan_file(parsed);
        let mut out = Vec::new();
        for scan in scans.iter().filter(|s| !s.writes.is_empty()) {
            let key = |o: &Originals| o.name == scan.name && o.start_line == scan.span.line;
            let count = scans
                .iter()
                .filter(|s| s.name == scan.name && s.span.line == scan.span.line);
            let joined = join_owner(
                file,
                scan,
                count.count(),
                self.owners.iter().find(|o| key(o)),
                defs,
                uses,
            );
            let at = |kind: &str| json!({"kind": kind, "file": file, "owner": scan.name, "owner_line": scan.span.line});
            let mut rec = at("owner");
            rec["span"] = bytes(scan.span);
            rec["writes"] = json!(scan.writes.len());
            rec["nodes"] = json!(scan.operands.len());
            rec["pairs"] = joined
                .pairs
                .iter()
                .map(|((w, r), e)| {
                    let (w, r) = (&scan.writes[*w], &scan.operands[*r]);
                    json!({"w": bytes(w.span), "r": bytes(r.span), "role": tag(r.verdict), "join": verdict(Some(*e))})
                })
                .collect();
            out.push(rec);
            for (w, j) in scan.writes.iter().zip(&joined.writes) {
                let mut rec = at("write");
                rec["bytes"] = bytes(w.span);
                rec["line"] = json!(w.span.line);
                rec["path"] = json!(path(&w.path, true));
                rec["binding"] = json!(w.path.root);
                rec["write"] = json!(tag(w.verdict));
                rec["join"] = verdict(*j);
                out.push(rec);
            }
            for (o, j) in scan.operands.iter().zip(&joined.operands) {
                let mut rec = at("node");
                rec["bytes"] = bytes(o.span);
                rec["outer"] = bytes(o.outer);
                rec["line"] = json!(o.span.line);
                rec["path"] = json!(path(&o.path, false));
                rec["binding"] = json!(o.path.root);
                rec["role"] = json!(tag(o.verdict));
                rec["join"] = verdict(*j);
                out.push(rec);
            }
        }
        out.into_iter().map(|r| format!("{PREFIX}{r}")).collect()
    }
}
