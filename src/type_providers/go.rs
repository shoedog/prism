//! Go type provider — extracts struct/interface definitions and method sets
//! from tree-sitter ASTs for interface satisfaction and dispatch resolution.
//!
//! Go's type system is structural: a type satisfies an interface if it has all
//! the interface's methods, regardless of explicit `implements` declarations.
//! This provider computes satisfaction by comparing method sets.

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::languages::Language;
use crate::type_provider::{
    DispatchProvider, ResolvedType, ResolvedTypeKind, TypeFieldInfo, TypeProvider,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Non-dispatchable, fail-closed — mints NO edge (spec §15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoDispatchGap {
    Generic,            // decl carries a type_parameter_list / interface type-set
    AnonymousInterface, // non-empty anonymous interface in a signature
    UnknownCanonType,   // unenumerated type node — fail closed
}

/// Admitted over-approximation — the Exact edge IS minted; a precision counter (spec §15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoDispatchOverApprox {
    CrossPackageBareName,         // io.Reader ≡ bufio.Reader under bare-name canon
    NonLocalConstructionFallback, // empty-live fallback fired: full satisfier set
}

pub struct InterfaceDispatchTable {
    pub impls: BTreeMap<(String, String), Vec<FunctionId>>,
    pub gaps: Vec<GoDispatchGap>,
    pub overapprox: Vec<GoDispatchOverApprox>,
    pub fanout: BTreeMap<(String, String), usize>,
    pub fallback_fired: BTreeMap<(String, String), bool>,
}

// ---------------------------------------------------------------------------
// Extracted type information
// ---------------------------------------------------------------------------

/// A Go struct definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GoStruct {
    /// Struct name.
    name: String,
    /// Fields: (name, type_string). Anonymous/embedded fields have name == type.
    fields: Vec<(String, String)>,
    /// Embedded fields (for promoted methods).
    embedded: Vec<GoEmbeddedField>,
    /// Source file path.
    file: String,
    /// Line number of the type declaration.
    line: usize,
}

/// An embedded struct field, preserving whether it was written as `T` or `*T`.
#[derive(Debug, Clone)]
struct GoEmbeddedField {
    name: String,
    is_pointer: bool,
}

/// A Go interface definition.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GoInterface {
    /// Interface name.
    name: String,
    /// Method signatures: method_name → canonical parameter/return signature.
    methods: BTreeMap<String, Result<String, GoDispatchGap>>,
    /// Embedded interfaces.
    embedded: Vec<String>,
    /// Whether the enclosing type_spec carries type parameters.
    generic: bool,
    /// Source file path.
    file: String,
}

/// A method attached to a type via a receiver.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GoMethod {
    /// Method name.
    name: String,
    /// Receiver type name (without pointer, e.g., "Server" not "*Server").
    receiver_type: String,
    /// Whether the receiver is a pointer receiver (*T).
    is_pointer_receiver: bool,
    /// Canonical signature for interface matching (parameter types → return types).
    signature: Result<String, GoDispatchGap>,
    /// Whether the method declaration carries type parameters.
    generic: bool,
    /// Source file.
    file: String,
    /// Start line.
    start_line: usize,
    /// End line.
    end_line: usize,
    /// Number of parameter names (excluding receiver); multi-name decls counted per-name.
    params: usize,
    /// True if the last parameter is variadic (`...T`).
    variadic: bool,
}

/// A concrete method promoted onto an outer struct via embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotedMethod {
    pub struct_name: String,
    pub method: String,
    pub func_id: FunctionId,
    pub depth: usize,
}

#[derive(Debug, Clone)]
struct PromotedMethodCandidate {
    promoted: PromotedMethod,
    value_method_set: bool,
}

// ---------------------------------------------------------------------------
// GoTypeProvider
// ---------------------------------------------------------------------------

/// Inner data for GoTypeProvider, shared via Arc when registered as both
/// TypeProvider and DispatchProvider.
pub struct GoTypeData {
    /// Struct definitions by name.
    structs: BTreeMap<String, GoStruct>,
    /// Interface definitions by name.
    interfaces: BTreeMap<String, GoInterface>,
    /// Methods grouped by receiver type name.
    methods: BTreeMap<String, Vec<GoMethod>>,
    /// Type aliases: alias_name → canonical_name.
    aliases: BTreeMap<String, String>,
    /// Precomputed interface satisfaction: interface_name → set of concrete types.
    satisfaction: BTreeMap<String, BTreeSet<String>>,
    /// Full-interface satisfier admission keys: (interface, method) → (T/*T, method id).
    sat_keys: BTreeMap<(String, String), Vec<(String, FunctionId)>>,
    /// Fail-closed dispatch gaps observed during canonical satisfaction.
    dispatch_gaps: Vec<GoDispatchGap>,
    /// Over-approximations admitted during canonical satisfaction.
    dispatch_overapprox: Vec<GoDispatchOverApprox>,
    /// P5 S1: every package-scoped struct identity extracted, regardless of
    /// whether it has any func-typed field. Lets a caller distinguish "struct
    /// known, field not func-typed" from "struct unknown entirely" (the S2
    /// registration scan's per-form fallback/counting depends on this).
    struct_identities: BTreeSet<crate::resolution::GoOwnerIdentity>,
    /// P5 S1: `(owner_identity, field_name)` pairs whose declared type begins
    /// with `func(` (named-type indirection, e.g. `type Handler func()`, is
    /// out of scope — noted, not attempted). Package-scoped so a registration
    /// in one package cannot donate a false hit to a same-named struct in
    /// another (spec-review MAJOR-1).
    func_typed_fields: BTreeSet<(crate::resolution::GoOwnerIdentity, String)>,
    /// P11 S2: `(owner_identity, field_name) -> raw declared field type`
    /// re-projection of every struct's `fields` (including embedded-field
    /// pseudo-entries — `GoStruct::extract_struct_fields` already names those
    /// by the embedded type's bare name). Package-scoped, same rationale as
    /// `func_typed_fields` above.
    field_types: BTreeMap<(crate::resolution::GoOwnerIdentity, String), String>,
    /// P11 S4: `struct_name -> method_name -> (interface_name, signature)` for
    /// a method supplied ONLY by exactly one directly-embedded in-repo
    /// interface, with no own/promoted concrete method or field shadowing it.
    /// Deliberately keyed by NAME, never a `FunctionId` — see the doc on
    /// `embedded_interface_promotions` for why a synthetic id must never
    /// reach `insert_sat_keys`/`sat_keys`. Bare-keyed like `structs`/
    /// `interfaces` — an ACCEPTED pre-existing approximation for the
    /// satisfaction-MEMBERSHIP check only (`method_set_satisfies_with_embedded`,
    /// consumed via `compute_satisfaction`'s `concrete_methods` iteration,
    /// itself already bare-collapsed upstream). Never used for routing — see
    /// `embedded_interface_routes` (package-scoped) for that (B2 fix).
    embedded_interface_promotions: BTreeMap<String, BTreeMap<String, (String, String)>>,
    /// P11 S4 (B2 fix, codex impl-review BLOCKER): package-scoped mirror of
    /// `GoStruct.embedded`, captured alongside `field_types` at extraction
    /// time (same rationale — `structs` collapses same-named structs across
    /// packages via last-file-wins `insert`, which would otherwise let one
    /// package's `Holder` donate its embedded-interface methods to an
    /// unrelated same-named `Holder` in another package). Keyed by the
    /// EMBEDDING struct's own `GoOwnerIdentity`.
    struct_embeds: BTreeMap<crate::resolution::GoOwnerIdentity, Vec<GoEmbeddedField>>,
    /// P11 S4 (B2 fix): bare interface name -> set of package dirs that
    /// declare an interface with that name. `interfaces` collapses same-named
    /// interfaces across packages (last-file-wins), so it alone cannot tell
    /// "unique to this package" from "some other package's interface of the
    /// same name happened to win the collapse". An embedded field is always
    /// written BARE for a same-package interface (Go scoping) — a bare name
    /// only ever safely routes through `interfaces.get(bare)` when this set
    /// shows the name is package-unique repo-wide AND that one package is the
    /// embedding struct's own; otherwise fail closed (B2's "not
    /// package-unique" case).
    interface_name_owners: BTreeMap<String, BTreeSet<String>>,
    /// P11 S4 (B2 fix): the package-scoped ROUTING map computed by
    /// `embedded_interface_routes`, captured onto `CallGraph.
    /// go_embedded_interface_methods` via `embedded_interface_method_routes`.
    embedded_interface_routes:
        BTreeMap<crate::resolution::GoOwnerIdentity, BTreeMap<String, String>>,
}

#[derive(Debug, Clone)]
struct CanonMethod {
    signature: String,
    func_id: FunctionId,
}

#[derive(Debug, Default)]
struct ReceiverMethodSet {
    value: BTreeMap<String, CanonMethod>,
    pointer: BTreeMap<String, CanonMethod>,
}

/// Go type provider that extracts struct/interface definitions and method sets
/// from tree-sitter ASTs, then computes interface satisfaction for dispatch.
///
/// Uses `Arc<GoTypeData>` so the same extracted data can be shared when
/// registered as both `TypeProvider` and `DispatchProvider` in the registry.
#[derive(Clone)]
pub struct GoTypeProvider {
    pub data: Arc<GoTypeData>,
}

impl GoTypeProvider {
    /// Build a GoTypeProvider by scanning all Go parsed files.
    pub fn from_parsed_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut inner = GoTypeData {
            structs: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            methods: BTreeMap::new(),
            aliases: BTreeMap::new(),
            satisfaction: BTreeMap::new(),
            sat_keys: BTreeMap::new(),
            dispatch_gaps: Vec::new(),
            dispatch_overapprox: Vec::new(),
            struct_identities: BTreeSet::new(),
            func_typed_fields: BTreeSet::new(),
            field_types: BTreeMap::new(),
            embedded_interface_promotions: BTreeMap::new(),
            struct_embeds: BTreeMap::new(),
            interface_name_owners: BTreeMap::new(),
            embedded_interface_routes: BTreeMap::new(),
        };

        for (path, parsed) in files {
            if parsed.language != Language::Go {
                continue;
            }
            Self::extract_from_file(&mut inner, path, parsed);
        }

        Self::compute_satisfaction(&mut inner);
        GoTypeProvider {
            data: Arc::new(inner),
        }
    }

    /// All method names declared on some known interface (Phase-IP PR-2 manifest
    /// denominator, spec §8a). Captured onto `CallGraph` at build time because the
    /// provider is not retained post-build; lets the manifest decide whether an
    /// interface-method call-site is in-scope without a live provider.
    pub fn interface_method_names(&self) -> std::collections::BTreeSet<String> {
        self.data
            .interfaces
            .values()
            .flat_map(|iface| iface.methods.keys().cloned())
            .collect()
    }

    /// P5 S1: every package-scoped struct identity extracted (regardless of
    /// func-typed fields). Captured onto `CallGraph.go_known_struct_identities`
    /// in `apply_go_func_value_fields`.
    pub fn go_known_struct_identities(&self) -> BTreeSet<crate::resolution::GoOwnerIdentity> {
        self.data.struct_identities.clone()
    }

    /// P5 S1: `(owner_identity, field_name)` pairs whose declared field type
    /// begins with `func(`. Captured onto `CallGraph.go_func_typed_fields` in
    /// `apply_go_func_value_fields`.
    pub fn go_func_typed_fields(&self) -> BTreeSet<(crate::resolution::GoOwnerIdentity, String)> {
        self.data.func_typed_fields.clone()
    }

    /// P11 S2: package-scoped struct-field re-projection. Captured onto
    /// `CallGraph.go_field_types` in `apply_go_interface_dispatch`.
    pub fn go_field_types(&self) -> BTreeMap<(crate::resolution::GoOwnerIdentity, String), String> {
        self.data.field_types.clone()
    }

    /// P11 S4 routing map: `GoOwnerIdentity (struct) -> method_name ->
    /// interface_bare_name` (signature dropped — routing only needs the
    /// providing interface's name to consult `interface_impls`).
    /// Package-scoped (B2 fix, codex impl-review BLOCKER) — see
    /// `embedded_interface_routes`'s doc for why this must NOT be the bare
    /// `embedded_interface_promotions` map. Captured onto `CallGraph.
    /// go_embedded_interface_methods` in `apply_go_interface_dispatch`.
    pub fn embedded_interface_method_routes(
        &self,
    ) -> BTreeMap<crate::resolution::GoOwnerIdentity, BTreeMap<String, String>> {
        self.data.embedded_interface_routes.clone()
    }

    /// Param arity for every Go method extracted from the parsed files.  Keyed by
    /// `FunctionId`; receiver is excluded from `params`; variadic is flagged.
    /// Captured onto `CallGraph.method_arity` in `apply_go_interface_dispatch`.
    pub fn method_arities(&self) -> BTreeMap<FunctionId, crate::call_graph::MethodArity> {
        self.data
            .methods
            .values()
            .flat_map(|methods| methods.iter())
            .map(|m| {
                (
                    Self::method_function_id(m),
                    crate::call_graph::MethodArity {
                        params: m.params,
                        variadic: m.variadic,
                    },
                )
            })
            .collect()
    }

    /// Compute named in-repo interface dispatch, RTA-pruned to `live` (admission keys),
    /// receiver-kind-aware, with the empty-live fallback kept Exact (spec §5/§7/§8).
    pub fn compute_interface_dispatch(&self, live: &BTreeSet<String>) -> InterfaceDispatchTable {
        let mut t = InterfaceDispatchTable {
            impls: BTreeMap::new(),
            gaps: self.data.dispatch_gaps.clone(),
            overapprox: self.data.dispatch_overapprox.clone(), // CrossPackageBareName seeded in Task 4
            fanout: BTreeMap::new(),
            fallback_fired: BTreeMap::new(),
        };
        // sat_keys: (iface, method) -> Vec<(admission_key, FunctionId)>  (Task 4)
        for ((iface, method), sats) in &self.data.sat_keys {
            if sats.is_empty() {
                continue;
            }
            let live_hits: Vec<&FunctionId> = sats
                .iter()
                .filter(|(k, _)| live.contains(k))
                .map(|(_, fid)| fid)
                .collect();
            let (chosen, fired): (Vec<FunctionId>, bool) = if live_hits.is_empty() {
                // receiver-kind-aware-empty fallback: full satisfier set, kept Exact.
                (sats.iter().map(|(_, fid)| fid.clone()).collect(), true)
            } else {
                (live_hits.into_iter().cloned().collect(), false)
            };
            // de-dup FunctionIds (a value+pointer satisfier may appear twice).
            let mut uniq: Vec<FunctionId> = Vec::new();
            for fid in chosen {
                if !uniq.contains(&fid) {
                    uniq.push(fid);
                }
            }
            let key = (iface.clone(), method.clone());
            t.fanout.insert(key.clone(), uniq.len());
            t.fallback_fired.insert(key.clone(), fired);
            if fired {
                t.overapprox
                    .push(GoDispatchOverApprox::NonLocalConstructionFallback);
            }
            if !uniq.is_empty() {
                t.impls.insert(key, uniq);
            }
        }
        t
    }

    // -----------------------------------------------------------------------
    // AST extraction (static methods operating on GoTypeData)
    // -----------------------------------------------------------------------

    /// Extract type information from a single Go file.
    fn extract_from_file(data: &mut GoTypeData, path: &str, parsed: &ParsedFile) {
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            match child.kind() {
                "type_declaration" => {
                    Self::extract_type_declaration(data, &child, path, parsed);
                }
                "method_declaration" => {
                    Self::extract_method(data, &child, path, parsed);
                }
                _ => {}
            }
        }
    }

    /// Extract type specs from a type_declaration node.
    fn extract_type_declaration(
        data: &mut GoTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_spec" => Self::extract_type_spec(data, &child, path, parsed),
                "type_alias" => Self::extract_type_alias(data, &child, parsed),
                _ => {}
            }
        }
    }

    /// Extract a single type_spec: `name type_definition`.
    fn extract_type_spec(
        data: &mut GoTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();
        if name.is_empty() {
            return;
        }

        let type_node = match node.child_by_field_name("type") {
            Some(n) => n,
            None => return,
        };

        let line = node.start_position().row + 1;

        match type_node.kind() {
            "struct_type" => {
                let (fields, embedded) = Self::extract_struct_fields(&type_node, parsed);
                // P5 S1: package-scoped owner identity, distinct from the bare
                // `name` key `data.structs` uses (spec-review MAJOR-1).
                let owner = crate::resolution::GoOwnerIdentity {
                    package_dir: crate::resolution::dir_of(path).to_string(),
                    name: name.clone(),
                };
                data.struct_identities.insert(owner.clone());
                for (field_name, field_type) in &fields {
                    if field_type.trim_start().starts_with("func(") {
                        data.func_typed_fields
                            .insert((owner.clone(), field_name.clone()));
                    }
                    // P11 S2: re-project every field (including embedded
                    // pseudo-fields) as a package-scoped owner/field -> type
                    // index, for the nested-selector receiver-recovery pass.
                    data.field_types
                        .insert((owner.clone(), field_name.clone()), field_type.clone());
                }
                // P11 S4 (B2 fix): package-scoped embed list, parallel to the
                // bare `data.structs` insert below (which last-file-wins
                // collapses same-named structs across packages).
                data.struct_embeds.insert(owner, embedded.clone());
                data.structs.insert(
                    name.clone(),
                    GoStruct {
                        name,
                        fields,
                        embedded,
                        file: path.to_string(),
                        line,
                    },
                );
            }
            "interface_type" => {
                let generic = Self::has_generic_syntax(node)
                    || Self::interface_type_has_type_set(&type_node, parsed);
                let (methods, embedded, overapprox) =
                    Self::extract_interface_methods(&type_node, parsed);
                data.dispatch_overapprox.extend(overapprox);
                // P11 S4 (B2 fix): track which package dir(s) declare an
                // interface with this bare name, so the routing computation
                // can fail closed on a cross-package bare-name collision
                // instead of trusting whichever file's entry the collapsing
                // `interfaces.insert` below happens to keep.
                data.interface_name_owners
                    .entry(name.clone())
                    .or_default()
                    .insert(crate::resolution::dir_of(path).to_string());
                data.interfaces.insert(
                    name.clone(),
                    GoInterface {
                        name,
                        methods,
                        embedded,
                        generic,
                        file: path.to_string(),
                    },
                );
            }
            _ => {
                // Type definition: `type Name OtherType`
                let target = parsed.node_text(&type_node).trim().to_string();
                if !target.is_empty() {
                    data.aliases.insert(name, target);
                }
            }
        }
    }

    fn has_generic_syntax(node: &tree_sitter::Node) -> bool {
        Self::node_has_any_kind(
            node,
            &["type_parameter_list", "generic_type", "type_arguments"],
        )
    }

    /// Generic-syntax detection scoped to a declaration's SIGNATURE (receiver / type
    /// parameters / parameters / result), never its body `block`. A generic
    /// instantiation *inside* a method body (`var _ Box[int]`) must NOT flag the method
    /// itself as generic — that would wrongly exclude it from satisfaction (review MAJOR).
    /// `pub(crate)` (P11 S1): reused by `go_receiver_index.rs`'s return-type
    /// extraction so the generic-decl gate never drifts from the dispatch
    /// provider's own signature-scoped definition of "generic".
    pub(crate) fn signature_has_generic_syntax(node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        children.iter().filter(|c| c.kind() != "block").any(|c| {
            Self::node_has_any_kind(
                c,
                &["type_parameter_list", "generic_type", "type_arguments"],
            )
        })
    }

    fn interface_type_has_type_set(node: &tree_sitter::Node, parsed: &ParsedFile) -> bool {
        if node.kind() != "interface_type" {
            return false;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        children.iter().any(|child| {
            child.kind() == "type_elem" && !Self::is_plain_embedded_interface_elem(child, parsed)
        })
    }

    fn is_plain_embedded_interface_elem(node: &tree_sitter::Node, parsed: &ParsedFile) -> bool {
        let text = parsed.node_text(node);
        let compact = text.replace(char::is_whitespace, "");
        if compact.contains('~') || compact.contains('|') || Self::is_predeclared_go_type(&compact)
        {
            return false;
        }

        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).collect();
        children.len() == 1 && matches!(children[0].kind(), "type_identifier" | "qualified_type")
    }

    fn is_predeclared_go_type(name: &str) -> bool {
        matches!(
            name,
            "any"
                | "bool"
                | "byte"
                | "comparable"
                | "complex64"
                | "complex128"
                | "error"
                | "float32"
                | "float64"
                | "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "rune"
                | "string"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uintptr"
        )
    }

    fn node_has_any_kind(node: &tree_sitter::Node, kinds: &[&str]) -> bool {
        if kinds.contains(&node.kind()) {
            return true;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        children
            .iter()
            .any(|child| Self::node_has_any_kind(child, kinds))
    }

    /// Extract a type alias: `type Name = OtherType`.
    fn extract_type_alias(data: &mut GoTypeData, node: &tree_sitter::Node, parsed: &ParsedFile) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let type_node = match node.child_by_field_name("type") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();
        let target = parsed.node_text(&type_node).trim().to_string();
        if !name.is_empty() && !target.is_empty() {
            data.aliases.insert(name, target);
        }
    }

    /// Extract fields from a struct_type node.
    fn extract_struct_fields(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (Vec<(String, String)>, Vec<GoEmbeddedField>) {
        let mut fields = Vec::new();
        let mut embedded = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "field_declaration_list" {
                let mut inner = child.walk();
                for field in child.children(&mut inner) {
                    if field.kind() == "field_declaration" {
                        Self::extract_one_field(&field, parsed, &mut fields, &mut embedded);
                    }
                }
            }
        }
        (fields, embedded)
    }

    /// Extract a single field_declaration.
    fn extract_one_field(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
        fields: &mut Vec<(String, String)>,
        embedded: &mut Vec<GoEmbeddedField>,
    ) {
        let mut names: Vec<String> = Vec::new();
        let mut type_str = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "field_identifier" => {
                    names.push(parsed.node_text(&child).trim().to_string());
                }
                _ if type_str.is_empty()
                    && child.kind() != "field_identifier"
                    && child.kind() != "tag"
                    && child.kind() != "comment"
                    && child.kind() != "," =>
                {
                    if child.kind() != "field_identifier" {
                        type_str = parsed.node_text(&child).trim().to_string();
                    }
                }
                _ => {}
            }
        }

        if names.is_empty() {
            let embedded_name = strip_pointer(&type_str);
            if !embedded_name.is_empty() {
                embedded.push(GoEmbeddedField {
                    name: embedded_name.to_string(),
                    is_pointer: type_str.trim().starts_with('*'),
                });
                fields.push((embedded_name.to_string(), type_str));
            }
        } else {
            for name in names {
                fields.push((name, type_str.clone()));
            }
        }
    }

    /// Extract method signatures from an interface_type node.
    fn extract_interface_methods(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> (
        BTreeMap<String, Result<String, GoDispatchGap>>,
        Vec<String>,
        Vec<GoDispatchOverApprox>,
    ) {
        let mut methods = BTreeMap::new();
        let mut embedded = Vec::new();
        let mut overapprox = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_interface_body(&child, parsed, &mut methods, &mut embedded, &mut overapprox);
        }

        (methods, embedded, overapprox)
    }

    fn walk_interface_body(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
        methods: &mut BTreeMap<String, Result<String, GoDispatchGap>>,
        embedded: &mut Vec<String>,
        overapprox: &mut Vec<GoDispatchOverApprox>,
    ) {
        match node.kind() {
            "method_spec" | "method_elem" => {
                let mut name = String::new();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "field_identifier" && name.is_empty() {
                        name = parsed.node_text(&child).trim().to_string();
                    }
                }
                if !name.is_empty() {
                    let sig = Self::extract_method_signature(node, parsed);
                    if sig.is_ok() && Self::signature_has_qualified_type(node) {
                        overapprox.push(GoDispatchOverApprox::CrossPackageBareName);
                    }
                    methods.insert(name, sig);
                }
            }
            "type_identifier" | "qualified_type" => {
                let iface_name = parsed.node_text(node).trim().to_string();
                if !iface_name.is_empty() {
                    embedded.push(iface_name);
                }
            }
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::walk_interface_body(&child, parsed, methods, embedded, overapprox);
                }
            }
        }
    }

    /// Extract a method's parameter+return type signature for comparison.
    fn extract_method_signature(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> Result<String, GoDispatchGap> {
        Self::canon_sig(
            node.child_by_field_name("parameters").as_ref(),
            node.child_by_field_name("result").as_ref(),
            parsed,
        )
    }

    /// Canonical type string, recursive. Fails closed on unknown nodes (spec §6).
    fn canon_type(node: &tree_sitter::Node, parsed: &ParsedFile) -> Result<String, GoDispatchGap> {
        match node.kind() {
            "type_identifier" | "qualified_type" => {
                // bare name; pkg.T -> T
                let txt = parsed.node_text(node);
                let bare = txt.trim().rsplit('.').next().unwrap_or(txt.trim()).trim();
                Ok(bare.to_string())
            }
            "pointer_type" => {
                let inner = node.named_child(0).ok_or(GoDispatchGap::UnknownCanonType)?;
                Ok(format!("*{}", Self::canon_type(&inner, parsed)?))
            }
            "slice_type" => {
                let inner = node
                    .child_by_field_name("element")
                    .ok_or(GoDispatchGap::UnknownCanonType)?;
                Ok(format!("[]{}", Self::canon_type(&inner, parsed)?))
            }
            "array_type" => {
                let inner = node
                    .child_by_field_name("element")
                    .ok_or(GoDispatchGap::UnknownCanonType)?;
                // Array length is part of Go type identity ([3]int != [4]int) — preserve the
                // literal length text (round-4 codex MINOR). Non-literal/const length kept as text.
                let len = node
                    .child_by_field_name("length")
                    .map(|n| parsed.node_text(&n).trim().to_string())
                    .unwrap_or_default();
                Ok(format!("[{len}]{}", Self::canon_type(&inner, parsed)?))
            }
            "map_type" => {
                let k = node
                    .child_by_field_name("key")
                    .ok_or(GoDispatchGap::UnknownCanonType)?;
                let v = node
                    .child_by_field_name("value")
                    .ok_or(GoDispatchGap::UnknownCanonType)?;
                Ok(format!(
                    "map[{}]{}",
                    Self::canon_type(&k, parsed)?,
                    Self::canon_type(&v, parsed)?
                ))
            }
            "channel_type" => {
                // direction-preserving: `chan<-` / `<-chan` / `chan`
                // Use only this node's prefix before the value child; nested channel
                // element types can contain their own direction tokens.
                let inner = node
                    .child_by_field_name("value")
                    .ok_or(GoDispatchGap::UnknownCanonType)?;
                let prefix = parsed
                    .source
                    .get(node.start_byte()..inner.start_byte())
                    .ok_or(GoDispatchGap::UnknownCanonType)?
                    .replace(char::is_whitespace, "");
                let dir = if prefix.starts_with("<-chan") {
                    "<-chan"
                } else if prefix.starts_with("chan<-") {
                    "chan<-"
                } else if prefix.starts_with("chan") {
                    "chan"
                } else {
                    return Err(GoDispatchGap::UnknownCanonType);
                };
                // element type is field `value` in tree-sitter-go 0.23.4 (round-4 claude MAJOR)
                Ok(format!("{dir} {}", Self::canon_type(&inner, parsed)?))
            }
            "function_type" => {
                let params = node.child_by_field_name("parameters");
                let result = node.child_by_field_name("result");
                Ok(format!(
                    "func{}",
                    Self::canon_sig(params.as_ref(), result.as_ref(), parsed)?
                ))
            }
            "interface_type" => {
                // empty interface{} / any -> "any"; non-empty anonymous -> gap
                let txt = parsed.node_text(node).replace(char::is_whitespace, "");
                if txt == "interface{}" || txt == "any" {
                    Ok("any".to_string())
                } else {
                    Err(GoDispatchGap::AnonymousInterface)
                }
            }
            "generic_type" | "type_arguments" => Err(GoDispatchGap::Generic),
            "variadic_parameter_declaration" => {
                // handled in canon_sig; reaching here is a structural surprise
                Err(GoDispatchGap::UnknownCanonType)
            }
            // `(T)` ≡ `T` recursively, at any nesting depth (review MINOR — previously
            // fail-closed gapped a parenthesized type instead of unwrapping it).
            "parenthesized_type" => {
                let inner = node.named_child(0).ok_or(GoDispatchGap::UnknownCanonType)?;
                Self::canon_type(&inner, parsed)
            }
            _ => Err(GoDispatchGap::UnknownCanonType),
        }
    }

    /// Canonical `(params)(results)`; names dropped, grouped params expanded, variadic
    /// as `...T`. Either side gapping fails the whole sig (spec §6).
    fn canon_sig(
        params: Option<&tree_sitter::Node>,
        result: Option<&tree_sitter::Node>,
        parsed: &ParsedFile,
    ) -> Result<String, GoDispatchGap> {
        let ps = Self::canon_param_list(params, parsed)?;
        // result may be a single type node OR a parameter_list (multi/parenthesized).
        let rs = match result {
            None => Vec::new(),
            Some(r) if r.kind() == "parameter_list" => Self::canon_param_list(Some(r), parsed)?,
            Some(r) => vec![Self::canon_type(r, parsed)?],
        };
        Ok(format!("({})({})", ps.join(","), rs.join(",")))
    }

    /// Expand a parameter_list to canonical type strings: drop names, expand grouped
    /// `(a, b int)` -> [int,int], variadic `...T` -> `...T`.
    fn canon_param_list(
        list: Option<&tree_sitter::Node>,
        parsed: &ParsedFile,
    ) -> Result<Vec<String>, GoDispatchGap> {
        let mut out = Vec::new();
        let list = match list {
            Some(l) => l,
            None => return Ok(out),
        };
        let mut cursor = list.walk();
        for decl in list.named_children(&mut cursor) {
            match decl.kind() {
                "parameter_declaration" => {
                    let ty = decl
                        .child_by_field_name("type")
                        .ok_or(GoDispatchGap::UnknownCanonType)?;
                    let canon = Self::canon_type(&ty, parsed)?;
                    // grouped `(a, b int)`: count name children (>=1) -> repeat the type.
                    let names = decl
                        .children(&mut decl.walk())
                        .filter(|c| c.kind() == "identifier")
                        .count()
                        .max(1);
                    for _ in 0..names {
                        out.push(canon.clone());
                    }
                }
                "variadic_parameter_declaration" => {
                    let ty = decl
                        .child_by_field_name("type")
                        .ok_or(GoDispatchGap::UnknownCanonType)?;
                    out.push(format!("...{}", Self::canon_type(&ty, parsed)?));
                }
                _ => {}
            }
        }
        Ok(out)
    }

    #[cfg(test)]
    pub fn first_canon_sig_for_test(
        parsed: &ParsedFile,
        method: &str,
    ) -> Result<String, GoDispatchGap> {
        // Walk for a method_declaration or interface method_spec named `method`.
        fn find<'a>(
            n: tree_sitter::Node<'a>,
            parsed: &ParsedFile,
            method: &str,
        ) -> Option<tree_sitter::Node<'a>> {
            let is_match = matches!(
                n.kind(),
                "method_declaration" | "method_spec" | "method_elem"
            ) && n
                .child_by_field_name("name")
                .map(|x| parsed.node_text(&x).trim() == method)
                .unwrap_or(false);
            if is_match {
                return Some(n);
            }
            let mut c = n.walk();
            for ch in n.children(&mut c) {
                if let Some(f) = find(ch, parsed, method) {
                    return Some(f);
                }
            }
            None
        }
        let node = find(parsed.tree.root_node(), parsed, method).expect("method not found");
        GoTypeProvider::canon_sig(
            node.child_by_field_name("parameters").as_ref(),
            node.child_by_field_name("result").as_ref(),
            parsed,
        )
    }

    #[cfg(test)]
    pub fn satisfier_admission_keys_for_test(&self, iface: &str, method: &str) -> Vec<String> {
        let mut v: Vec<String> = self
            .data
            .sat_keys
            .get(&(iface.to_string(), method.to_string()))
            .map(|s| s.iter().map(|(k, _)| k.clone()).collect())
            .unwrap_or_default();
        v.sort();
        v.dedup();
        v
    }

    #[cfg(test)]
    pub fn dispatch_gaps_for_test(&self) -> Vec<GoDispatchGap> {
        self.data.dispatch_gaps.clone()
    }

    /// Extract a method_declaration (method with receiver).
    fn extract_method(
        data: &mut GoTypeData,
        node: &tree_sitter::Node,
        path: &str,
        parsed: &ParsedFile,
    ) {
        let name_node = match node.child_by_field_name("name") {
            Some(n) => n,
            None => return,
        };
        let name = parsed.node_text(&name_node).trim().to_string();

        let (receiver_type, is_pointer) = match Self::extract_receiver(node, parsed) {
            Some(r) => r,
            None => return,
        };

        let sig = Self::extract_func_signature(node, parsed);
        if sig.is_ok() && Self::signature_has_qualified_type(node) {
            data.dispatch_overapprox
                .push(GoDispatchOverApprox::CrossPackageBareName);
        }
        let generic = Self::signature_has_generic_syntax(node);
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let (params, variadic) = Self::count_method_params(node);

        data.methods
            .entry(receiver_type.clone())
            .or_default()
            .push(GoMethod {
                name,
                receiver_type,
                is_pointer_receiver: is_pointer,
                signature: sig,
                generic,
                file: path.to_string(),
                start_line,
                end_line,
                params,
                variadic,
            });
    }

    /// Count parameter names in a method_declaration's parameter list, excluding the
    /// receiver.  Multi-name declarations (`a, b int`) are counted per-name.  A
    /// `variadic_parameter_declaration` contributes 1 to `params` and sets the flag.
    fn count_method_params(method_node: &tree_sitter::Node) -> (usize, bool) {
        let Some(params_node) = method_node.child_by_field_name("parameters") else {
            return (0, false);
        };
        let mut count = 0usize;
        let mut is_variadic = false;
        let mut cursor = params_node.walk();
        for decl in params_node.named_children(&mut cursor) {
            match decl.kind() {
                "parameter_declaration" => {
                    // Count identifier children; unnamed params have 0 identifiers but
                    // still represent one parameter, so max(names, 1).
                    let names = decl
                        .children(&mut decl.walk())
                        .filter(|c| c.kind() == "identifier")
                        .count()
                        .max(1);
                    count += names;
                }
                "variadic_parameter_declaration" => {
                    count += 1;
                    is_variadic = true;
                }
                _ => {}
            }
        }
        (count, is_variadic)
    }

    /// Extract receiver type from a method_declaration.
    fn extract_receiver(node: &tree_sitter::Node, parsed: &ParsedFile) -> Option<(String, bool)> {
        let receiver = node.child_by_field_name("receiver")?;
        let mut cursor = receiver.walk();
        for child in receiver.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                if let Some(type_node) = child.child_by_field_name("type") {
                    let type_text = parsed.node_text(&type_node).trim().to_string();
                    let is_pointer = type_text.starts_with('*');
                    let base_type = strip_pointer(&type_text).to_string();
                    if !base_type.is_empty() {
                        return Some((base_type, is_pointer));
                    }
                }
            }
        }
        None
    }

    /// Extract function signature (parameters + return) for interface matching.
    fn extract_func_signature(
        node: &tree_sitter::Node,
        parsed: &ParsedFile,
    ) -> Result<String, GoDispatchGap> {
        Self::canon_sig(
            node.child_by_field_name("parameters").as_ref(),
            node.child_by_field_name("result").as_ref(),
            parsed,
        )
    }

    fn signature_has_qualified_type(node: &tree_sitter::Node) -> bool {
        node.child_by_field_name("parameters")
            .map(|params| Self::node_has_kind(&params, "qualified_type"))
            .unwrap_or(false)
            || node
                .child_by_field_name("result")
                .map(|result| Self::node_has_kind(&result, "qualified_type"))
                .unwrap_or(false)
    }

    fn node_has_kind(node: &tree_sitter::Node, kind: &str) -> bool {
        if node.kind() == kind {
            return true;
        }
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        children
            .iter()
            .any(|child| Self::node_has_kind(child, kind))
    }

    // -----------------------------------------------------------------------
    // Interface satisfaction (static, operates on GoTypeData)
    // -----------------------------------------------------------------------

    /// Compute which concrete types satisfy which interfaces.
    fn compute_satisfaction(data: &mut GoTypeData) {
        data.satisfaction.clear();
        data.sat_keys.clear();
        data.dispatch_gaps.clear();

        let interface_methods = Self::resolve_all_interface_methods(data);
        let concrete_methods = Self::resolve_all_concrete_methods(data);
        // P11 S4 (i): satisfaction-map MEMBERSHIP only — never feeds
        // `insert_sat_keys` (no FunctionId exists for these methods).
        data.embedded_interface_promotions =
            Self::embedded_interface_promotions(data, &concrete_methods);
        // P11 S4 (B2 fix): the package-scoped ROUTING computation, kept
        // separate from the bare-keyed membership map above.
        data.embedded_interface_routes = Self::embedded_interface_routes(data, &concrete_methods);

        for (iface_name, iface_methods) in &interface_methods {
            let Some(iface) = data.interfaces.get(iface_name) else {
                continue;
            };
            if iface.generic {
                data.dispatch_gaps.push(GoDispatchGap::Generic);
                data.satisfaction
                    .insert(iface_name.clone(), BTreeSet::new());
                continue;
            }

            let mut canonical_iface_methods = BTreeMap::new();
            let mut has_gap = false;
            for (method_name, sig) in iface_methods {
                match sig {
                    Ok(sig) => {
                        canonical_iface_methods.insert(method_name.clone(), sig.clone());
                    }
                    Err(gap) => {
                        data.dispatch_gaps.push(*gap);
                        has_gap = true;
                    }
                }
            }
            if has_gap {
                data.satisfaction
                    .insert(iface_name.clone(), BTreeSet::new());
                continue;
            }

            let mut satisfying = BTreeSet::new();
            for (type_name, method_set) in &concrete_methods {
                let embedded = data.embedded_interface_promotions.get(type_name);
                let value_matches = Self::method_set_satisfies_with_embedded(
                    &method_set.value,
                    embedded,
                    &canonical_iface_methods,
                );
                let pointer_matches = Self::method_set_satisfies_with_embedded(
                    &method_set.pointer,
                    embedded,
                    &canonical_iface_methods,
                );

                if value_matches || pointer_matches {
                    satisfying.insert(type_name.clone());
                }

                if value_matches {
                    Self::insert_sat_keys(
                        data,
                        iface_name,
                        &canonical_iface_methods,
                        &method_set.value,
                        type_name,
                        false,
                    );
                    Self::insert_sat_keys(
                        data,
                        iface_name,
                        &canonical_iface_methods,
                        &method_set.pointer,
                        type_name,
                        true,
                    );
                } else if pointer_matches {
                    Self::insert_sat_keys(
                        data,
                        iface_name,
                        &canonical_iface_methods,
                        &method_set.pointer,
                        type_name,
                        true,
                    );
                }
            }
            data.satisfaction.insert(iface_name.clone(), satisfying);
        }
    }

    /// Resolve all methods for each interface, flattening embedded interfaces.
    fn resolve_all_interface_methods(
        data: &GoTypeData,
    ) -> BTreeMap<String, BTreeMap<String, Result<String, GoDispatchGap>>> {
        let mut result = BTreeMap::new();
        for iface_name in data.interfaces.keys() {
            let methods =
                Self::collect_interface_methods_from(data, iface_name, &mut BTreeSet::new());
            result.insert(iface_name.clone(), methods);
        }
        result
    }

    fn collect_interface_methods_from(
        data: &GoTypeData,
        name: &str,
        visited: &mut BTreeSet<String>,
    ) -> BTreeMap<String, Result<String, GoDispatchGap>> {
        if !visited.insert(name.to_string()) {
            return BTreeMap::new();
        }

        let mut methods = BTreeMap::new();
        if let Some(iface) = data.interfaces.get(name) {
            methods.extend(iface.methods.clone());
            for embedded in &iface.embedded {
                match data.interfaces.get(embedded) {
                    Some(e) if !e.generic => {
                        methods.extend(Self::collect_interface_methods_from(
                            data, embedded, visited,
                        ));
                    }
                    _ => {
                        // Review BLOCKER (codex): an embedded interface term that is
                        // external/qualified (`io.Reader`), missing, or generic cannot
                        // contribute its (unknowable) method set. Fail closed — gap the
                        // whole interface rather than under-constrain satisfaction and
                        // over-admit a satisfier (a wrong Exact InterfaceDispatch edge).
                        methods.insert(embedded.clone(), Err(GoDispatchGap::UnknownCanonType));
                    }
                }
            }
        }
        methods
    }

    /// Resolve all methods for each concrete type, including promoted methods.
    fn resolve_all_concrete_methods(data: &GoTypeData) -> BTreeMap<String, ReceiverMethodSet> {
        let mut result = BTreeMap::new();

        for type_name in data.structs.keys() {
            result
                .entry(type_name.clone())
                .or_insert_with(ReceiverMethodSet::default);
        }

        for (type_name, type_methods) in &data.methods {
            let method_set = result.entry(type_name.clone()).or_default();
            for m in type_methods {
                if m.generic {
                    continue;
                }
                let Ok(signature) = &m.signature else {
                    continue;
                };
                Self::insert_concrete_method(
                    method_set,
                    &m.name,
                    signature,
                    Self::method_function_id(m),
                    m.is_pointer_receiver,
                );
            }
        }

        Self::collect_promoted_methods_from(data, &mut result);

        result
    }

    fn collect_promoted_methods_from(
        data: &GoTypeData,
        result: &mut BTreeMap<String, ReceiverMethodSet>,
    ) {
        let mut methods_by_id = BTreeMap::new();
        for methods in data.methods.values() {
            for method in methods {
                methods_by_id.insert(Self::method_function_id(method), method);
            }
        }

        let mut by_key: BTreeMap<(String, String), Vec<PromotedMethodCandidate>> = BTreeMap::new();
        for candidate in Self::promoted_struct_method_candidates_from_data(data) {
            let key = (
                candidate.promoted.struct_name.clone(),
                candidate.promoted.method.clone(),
            );
            by_key.entry(key).or_default().push(candidate);
        }

        for ((struct_name, method_name), candidates) in by_key {
            let direct_value = data
                .methods
                .get(&struct_name)
                .map(|methods| {
                    methods
                        .iter()
                        .any(|m| m.name == method_name && !m.is_pointer_receiver)
                })
                .unwrap_or(false);
            let direct_pointer = data
                .methods
                .get(&struct_name)
                .map(|methods| methods.iter().any(|m| m.name == method_name))
                .unwrap_or(false);

            if !direct_value {
                let value_candidates = candidates
                    .iter()
                    .filter(|candidate| candidate.value_method_set);
                if let Some(candidate) =
                    Self::unique_shallowest_promoted_candidate(value_candidates)
                {
                    if let Some(method) = Self::canonical_promoted_method(candidate, &methods_by_id)
                    {
                        let method_set = result.entry(struct_name.clone()).or_default();
                        method_set
                            .value
                            .entry(method_name.clone())
                            .or_insert(method);
                    }
                }
            }

            if !direct_pointer {
                if let Some(candidate) =
                    Self::unique_shallowest_promoted_candidate(candidates.iter())
                {
                    if let Some(method) = Self::canonical_promoted_method(candidate, &methods_by_id)
                    {
                        let method_set = result.entry(struct_name).or_default();
                        method_set.pointer.entry(method_name).or_insert(method);
                    }
                }
            }
        }
    }

    fn unique_shallowest_promoted_candidate<'a>(
        candidates: impl Iterator<Item = &'a PromotedMethodCandidate>,
    ) -> Option<&'a PromotedMethodCandidate> {
        let mut candidates: Vec<_> = candidates.collect();
        if candidates.is_empty() {
            return None;
        }
        candidates.sort_by_key(|candidate| candidate.promoted.depth);
        let min_depth = candidates[0].promoted.depth;
        let shallow_count = candidates
            .iter()
            .filter(|candidate| candidate.promoted.depth == min_depth)
            .count();
        (shallow_count == 1).then_some(candidates[0])
    }

    fn canonical_promoted_method(
        candidate: &PromotedMethodCandidate,
        methods_by_id: &BTreeMap<FunctionId, &GoMethod>,
    ) -> Option<CanonMethod> {
        let method = methods_by_id.get(&candidate.promoted.func_id)?;
        if method.generic {
            return None;
        }
        let Ok(signature) = &method.signature else {
            return None;
        };
        Some(CanonMethod {
            signature: signature.to_string(),
            func_id: candidate.promoted.func_id.clone(),
        })
    }

    /// Whether `concrete`'s method set satisfies `iface`'s canonical method
    /// set. A method missing from `concrete` may
    /// still be covered by `embedded` (P11 S4: a directly-embedded in-repo
    /// interface's signature-only promoted method) with a matching signature.
    /// `concrete` still wins when both are present (checked first) — matches
    /// Go's own-method-shadows-embedded rule and keeps `insert_sat_keys`
    /// (which reads `concrete` only) in lockstep with what actually decided
    /// satisfaction here.
    fn method_set_satisfies_with_embedded(
        concrete: &BTreeMap<String, CanonMethod>,
        embedded: Option<&BTreeMap<String, (String, String)>>,
        iface: &BTreeMap<String, String>,
    ) -> bool {
        iface.iter().all(|(method_name, iface_sig)| {
            if let Some(method) = concrete.get(method_name) {
                return method.signature.as_str() == iface_sig.as_str();
            }
            embedded
                .and_then(|e| e.get(method_name))
                .map(|(_, sig)| sig.as_str() == iface_sig.as_str())
                .unwrap_or(false)
        })
    }

    /// P11 S4: for every struct, the method set contributed by EXACTLY ONE
    /// directly (depth-1) embedded in-repo interface, excluding any method
    /// that:
    /// - is supplied by more than one embedded interface (ambiguous
    ///   promotion — Go itself would treat same-depth multi-interface
    ///   promotion as an invalid selector, so contribute nothing rather than
    ///   guess),
    /// - is shadowed by the struct's own depth-0 field or method name (Go's
    ///   selector-shadowing rule), or
    /// - is already present in `concrete_methods[struct]` (own OR
    ///   struct-embedding-promoted — that existing, FunctionId-backed path
    ///   wins over this signature-only one).
    ///
    /// An embedded interface with a gapped (gen fail-closed) method signature
    /// contributes none of its methods (conservative: no partial trust).
    fn embedded_interface_promotions(
        data: &GoTypeData,
        concrete_methods: &BTreeMap<String, ReceiverMethodSet>,
    ) -> BTreeMap<String, BTreeMap<String, (String, String)>> {
        let mut out: BTreeMap<String, BTreeMap<String, (String, String)>> = BTreeMap::new();
        for (struct_name, go_struct) in &data.structs {
            let mut candidates: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
            for embedded in &go_struct.embedded {
                let bare = embedded.name.as_str();
                let Some(iface) = data.interfaces.get(bare) else {
                    continue; // not a known in-repo interface (struct embed, or external) -- skip.
                };
                if iface.generic {
                    continue;
                }
                let methods =
                    Self::collect_interface_methods_from(data, bare, &mut BTreeSet::new());
                for (m, sig) in &methods {
                    let Ok(sig) = sig else { continue };
                    candidates
                        .entry(m.clone())
                        .or_default()
                        .push((bare.to_string(), sig.clone()));
                }
            }
            if candidates.is_empty() {
                continue;
            }
            let own_fields: BTreeSet<&str> =
                go_struct.fields.iter().map(|(n, _)| n.as_str()).collect();
            let own_methods: BTreeSet<&str> = data
                .methods
                .get(struct_name)
                .map(|ms| ms.iter().map(|m| m.name.as_str()).collect())
                .unwrap_or_default();
            let already = concrete_methods.get(struct_name);
            let mut per_struct = BTreeMap::new();
            for (method_name, ifaces) in candidates {
                if ifaces.len() != 1 {
                    continue;
                }
                if own_fields.contains(method_name.as_str())
                    || own_methods.contains(method_name.as_str())
                {
                    continue;
                }
                let already_concrete = already
                    .map(|rs| {
                        rs.value.contains_key(&method_name) || rs.pointer.contains_key(&method_name)
                    })
                    .unwrap_or(false);
                if already_concrete {
                    continue;
                }
                per_struct.insert(method_name, ifaces.into_iter().next().unwrap());
            }
            if !per_struct.is_empty() {
                out.insert(struct_name.clone(), per_struct);
            }
        }
        out
    }

    /// P11 S4 (B2 fix, codex impl-review BLOCKER): package-scoped ROUTING
    /// computation feeding `CallGraph.go_embedded_interface_methods` via
    /// `embedded_interface_method_routes` — distinct from
    /// `embedded_interface_promotions` above (bare-keyed, feeds ONLY the
    /// interface-satisfaction MEMBERSHIP check, an existing accepted
    /// approximation this fix does not touch). The routing map mints a
    /// concrete Exact `InterfaceDispatch` edge at resolution time, so a
    /// bare-name collision here is a real false-edge risk, not just an
    /// approximation: a `Holder` in package A must never donate its embedded
    /// interface's methods to an unrelated same-named `Holder` in package B.
    ///
    /// Iterates `data.struct_embeds` (package-scoped: `GoOwnerIdentity ->
    /// embedded fields`) instead of the bare `data.structs`/`data.methods`,
    /// so each package's struct is considered independently. An embedded
    /// interface reference is trusted only when its bare name is BOTH
    /// package-unique repo-wide and owned by the embedding struct's OWN
    /// package (`interface_name_owners`) — Go requires a bare (unqualified)
    /// embedded name to resolve within the SAME package, and a same-named
    /// interface existing in some OTHER package as well means
    /// `data.interfaces.get(bare)` (itself bare-collapsed) cannot be trusted
    /// to be the RIGHT one; either case fails closed (skip this candidate).
    fn embedded_interface_routes(
        data: &GoTypeData,
        concrete_methods: &BTreeMap<String, ReceiverMethodSet>,
    ) -> BTreeMap<crate::resolution::GoOwnerIdentity, BTreeMap<String, String>> {
        let mut out: BTreeMap<crate::resolution::GoOwnerIdentity, BTreeMap<String, String>> =
            BTreeMap::new();
        for (owner, embeds) in &data.struct_embeds {
            let mut candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for embedded in embeds {
                let bare = embedded.name.as_str();
                let package_unique_here = data
                    .interface_name_owners
                    .get(bare)
                    .map(|dirs| dirs.len() == 1 && dirs.contains(&owner.package_dir))
                    .unwrap_or(false);
                if !package_unique_here {
                    continue; // fail closed: not package-unique, or not this package's.
                }
                let Some(iface) = data.interfaces.get(bare) else {
                    continue;
                };
                if iface.generic {
                    continue;
                }
                let methods =
                    Self::collect_interface_methods_from(data, bare, &mut BTreeSet::new());
                for (m, sig) in &methods {
                    if sig.is_err() {
                        continue;
                    }
                    candidates
                        .entry(m.clone())
                        .or_default()
                        .push(bare.to_string());
                }
            }
            if candidates.is_empty() {
                continue;
            }
            // Package-scoped shadow checks (own field / own method / already
            // FunctionId-backed concrete method) -- filter the existing
            // bare-collapsed indices down to entries whose OWN file lives in
            // this struct's package, rather than trusting them wholesale.
            let own_fields: BTreeSet<&str> = data
                .field_types
                .keys()
                .filter(|(o, _)| o == owner)
                .map(|(_, f)| f.as_str())
                .collect();
            let own_methods: BTreeSet<&str> = data
                .methods
                .get(&owner.name)
                .map(|ms| {
                    ms.iter()
                        .filter(|m| crate::resolution::dir_of(&m.file) == owner.package_dir)
                        .map(|m| m.name.as_str())
                        .collect()
                })
                .unwrap_or_default();
            let already = concrete_methods.get(&owner.name);
            let mut per_struct = BTreeMap::new();
            for (method_name, ifaces) in candidates {
                if ifaces.len() != 1 {
                    continue;
                }
                if own_fields.contains(method_name.as_str())
                    || own_methods.contains(method_name.as_str())
                {
                    continue;
                }
                let already_concrete = already
                    .map(|rs| {
                        Self::method_in_package(&rs.value, &method_name, &owner.package_dir)
                            || Self::method_in_package(
                                &rs.pointer,
                                &method_name,
                                &owner.package_dir,
                            )
                    })
                    .unwrap_or(false);
                if already_concrete {
                    continue;
                }
                per_struct.insert(method_name, ifaces.into_iter().next().unwrap());
            }
            if !per_struct.is_empty() {
                out.insert(owner.clone(), per_struct);
            }
        }
        out
    }

    fn method_in_package(
        methods: &BTreeMap<String, CanonMethod>,
        name: &str,
        package_dir: &str,
    ) -> bool {
        methods
            .get(name)
            .map(|m| crate::resolution::dir_of(&m.func_id.file) == package_dir)
            .unwrap_or(false)
    }

    fn insert_sat_keys(
        data: &mut GoTypeData,
        iface_name: &str,
        iface_methods: &BTreeMap<String, String>,
        concrete_methods: &BTreeMap<String, CanonMethod>,
        type_name: &str,
        is_pointer: bool,
    ) {
        let key = crate::resolution::admission_key(type_name, is_pointer);
        for method_name in iface_methods.keys() {
            if let Some(method) = concrete_methods.get(method_name) {
                data.sat_keys
                    .entry((iface_name.to_string(), method_name.clone()))
                    .or_default()
                    .push((key.clone(), method.func_id.clone()));
            }
        }
    }

    fn insert_concrete_method(
        method_set: &mut ReceiverMethodSet,
        method_name: &str,
        signature: &str,
        func_id: FunctionId,
        is_pointer_receiver: bool,
    ) {
        let method = CanonMethod {
            signature: signature.to_string(),
            func_id,
        };
        if is_pointer_receiver {
            method_set
                .pointer
                .entry(method_name.to_string())
                .or_insert(method);
        } else {
            method_set
                .value
                .entry(method_name.to_string())
                .or_insert(method.clone());
            method_set
                .pointer
                .entry(method_name.to_string())
                .or_insert(method);
        }
    }

    fn method_function_id(method: &GoMethod) -> FunctionId {
        FunctionId {
            name: method.name.clone(),
            file: method.file.clone(),
            start_line: method.start_line,
            end_line: method.end_line,
        }
    }

    /// All concrete methods promoted onto each struct via transitive **struct**
    /// embedding, with depth. One walk per struct collects (a) method candidates at
    /// depth>=1 and (b) the shallowest depth of every field name (incl. the outer
    /// struct's own fields at depth 0). A method at depth `d` is emitted only if NO
    /// same-name field exists at depth `<= d` — the Go selector rule, which lets an
    /// intermediate embedded struct's field shadow a deeper promoted method. Embedded
    /// **interface** fields are skipped (no concrete body — interface dispatch,
    /// deferred). Duplicates from different embed paths are kept (the caller resolves
    /// direct-method-wins + equal-depth ambiguity). **Path-local** cycle detection so
    /// diamond paths are preserved.
    pub fn promoted_struct_methods(&self) -> Vec<PromotedMethod> {
        Self::promoted_struct_method_candidates_from_data(self.data.as_ref())
            .into_iter()
            .map(|candidate| candidate.promoted)
            .collect()
    }

    fn promoted_struct_method_candidates_from_data(
        data: &GoTypeData,
    ) -> Vec<PromotedMethodCandidate> {
        let mut out = Vec::new();
        for struct_name in data.structs.keys() {
            let mut field_depth: BTreeMap<String, usize> = BTreeMap::new();
            let mut cands: Vec<PromotedMethodCandidate> = Vec::new();
            let mut path: BTreeSet<String> = BTreeSet::new();
            path.insert(struct_name.clone());
            Self::walk_embedding(
                data,
                struct_name,
                struct_name,
                0,
                &mut path,
                &mut field_depth,
                &mut cands,
                false,
            );
            for candidate in cands {
                // Shadowed if a same-name field sits at a shallower-or-equal depth.
                match field_depth.get(&candidate.promoted.method) {
                    Some(fd) if *fd <= candidate.promoted.depth => {}
                    _ => out.push(candidate),
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_embedding(
        data: &GoTypeData,
        outer: &str,
        current: &str,
        depth: usize, // depth of `current` within `outer` (0 = the outer struct itself)
        path: &mut BTreeSet<String>,
        field_depth: &mut BTreeMap<String, usize>,
        cands: &mut Vec<PromotedMethodCandidate>,
        value_can_use_pointer_receiver: bool,
    ) {
        let go_struct = match data.structs.get(current) {
            Some(s) => s,
            None => return,
        };
        // Record this struct's own field names at `depth` (keep the min).
        for (fname, _) in &go_struct.fields {
            field_depth
                .entry(fname.clone())
                .and_modify(|d| {
                    if depth < *d {
                        *d = depth;
                    }
                })
                .or_insert(depth);
        }
        // Methods of `current` promote to `outer` (depth>=1; the outer struct's own
        // depth-0 methods are NOT promoted — direct-method-wins is the caller's job).
        if depth >= 1 {
            if let Some(methods) = data.methods.get(current) {
                for m in methods {
                    cands.push(PromotedMethodCandidate {
                        promoted: PromotedMethod {
                            struct_name: outer.to_string(),
                            method: m.name.clone(),
                            func_id: FunctionId {
                                name: m.name.clone(),
                                file: m.file.clone(),
                                start_line: m.start_line,
                                end_line: m.end_line,
                            },
                            depth,
                        },
                        value_method_set: !m.is_pointer_receiver || value_can_use_pointer_receiver,
                    });
                }
            }
        }
        for embedded in &go_struct.embedded {
            let bare = embedded.name.as_str();
            if data.interfaces.contains_key(bare) {
                continue; // embedded interface -> interface dispatch, deferred
            }
            if !path.insert(bare.to_string()) {
                continue; // cycle along THIS path only
            }
            Self::walk_embedding(
                data,
                outer,
                bare,
                depth + 1,
                path,
                field_depth,
                cands,
                value_can_use_pointer_receiver || embedded.is_pointer,
            );
            path.remove(bare); // restore for sibling paths (path-local)
        }
    }
}

// ---------------------------------------------------------------------------
// TypeProvider implementation
// ---------------------------------------------------------------------------

impl TypeProvider for GoTypeProvider {
    fn resolve_type(&self, _file: &str, expr: &str, _line: usize) -> Option<ResolvedType> {
        // Check if expr is a known type name.
        if let Some(_s) = self.data.structs.get(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Concrete,
                type_params: Vec::new(),
            });
        }
        if self.data.interfaces.contains_key(expr) {
            return Some(ResolvedType {
                name: expr.to_string(),
                kind: ResolvedTypeKind::Interface,
                type_params: Vec::new(),
            });
        }
        if let Some(target) = self.data.aliases.get(expr) {
            return Some(ResolvedType {
                name: target.clone(),
                kind: ResolvedTypeKind::Alias,
                type_params: Vec::new(),
            });
        }
        None
    }

    fn field_layout(&self, type_name: &str) -> Option<Vec<TypeFieldInfo>> {
        let go_struct = self.data.structs.get(type_name)?;
        Some(
            go_struct
                .fields
                .iter()
                .map(|(name, type_str)| TypeFieldInfo {
                    name: name.clone(),
                    type_str: type_str.clone(),
                })
                .collect(),
        )
    }

    fn subtypes_of(&self, type_name: &str) -> Vec<String> {
        // In Go, "subtypes" of an interface means concrete types that satisfy it.
        self.data
            .satisfaction
            .get(type_name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn resolve_alias(&self, type_name: &str) -> String {
        self.data
            .aliases
            .get(type_name)
            .cloned()
            .unwrap_or_else(|| type_name.to_string())
    }

    fn languages(&self) -> Vec<Language> {
        vec![Language::Go]
    }
}

// ---------------------------------------------------------------------------
// DispatchProvider implementation
// ---------------------------------------------------------------------------

impl DispatchProvider for GoTypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId> {
        // If receiver_type is a concrete type, resolve directly.
        if let Some(type_methods) = self.data.methods.get(receiver_type) {
            for m in type_methods {
                if m.name == method {
                    return vec![FunctionId {
                        name: method.to_string(),
                        file: m.file.clone(),
                        start_line: m.start_line,
                        end_line: m.end_line,
                    }];
                }
            }
        }

        // If receiver_type is an interface, find all satisfying types that have this method.
        if let Some(satisfying) = self.data.satisfaction.get(receiver_type) {
            let candidates: BTreeSet<&String> = if live_types.is_empty() {
                satisfying.iter().collect()
            } else {
                satisfying.intersection(live_types).collect()
            };

            // If RTA filtering eliminates all targets, fall back to full set.
            let fallback_to_full_set = candidates.is_empty() && !live_types.is_empty();
            let targets = if fallback_to_full_set {
                satisfying.iter().collect::<Vec<_>>()
            } else {
                candidates.into_iter().collect::<Vec<_>>()
            };

            // NOTE: interface dispatch for Phase-IP flows through the CallGraph-internal
            // `compute_interface_dispatch` + the resolution seam, NOT this DispatchProvider
            // path. `resolve_dispatch` keeps its original bare-name contract: `live_types`
            // here is a bare-name set, so it must NOT be filtered against admission-keyed
            // `sat_keys` (`*T` would never match a bare `T`). Stay on `data.satisfaction`.
            let mut results = Vec::new();
            let target_types: BTreeSet<String> = targets.into_iter().cloned().collect();
            for type_name in target_types {
                results.extend(self.resolve_dispatch(&type_name, method, &BTreeSet::new()));
            }
            return results;
        }

        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip leading `*` from a pointer type: `*Server` → `Server`.
fn strip_pointer(s: &str) -> &str {
    s.strip_prefix('*').unwrap_or(s).trim()
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use std::collections::{BTreeMap, BTreeSet};

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert(
            "t.go".to_string(),
            ParsedFile::parse("t.go", src, Language::Go).unwrap(),
        );
        GoTypeProvider::from_parsed_files(&files)
    }
    const SRC: &str = "package p\n\
        type I interface { Do() }\n\
        type Fast struct{}\nfunc (f Fast) Do() {}\n\
        type Slow struct{}\nfunc (s Slow) Do() {}\n";

    #[test]
    fn rta_prunes_to_live() {
        let p = provider(SRC);
        let live: BTreeSet<String> = ["Fast".to_string()].into_iter().collect();
        let t = p.compute_interface_dispatch(&live);
        let ids = t.impls.get(&("I".to_string(), "Do".to_string())).unwrap();
        assert_eq!(ids.len(), 1, "only live Fast");
        assert_eq!(ids[0].name, "Do");
        assert_eq!(ids[0].file, "t.go");
        assert!(!t.fallback_fired[&("I".to_string(), "Do".to_string())]);
    }

    #[test]
    fn empty_live_fires_fallback_full_set() {
        let p = provider(SRC);
        let live: BTreeSet<String> = BTreeSet::new();
        let t = p.compute_interface_dispatch(&live);
        let ids = t.impls.get(&("I".to_string(), "Do".to_string())).unwrap();
        assert_eq!(ids.len(), 2, "fallback -> all satisfiers");
        assert!(t.fallback_fired[&("I".to_string(), "Do".to_string())]);
        assert!(t
            .overapprox
            .contains(&GoDispatchOverApprox::NonLocalConstructionFallback));
    }
}

#[cfg(test)]
mod satisfaction_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use std::collections::{BTreeMap, BTreeSet};

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert(
            "t.go".to_string(),
            ParsedFile::parse("t.go", src, Language::Go).unwrap(),
        );
        GoTypeProvider::from_parsed_files(&files)
    }

    #[test]
    fn signature_mismatch_does_not_satisfy() {
        let p = provider(
            "package p\n\
             type I interface { Do() error }\n\
             type T struct{}\nfunc (t T) Do() string { return \"\" }\n",
        );
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
    }

    #[test]
    fn multi_method_partial_satisfier_not_admitted() {
        // codex#1: T has only Do(), interface needs Do()+Stop() -> NOT a satisfier.
        let p = provider(
            "package p\n\
             type I interface { Do(); Stop() }\n\
             type T struct{}\nfunc (t T) Do() {}\n",
        );
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
        assert!(p.satisfier_admission_keys_for_test("I", "Stop").is_empty());
    }

    #[test]
    fn value_receiver_admits_as_both_t_and_ptr_t() {
        // set_ptr ⊇ set_value: a value-receiver method satisfies via BOTH T and *T.
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type T struct{}\nfunc (t T) Do() {}\n",
        );
        assert_eq!(
            p.satisfier_admission_keys_for_test("I", "Do"),
            vec!["*T".to_string(), "T".to_string()]
        ); // sorted: '*' < 'T'
    }

    #[test]
    fn pointer_receiver_only_admits_as_pointer() {
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type T struct{}\nfunc (t *T) Do() {}\n",
        );
        assert_eq!(
            p.satisfier_admission_keys_for_test("I", "Do"),
            vec!["*T".to_string()]
        );
    }

    #[test]
    fn mixed_value_and_pointer_method_admits_as_pointer_only() {
        // I needs A()+B(); T has value A + pointer B -> only *T's set has both.
        let p = provider(
            "package p\n\
             type I interface { A(); B() }\n\
             type T struct{}\nfunc (t T) A() {}\nfunc (t *T) B() {}\n",
        );
        assert_eq!(
            p.satisfier_admission_keys_for_test("I", "A"),
            vec!["*T".to_string()]
        );
        assert_eq!(
            p.satisfier_admission_keys_for_test("I", "B"),
            vec!["*T".to_string()]
        );
    }

    #[test]
    fn embedded_method_satisfies() {
        // codex#11: Wrap embeds Base (Base has Do); Wrap satisfies I via the promoted Do.
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type Base struct{}\nfunc (b Base) Do() {}\n\
             type Wrap struct { Base }\n",
        );
        let sats = p.satisfier_admission_keys_for_test("I", "Do");
        // Wrap admits (value-embedded value method) as both; Base also satisfies.
        assert!(
            sats.contains(&"Wrap".to_string()) || sats.contains(&"*Wrap".to_string()),
            "promoted method must make Wrap satisfy I: {sats:?}"
        );
    }

    #[test]
    fn ambiguous_equal_depth_promoted_method_does_not_satisfy() {
        let p = provider(
            "package p\n\
             type I interface { M() }\n\
             type X struct{}\nfunc (x X) M() {}\n\
             type Y struct{}\nfunc (y Y) M() {}\n\
             type A struct { X; Y }\n",
        );
        let sats = p.satisfier_admission_keys_for_test("I", "M");
        assert!(!sats.contains(&"A".to_string()));
        assert!(!sats.contains(&"*A".to_string()));
    }

    #[test]
    fn type_set_interface_is_non_dispatchable() {
        let p = provider(
            "package p\n\
             type I interface { ~int }\n\
             type T struct{}\n",
        );
        assert!(p.dispatch_gaps_for_test().contains(&GoDispatchGap::Generic));
        assert!(p
            .data
            .satisfaction
            .get("I")
            .map(|s| s.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn generic_receiver_method_is_excluded_from_satisfaction() {
        let p = provider(
            "package p\n\
             type I interface { M(int) }\n\
             type R[T any] struct{}\nfunc (r R[T]) M(x int) {}\n",
        );
        assert!(p.satisfier_admission_keys_for_test("I", "M").is_empty());
    }

    #[test]
    fn generic_interface_is_non_dispatchable() {
        let p = provider(
            "package p\n\
             type I[X any] interface { Do(X) }\n\
             type T struct{}\nfunc (t T) Do(int) {}\n",
        );
        assert!(p.dispatch_gaps_for_test().contains(&GoDispatchGap::Generic));
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
    }

    #[test]
    fn anonymous_interface_method_is_gap() {
        // §13.3: a method whose param is a non-empty anonymous interface is non-dispatchable.
        let p = provider(
            "package p\n\
             type I interface { Do(x interface{ Run() }) }\n\
             type T struct{}\nfunc (t T) Do(x interface{ Run() }) {}\n",
        );
        assert!(p
            .dispatch_gaps_for_test()
            .contains(&GoDispatchGap::AnonymousInterface));
        assert!(p.satisfier_admission_keys_for_test("I", "Do").is_empty());
    }

    #[test]
    fn embedded_external_interface_fails_closed_not_partial() {
        // Review BLOCKER (codex): `interface { io.Reader; Close() }` embeds an UNRESOLVED
        // external interface. Its full method set is unknowable, so it must fail closed
        // (gap) — NOT be under-constrained to just {Close}, which would over-admit a
        // Close-only type and mint a WRONG Exact InterfaceDispatch edge.
        let p = provider(
            "package p\n\
             type I interface {\n\tio.Reader\n\tClose() error\n}\n\
             type T struct{}\nfunc (t T) Close() error { return nil }\n",
        );
        assert!(
            p.satisfier_admission_keys_for_test("I", "Close").is_empty(),
            "a Close-only type must not satisfy an interface embedding unresolved io.Reader"
        );
    }

    #[test]
    fn generic_instantiation_in_method_body_does_not_exclude_method() {
        // Review MAJOR (codex): the generic gate must inspect the method SIGNATURE only.
        // A non-generic method whose BODY contains a generic instantiation (`var _ Box[int]`)
        // must still count toward satisfaction (else a recall miss).
        let p = provider(
            "package p\n\
             type I interface { Do() }\n\
             type Box[T any] struct{}\n\
             type S struct{}\nfunc (s S) Do() { var _ Box[int] }\n",
        );
        let sats = p.satisfier_admission_keys_for_test("I", "Do");
        assert!(
            sats.contains(&"S".to_string()) || sats.contains(&"*S".to_string()),
            "a method with a generic instantiation in its body must still satisfy: {sats:?}"
        );
    }
}

#[cfg(test)]
mod embedding_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn provider(src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert(
            "main.go".to_string(),
            crate::ast::ParsedFile::parse("main.go", src, Language::Go).unwrap(),
        );
        GoTypeProvider::from_parsed_files(&files)
    }

    fn ms<'a>(v: &'a [PromotedMethod], s: &str, m: &str) -> Vec<&'a PromotedMethod> {
        v.iter()
            .filter(|p| p.struct_name == s && p.method == m)
            .collect()
    }

    #[test]
    fn promotes_concrete_method_from_embedded_struct() {
        let v = provider("package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n}\n").promoted_struct_methods();
        let p = ms(&v, "Wrap", "Ping");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].func_id.name, "Ping");
        assert_eq!(p[0].func_id.file, "main.go");
        assert_eq!(p[0].depth, 1);
    }

    #[test]
    fn promotes_transitively_with_depth() {
        let v = provider("package main\ntype C struct{}\nfunc (c C) M() {}\ntype B struct{ C }\ntype A struct{ B }\n").promoted_struct_methods();
        let p = ms(&v, "A", "M");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].depth, 2);
    }

    #[test]
    fn diamond_returns_both_equal_depth_paths() {
        // A embeds B,C; B,C embed D; D.M reachable via two depth-2 paths.
        // Path-local visited must NOT suppress the second path (so CallGraph sees the ambiguity).
        let v = provider("package main\ntype D struct{}\nfunc (d D) M() {}\ntype B struct{ D }\ntype C struct{ D }\ntype A struct{\n\tB\n\tC\n}\n").promoted_struct_methods();
        let p = ms(&v, "A", "M");
        assert_eq!(p.len(), 2, "both depth-2 paths to D.M returned");
        assert!(p.iter().all(|m| m.depth == 2));
    }

    #[test]
    fn embedded_interface_not_promoted() {
        let v = provider("package main\ntype R interface { Read() }\ntype S struct {\n\tR\n}\n")
            .promoted_struct_methods();
        assert!(ms(&v, "S", "Read").is_empty());
    }

    #[test]
    fn direct_field_shadows_promoted_method() {
        // Wrap has a field named Ping -> the embedded Base.Ping is shadowed, not promoted.
        let v = provider("package main\ntype Base struct{}\nfunc (b Base) Ping() {}\ntype Wrap struct {\n\tBase\n\tPing int\n}\n").promoted_struct_methods();
        assert!(
            ms(&v, "Wrap", "Ping").is_empty(),
            "a direct field named Ping shadows the promoted method"
        );
    }

    #[test]
    fn intermediate_embedded_field_shadows_deeper_method() {
        // A embeds B; B has field M AND embeds D; D has method M. Go selector rules:
        // B's field M (depth 1) shadows D.M (depth 2) -> NOT promoted to A.
        let v = provider("package main\ntype D struct{}\nfunc (d D) M() {}\ntype B struct {\n\tD\n\tM int\n}\ntype A struct{ B }\n").promoted_struct_methods();
        assert!(
            ms(&v, "A", "M").is_empty(),
            "intermediate field M shadows the deeper promoted method"
        );
    }

    #[test]
    fn embedded_field_name_shadows_promoted_method() {
        // A's embedded field F is still a selector field, so it shadows D.F.
        let v = provider("package main\ntype F struct{}\ntype D struct{}\nfunc (d D) F() {}\ntype A struct {\n\tF\n\tD\n}\n").promoted_struct_methods();
        assert!(
            ms(&v, "A", "F").is_empty(),
            "an embedded field named F shadows the promoted method D.F"
        );
    }
}

#[cfg(test)]
mod canon_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;

    // Parse a Go file and return the canon_sig of the first method named `m` on any
    // receiver, or the first interface method spec named `m`. Helper for byte-equality.
    fn sig_of(src: &str, method: &str) -> Result<String, GoDispatchGap> {
        let p = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        GoTypeProvider::first_canon_sig_for_test(&p, method)
    }

    #[test]
    fn param_names_dropped_iface_eq_concrete() {
        let iface = "package p\ntype I interface { Do(a int, b string) error }\n";
        let conc =
            "package p\ntype T struct{}\nfunc (t T) Do(x int, y string) error { return nil }\n";
        assert_eq!(sig_of(iface, "Do").unwrap(), sig_of(conc, "Do").unwrap());
    }

    #[test]
    fn grouped_params_expand() {
        let grouped = "package p\ntype T struct{}\nfunc (t T) F(a, b int) {}\n";
        let expanded = "package p\ntype I interface { F(int, int) }\n";
        assert_eq!(
            sig_of(grouped, "F").unwrap(),
            sig_of(expanded, "F").unwrap()
        );
    }

    #[test]
    fn channel_direction_distinguishes() {
        let send = "package p\ntype T struct{}\nfunc (t T) C(c chan<- int) {}\n";
        let bidi = "package p\ntype I interface { C(c chan int) }\n";
        assert_ne!(sig_of(send, "C").unwrap(), sig_of(bidi, "C").unwrap());
    }

    #[test]
    fn any_equals_empty_interface() {
        let a = "package p\ntype T struct{}\nfunc (t T) F(x any) {}\n";
        let b = "package p\ntype I interface { F(x interface{}) }\n";
        assert_eq!(sig_of(a, "F").unwrap(), sig_of(b, "F").unwrap());
    }

    #[test]
    fn variadic_canonicalizes() {
        let a = "package p\ntype T struct{}\nfunc (t T) F(xs ...int) {}\n";
        let b = "package p\ntype I interface { F(...int) }\n";
        assert_eq!(sig_of(a, "F").unwrap(), sig_of(b, "F").unwrap());
    }

    #[test]
    fn return_mismatch_differs() {
        let a = "package p\ntype T struct{}\nfunc (t T) F() error { return nil }\n";
        let b = "package p\ntype I interface { F() string }\n";
        assert_ne!(sig_of(a, "F").unwrap(), sig_of(b, "F").unwrap());
    }

    #[test]
    fn parenthesized_type_unwraps() {
        // `chan (int)` ≡ `chan int` — a parenthesized element type must unwrap, not gap
        // (review MINOR). tree-sitter-go parses `chan (int)` value as a parenthesized_type.
        let paren = "package p\ntype T struct{}\nfunc (t T) C(c chan (int)) {}\n";
        let plain = "package p\ntype I interface { C(c chan int) }\n";
        assert_eq!(sig_of(paren, "C").unwrap(), sig_of(plain, "C").unwrap());
    }
}

// ---------------------------------------------------------------------------
// P5 S1: func-typed-field index (package-scoped owner identity)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod func_typed_field_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use crate::resolution::GoOwnerIdentity;
    use std::collections::BTreeMap;

    fn provider_at(path: &str, src: &str) -> GoTypeProvider {
        let mut files = BTreeMap::new();
        files.insert(
            path.to_string(),
            ParsedFile::parse(path, src, Language::Go).unwrap(),
        );
        GoTypeProvider::from_parsed_files(&files)
    }

    fn owner(dir: &str, name: &str) -> GoOwnerIdentity {
        GoOwnerIdentity {
            package_dir: dir.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn func_typed_field_detected() {
        let p = provider_at(
            "main.go",
            "package main\ntype Command struct {\n\tRun func()\n}\n",
        );
        let fields = p.go_func_typed_fields();
        assert!(fields.contains(&(owner("", "Command"), "Run".to_string())));
    }

    #[test]
    fn non_func_field_not_recorded() {
        let p = provider_at(
            "main.go",
            "package main\ntype Command struct {\n\tName string\n}\n",
        );
        let fields = p.go_func_typed_fields();
        assert!(!fields.iter().any(|(_, f)| f == "Name"));
    }

    #[test]
    fn multi_name_declaration_types_each_field() {
        // `a, b func()` — extract_struct_fields already pushes the type for
        // each name; S1 must record BOTH as func-typed.
        let p = provider_at(
            "main.go",
            "package main\ntype Hooks struct {\n\tBefore, After func()\n}\n",
        );
        let fields = p.go_func_typed_fields();
        assert!(fields.contains(&(owner("", "Hooks"), "Before".to_string())));
        assert!(fields.contains(&(owner("", "Hooks"), "After".to_string())));
    }

    #[test]
    fn known_struct_identity_recorded_regardless_of_func_fields() {
        let p = provider_at(
            "main.go",
            "package main\ntype Config struct {\n\tName string\n}\n",
        );
        assert!(p
            .go_known_struct_identities()
            .contains(&owner("", "Config")));
    }

    #[test]
    fn package_scoped_identity_distinguishes_same_named_structs() {
        // Two `Command` structs in different packages (directories) must get
        // DISTINCT owner identities — a bare-name-only key would collide them
        // (spec-review MAJOR-1).
        let mut files = BTreeMap::new();
        files.insert(
            "pkga/a.go".to_string(),
            ParsedFile::parse(
                "pkga/a.go",
                "package pkga\ntype Command struct {\n\tRun func()\n}\n",
                Language::Go,
            )
            .unwrap(),
        );
        files.insert(
            "pkgb/b.go".to_string(),
            ParsedFile::parse(
                "pkgb/b.go",
                "package pkgb\ntype Command struct {\n\tName string\n}\n",
                Language::Go,
            )
            .unwrap(),
        );
        let p = GoTypeProvider::from_parsed_files(&files);
        let fields = p.go_func_typed_fields();
        assert!(fields.contains(&(owner("pkga", "Command"), "Run".to_string())));
        // pkgb's Command has no func-typed field at all, and critically is a
        // DIFFERENT owner identity from pkga's Command despite the same bare name.
        assert!(!fields.contains(&(owner("pkgb", "Command"), "Run".to_string())));
        let known = p.go_known_struct_identities();
        assert!(known.contains(&owner("pkga", "Command")));
        assert!(known.contains(&owner("pkgb", "Command")));
    }
}
