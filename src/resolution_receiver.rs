use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, FunctionId, MethodKind};
use crate::name_resolution::binding_lookup::{
    lookup_visible_binding, BindingKind, InitExpr, LocalFact,
};
use crate::name_resolution::engine::{resolve, resolve_path};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{RustPolicy, NS_TYPE, NS_VALUE};
use crate::name_resolution::rust_populator::enclosing_scope;
use crate::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Candidate, CfgCtx, FileId, PolicyQueryCtx, RawPath, ResStatus,
    ResolveQuery, ScopeId, ScopeKind, SourceLoc, Target,
};
use crate::resolution::{owner_key, peel_type, ReceiverRecovery};
use crate::resolution_identity::{ReceiverOutcome, ReceiverTypeKey, TypeKey};
use std::collections::BTreeSet;
use tree_sitter::Node;

const MAX_RECEIVER_TYPE_DEPTH: usize = 4;

#[derive(Clone, Copy)]
pub struct ReceiverTypeCtx<'a> {
    pub parsed: &'a ParsedFile,
    pub caller: &'a FunctionId,
    pub fn_node: tree_sitter::Node<'a>,
    pub receiver_expr: Option<tree_sitter::Node<'a>>,
    pub qualifier: Option<&'a str>,
    pub call_start_byte: usize,
}

pub struct RustReceiverTyper<'a> {
    cg: &'a CallGraph,
    graph: &'a ScopeGraph,
}

#[derive(Default)]
struct TypeVisit {
    locals: BTreeSet<(FileId, usize)>,
    fns: BTreeSet<FunctionId>,
}

struct RecursionCtx<'a, 'v> {
    cg: &'a CallGraph,
    graph: &'a ScopeGraph,
    parsed: &'a ParsedFile,
    generic_params: BTreeSet<String>,
    file: FileId,
    at_byte: usize,
    module_scope: ScopeId,
    caller: &'a FunctionId,
    visit: &'v mut TypeVisit,
    depth: usize,
}

impl<'a, 'v> RecursionCtx<'a, 'v> {
    fn descend<T>(&mut self, f: impl FnOnce(&mut RecursionCtx<'a, '_>) -> T) -> T {
        let mut child = RecursionCtx {
            cg: self.cg,
            graph: self.graph,
            parsed: self.parsed,
            generic_params: self.generic_params.clone(),
            file: self.file,
            at_byte: self.at_byte,
            module_scope: self.module_scope,
            caller: self.caller,
            visit: self.visit,
            depth: self.depth + 1,
        };
        f(&mut child)
    }
}

impl<'a> RustReceiverTyper<'a> {
    pub fn new(cg: &'a CallGraph) -> Self {
        // only valid with a complete scope graph
        let graph = cg
            .scope_graph
            .as_ref()
            .expect("RustReceiverTyper requires a populated scope graph");
        Self { cg, graph }
    }

    pub fn type_of_receiver(&self, ctx: ReceiverTypeCtx<'_>) -> Option<ReceiverOutcome> {
        if !matches!(ctx.parsed.language, crate::languages::Language::Rust) {
            return None;
        }
        let file = self.graph.file_paths.get(&ctx.caller.file).copied()?;
        let module_scope = module_scope_for_byte(self.graph, file, ctx.fn_node.start_byte())?;
        let mut visit = TypeVisit::default();
        let mut recursion = RecursionCtx {
            cg: self.cg,
            graph: self.graph,
            parsed: ctx.parsed,
            generic_params: enclosing_generic_type_params(ctx.parsed, ctx.fn_node),
            file,
            at_byte: ctx.call_start_byte,
            module_scope,
            caller: ctx.caller,
            visit: &mut visit,
            depth: 0,
        };
        if let Some(node) = ctx.receiver_expr {
            type_of_node(&mut recursion, node)
        } else {
            type_of_expr(&mut recursion, ctx.qualifier?)
        }
    }
}

fn type_of_expr(ctx: &mut RecursionCtx<'_, '_>, expr: &str) -> Option<ReceiverOutcome> {
    if ctx.depth > MAX_RECEIVER_TYPE_DEPTH {
        return None;
    }
    let expr = expr.trim();
    if matches!(expr, "self" | "Self") {
        return self_receiver_type(ctx);
    }
    if let Some((base, field)) = split_field_expr(expr) {
        return ctx.descend(|ctx| field_type_from_base(ctx, base, field));
    }
    if let Some(function) = split_call_expr(expr) {
        return ctx.descend(|ctx| return_type_from_call(ctx, function));
    }
    if is_simple_ident(expr) {
        return ctx.descend(|ctx| local_receiver_type(ctx, expr));
    }
    None
}

fn type_of_node<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    node: tree_sitter::Node<'a>,
) -> Option<ReceiverOutcome> {
    if ctx.depth > MAX_RECEIVER_TYPE_DEPTH {
        return None;
    }
    match node.kind() {
        "call_expression" => {
            if let Some(call) = method_call_parts(ctx.parsed, node) {
                return ctx.descend(|ctx| method_chain_type(ctx, call));
            }
            let function = node
                .child_by_field_name("function")
                .or_else(|| node.child_by_field_name("name"))?;
            let function_text = ctx.parsed.node_text(&function).trim().to_string();
            ctx.descend(|ctx| return_type_from_call(ctx, &function_text))
        }
        "field_expression" => {
            let base = node.child_by_field_name("value")?;
            let field = node.child_by_field_name("field")?;
            let field_text = ctx.parsed.node_text(&field).trim().to_string();
            if !is_simple_ident(&field_text) {
                return None;
            }
            ctx.descend(|ctx| field_type_from_node_base(ctx, base, &field_text))
        }
        _ => {
            let expr = ctx.parsed.node_text(&node).trim().to_string();
            type_of_expr(ctx, &expr)
        }
    }
}

fn self_receiver_type(ctx: &RecursionCtx<'_, '_>) -> Option<ReceiverOutcome> {
    let owner_scope = owner_scope_for_method(ctx)?;
    outcome_for_type_key(
        ctx.graph,
        TypeKey::InRepo(owner_scope),
        ReceiverRecovery::TypedParam,
    )
}

fn local_receiver_type(ctx: &mut RecursionCtx<'_, '_>, name: &str) -> Option<ReceiverOutcome> {
    let binding = lookup_visible_binding(ctx.graph, ctx.file, ctx.at_byte, name)?;
    let def_byte = binding.vis_extents.first()?.lo.byte;
    if !ctx.visit.locals.insert((ctx.file, def_byte)) {
        return None;
    }
    let binding_module_scope =
        module_scope_for_byte(ctx.graph, ctx.file, def_byte).unwrap_or(ctx.module_scope);
    let fact = ctx.graph.local_facts.get(&(ctx.file, def_byte))?;
    let result = ctx.descend(|ctx| {
        ctx.at_byte = def_byte;
        ctx.module_scope = binding_module_scope;
        type_from_local_fact(ctx, fact)
    });
    ctx.visit.locals.remove(&(ctx.file, def_byte));
    result
}

fn type_from_local_fact<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    fact: &LocalFact,
) -> Option<ReceiverOutcome> {
    if matches!(fact.kind, BindingKind::Param | BindingKind::Let) {
        if let Some(annotation) = fact.annotation.as_deref() {
            let recovery = match fact.kind {
                BindingKind::Param => ReceiverRecovery::TypedParam,
                BindingKind::Let => ReceiverRecovery::TypedLet,
                BindingKind::Pattern => unreachable!(),
            };
            return type_from_annotation(ctx, annotation, recovery);
        }
    }
    // A destructuring/tuple/struct pattern binding's type is NOT its initializer's
    // type: `let Pair(x, _) = a.pair();` binds `x` to a FIELD of Pair, not Pair.
    // Only a plain `let x = <init>` binds the whole initializer to the name, so
    // init-based type recovery is sound only for `BindingKind::Let`.
    if !matches!(fact.kind, BindingKind::Let) {
        return None;
    }
    match fact.init.as_ref()? {
        InitExpr::Ctor(expr) => {
            let ty = ctor_type_syntax(expr)?;
            type_from_annotation(ctx, ty, ReceiverRecovery::ConstructorLocal)
        }
        InitExpr::Field(expr) => {
            let (base, field) = split_field_expr(expr)?;
            ctx.descend(|ctx| field_type_from_base(ctx, base, field))
        }
        InitExpr::Call(expr) => {
            if let Some(node) = call_init_node_at(ctx) {
                return ctx.descend(|ctx| type_of_node(ctx, node));
            }
            let function = split_call_expr(expr)?;
            ctx.descend(|ctx| return_type_from_call(ctx, function))
        }
        InitExpr::Other => None,
    }
}

fn type_from_annotation(
    ctx: &RecursionCtx<'_, '_>,
    annotation: &str,
    default_recovery: ReceiverRecovery,
) -> Option<ReceiverOutcome> {
    let type_syntax = peel_type(annotation);
    if type_syntax.is_empty() {
        return None;
    }
    let recovery = if std_wrapper_was_peeled(annotation) {
        ReceiverRecovery::StdWrapperPeel
    } else {
        default_recovery
    };
    let bare = owner_key(&type_syntax);
    match crate::resolution_identity::resolve_type_path_to_type_scope(
        ctx.graph,
        ctx.module_scope,
        &type_syntax,
    ) {
        Some(key) => Some(ReceiverOutcome {
            key: receiver_key(key),
            bare,
            recovery,
        }),
        None if is_enclosing_generic_type_param(ctx, &type_syntax) => None,
        None => Some(ReceiverOutcome {
            key: ReceiverTypeKey::Bare(bare.clone()),
            bare,
            recovery,
        }),
    }
}

fn is_enclosing_generic_type_param(ctx: &RecursionCtx<'_, '_>, type_syntax: &str) -> bool {
    is_simple_ident(type_syntax) && ctx.generic_params.contains(type_syntax)
}

fn field_type_from_base(
    ctx: &mut RecursionCtx<'_, '_>,
    base: &str,
    field: &str,
) -> Option<ReceiverOutcome> {
    let base_ty = type_of_expr(ctx, base)?;
    let ReceiverTypeKey::InRepo(owner_scope) = base_ty.key else {
        return None;
    };
    let key = certain_index_type(ctx.cg.field_types.get(&(owner_scope, field.to_string()))?)?;
    outcome_for_type_key(ctx.graph, key, ReceiverRecovery::FieldTyped)
}

fn field_type_from_node_base<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    base: tree_sitter::Node<'a>,
    field: &str,
) -> Option<ReceiverOutcome> {
    let base_ty = type_of_node(ctx, base)?;
    let ReceiverTypeKey::InRepo(owner_scope) = base_ty.key else {
        return None;
    };
    let key = certain_index_type(ctx.cg.field_types.get(&(owner_scope, field.to_string()))?)?;
    outcome_for_type_key(ctx.graph, key, ReceiverRecovery::FieldTyped)
}

fn return_type_from_call(
    ctx: &mut RecursionCtx<'_, '_>,
    function: &str,
) -> Option<ReceiverOutcome> {
    if ctx.depth > MAX_RECEIVER_TYPE_DEPTH {
        return None;
    }
    let fid = resolve_function_to_fid(ctx, function)?;
    if !ctx.visit.fns.insert(fid.clone()) {
        return None;
    }
    let key = ctx.cg.return_types.get(&fid).and_then(|entries| {
        certain_index_type(entries)
            .and_then(|key| outcome_for_type_key(ctx.graph, key, ReceiverRecovery::ReturnTyped))
    });
    ctx.visit.fns.remove(&fid);
    key
}

fn method_chain_type<'a>(
    ctx: &mut RecursionCtx<'a, '_>,
    call: MethodCallParts<'a>,
) -> Option<ReceiverOutcome> {
    let recv_ty = type_of_node(ctx, call.receiver)?;
    let recovery = recv_ty.recovery;
    let ReceiverTypeKey::InRepo(scope) = recv_ty.key else {
        return None;
    };
    let fid = dispatch_method_single_exact(
        ctx.cg,
        scope,
        call.method,
        recovery,
        Some(call.arg_count),
        false,
    )?;
    if !ctx.visit.fns.insert(fid.clone()) {
        return None;
    }
    let result = ctx.cg.return_types.get(&fid).and_then(|entries| {
        let key = certain_index_type(entries)?;
        if !matches!(&key, TypeKey::InRepo(_)) {
            return None;
        }
        outcome_for_type_key(ctx.graph, key, ReceiverRecovery::ReturnTyped)
    });
    ctx.visit.fns.remove(&fid);
    result
}

fn dispatch_method_single_exact(
    cg: &CallGraph,
    scope: ScopeId,
    method: &str,
    recovery: ReceiverRecovery,
    arg_count: Option<usize>,
    arg_spread: bool,
) -> Option<FunctionId> {
    if recovery == ReceiverRecovery::StdWrapperPeel {
        return None;
    }
    let cands = cg.methods_by_scope.get(&(scope, method.to_string()))?;
    let kept: Vec<&FunctionId> = cands
        .iter()
        .filter(|fid| {
            let Some(fact) = cg.method_facts.get(*fid) else {
                return false;
            };
            fact.has_self
                && !matches!(
                    arg_count,
                    Some(n) if !arg_spread && fact.arity_excl_self != n
                )
        })
        .collect();
    match kept.as_slice() {
        [fid]
            if matches!(
                cg.method_facts.get(*fid),
                Some(fact) if matches!(&fact.kind, MethodKind::Inherent)
            ) =>
        {
            Some((*fid).clone())
        }
        _ => None,
    }
}

fn resolve_function_to_fid(ctx: &RecursionCtx<'_, '_>, function: &str) -> Option<FunctionId> {
    let function = function.trim();
    if function.contains('.') {
        return None;
    }
    let from = enclosing_scope(ctx.graph, ctx.file, ctx.at_byte)?;
    let target = if function.contains("::") {
        let (anchor, path) = rust_path_anchor(function)?;
        let policy = RustPolicy::new(ctx.graph, ctx.graph.edition);
        let at = SourceLoc {
            file: ctx.file,
            byte: ctx.at_byte,
        };
        let res = resolve_path(
            ctx.graph, &path, NS_VALUE, &anchor, from, NS_TYPE, &at, &policy,
        );
        single_callable_target(res.status, res.candidates.as_slice())?
    } else {
        let q = ResolveQuery {
            name: function.to_string(),
            ns: NS_VALUE,
            from,
            at: SourceLoc {
                file: ctx.file,
                byte: ctx.at_byte,
            },
            cfg: CfgCtx::default(),
            ctx: PolicyQueryCtx::default(),
        };
        let policy = RustPolicy::new(ctx.graph, ctx.graph.edition);
        let res = resolve(ctx.graph, &q, &policy);
        single_callable_target(res.status, res.candidates.as_slice())?
    };
    target_to_function_id(ctx, &target)
}

fn target_to_function_id(ctx: &RecursionCtx<'_, '_>, target: &Target) -> Option<FunctionId> {
    if !matches!(target, Target::Item { callable: true, .. }) {
        return None;
    }
    let mut ids = Vec::new();
    for binding in &ctx.graph.bindings {
        if !matches!(&binding.target, BindTarget::Resolved(t) if t == target) {
            continue;
        }
        let Some(file) = graph_file_for_scope(ctx.graph, binding.scope) else {
            continue;
        };
        let owner = type_syntax_for_scope(ctx.graph, binding.scope);
        if let Some(functions) = ctx.cg.functions.get(&binding.name) {
            for fid in functions
                .iter()
                .filter(|fid| ctx.graph.file_paths.get(&fid.file).copied() == Some(file))
            {
                match owner.as_deref() {
                    Some(owner)
                        if ctx.cg.method_owners.get(fid).map(String::as_str) != Some(owner) =>
                    {
                        continue;
                    }
                    None if ctx.cg.method_owners.contains_key(fid) => continue,
                    _ => {}
                }
                if fid.file == ctx.caller.file || !ids.contains(fid) {
                    ids.push(fid.clone());
                }
            }
        }
    }
    match ids.as_slice() {
        [fid] => Some(fid.clone()),
        _ => None,
    }
}

fn owner_scope_for_method(ctx: &RecursionCtx<'_, '_>) -> Option<ScopeId> {
    let mut scopes = BTreeSet::new();
    for ((scope, name), fids) in &ctx.cg.methods_by_scope {
        if name == &ctx.caller.name && fids.iter().any(|fid| fid == ctx.caller) {
            scopes.insert(*scope);
        }
    }
    match scopes.iter().copied().collect::<Vec<_>>().as_slice() {
        [scope] => Some(*scope),
        _ => None,
    }
}

fn receiver_key(key: TypeKey) -> ReceiverTypeKey {
    match key {
        TypeKey::InRepo(scope) => ReceiverTypeKey::InRepo(scope),
        TypeKey::External(name) => ReceiverTypeKey::External(name),
    }
}

fn outcome_for_type_key(
    graph: &ScopeGraph,
    key: TypeKey,
    recovery: ReceiverRecovery,
) -> Option<ReceiverOutcome> {
    let bare = match &key {
        TypeKey::InRepo(scope) => owner_key(&type_syntax_for_scope(graph, *scope)?),
        TypeKey::External(name) => owner_key(name),
    };
    Some(ReceiverOutcome {
        key: receiver_key(key),
        bare,
        recovery,
    })
}

fn call_init_node_at<'a>(ctx: &RecursionCtx<'a, '_>) -> Option<tree_sitter::Node<'a>> {
    let mut node = ctx
        .parsed
        .tree
        .root_node()
        .descendant_for_byte_range(ctx.at_byte, ctx.at_byte.saturating_add(1))?;
    loop {
        if node.kind() == "let_declaration" {
            let value = node.child_by_field_name("value")?;
            return (value.kind() == "call_expression").then_some(value);
        }
        if node.kind() == "function_item" {
            return None;
        }
        node = node.parent()?;
    }
}

#[derive(Clone, Copy)]
struct MethodCallParts<'a> {
    receiver: Node<'a>,
    method: &'a str,
    arg_count: usize,
}

fn method_call_parts<'a>(parsed: &'a ParsedFile, call: Node<'a>) -> Option<MethodCallParts<'a>> {
    if call.kind() != "call_expression" {
        return None;
    }
    let function = call.child_by_field_name("function")?;
    if function.kind() != "field_expression" {
        return None;
    }
    let receiver = function.child_by_field_name("value")?;
    let field = function.child_by_field_name("field")?;
    if field.kind() != "field_identifier" {
        return None;
    }
    let method = parsed.node_text(&field).trim();
    if !is_simple_ident(method) {
        return None;
    }
    let arguments = call.child_by_field_name("arguments")?;
    Some(MethodCallParts {
        receiver,
        method,
        arg_count: rust_arg_count(arguments),
    })
}

fn rust_arg_count(arguments: Node<'_>) -> usize {
    let mut cursor = arguments.walk();
    arguments
        .children(&mut cursor)
        .filter(|child| child.is_named())
        .count()
}

fn certain_index_type(entries: &[(Option<String>, TypeKey)]) -> Option<TypeKey> {
    let mut key: Option<TypeKey> = None;
    for (_, candidate) in entries {
        match &key {
            Some(existing) if existing != candidate => return None,
            Some(_) => {}
            None => key = Some(candidate.clone()),
        }
    }
    key
}

fn module_scope_for_byte(graph: &ScopeGraph, file: FileId, byte: usize) -> Option<ScopeId> {
    let mut scope = enclosing_scope(graph, file, byte)?;
    loop {
        let record = graph.scope(scope)?;
        if matches!(record.kind, ScopeKind::Root | ScopeKind::Module) {
            return Some(scope);
        }
        scope = graph.parent_of(scope)?;
    }
}

fn enclosing_generic_type_params(
    parsed: &ParsedFile,
    fn_node: tree_sitter::Node<'_>,
) -> BTreeSet<String> {
    let mut params = BTreeSet::new();
    collect_direct_type_params(parsed, fn_node, &mut params);

    let mut parent = fn_node.parent();
    while let Some(node) = parent {
        if node.kind() == "impl_item" {
            collect_direct_type_params(parsed, node, &mut params);
            break;
        }
        parent = node.parent();
    }
    params
}

fn collect_direct_type_params(
    parsed: &ParsedFile,
    node: tree_sitter::Node<'_>,
    params: &mut BTreeSet<String>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "type_parameters" {
            continue;
        }
        let mut param_cursor = child.walk();
        for param in child.named_children(&mut param_cursor) {
            if param.kind() == "type_parameter" {
                if let Some(name) = type_parameter_name(param) {
                    params.insert(parsed.node_text(&name).to_string());
                }
            }
        }
    }
}

fn type_parameter_name(param: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut cursor = param.walk();
    let name = param
        .named_children(&mut cursor)
        .find(|child| matches!(child.kind(), "type_identifier" | "identifier"));
    name
}

fn type_syntax_for_scope(graph: &ScopeGraph, scope: ScopeId) -> Option<String> {
    graph
        .bindings
        .iter()
        .find_map(|binding| match &binding.target {
            BindTarget::Resolved(Target::Item {
                owns: Some(owner), ..
            }) if *owner == scope => Some(binding.name.clone()),
            _ => None,
        })
}

fn graph_file_for_scope(graph: &ScopeGraph, scope: ScopeId) -> Option<FileId> {
    graph
        .scope(scope)?
        .extents
        .first()
        .map(|extent| extent.file)
}

fn split_field_expr(expr: &str) -> Option<(&str, &str)> {
    let (base, field) = expr.rsplit_once('.')?;
    if base.trim().is_empty() || !is_simple_ident(field.trim()) {
        return None;
    }
    Some((base.trim(), field.trim()))
}

fn split_call_expr(expr: &str) -> Option<&str> {
    let function = expr
        .trim()
        .strip_suffix("(...)")
        .or_else(|| expr.trim().strip_suffix("()"))?;
    if function.contains('.') {
        return None;
    }
    Some(function.trim())
}

fn ctor_type_syntax(expr: &str) -> Option<&str> {
    let expr = expr.trim();
    if let Some(function) = expr.strip_suffix("()") {
        return function.rsplit_once("::").map(|(ty, _)| ty.trim());
    }
    expr.strip_suffix("{}").map(str::trim)
}

pub(crate) fn std_wrapper_was_peeled(annotation: &str) -> bool {
    let mut t = annotation.trim();
    loop {
        let before = t;
        t = t
            .trim_start_matches("&mut ")
            .trim_start_matches('&')
            .trim_start_matches("*const ")
            .trim_start_matches("*mut ")
            .trim_start_matches('*')
            .trim();
        if let Some(rest) = t.strip_prefix('\'') {
            let rest = rest.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
            t = rest.trim().trim_start_matches("mut ").trim();
        }
        if t == before {
            break;
        }
    }
    ["Box", "Arc", "Rc", "Pin"].iter().any(|wrapper| {
        t.strip_prefix(wrapper)
            .is_some_and(|rest| rest.trim_start().starts_with('<'))
    })
}

fn is_simple_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn single_callable_target(status: ResStatus, candidates: &[Candidate]) -> Option<Target> {
    match (status, candidates) {
        (ResStatus::Resolved, [Candidate { target, .. }])
            if matches!(target, Target::Item { callable: true, .. }) =>
        {
            Some(target.clone())
        }
        _ => None,
    }
}

fn rust_path_anchor(raw: &str) -> Option<(Anchor, RawPath)> {
    let mut segs: Vec<String> = raw.split("::").map(str::to_string).collect();
    if segs.is_empty() {
        return None;
    }
    let anchor = match segs.first().map(String::as_str) {
        Some("") => {
            segs.remove(0);
            Anchor {
                kind: AnchorKind::LeadingColon,
                prelude: None,
            }
        }
        Some("crate") => {
            segs.remove(0);
            Anchor::crate_root()
        }
        Some("self") => {
            segs.remove(0);
            Anchor::self_mod()
        }
        Some("super") => {
            let mut n = 0u32;
            while matches!(segs.first().map(String::as_str), Some("super")) {
                segs.remove(0);
                n += 1;
            }
            Anchor::super_n(n)
        }
        Some(_) => Anchor::bare(),
        None => return None,
    };
    Some((anchor, RawPath(segs)))
}

#[cfg(test)]
mod tests;
