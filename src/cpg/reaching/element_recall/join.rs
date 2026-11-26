//! Physical join of the I0 inventory to the producer's own occurrences (lane 10, #847).
//!
//! The Python CPG keys a variable node by `(file, owner, owner line, line, path, access)` and keeps
//! the first location that reaches that key. A W Def or R Use is therefore usable only when it is
//! the one ordinary location of its key, with exactly the syntax bytes. The join counts, before
//! first-wins materialization, the raw rows of the final file-wide buckets, the ordinary locations
//! the owner pass produced and the competing syntax of the inventory.
use super::*;
use crate::access_path::{AccessPath, ARRAY_FIELD};
use crate::data_flow::{VarAccessKind, VarLocation};
use std::collections::{BTreeMap, BTreeSet};

/// The final file-wide buckets, keyed `(file, owner, owner start line, path)`.
pub(crate) type Buckets = BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>>;

/// One ordinary occurrence an owner pass produced: path, line, start and end byte.
pub(crate) type Occurrence = (AccessPath, usize, usize, usize);

/// The ordinary Def and Use occurrences one owner pass handed to materialization.
#[derive(Clone, Debug, Default)]
pub(crate) struct Originals {
    pub name: String,
    pub start_line: usize,
    pub defs: Vec<Occurrence>,
    pub uses: Vec<Occurrence>,
}

/// Why an endpoint is not the unique, nonempty, complete occurrence of its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JoinRefusal {
    AmbiguousOwner,
    /// No row at the key and no ordinary occurrence of the bytes.
    Missing,
    /// The owner pass produced the occurrence but the final bucket lost it.
    Hidden,
    /// The only row lies strictly inside the syntax bytes.
    Truncated,
    /// The only row has other bytes (an anchor included).
    Mismatched,
    /// The exact row exists, but no ordinary location produced it (twin, recall, anchor).
    Supplemental,
    /// More than one row, ordinary location or syntax occurrence shares the key.
    Collapsed,
}

pub(crate) type Endpoint = Result<(), JoinRefusal>;

/// Join results of one owner scan. `None` marks a syntax-refused write or operand.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct OwnerJoin {
    pub writes: Vec<Option<Endpoint>>,
    pub operands: Vec<Option<Endpoint>>,
    /// Per admitted pair of `OwnerScan::pairs`: the first endpoint refusal, W before R.
    pub pairs: Vec<((usize, usize), Endpoint)>,
}

/// The path the producer records for a store: root, pre-bracket fields, one `[]`.
pub(crate) fn write_path(p: &StaticPath) -> Option<AccessPath> {
    let mut fields = p.fields.clone();
    fields.push(ARRAY_FIELD.to_string());
    Some(AccessPath::with_fields(p.root.clone()?, fields))
}

/// The path of an operand: root and selector fields.
pub(crate) fn operand_path(p: &StaticPath) -> Option<AccessPath> {
    Some(AccessPath::with_fields(p.root.clone()?, p.fields.clone()))
}

struct Key {
    bucket: (String, String, usize, AccessPath),
    line: usize,
    target: (usize, usize),
}

fn endpoint(
    kind: VarAccessKind,
    key: &Key,
    buckets: &Buckets,
    ordinary: &[Occurrence],
    syntax: BTreeSet<(usize, usize)>,
) -> Endpoint {
    let (t0, t1) = key.target;
    let path = &key.bucket.3;
    // Identity counts distinct spans (DESIGN section 4): the producer records some reads twice at
    // one span (`v = take(d)`), and first-wins materialization keeps that one node. Competing
    // spans, anchors and syntax occurrences still collapse the key; the repeated rows themselves
    // count toward the raw work of `limits.rs`.
    let rows: BTreeSet<_> = buckets
        .get(&key.bucket)
        .into_iter()
        .flatten()
        .filter(|l| l.kind == kind && l.line == key.line)
        .map(|l| (l.start_byte, l.end_byte))
        .collect();
    let rows: Vec<_> = rows.into_iter().collect();
    let ordinary: BTreeSet<_> = ordinary
        .iter()
        .filter(|(p, line, ..)| p == path && *line == key.line)
        .map(|(_, _, s, e)| (*s, *e))
        .collect();
    let own = ordinary.contains(&key.target);
    match rows.as_slice() {
        _ if t0 >= t1 => Err(JoinRefusal::Mismatched),
        [] if own => Err(JoinRefusal::Hidden),
        [] => Err(JoinRefusal::Missing),
        &[(s, e)] if (s, e) != key.target => {
            let inside = t0 <= s && e <= t1 && s < e;
            Err(if inside {
                JoinRefusal::Truncated
            } else {
                JoinRefusal::Mismatched
            })
        }
        [_] if !own => Err(JoinRefusal::Supplemental),
        [_] => {
            let all: BTreeSet<_> = ordinary.iter().copied().chain(syntax).collect();
            let single = all.len() == 1 && ordinary.len() == 1;
            single.then_some(()).ok_or(JoinRefusal::Collapsed)
        }
        _ => Err(JoinRefusal::Collapsed),
    }
}

/// Join every admitted write, operand and pair of `scan`. `owners` counts scans of this file with
/// the owner's name and start line; `originals` is the owner pass, if the producer ran one.
pub(crate) fn join_owner(
    file: &str,
    scan: &OwnerScan,
    owners: usize,
    originals: Option<&Originals>,
    defs: &Buckets,
    uses: &Buckets,
) -> OwnerJoin {
    let line = scan.span.line;
    let none = Originals::default();
    let ord = originals.unwrap_or(&none);
    let unique = owners == 1;
    let key = |path: Option<AccessPath>, s: Span| -> Option<Key> {
        Some(Key {
            bucket: (file.into(), scan.name.clone(), line, path?),
            line: s.line,
            target: (s.start, s.end),
        })
    };
    let same =
        |k: &Key, b: Option<AccessPath>, m: usize| b.as_ref() == Some(&k.bucket.3) && k.line == m;
    let judge = |key: Option<Key>, kind, syntax: BTreeSet<(usize, usize)>| -> Endpoint {
        let Some(key) = key else {
            return Err(JoinRefusal::Missing);
        };
        if !unique {
            return Err(JoinRefusal::AmbiguousOwner);
        }
        let own = if kind == VarAccessKind::Def {
            &ord.defs
        } else {
            &ord.uses
        };
        endpoint(
            kind,
            &key,
            if kind == VarAccessKind::Def {
                defs
            } else {
                uses
            },
            own,
            syntax,
        )
    };
    let writes: Vec<_> = scan
        .writes
        .iter()
        .map(|w| {
            let key = key(write_path(&w.path), w.span);
            let syntax = key.iter().flat_map(|k| {
                scan.writes
                    .iter()
                    .filter(|x| same(k, write_path(&x.path), x.span.line))
                    .map(|x| (x.span.start, x.span.end))
            });
            let syntax: BTreeSet<_> = syntax.collect();
            w.verdict
                .is_ok()
                .then(|| judge(key, VarAccessKind::Def, syntax))
        })
        .collect();
    let operands: Vec<_> = scan
        .operands
        .iter()
        .map(|o| {
            let key = key(operand_path(&o.path), o.span);
            let syntax = key.iter().flat_map(|k| {
                scan.operands
                    .iter()
                    .filter(|x| same(k, operand_path(&x.path), x.span.line))
                    .map(|x| (x.span.start, x.span.end))
            });
            let syntax: BTreeSet<_> = syntax.collect();
            o.verdict
                .is_ok()
                .then(|| judge(key, VarAccessKind::Use, syntax))
        })
        .collect();
    let pairs = scan
        .pairs()
        .into_iter()
        .map(|(w, r)| {
            let both = [writes[w], operands[r]];
            let verdict = both.into_iter().flatten().find(|e| e.is_err());
            ((w, r), verdict.unwrap_or(Ok(())))
        })
        .collect();
    OwnerJoin {
        writes,
        operands,
        pairs,
    }
}
