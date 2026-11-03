//! Closed tables for the Python SQL weak proof (#636 design A). Anything not listed here is
//! unproven and the finding stays strong. Every entry is a call that never renders an argument as
//! SQL text, or renders it only as a quoted identifier or a bound parameter.
//!
//! Argument classes: `C` clause (becomes SQL structure), `V` value (becomes a bound parameter),
//! `M` either, `F` from, `S` statement, `L` plain literal, `T` type, `O` loader option, `X` literal
//! or type, `N`/`-` nothing. A positional list repeats its last class; a keyword table names the
//! class per keyword, `*` for any.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Lib {
    Sqla,
    Peewee,
    Django,
}

/// `(library, path below the library root)` for an import `from module import member`
/// (`member` empty for `import module`). The package boundary is exact: `sqlalchemy_utils` is
/// not `sqlalchemy`.
pub(super) fn lib_path(module: &str, member: &str) -> Option<(Lib, Vec<String>)> {
    let full = if member.is_empty() {
        module.to_string()
    } else {
        format!("{module}.{member}")
    };
    for (root, lib) in [
        ("sqlalchemy", Lib::Sqla),
        ("peewee", Lib::Peewee),
        ("django.db.models", Lib::Django),
    ] {
        if full == root {
            return Some((lib, Vec::new()));
        }
        if let Some(rest) = full.strip_prefix(root).and_then(|r| r.strip_prefix('.')) {
            return Some((lib, rest.split('.').map(str::to_string).collect()));
        }
    }
    None
}

pub(super) type Kw = &'static [(&'static str, char)];
pub(super) struct Entry {
    pub name: &'static str,
    pub pos: &'static str,
    pub kw: Kw,
}
const fn e(name: &'static str, pos: &'static str, kw: Kw) -> Entry {
    Entry { name, pos, kw }
}
const NO: Kw = &[];
const ANY_V: Kw = &[("*", 'V')];
const ANY_L: Kw = &[("*", 'L')];
const TYPE_KW: Kw = &[("type_", 'T')];

pub(super) fn find(table: &'static [Entry], name: &str) -> Option<&'static Entry> {
    table.iter().find(|x| x.name == name)
}

/// Methods of a statement, query or manager (the receiver must itself prove as a statement).
pub(super) const CHAIN: &[Entry] = &[
    // Clause methods: the argument becomes WHERE/HAVING/ORDER BY/GROUP BY structure.
    e("where", "C", ANY_V),
    e("filter", "C", ANY_V),
    e("exclude", "C", ANY_V),
    e("having", "C", NO),
    e("order_by", "C", NO),
    e("group_by", "C", NO),
    e("add_columns", "C", NO),
    e("with_only_columns", "C", NO),
    e("with_entities", "C", NO),
    e("columns", "C", NO),
    e("returning", "C", NO),
    e("distinct", "C", NO),
    e("select", "C", NO),
    // Bound keywords only.
    e("filter_by", "-", ANY_V),
    e("values", "V", ANY_V),
    e("params", "V", ANY_V),
    e("bindparams", "V", ANY_V),
    e("update", "V", ANY_L),
    e("execution_options", "-", ANY_V),
    e("limit", "V", NO),
    e("offset", "V", NO),
    e("yield_per", "V", NO),
    e("using", "V", NO),
    e("get", "V", NO),
    e("paginate", "-", ANY_V),
    // FROM structure.
    e("join", "F C", ANY_L),
    e("outerjoin", "F C", ANY_L),
    e("join_from", "F F C", ANY_L),
    e("select_from", "F", NO),
    e("correlate", "F", NO),
    e("correlate_except", "F", NO),
    // Names are quoted identifiers and must be literals.
    e("subquery", "L", ANY_L),
    e("alias", "L", ANY_L),
    e("cte", "L", ANY_L),
    e("lateral", "L", ANY_L),
    e("label", "L", NO),
    e("values_list", "L", ANY_L),
    e("with_for_update", "-", ANY_L),
    e("delete", "N", ANY_L),
    e("options", "O", NO),
    e("union", "S", NO),
    e("union_all", "S", NO),
    e("intersect", "S", NO),
    e("except_", "S", NO),
    // No arguments.
    e("scalar_subquery", "N", NO),
    e("exists", "N", NO),
    e("insert", "N", NO),
    e("statement", "N", NO),
    e("populate_existing", "N", NO),
    e("all", "N", NO),
    e("first", "N", NO),
    e("one", "N", NO),
    e("one_or_none", "N", NO),
    e("scalar", "N", NO),
    e("scalar_one", "N", NO),
    e("scalar_one_or_none", "N", NO),
    e("scalars", "N", NO),
    e("mappings", "N", NO),
    e("count", "N", NO),
    e("unique", "N", NO),
    e("first_or_404", "N", NO),
    e("one_or_404", "N", NO),
];

/// Chain names that read a result in a value position: data, not a statement builder.
pub(super) const TERMINALS: &[&str] = &[
    "all",
    "first",
    "one",
    "one_or_none",
    "scalar",
    "scalar_one",
    "scalar_one_or_none",
    "scalars",
    "mappings",
    "count",
    "get",
    "unique",
    "first_or_404",
    "one_or_404",
    "paginate",
];

/// `<schema root>.select(*C)` and the like (peewee models, SQLAlchemy tables).
pub(super) const ROOT_METHODS: &[Entry] = &[
    e("select", "C", NO),
    e("insert", "N", ANY_V),
    e("update", "N", ANY_V),
    e("delete", "N", NO),
];

/// Statement roots imported from SQLAlchemy.
pub(super) const ROOTS: &[Entry] = &[
    e("select", "C", NO),
    e("exists", "C", NO),
    e("insert", "F", NO),
    e("update", "F", NO),
    e("delete", "F", NO),
    e("union", "S", NO),
    e("union_all", "S", NO),
    e("intersect", "S", NO),
    e("except_", "S", NO),
];

/// SQLAlchemy and Django expression constructs (`case`, `cast` and `type_coerce` have special
/// argument shapes handled in the grader).
pub(super) const CONSTRUCTS: &[Entry] = &[
    e("and_", "C", NO),
    e("or_", "C", NO),
    e("not_", "C", NO),
    e("distinct", "C", NO),
    e("desc", "C", NO),
    e("asc", "C", NO),
    e("nulls_first", "C", NO),
    e("nulls_last", "C", NO),
    e("nullsfirst", "C", NO),
    e("nullslast", "C", NO),
    e("any_", "C", NO),
    e("all_", "C", NO),
    e("between", "C V V", &[("symmetric", 'L')]),
    e("extract", "L C", NO),
    e("collate", "C L", NO),
    e("literal", "V", TYPE_KW),
    e(
        "bindparam",
        "L V",
        &[
            ("type_", 'T'),
            ("expanding", 'L'),
            ("value", 'V'),
            ("unique", 'L'),
        ],
    ),
    e("tuple_", "M", NO),
    e("null", "N", NO),
    e("true", "N", NO),
    e("false", "N", NO),
    e("aliased", "F", &[("name", 'L'), ("flat", 'L')]),
    e("inspect", "F", NO),
    e("Table", "L V M", NO),
    e("Column", "L X", &[("*", 'X')]),
    // Django (D4): lookups and references only.
    e("Q", "C", ANY_V),
    e("Exists", "S", NO),
    e("Subquery", "S", NO),
    e("OuterRef", "L", NO),
    e("F", "L", NO),
    e("Value", "V", &[("output_field", 'T')]),
];

/// Methods of a column element; the receiver must prove as a clause.
pub(super) const COLUMN_METHODS: &[Entry] = &[
    e("in_", "V", NO),
    e("not_in", "V", NO),
    e("notin_", "V", NO),
    e("like", "V", &[("escape", 'L')]),
    e("ilike", "V", &[("escape", 'L')]),
    e("notlike", "V", &[("escape", 'L')]),
    e("notilike", "V", &[("escape", 'L')]),
    e("not_like", "V", &[("escape", 'L')]),
    e("not_ilike", "V", &[("escape", 'L')]),
    e("startswith", "V", &[("escape", 'L'), ("autoescape", 'L')]),
    e("endswith", "V", &[("escape", 'L'), ("autoescape", 'L')]),
    e("contains", "V", &[("escape", 'L'), ("autoescape", 'L')]),
    e("icontains", "V", NO),
    e("istartswith", "V", NO),
    e("iendswith", "V", NO),
    e("is_", "V", NO),
    e("is_not", "V", NO),
    e("isnot", "V", NO),
    e("is_distinct_from", "V", NO),
    e("is_not_distinct_from", "V", NO),
    e("isnot_distinct_from", "V", NO),
    e("between", "V V", NO),
    e("match", "V", NO),
    e("concat", "V", NO),
    e("desc", "N", NO),
    e("asc", "N", NO),
    e("nulls_first", "N", NO),
    e("nulls_last", "N", NO),
    e("nullsfirst", "N", NO),
    e("nullslast", "N", NO),
    e("distinct", "N", NO),
    e("is_true", "N", NO),
    e("is_false", "N", NO),
    e("any_", "N", NO),
    e("all_", "N", NO),
    e("label", "L", NO),
    e("collate", "L", NO),
    e("cast", "T", NO),
    e("any", "C", &[("*", 'V')]),
    e("has", "C", &[("*", 'V')]),
    e("over", "-", &[("partition_by", 'C'), ("order_by", 'C')]),
    e("filter", "C", NO),
    e("within_group", "C", NO),
];

/// Loader options of `options(..)`: arguments are column references or literals.
pub(super) const LOADERS: &[&str] = &[
    "joinedload",
    "selectinload",
    "subqueryload",
    "lazyload",
    "load_only",
    "defer",
    "undefer",
    "contains_eager",
    "noload",
    "raiseload",
    "defaultload",
    "with_expression",
];

/// Callables that put an argument into SQL text, whoever the receiver is (`db.literal_column`).
pub(super) const BELT: &[&str] = &[
    "text",
    "literal_column",
    "column",
    "table",
    "TextClause",
    "ColumnClause",
    "quoted_name",
    "SQL",
    "RawSQL",
    "extra",
    "raw",
    "op",
    "bool_op",
    "custom_op",
    "prefix_with",
    "suffix_with",
];

/// A statement-typed annotation (`stmt: sa.Select`).
pub(super) const STMT_ANNOTATIONS: &[&str] =
    &["Select", "Insert", "Update", "Delete", "CompoundSelect"];
/// A schema-root annotation (D2): a table-like object, or `type[...]`.
pub(super) const ROOT_ANNOTATIONS: &[&str] = &[
    "Table",
    "Select",
    "Subquery",
    "Alias",
    "CTE",
    "FromClause",
    "TableClause",
    "Join",
];
