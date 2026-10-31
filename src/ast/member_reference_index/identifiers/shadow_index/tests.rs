use super::*;
use crate::ast::member_reference_index::identifiers::IdentifierReferenceSearch;
use crate::languages::Language;
use std::collections::BTreeSet;

const GO: &str = "package p

type T struct{}

func use(a ...int) {}

func (r *T) Parse(l int) int {
	var yyn int
	x := 1
	{
		x := 2
		use(x)
	}
	switch yyn {
	case 1:
		x := 3
		use(x)
		{
			x, ok := l, true
			use(x, ok)
		}
	case 2:
		fn := func() int {
			x := 4
			return x
		}
		use(fn())
	default:
		x = 5
	}
	{
		x := 6
		if l > 0 {
			use(x)
		}
	}
	for i := 0; i < 3; i++ {
		x := i
		use(x)
	}
L:
	if x > 0 {
		x = x - 1
		goto L
	}
	a, b := 1, 2
	a, c := 3, 4
	use(a, b, c)
	if y := x; y > 0 {
		use(y)
	}
	return x
}

func other(p int) int {
	x := p
	{
		x := x + 1
		use(x)
	}
	return x
}
";

const PY: &str = "def f(a):
    x = 1
    if a:
        x = 2
        y = x
    else:
        for x in range(3):
            y = x
    return x

def g(a):
    x = a
    return x
";

const C: &str = "int f(int a) {
    int x = 1;
    {
        int x = 2;
        a = x;
    }
    for (int i = 0; i < 3; i++) {
        int x = i;
        a += x;
    }
    return x + a;
}
int g(int a) { int x = a; return x; }
";

const CPP: &str = "int f(int a) {
    int x = 1;
    {
        int x = 2;
        a = x;
    }
    if (a) {
        int x = a;
        return x;
    }
    return x;
}
int g(int a) { int x = a; return x; }
";

const LUA: &str = "function f(a)
  local x = 1
  do
    local x = 2
    a = x
  end
  return x + a
end
function g(a) local x = a return x end
";

const BASH: &str = "f() {
  local x=1
  if [ \"$1\" ]; then
    local x=2
    echo $x
  fi
  echo $x
}
g() { local x=$1; echo $x; }
";

const TF: &str = "resource \"a\" \"b\" {
  name = var.x
  nested {
    name = var.x
    other = local.y
  }
}
resource \"a\" \"c\" {
  name = var.x
  nested {
    other = local.y
  }
}
variable \"x\" {}
";

const RS: &str = "fn f(a: i32) -> i32 {
    let x = 1;
    {
        let x = 2;
        let _ = x + a;
    }
    if a > 0 {
        let x = a;
        return x;
    }
    x
}
fn g(a: i32) -> i32 { let x = a; x }
";

const JAVA: &str = "class A {
    int f(int a) {
        int x = 1;
        {
            int x = 2;
            a += x;
        }
        for (int i = 0; i < 3; i++) {
            int x = i;
            a += x;
        }
        return x + a;
    }
    int g(int a) { int x = a; return x; }
}
";

fn identifiers(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut current = String::new();
    for ch in source.chars().chain(std::iter::once(' ')) {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if current.chars().next().is_some_and(|c| !c.is_ascii_digit()) {
                names.insert(current.clone());
            }
            current.clear();
        }
    }
    names
}

struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.with(|m| m.set(Mode::Auto));
    }
}

fn set(mode: Mode) -> Reset {
    MODE.with(|m| m.set(mode));
    STATS.with(|s| s.set(Stats::default()));
    Reset
}

/// Every `(owner, name, def_line)` of the file under the original walk and
/// under the index, one search object per mode, owners interleaved.
fn assert_equivalent(language: Language, file: &str, source: &str) -> Stats {
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    let owners = parsed.all_functions();
    assert!(!owners.is_empty());
    let lines = source.lines().count() + 1;
    let names = identifiers(source);
    let reference = IdentifierReferenceSearch::new(&parsed);
    let indexed = IdentifierReferenceSearch::new(&parsed);
    let mut compared = 0;
    let _guard;
    let mut expected = Vec::new();
    {
        _guard = set(Mode::Reference);
        for owner in owners.iter().chain(owners.iter().rev()) {
            for name in &names {
                for line in 1..=lines {
                    expected.push(reference.scoped::<false>(owner, name, line));
                }
            }
        }
    }
    let _guard = set(Mode::Force);
    let mut at = 0;
    for owner in owners.iter().chain(owners.iter().rev()) {
        for name in &names {
            for line in 1..=lines {
                let actual = indexed.scoped::<false>(owner, name, line);
                assert_eq!(
                    actual,
                    expected[at],
                    "{file}: owner@{} name={name} def_line={line}",
                    owner.start_position().row + 1
                );
                at += 1;
                compared += 1;
            }
        }
    }
    assert!(compared > 200, "{file}: only {compared} comparisons");
    STATS.with(|s| s.get())
}

#[test]
fn index_matches_the_walk_for_every_name_and_definition_line() {
    let go = assert_equivalent(Language::Go, "a.go", GO);
    // `x` is declared in many blocks: guarded candidates must be decided from
    // the index, and the index must have been built.
    assert!(go.indexed > 0, "no guarded candidate was decided: {go:?}");
    assert!(go.tree_builds >= 2, "owners alternate: {go:?}");
    for (language, file, source) in [
        (Language::Python, "a.py", PY),
        (Language::C, "a.c", C),
        (Language::Rust, "a.rs", RS),
        (Language::Java, "A.java", JAVA),
        (Language::Cpp, "a.cpp", CPP),
        (Language::Lua, "a.lua", LUA),
        (Language::Bash, "a.sh", BASH),
        (Language::Terraform, "a.tf", TF),
    ] {
        assert_equivalent(language, file, source);
    }
}

#[test]
fn shadowed_candidates_depend_on_the_definition_line() {
    // `x` at line 12 sits inside the block that redeclares it at line 11.
    let source =
        "package p\nfunc f() int {\n\tx := 1\n\t{\n\t\tx := 2\n\t\t_ = x\n\t}\n\treturn x\n}\n";
    let parsed = ParsedFile::parse("a.go", source, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    let search = IdentifierReferenceSearch::new(&parsed);
    let _guard = set(Mode::Force);
    // Definition on line 3 (outer x): the inner block redeclares x and does
    // not contain line 3, so its uses (lines 5 and 6) are shadowed.
    assert_eq!(
        search.scoped::<false>(&owner, "x", 3),
        BTreeSet::from([3, 8])
    );
    // Definition on line 5 (inner x): inside the redeclaring block, so the
    // block does not shadow itself; line 8 is outside and not shadowed either.
    assert_eq!(
        search.scoped::<false>(&owner, "x", 5),
        BTreeSet::from([3, 5, 6, 8])
    );
    let stats = STATS.with(|s| s.get());
    assert_eq!(stats.slow, 0);
    assert!(stats.indexed >= 2);
}

#[test]
fn candidates_the_index_cannot_prove_keep_the_original_walk() {
    // Candidates of another function are outside the owner: never indexed.
    let source = "package p\nfunc a() int {\n\tx := 1\n\treturn x\n}\nfunc b() int {\n\tx := 2\n\t{\n\t\tx := 3\n\t\t_ = x\n\t}\n\treturn x\n}\n";
    let parsed = ParsedFile::parse("a.go", source, Language::Go).unwrap();
    let functions = parsed.all_functions();
    let (a, b) = (functions[0], functions[1]);
    let foreign = || {
        let mut nodes = Vec::new();
        let mut cursor = b.walk();
        let mut stack = vec![b];
        while let Some(node) = stack.pop() {
            if node.kind() == "identifier" && parsed.node_text(&node) == "x" {
                nodes.push(node);
            }
            for child in node.children(&mut cursor).collect::<Vec<_>>() {
                stack.push(child);
            }
        }
        nodes.sort_by_key(|n| n.start_byte());
        nodes
    };
    let index = ShadowIndex::default();
    let _guard = set(Mode::Force);
    let got = index.lines::<true>(&parsed, &a, "x", 3, foreign);
    let mut expected = BTreeSet::new();
    for node in foreign() {
        if !shadow::<false>(&parsed, &node, &a, "x", 3) {
            expected.insert(node.start_position().row + 1);
        }
    }
    assert_eq!(got, expected);
    let stats = STATS.with(|s| s.get());
    assert_eq!(stats.indexed, 0, "{stats:?}");
    assert!(stats.slow >= 4, "{stats:?}");
}

#[test]
fn small_owners_and_the_javascript_family_keep_the_walk() {
    let parsed = ParsedFile::parse("a.go", GO, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    assert!(owner.byte_range().len() < MIN_OWNER_BYTES);
    MODE.with(|m| m.set(Mode::Auto));
    assert!(!enabled_for(&parsed, &owner));
    let mut big = String::from("package p\nfunc big() int {\n\tx := 0\n");
    while big.len() < MIN_OWNER_BYTES + 64 {
        big.push_str("\tx = x + 1\n");
    }
    big.push_str("\treturn x\n}\n");
    let parsed = ParsedFile::parse("b.go", &big, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    assert!(enabled_for(&parsed, &owner));
    let js = ParsedFile::parse(
        "a.js",
        "function f() { let x = 1; { let x = 2; } return x; }",
        Language::JavaScript,
    )
    .unwrap();
    let owner = js.all_functions()[0];
    let _guard = set(Mode::Force);
    assert!(!enabled_for(&js, &owner), "JS-family gates stay unindexed");
}

#[test]
fn shadow_check_counter_counts_only_walks_that_ran() {
    use crate::profile::GATE_COUNTERS;
    let mut big = String::from("package p\nfunc big() int {\n\tx := 0\n");
    for i in 0..400 {
        big.push_str(&format!("\t{{\n\t\ty := {i}\n\t\tx = x + y\n\t}}\n"));
    }
    big.push_str("\treturn x\n}\n");
    let parsed = ParsedFile::parse("b.go", &big, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    assert!(owner.byte_range().len() >= MIN_OWNER_BYTES);
    let count = |mode: Mode| {
        let _guard = set(mode);
        GATE_COUNTERS.with(|c| *c.borrow_mut() = Some(Default::default()));
        let search = IdentifierReferenceSearch::new(&parsed);
        let lines = search.scoped::<true>(&owner, "x", 3);
        let counts = GATE_COUNTERS.with(|c| c.borrow_mut().take().unwrap());
        (lines, counts.shadow_check_calls)
    };
    let (walk_lines, walk_calls) = count(Mode::Reference);
    let (index_lines, index_calls) = count(Mode::Auto);
    assert_eq!(walk_lines, index_lines);
    assert!(walk_calls >= 800, "walk ran {walk_calls}");
    assert_eq!(index_calls, 0, "index decides every x without a walk");
}

#[test]
fn definition_on_the_first_and_last_row_of_a_redeclaring_block_is_inside_it() {
    // The inner block spans rows 4..=6; rows are inclusive at both ends.
    let source =
        "package p\nfunc f() int {\n\tx := 1\n\t{ x := 2\n\t\t_ = x\n\t_ = x }\n\treturn x\n}\n";
    let parsed = ParsedFile::parse("a.go", source, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    let _guard = set(Mode::Force);
    for def_line in [4, 5, 6] {
        MODE.with(|m| m.set(Mode::Force));
        let got = IdentifierReferenceSearch::new(&parsed).scoped::<false>(&owner, "x", def_line);
        MODE.with(|m| m.set(Mode::Reference));
        let walk = IdentifierReferenceSearch::new(&parsed).scoped::<false>(&owner, "x", def_line);
        MODE.with(|m| m.set(Mode::Force));
        assert_eq!(got, walk, "def_line={def_line}");
        // Inside the block nothing in it is shadowed by itself.
        assert!(got.contains(&4) && got.contains(&5) && got.contains(&6));
    }
    for def_line in [3, 7] {
        let got = IdentifierReferenceSearch::new(&parsed).scoped::<false>(&owner, "x", def_line);
        assert!(
            !got.contains(&5) && !got.contains(&6),
            "def_line={def_line}"
        );
    }
}

#[test]
fn python_block_with_the_range_of_its_only_candidate_takes_the_walk() {
    // The block `x` has exactly the identifier's range: ancestry is not implied
    // by containment, so the candidate must not be decided from the index.
    let source = "def f(a):\n    if a:\n        x\n    return a\n";
    let parsed = ParsedFile::parse("a.py", source, Language::Python).unwrap();
    let owner = parsed.all_functions()[0];
    let _guard = set(Mode::Force);
    let search = IdentifierReferenceSearch::new(&parsed);
    assert_eq!(search.scoped::<false>(&owner, "x", 1), BTreeSet::from([3]));
    let stats = STATS.with(|s| s.get());
    assert_eq!(stats.slow, 1, "{stats:?}");
}
