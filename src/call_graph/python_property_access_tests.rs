use super::*;
use crate::languages::Language::{JavaScript, Python};
use std::collections::BTreeMap;

fn build_py(files: &[(&str, &str)]) -> CallGraph {
    let mut map = BTreeMap::new();
    for (path, src) in files {
        map.insert(
            path.to_string(),
            ParsedFile::parse(path, src, Python).unwrap(),
        );
    }
    CallGraph::build(&map)
}

fn fid_named<'a>(cg: &'a CallGraph, name: &str) -> &'a FunctionId {
    cg.functions
        .get(name)
        .unwrap_or_else(|| panic!("no function named {name}"))
        .first()
        .unwrap()
}

// ---- S1: property getter index -----------------------------------

#[test]
fn property_getter_is_recorded_in_index() {
    let cg = build_py(&[(
        "resp.py",
        "class Response:\n    @property\n    def text(self):\n        return self._text\n",
    )]);
    let getters = cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .expect("@property getter recorded");
    assert_eq!(getters.len(), 1);
    assert!(cg.cached_property_getters.is_empty());
}

#[test]
fn cached_property_getter_is_recorded_and_tracked_separately() {
    let cg = build_py(&[(
            "resp.py",
            "import functools\n\nclass Response:\n    @functools.cached_property\n    def text(self):\n        return self._text\n",
        )]);
    let getters = cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .expect("@functools.cached_property getter recorded");
    assert_eq!(getters.len(), 1);
    assert_eq!(cg.cached_property_getters.len(), 1);
    assert!(cg.cached_property_getters.contains(&getters[0]));
}

#[test]
fn bare_cached_property_decorator_is_recorded_and_tracked_separately() {
    let cg = build_py(&[(
            "resp.py",
            "from functools import cached_property\n\nclass Response:\n    @cached_property\n    def text(self):\n        return self._text\n",
        )]);
    let getters = cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .expect("@cached_property getter recorded");
    assert_eq!(cg.cached_property_getters.len(), 1);
    assert!(cg.cached_property_getters.contains(&getters[0]));
}

#[test]
fn setter_decorated_method_never_pollutes_getter_index() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    @text.setter\n    def text(self, value):\n        self._text = value\n",
        )]);
    let getters = cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .expect("getter recorded");
    // Exactly the getter, never the `@text.setter`-decorated definition.
    assert_eq!(getters.len(), 1);
    assert_eq!(getters[0].start_line, 2);
}

#[test]
fn non_decorated_same_name_method_is_not_indexed() {
    let cg = build_py(&[(
        "resp.py",
        "class Response:\n    def text(self):\n        return self._text\n",
    )]);
    assert!(cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .is_none());
}

#[test]
fn non_python_files_leave_property_state_empty() {
    let mut files = BTreeMap::new();
    files.insert(
            "resp.js".to_string(),
            ParsedFile::parse(
                "resp.js",
                "class Response {\n  get text() { return this._text; }\n}\nfunction f(r) {\n  return r.text;\n}\n",
                JavaScript,
            )
            .unwrap(),
        );
    let cg = CallGraph::build(&files);
    assert!(cg.property_getters.is_empty());
    assert!(cg.cached_property_getters.is_empty());
    assert!(cg.property_accesses.is_empty());
    assert_eq!(cg.property_access_fanout_skips, 0);
    assert_eq!(cg.property_access_store_skips, 0);
}

// ---- S2: access-site extraction ------------------------------------

#[test]
fn self_attr_records_against_own_class_getter() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        return self.text\n",
        )]);
    let getter = fid_named(&cg, "text").clone();
    let enclosing = fid_named(&cg, "dump").clone();
    assert!(cg
        .property_accesses
        .iter()
        .any(|a| a.getter == getter && a.enclosing == enclosing));
}

#[test]
fn unknown_receiver_single_owner_is_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    return r.text\n",
        )]);
    let getter = fid_named(&cg, "text").clone();
    let enclosing = fid_named(&cg, "f").clone();
    assert!(cg
        .property_accesses
        .iter()
        .any(|a| a.getter == getter && a.enclosing == enclosing));
    assert_eq!(cg.property_access_fanout_skips, 0);
}

#[test]
fn plain_assignment_target_is_not_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text = \"v\"\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn augmented_assignment_target_is_not_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text += \"v\"\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

// ---- F5: property_access_store_skips telemetry ---------------------

#[test]
fn store_target_with_indexed_name_increments_store_skip_counter() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text = \"v\"\n",
        )]);
    assert_eq!(cg.property_access_store_skips, 1);
}

#[test]
fn store_target_with_non_indexed_name_does_not_increment_store_skip_counter() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.other = \"v\"\n",
        )]);
    assert_eq!(cg.property_access_store_skips, 0);
}

#[test]
fn delete_for_and_with_alias_store_skips_are_all_counted() {
    // F4 + F5 together: every one of the store/delete contexts F4 fences
    // (assignment/augmented left, del target, for target, with-alias)
    // increments the same counter when the name is indexed.
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\n\
def f(r, xs, cm):\n    r.text = \"v\"\n    r.text += \"v\"\n    del r.text\n    for r.text in xs:\n        pass\n    with cm() as r.text:\n        pass\n",
        )]);
    assert_eq!(cg.property_access_store_skips, 5);
}

// ---- F4: delete/for/with-target store contexts ---------------------

#[test]
fn delete_statement_target_is_not_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    del r.text\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn for_target_attribute_is_not_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r, xs):\n    for r.text in xs:\n        pass\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn comprehension_for_target_attribute_is_not_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r, xs):\n    return [x for r.text in xs]\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn for_iterable_attribute_load_is_still_recorded() {
    // The right-hand/iterable side of a `for` remains LOAD context.
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    for x in r.text:\n        pass\n",
        )]);
    let getter = fid_named(&cg, "text").clone();
    let enclosing = fid_named(&cg, "f").clone();
    assert!(cg
        .property_accesses
        .iter()
        .any(|a| a.getter == getter && a.enclosing == enclosing));
}

#[test]
fn with_alias_target_attribute_is_not_recorded() {
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r, cm):\n    with cm() as r.text:\n        pass\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn with_item_value_attribute_load_is_still_recorded() {
    // The context-manager-expression side of `with ... as x:` remains
    // LOAD context, even though the alias target does not.
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    with r.text as x:\n        pass\n",
        )]);
    let getter = fid_named(&cg, "text").clone();
    let enclosing = fid_named(&cg, "f").clone();
    assert!(cg
        .property_accesses
        .iter()
        .any(|a| a.getter == getter && a.enclosing == enclosing));
}

#[test]
fn call_of_attribute_is_not_double_recorded() {
    // `r.text()` — the attribute is the function child of a call, not a
    // load; the property-access table must not record it (the ordinary
    // call-resolution path, independent of this table, is what handles
    // `r.text()` syntactically as a method call).
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    return r.text()\n",
        )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn fanout_at_cap_records_all_three() {
    let cg = build_py(&[(
        "resp.py",
        "class A:\n    @property\n    def text(self):\n        return 1\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n\n\n\
class C:\n    @property\n    def text(self):\n        return 3\n\n\n\
def f(r):\n    return r.text\n",
    )]);
    assert_eq!(cg.property_accesses.len(), 3);
    assert_eq!(cg.property_access_fanout_skips, 0);
}

#[test]
fn fanout_over_cap_is_skipped_and_counted() {
    let cg = build_py(&[(
        "resp.py",
        "class A:\n    @property\n    def text(self):\n        return 1\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n\n\n\
class C:\n    @property\n    def text(self):\n        return 3\n\n\n\
class D:\n    @property\n    def text(self):\n        return 4\n\n\n\
def f(r):\n    return r.text\n",
    )]);
    assert!(cg.property_accesses.is_empty());
    assert_eq!(cg.property_access_fanout_skips, 1);
}

#[test]
fn cls_attr_does_not_narrow_to_its_own_class() {
    // `cls.text` inside a classmethod of `A` must NOT be narrowed to just
    // A's getter (the self-like shortcut is explicitly excluded) — with
    // 2 classes defining `text`, it must fan out to BOTH (under cap),
    // pinning that `cls` gets no special treatment vs. any other
    // unrecognized receiver.
    let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n    @classmethod\n    def make(cls):\n        return cls.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
    let a_getter = cg
        .property_getters
        .get(&("A".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let b_getter = cg
        .property_getters
        .get(&("B".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let enclosing = fid_named(&cg, "make").clone();
    assert!(cg
        .property_accesses
        .iter()
        .any(|a| a.getter == a_getter && a.enclosing == enclosing));
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == b_getter && a.enclosing == enclosing),
        "cls.text must fan out to B's getter too, not narrow to A alone"
    );
}

#[test]
fn staticmethod_self_param_does_not_get_same_class_narrowing() {
    // F2 (codex MAJOR 2): `@staticmethod def make(self)` — Python's
    // generic `method_owner` marks it as owned by `A` (any
    // class-contained function), but `self` here is an ordinary
    // parameter of unknown type, not a receiver. It must NOT get tier-1
    // same-class narrowing; it must route to tier-3 (unknown-receiver,
    // capped fanout) like any other unrecognized receiver — so with
    // A and B both defining `text`, it must fan out to BOTH.
    let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n    @staticmethod\n    def make(self):\n        return self.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
    let a_getter = cg
        .property_getters
        .get(&("A".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let b_getter = cg
        .property_getters
        .get(&("B".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let enclosing = fid_named(&cg, "make").clone();
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == a_getter && a.enclosing == enclosing),
        "staticmethod self.text must still fan out to A's getter"
    );
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == b_getter && a.enclosing == enclosing),
        "staticmethod self.text must fan out to B's getter too, not narrow to A alone"
    );
}

#[test]
fn top_level_self_param_routes_to_tier3_fanout_not_drop() {
    // Item 1 (MAJOR, codex re-review): a top-level `def f(self)` is not
    // contained in any class, so `method_owners` has no entry for it.
    // Tier-1 eligibility now requires `method_owners.contains_key`, so
    // this must NOT enter tier-1 (and then drop when
    // `self_property_owner_getters` finds no owner) — it must route to
    // tier-3 unknown-receiver fanout, same as any other unrecognized
    // receiver. With exactly one class defining `text`, that fanout
    // yields a single edge, not a drop.
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(self):\n    return self.text\n",
        )]);
    let getter = fid_named(&cg, "text").clone();
    let enclosing = fid_named(&cg, "f").clone();
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == getter && a.enclosing == enclosing),
        "top-level def f(self) reading self.text must fan out to Response's getter, not drop"
    );
    assert_eq!(cg.property_access_fanout_skips, 0);
}

#[test]
fn keyword_only_self_param_does_not_narrow_to_owning_class() {
    // Item 1 (MAJOR): `def make(*, self)` inside class A — `self` is
    // keyword-only (appears after the bare `*` separator), so it is NOT
    // the first positional parameter and must fail the instance-method
    // gate even though `make` IS owned by A (`method_owners` has an
    // entry). Must NOT narrow to A alone; must fan out to both A and B,
    // proving the keyword-only receiver gets no special treatment.
    let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n    def make(*, self):\n        return self.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
    let a_getter = cg
        .property_getters
        .get(&("A".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let b_getter = cg
        .property_getters
        .get(&("B".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let enclosing = fid_named(&cg, "make").clone();
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == a_getter && a.enclosing == enclosing),
        "keyword-only self.text must still fan out to A's getter"
    );
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == b_getter && a.enclosing == enclosing),
        "keyword-only self.text must fan out to B's getter too, not narrow to A alone"
    );
}

#[test]
fn positional_only_self_param_is_still_tier1_narrowed() {
    // Item 1 (c): `self` before a bare `/` positional-only separator
    // still counts as the first positional parameter (`def m(self, /)`)
    // — must still get tier-1 same-class narrowing, not fan out to an
    // unrelated class's getter of the same name.
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self, /):\n        return self.text\n\n\n\
class Other:\n    @property\n    def text(self):\n        return 2\n",
        )]);
    let response_getter = cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let other_getter = cg
        .property_getters
        .get(&("Other".to_string(), "text".to_string()))
        .unwrap()[0]
        .clone();
    let enclosing = fid_named(&cg, "dump").clone();
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == response_getter && a.enclosing == enclosing),
        "positional-only self.text must narrow to Response's own getter"
    );
    assert!(
        !cg.property_accesses
            .iter()
            .any(|a| a.getter == other_getter && a.enclosing == enclosing),
        "positional-only self must NOT fan out to Other's unrelated getter"
    );
}

#[test]
fn property_getter_reading_self_other_prop_is_still_tier1_narrowed() {
    // F2: the enclosing function MAY legitimately carry other
    // decorators, including `@property` itself — a getter reading
    // `self.other_prop` is a genuine instance method and must still get
    // tier-1 same-class narrowing (not fan out to an unrelated class
    // that also happens to define `other_prop`).
    let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def other_prop(self):\n        return 1\n\n    @property\n    def text(self):\n        return self.other_prop\n\n\n\
class B:\n    @property\n    def other_prop(self):\n        return 2\n",
        )]);
    let a_other_prop = cg
        .property_getters
        .get(&("A".to_string(), "other_prop".to_string()))
        .unwrap()[0]
        .clone();
    let b_other_prop = cg
        .property_getters
        .get(&("B".to_string(), "other_prop".to_string()))
        .unwrap()[0]
        .clone();
    let enclosing = fid_named(&cg, "text").clone();
    assert!(
        cg.property_accesses
            .iter()
            .any(|a| a.getter == a_other_prop && a.enclosing == enclosing),
        "text() reading self.other_prop must narrow to A's own other_prop getter"
    );
    assert!(
        !cg.property_accesses
            .iter()
            .any(|a| a.getter == b_other_prop && a.enclosing == enclosing),
        "must NOT fan out to B's unrelated other_prop getter"
    );
}

#[test]
fn self_attr_with_no_matching_class_does_not_fan_out() {
    // `self.text` inside `A` (which does NOT define `text`) must not be
    // attributed to `B`'s unrelated getter, even though the name is
    // indexed globally — a known non-matching receiver is a negative
    // result, not "unknown".
    let cg = build_py(&[(
        "resp.py",
        "class A:\n    def dump(self):\n        return self.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
    )]);
    assert!(cg.property_accesses.is_empty());
}

#[test]
fn self_attr_own_class_hit_filtered_by_caller_file_and_span() {
    // F1 (codex MAJOR 1): two files each define `class Response` with
    // `@property def text`. The bare `(owner, attr)` index key collides
    // across files, so an unfiltered own-class hit would fan out to
    // BOTH getters when a method in file A's Response reads `self.text`.
    // The own-class hit must be filtered to the caller's own file AND
    // class span, exactly like the inherited-base fallback already is.
    let cg = build_py(&[
            (
                "resp_a.py",
                "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        return self.text\n",
            ),
            (
                "resp_b.py",
                "class Response:\n    @property\n    def text(self):\n        return self._text\n",
            ),
        ]);
    let getter_a = cg
        .property_getters
        .get(&("Response".to_string(), "text".to_string()))
        .expect("text getters indexed")
        .iter()
        .find(|fid| fid.file == "resp_a.py")
        .expect("resp_a.py getter present")
        .clone();
    let enclosing = cg
        .functions
        .get("dump")
        .expect("dump indexed")
        .iter()
        .find(|fid| fid.file == "resp_a.py")
        .expect("dump in resp_a.py")
        .clone();
    let matches: Vec<_> = cg
        .property_accesses
        .iter()
        .filter(|a| a.enclosing == enclosing)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "self.text in resp_a.py's Response.dump must produce exactly one edge, got {:?}",
        matches
    );
    assert_eq!(matches[0].getter, getter_a);
}

#[test]
fn self_attr_narrows_via_same_file_single_base() {
    let cg = build_py(&[(
        "resp.py",
        "class Base:\n    @property\n    def text(self):\n        return self._text\n\n\n\
class Child(Base):\n    def dump(self):\n        return self.text\n",
    )]);
    let getter = fid_named(&cg, "text").clone();
    let enclosing = fid_named(&cg, "dump").clone();
    assert!(cg
        .property_accesses
        .iter()
        .any(|a| a.getter == getter && a.enclosing == enclosing));
}

// ---- F3: nested callable/class scope fencing -----------------------

#[test]
fn nested_def_self_attr_not_attributed_to_outer_method() {
    // F3 (codex MAJOR 3): a nested `def` has the SAME kind as the outer
    // function but its own scope — `self.text` inside it must not be
    // recorded against the OUTER method's FunctionId (misattribution +
    // double-scan, since `all_functions()` visits the nested def
    // separately anyway).
    //
    // Item 2 (MINOR, codex re-review): this pin is only load-bearing if
    // it also proves the access isn't dropped entirely — the nested
    // def's OWN walk must record it under the nested FunctionId (via
    // tier-3: `helper` has no `self` parameter of its own, so it never
    // qualifies for tier-1 in the first place — this is unaffected by
    // which routing failure sends it to tier-3). With exactly one class
    // defining `text`, tier-3 fanout yields a single owner edge.
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        def helper():\n            return self.text\n        return helper()\n",
        )]);
    let dump = fid_named(&cg, "dump").clone();
    let helper = fid_named(&cg, "helper").clone();
    let getter = fid_named(&cg, "text").clone();
    assert!(
        !cg.property_accesses.iter().any(|a| a.enclosing == dump),
        "self.text inside a nested def must not be attributed to the outer method"
    );
    assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == getter && a.enclosing == helper),
            "self.text inside the nested def must be recorded under the nested function's own FunctionId, not dropped"
        );
}

#[test]
fn lambda_body_self_attr_is_not_scanned() {
    // F3: lambda bodies are fenced out of the walk entirely (accepted
    // recall gap — no other scanner covers lambda bodies either, since
    // lambdas never get their own FunctionId from `all_functions()`).
    let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        f = lambda: self.text\n        return f()\n",
        )]);
    assert!(
        cg.property_accesses.is_empty(),
        "self.text inside a lambda body must not surface any edge"
    );
}

/// P8 non-Rust guard: the macro-arg extractor is gated on
/// `self.language == Language::Rust` AND a real `macro_invocation` node
/// kind (which only tree-sitter-rust ever produces) -- literal
/// `assert!(f())`-shaped TEXT sitting inside a JS template literal or a
/// Python string must never mint a call for `f`/`assert`/`check`.
#[test]
fn non_rust_assert_bang_text_in_string_or_template_literal_mints_nothing() {
    let py = build_py(&[(
        "notes.py",
        "def host():\n    s = \"assert!(check(1))\"\n    return s\n",
    )]);
    assert!(
        !py.functions.contains_key("check"),
        "a Python string literal containing assert!(check(1)) text must not mint a call"
    );
    assert!(py
        .calls
        .values()
        .flat_map(|s| s.iter())
        .all(|s| s.callee_name != "check" && s.callee_name != "assert"));

    let mut js_map = BTreeMap::new();
    js_map.insert(
        "notes.js".to_string(),
        ParsedFile::parse(
            "notes.js",
            "function host() {\n    const s = `assert!(check(1))`;\n    return s;\n}\n",
            JavaScript,
        )
        .unwrap(),
    );
    let js = CallGraph::build(&js_map);
    assert!(js
        .calls
        .values()
        .flat_map(|s| s.iter())
        .all(|s| s.callee_name != "check" && s.callee_name != "assert"));
}
