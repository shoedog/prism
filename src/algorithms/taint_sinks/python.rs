//! Python sink rules (#622 S1): call position first, receiver second.
//!
//! Same hybrid policy as the JS rules: the identifier must be the callee of a
//! call; an unknown receiver keeps firing for the high-severity names; only a
//! proven non-sink suppresses (a logger or channel receiver, a receiver bound to
//! a collection or file object, a `def` of the same name in this file, a call
//! with no argument such as `model.eval()` / `platform.system()`).

use super::callee::{args, callee_of, chain_names, literal_string, walk, Callee, Src};
use super::py_scope::{is_rebound, resolve, PyBind, Sites};
use super::sqla;
use super::SinkHit;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// `os.exec*` / `os.spawn*` and the process-launching names with no ambiguity.
const PROCESS_NAMES: &[&str] = &[
    "execl",
    "execle",
    "execlp",
    "execlpe",
    "execv",
    "execve",
    "execvp",
    "execvpe",
    "spawnl",
    "spawnle",
    "spawnlp",
    "spawnlpe",
    "spawnv",
    "spawnve",
    "spawnvp",
    "spawnvpe",
    "posix_spawn",
    "posix_spawnp",
    "startfile",
    "create_subprocess_exec",
    "create_subprocess_shell",
    "getoutput",
    "getstatusoutput",
];
const SQL_METHODS: &[&str] = &["execute", "executemany", "executescript", "exec_driver_sql"];
const PANDAS_SQL: &[&str] = &["read_sql", "read_sql_query", "read_sql_table"];
const TEMPLATE_MODULES: &[&str] = &[
    "jinja2",
    "mako.template",
    "tornado.template",
    "django.template",
];
/// Proven non-sink receivers (M0: `logger.warning(source)`).
const LOGGERS: &[&str] = &["logger", "log", "logging", "_logger", "_log"];
/// Message channels and stdio for `send` / `write`.
const CHANNELS: &[&str] = &[
    "sock",
    "socket",
    "ws",
    "websocket",
    "emitter",
    "queue",
    "pipe",
    "channel",
    "stdout",
    "stderr",
    "sys",
    "buf",
    "buffer",
];
/// Callables that build a file object or an in-memory stream.
const FILE_MAKERS: &[&str] = &[
    "open",
    "StringIO",
    "BytesIO",
    "NamedTemporaryFile",
    "TemporaryFile",
    "SpooledTemporaryFile",
];
const COLLECTION_MAKERS: &[&str] = &[
    "list",
    "set",
    "dict",
    "deque",
    "frozenset",
    "defaultdict",
    "OrderedDict",
    "Counter",
    "sorted",
];
const COLLECTION_HINTS: &[&str] = &[
    "list", "set", "dict", "sequence", "deque", "iterable", "tuple",
];

/// Import aliases: `import os as o` and `from os import system as sh`.
#[derive(Default)]
pub(super) struct Facts {
    /// Local name -> module (`o` -> `os`).
    modules: BTreeMap<String, String>,
    /// Local name -> (module, original name).
    pub(super) names: BTreeMap<String, (String, String)>,
    /// Binding-site census of the file (library identity and local resolution).
    pub(super) sites: Sites,
}

impl Facts {
    pub(super) fn collect(p: Src<'_>, root: Node<'_>) -> Self {
        let mut facts = Facts {
            sites: Sites::collect(p, root),
            ..Facts::default()
        };
        walk(root, &mut |node| match node.kind() {
            "import_statement" => facts.import(p, node),
            "import_from_statement" => facts.import_from(p, node),
            _ => {}
        });
        facts
    }

    fn import(&mut self, p: Src<'_>, node: Node<'_>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            let (module, local) = module_and_local(p, child);
            let local = if module == local {
                local.split('.').next().unwrap_or(local)
            } else {
                local
            };
            self.modules.insert(local.to_string(), module.to_string());
        }
    }

    fn import_from(&mut self, p: Src<'_>, node: Node<'_>) {
        let module = node
            .child_by_field_name("module_name")
            .map(|m| p.node_text(&m))
            .unwrap_or("");
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            let (orig, local) = module_and_local(p, name);
            self.names
                .insert(local.to_string(), (module.to_string(), orig.to_string()));
        }
    }

    /// Module a single-identifier receiver stands for (`o` -> `os`, else its own name).
    fn module_of<'a>(&'a self, name: &'a str) -> &'a str {
        self.modules.get(name).map(String::as_str).unwrap_or(name)
    }
}

/// `(name, local alias)` of a `dotted_name` or `aliased_import`.
fn module_and_local<'p>(p: Src<'p>, node: Node<'_>) -> (&'p str, &'p str) {
    if node.kind() == "aliased_import" {
        let name = node.child_by_field_name("name").map(|n| p.node_text(&n));
        let alias = node.child_by_field_name("alias").map(|n| p.node_text(&n));
        if let (Some(name), Some(alias)) = (name, alias) {
            return (name, alias);
        }
    }
    let t = p.node_text(&node);
    (t, t)
}

fn is_literal(p: Src<'_>, node: Node<'_>) -> bool {
    match node.kind() {
        "string" => literal_string(p, node).is_some(),
        "concatenated_string" => {
            let mut cursor = node.walk();
            let all = node
                .named_children(&mut cursor)
                .all(|c| c.kind() == "comment" || is_literal(p, c));
            all
        }
        _ => false,
    }
}

/// An argument that builds SQL text rather than naming a model: an f-string, a
/// `+` / `%` / `.format` composition with a string, or a lowercase variable
/// (`session.query(Model)` and `session.query(Model.id)` are ORM builders).
fn is_sql_text(p: Src<'_>, node: Node<'_>) -> bool {
    let node = peel_parens(node);
    match node.kind() {
        "string" | "concatenated_string" => !is_literal(p, node),
        "binary_operator" => {
            let op = node
                .child_by_field_name("operator")
                .map(|o| p.node_text(&o));
            let operand_text = |f: &str| {
                node.child_by_field_name(f)
                    .map(peel_parens)
                    .is_some_and(|c| {
                        matches!(c.kind(), "string" | "concatenated_string" | "identifier")
                            || (c.kind() == "binary_operator" && is_sql_text(p, c))
                    })
            };
            match op {
                // `template % args` is the DB-API injection shape, whatever the template is.
                Some("%") => true,
                // Attribute-only arithmetic (`User.a + User.b`) is an ORM column expression.
                Some("+") => operand_text("left") || operand_text("right"),
                _ => false,
            }
        }
        "call" => node
            .child_by_field_name("function")
            .is_some_and(|f| f.kind() == "attribute" && p.node_text(&f).ends_with(".format")),
        "identifier" => p
            .node_text(&node)
            .chars()
            .next()
            .is_some_and(|c| !c.is_ascii_uppercase()),
        _ => false,
    }
}

fn peel_parens(mut n: Node<'_>) -> Node<'_> {
    while n.kind() == "parenthesized_expression" {
        match n.named_child(0) {
            Some(c) => n = c,
            None => break,
        }
    }
    n
}

/// The SQL argument of `query(..)`: the first positional argument, or the value of a
/// `query=` / `sql=` / `statement=` keyword.
fn query_sql_arg<'t>(p: Src<'_>, call: Node<'t>) -> Option<Node<'t>> {
    let all = args(call);
    for a in &all {
        if a.kind() == "keyword_argument" {
            let named = a.child_by_field_name("name").map(|n| p.node_text(&n));
            if matches!(named, Some("query" | "sql" | "statement")) {
                return a.child_by_field_name("value");
            }
        }
    }
    all.into_iter()
        .find(|a| !matches!(a.kind(), "keyword_argument" | "dictionary_splat"))
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn any_name(names: &[&str], set: &[&str]) -> bool {
    names.iter().any(|n| set.contains(&lower(n).as_str()))
}

/// Last segment of a callee's text (`io.StringIO` -> `StringIO`).
fn last_segment(text: &str) -> &str {
    text.rsplit('.').next().unwrap_or(text)
}

/// A receiver bound (in its function or module) to a file object or an in-memory stream.
fn is_file_like(p: Src<'_>, r: Node<'_>) -> bool {
    if r.kind() != "identifier" {
        return false;
    }
    match resolve(p, r, p.node_text(&r)) {
        PyBind::WithAs(v) | PyBind::Assigned(v) => {
            !is_rebound(p, r, p.node_text(&r), 1)
                && v.kind() == "call"
                && v.child_by_field_name("function")
                    .is_some_and(|f| FILE_MAKERS.contains(&last_segment(p.node_text(&f))))
        }
        _ => false,
    }
}

/// A receiver bound to a list/set/dict (literal, comprehension, constructor) or
/// annotated as a collection: `items.remove(x)` is not a file removal.
fn is_collection(p: Src<'_>, r: Node<'_>) -> bool {
    if r.kind() != "identifier" {
        return false;
    }
    match resolve(p, r, p.node_text(&r)) {
        PyBind::Assigned(_) if is_rebound(p, r, p.node_text(&r), 1) => false,
        PyBind::Param { .. } if is_rebound(p, r, p.node_text(&r), 0) => false,
        PyBind::Assigned(v) => {
            matches!(
                v.kind(),
                "list"
                    | "set"
                    | "dictionary"
                    | "tuple"
                    | "list_comprehension"
                    | "set_comprehension"
                    | "dictionary_comprehension"
            ) || (v.kind() == "call"
                && v.child_by_field_name("function")
                    .is_some_and(|f| COLLECTION_MAKERS.contains(&last_segment(p.node_text(&f)))))
        }
        PyBind::Param {
            annotation: Some(a),
        } => annotation_is_collection(p.node_text(&a)),
        _ => false,
    }
}

/// An annotation names a collection when its base identifier is a collection type
/// (`set[str]`, `typing.Set[str]`); generic arguments are ignored (`Storage[list]` is not one).
/// For a union (`A | B`, `Optional[A]`, `Union[A, B]`) every non-`None` alternative must be.
/// `AssetStore` does not match (no substring matching).
fn annotation_is_collection(text: &str) -> bool {
    let t = text.trim();
    let t = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(t);
    let alts = split_top(t, '|');
    if alts.len() > 1 {
        return alts_are_collections(&alts);
    }
    if let Some((base, inner)) = t.split_once('[') {
        let base = last_segment(base.trim()).to_ascii_lowercase();
        let inner = inner.strip_suffix(']').unwrap_or(inner);
        if base == "optional" {
            return alts_are_collections(&[inner]);
        }
        if base == "union" {
            return alts_are_collections(&split_top(inner, ','));
        }
        return COLLECTION_HINTS.contains(&base.as_str());
    }
    COLLECTION_HINTS.contains(&lower(last_segment(t)).as_str())
}

fn alts_are_collections(alts: &[&str]) -> bool {
    let real: Vec<&&str> = alts.iter().filter(|a| a.trim() != "None").collect();
    !real.is_empty() && real.iter().all(|a| annotation_is_collection(a))
}

/// Split `text` at `sep` outside brackets.
fn split_top(text: &str, sep: char) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in text.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(&text[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

/// A `py-db-call` hit, weak only on proof (#636 design A): the query is a string literal
/// without interpolation, or proves as a statement by the closed allow-list in `sqla`.
fn db_hit(p: Src<'_>, facts: &Facts, query: Option<Node<'_>>) -> SinkHit {
    let weak = query.is_some_and(|q| sqla::stmt_weak(p, facts, q));
    SinkHit::new("py-db-call", "CWE-89").weak_if(weak)
}

fn hit(rule: &'static str, cwe: &'static str) -> Option<SinkHit> {
    Some(SinkHit::new(rule, cwe))
}

pub(super) fn classify(p: Src<'_>, facts: &Facts, id: Node<'_>, text: &str) -> Option<SinkHit> {
    let c = callee_of(p, id)?;
    // A bare callee imported under another name dispatches on its original name.
    let imported = if c.receiver.is_none() {
        facts.names.get(text)
    } else {
        None
    };
    let name = imported.map(|(_, orig)| orig.as_str()).unwrap_or(text);
    let from_module = imported.map(|(m, _)| m.as_str());
    let nargs = args(c.call).len();
    let names = c.receiver.map(|r| chain_names(p, r)).unwrap_or_default();
    // Module a single-identifier receiver or an import stands for.
    let module = match (from_module, c.receiver) {
        (Some(m), _) => Some(m),
        (None, Some(r)) if r.kind() == "identifier" => Some(facts.module_of(p.node_text(&r))),
        _ => None,
    };
    let logger = any_name(&names, LOGGERS);
    let local_def =
        c.receiver.is_none() && imported.is_none() && matches!(resolve(p, id, text), PyBind::Def);
    match name {
        "eval" | "exec" => {
            (nargs > 0 && !logger && !local_def).then(|| SinkHit::new("py-eval", "CWE-95"))
        }
        "system" | "popen" => {
            (nargs > 0 && !local_def).then(|| SinkHit::new("py-os-command", "CWE-78"))
        }
        n if PROCESS_NAMES.contains(&n) => {
            (nargs > 0).then(|| SinkHit::new("py-os-command", "CWE-78"))
        }
        "spawn" => (module == Some("pty")).then(|| SinkHit::new("py-os-command", "CWE-78")),
        "Popen" | "check_call" | "check_output" => hit("py-subprocess", "CWE-78"),
        "run" | "call" => {
            (module == Some("subprocess")).then(|| SinkHit::new("py-subprocess", "CWE-78"))
        }
        n if SQL_METHODS.contains(&n) || n == "raw" => {
            (nargs > 0 && !logger && !local_def).then(|| db_hit(p, facts, query_sql_arg(p, c.call)))
        }
        n if PANDAS_SQL.contains(&n) => hit("py-db-call", "CWE-89"),
        "query" => {
            let arg = query_sql_arg(p, c.call).filter(|a| is_sql_text(p, *a))?;
            let weak = sqla::query_weak(p, facts, arg);
            Some(SinkHit::new("py-db-call", "CWE-89").weak_if(weak))
        }
        // ORM clause entry points: the clause argument is where raw SQL can enter.
        // Keyword-only calls (`Model.objects.filter(pk=x)`, `filter_by(id=x)`) are
        // parameterised lookups, not clauses. A positional expression such as
        // `User.id == x` is reported too (owner decision 2026-11-02: weak TP).
        "where" | "filter" | "filter_by" => {
            let clause = args(c.call)
                .iter()
                .any(|a| !matches!(a.kind(), "keyword_argument" | "dictionary_splat"));
            // Weak (#636): the owner scores the positional ORM clause as a defused TP, when
            // the whole chain proves by the allow-list.
            (c.receiver.is_some() && clause && !logger).then(|| {
                SinkHit::new("py-sql-clause", "CWE-89").weak_if(sqla::clause_weak(p, facts, c.call))
            })
        }
        "text" => {
            let from_sqlalchemy = module == Some("sqlalchemy");
            (from_sqlalchemy && args(c.call).first().is_some_and(|a| !is_literal(p, *a)))
                .then(|| SinkHit::new("py-sql-clause", "CWE-89"))
        }
        "Template" => {
            let known = module.is_some_and(|m| TEMPLATE_MODULES.contains(&m));
            (known && args(c.call).first().is_some_and(|a| !is_literal(p, *a)))
                .then(|| SinkHit::new("py-template", "CWE-1336"))
        }
        "mark_safe" | "Markup" => hit("py-html", "CWE-79"),
        "render_template_string" => hit("py-template", "CWE-1336"),
        "getattr" | "setattr" => {
            let name_arg = args(c.call).into_iter().nth(1)?;
            (c.receiver.is_none() && !is_literal(p, name_arg))
                .then(|| SinkHit::new("py-dynamic-attr", "CWE-470"))
        }
        "redirect" | "HttpResponseRedirect" => hit("py-redirect", "CWE-601"),
        "open" => {
            let pil = names.iter().any(|n| *n == "Image");
            ((nargs > 0 || c.receiver.is_some_and(|r| r.kind() == "call")) && !pil)
                .then(|| SinkHit::new("py-file-open", "CWE-22"))
        }
        "remove" => remove_hit(p, &c, nargs, logger, local_def),
        "unlink" | "rmdir" | "removedirs" | "rmtree" => hit("py-file-remove", "CWE-22"),
        "send" | "write" => {
            let channel = any_name(&names, CHANNELS);
            let file = c.receiver.is_some_and(|r| is_file_like(p, r));
            (nargs > 0 && !logger && !channel && !file && !local_def)
                .then(|| SinkHit::new("py-output", "CWE-79"))
        }
        _ => None,
    }
}

fn remove_hit(
    p: Src<'_>,
    c: &Callee<'_>,
    nargs: usize,
    logger: bool,
    local_def: bool,
) -> Option<SinkHit> {
    let collection = c.receiver.is_some_and(|r| is_collection(p, r));
    (nargs > 0 && !logger && !collection && !local_def)
        .then(|| SinkHit::new("py-file-remove", "CWE-22"))
}
