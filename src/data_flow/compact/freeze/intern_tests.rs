//! `Intern` against the ordered-map model it replaced.
use super::*;
use crate::access_path::AccessPath;

fn loc(file: &str, function: &str, line: usize, name: &str, byte: usize) -> VarLocation {
    VarLocation {
        file: file.into(),
        function: function.into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple(name.to_string()),
        start_byte: byte,
        end_byte: byte + 1,
        kind: if byte % 2 == 0 {
            VarAccessKind::Def
        } else {
            VarAccessKind::Use
        },
    }
}

/// The pre-hash implementation, kept as the model.
#[derive(Default)]
struct Model {
    rows: BTreeMap<ExactVarLocation, OccId>,
}
impl Model {
    fn put(&mut self, v: VarLocation) -> OccId {
        let key = own_exact(v);
        if let Some(&id) = self.rows.get(&key) {
            return id;
        }
        let id = OccId(self.rows.len() as u32);
        self.rows.insert(key, id);
        id
    }
    fn finish(self) -> (Vec<VarLocation>, Vec<OccId>) {
        let mut remap = vec![OccId(0); self.rows.len()];
        let mut rows = Vec::new();
        for (key, old) in self.rows {
            remap[old.0 as usize] = OccId(rows.len() as u32);
            rows.push(own_legacy(key));
        }
        (rows, remap)
    }
}

fn ids_remap_and_arena_order_match_the_ordered_model<T: Table>() {
    // Identity is bytes-inclusive here: same line/path with other bytes is a
    // distinct row. Keys differ in length-8 chunk boundaries and tails.
    let mut sequence = Vec::new();
    for _ in 0..3 {
        for file in ["a.go", "sql_parser_file.go"] {
            for function in ["Parse", "P"] {
                for line in [9, 2, 400] {
                    for name in ["x", "yyDollar", "yyDollars", "a_name_of_16chr"] {
                        for byte in [0usize, 1, 7, 8, 65] {
                            sequence.push(loc(file, function, line, name, byte));
                        }
                    }
                }
            }
        }
    }
    sequence.reverse();
    let mut model = Model::default();
    let mut intern = Intern::<T>::default();
    for v in &sequence {
        assert_eq!(intern.put(v.clone()).unwrap(), model.put(v.clone()));
    }
    let distinct = model.rows.len();
    assert!(distinct > 100 && distinct < sequence.len(), "{distinct}");
    let (model_rows, model_remap) = model.finish();
    let (arena, remap) = intern.finish().unwrap();
    assert_eq!(remap, model_remap);
    assert_eq!(arena.rows().len(), model_rows.len());
    for (got, want) in arena.rows().iter().zip(&model_rows) {
        assert_eq!(own_exact(got.clone()), own_exact(want.clone()));
    }
    let sorted: Vec<_> = arena.rows().iter().cloned().map(own_exact).collect();
    assert!(sorted.windows(2).all(|w| w[0] < w[1]), "arena rows sorted");
}

#[test]
fn hashed_intern_ids_remap_and_arena_order_match_the_ordered_model() {
    ids_remap_and_arena_order_match_the_ordered_model::<HashTable>();
}

/// `combine` interns the whole repository through this table.
#[test]
fn ordered_intern_ids_remap_and_arena_order_match_the_ordered_model() {
    ids_remap_and_arena_order_match_the_ordered_model::<OrderedTable>();
}

#[test]
fn hasher_separates_inputs_that_differ_only_in_tail_or_length() {
    use std::hash::{BuildHasher, Hasher};
    let build = FxBuild::default();
    let hash = |bytes: &[u8]| {
        let mut h = build.build_hasher();
        h.write(bytes);
        h.finish()
    };
    // Same 8-byte chunk, different remainder lengths and contents.
    assert_ne!(hash(b"abcdefgh"), hash(b"abcdefgh\0"));
    assert_ne!(hash(b"abcdefghi"), hash(b"abcdefghj"));
    assert_ne!(hash(b"abcdefghi"), hash(b"abcdefghi\0\0"));
    assert_eq!(hash(b"abcdefghi"), hash(b"abcdefghi"));
    // Whole chunks are mixed in, not only the remainder.
    assert_ne!(hash(b"aaaaaaaa"), hash(b"bbbbbbbb"));
    assert_ne!(hash(b"aaaaaaaabbbbbbbb"), hash(b"bbbbbbbbaaaaaaaa"));
}
