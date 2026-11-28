//! Replay the positional actual and its formal initializers in one activation.

use super::*;

pub(in super::super) struct Replay<'a> {
    pub file: &'a str,
    pub actual: Option<Node<'a>>,
    pub defaults: Vec<(Node<'a>, Node<'a>)>,
    pub selections: Vec<(String, Option<Expr<'a>>)>,
    pub value: Option<Expr<'a>>,
    pub timing: Option<&'static str>,
    pub(super) tracked_default: bool,
}

impl<'a> Replay<'a> {
    pub(super) fn new(file: &'a str, actual: Option<Node<'a>>) -> Self {
        Self {
            file,
            actual,
            defaults: Vec::new(),
            selections: Vec::new(),
            value: None,
            timing: None,
            tracked_default: false,
        }
    }
}

pub(super) fn baseline_expr<'a>(hop: &Hop<'a>) -> Option<Expr<'a>> {
    match hop.value? {
        Value::Node(file, node) => normalize(Some((file, node))),
        _ => None,
    }
}

impl<'a> Proof<'a, '_> {
    pub(super) fn replay(
        &self,
        hop: &Hop<'a>,
        args: &[Node<'a>],
        copy: Option<Node<'a>>,
        recipe: &mut Replay<'a>,
    ) -> Check<()> {
        let p = &hop.parameter;
        let (file, parsed) = self
            .context
            .files
            .get_key_value(&hop.slot.holder.file)
            .expect("indexed holder file");
        let file = file.as_str();
        if args.iter().any(|arg| arg.kind() == "spread_element") {
            return Err("unknown_spread");
        }
        let formals = formal_list(p.owner)?;
        let mut names = BTreeSet::new();
        let mut paths = Vec::new();
        for (position, (binding, default)) in formals.into_iter().enumerate() {
            let mut steps = Vec::new();
            let leaf = pattern(parsed, binding, 0, &mut steps)?;
            if !names.insert(name(parsed, leaf).ok_or("path_step")?)
                || (!steps.is_empty() && (position != p.position || p.body_read.is_some()))
                || (position == p.position && p.body_read.is_none() && leaf != p.binding)
            {
                return Err("path_step");
            }
            paths.push((leaf, default, steps));
        }
        let (carrier_binding, _, path) = paths.get_mut(p.position).ok_or("path_step")?;
        if let Some(read) = p.body_read {
            let decl = copy.ok_or("path_step")?;
            let statement = decl.parent().ok_or("path_step")?;
            if !children(statement).eq([decl])
                || statement.parent() != p.owner.child_by_field_name("body")
                || pattern(
                    parsed,
                    decl.child_by_field_name("name").ok_or("path_step")?,
                    0,
                    path,
                )? != p.binding
                || path.is_empty()
                || param_slot::parameter(parsed, read, &mut self.context.bindings[file].cache())
                    .is_none_or(|carrier| {
                        carrier.binding != *carrier_binding || carrier.owner != p.owner
                    })
            {
                return Err("path_step");
            }
        }

        let forwarded = hop.relay.is_some()
            && scalar(p)
            && normalize(recipe.actual.map(|node| (recipe.file, node))) == baseline_expr(hop);
        let mut failure = None;
        for (position, (binding, default, path)) in paths.into_iter().enumerate() {
            let tracked = position == p.position;
            let mut value = args.get(position).copied().map(|node| (recipe.file, node));
            let current = (|| {
                if !(forwarded && tracked) {
                    value = self.default_value(value, (file, binding, default), tracked, recipe)?;
                }
                for (key, default) in path {
                    let expr = normalize(value).ok_or("path_step")?;
                    let selected = self.select(expr, &key, default.is_some())?;
                    recipe.selections.push((key, selected));
                    value =
                        self.default_value(selected, (file, p.binding, default), true, recipe)?;
                }
                Ok(normalize(value))
            })();
            match current {
                Err(reason) => failure = Some(failure.map_or(reason, |prior| first(prior, reason))),
                Ok(value) if tracked => recipe.value = value,
                Ok(_) => {}
            }
        }
        failure.map_or(Ok(()), Err)
    }

    fn default_value(
        &self,
        value: Option<Expr<'a>>,
        (file, binding, default): (&'a str, Node<'a>, Option<Node<'a>>),
        tracked: bool,
        recipe: &mut Replay<'a>,
    ) -> Check<Option<Expr<'a>>> {
        let Some(default) = default else {
            return Ok(value);
        };
        let undefined =
            value.is_none() || normalize(value).is_some_and(|expr| self.undefined(expr));
        if undefined && tracked {
            recipe.tracked_default = true;
        }
        if !undefined
            && normalize(value).is_some_and(|expr| {
                matches!(
                    expr.1.kind(),
                    "object"
                        | "array"
                        | "arrow_function"
                        | "function_expression"
                        | "generator_function"
                ) || self.closed(expr, 0).is_ok()
            })
        {
            return Ok(value);
        }
        self.default_dependency(file, default)?;
        self.closed((file, default), 0)
            .map_err(|_| "default_form")?;
        if undefined {
            recipe.defaults.push((binding, default));
            Ok(Some((file, default)))
        } else if tracked {
            Err("undefined_lookup")
        } else {
            Ok(value)
        }
    }

    fn select(&self, expr: Expr<'a>, key: &str, default: bool) -> Check<Option<Expr<'a>>> {
        let (file, node) = expr;
        let selected = match node.kind() {
            "array" => {
                let index = key
                    .parse::<u32>()
                    .ok()
                    .filter(|index| *index != u32::MAX && index.to_string() == key)
                    .ok_or("path_step")?;
                let entry = param_slot::array_entries(node)
                    .get(index as usize)
                    .copied()
                    .flatten()
                    .ok_or("path_step")?;
                self.closed(expr, 0)?;
                Some(entry)
            }
            "object" => {
                let (entries, null_proto) = self.object(expr, 0)?;
                match entries.into_iter().rev().find(|(name, _)| name == key) {
                    Some((_, node)) => Some(node),
                    None if null_proto && default => None,
                    None => return Err("undefined_lookup"),
                }
            }
            "null" | "undefined" | "number" | "string" | "true" | "false" => {
                return Err("path_step");
            }
            _ => return Err("hidden_literal_key"),
        };
        Ok(selected.map(|node| (file, node)))
    }
}
