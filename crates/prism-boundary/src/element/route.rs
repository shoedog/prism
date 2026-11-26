//! Projection of admitted route declarations. Source extraction belongs to the adapters.

mod matching;
mod path;

use super::{CanonicalPath, OperationId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dialect {
    Literal,
    Brace,
    Gin,
    Express4,
    Express5,
    Hapi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    Public,
    Local,
    Internal,
    Detached,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StarPolicy {
    Unchecked,
    Closed(Vec<String>),
    Open,
}

/// A policy slot required by this declaration but absent from its admitted evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Withheld {
    Method,
    Path,
    RouterOptions,
    EntryOptions,
    Mount,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteDeclaration {
    pub dialect: Dialect,
    pub version: Option<String>,
    pub methods: Vec<Option<String>>,
    pub paths: Vec<String>,
    /// Each outer entry is one admitted mount alternative. Inner entries are nested joins.
    pub prefixes: Vec<Vec<String>>,
    pub mount: Option<String>,
    pub uppercase: bool,
    pub ascii_fold: bool,
    pub strip_slash: bool,
    pub raw_path: bool,
    pub empty_tail: bool,
    pub bare_parent: bool,
    pub implicit_methods: bool,
    pub selector: Option<String>,
    pub api: String,
    pub authority: String,
    pub file: String,
    pub line: usize,
    pub policy_lines: Vec<usize>,
    pub star: StarPolicy,
    pub visibility: Visibility,
    pub withheld: Option<Withheld>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Limit {
    MethodUnchecked,
    StarPolicyUnchecked,
    TailWidthUnchecked,
    OptionalSeparatorUnchecked,
    RouterPolicyUnchecked,
    EntryOptionsUnchecked,
    MountUnchecked,
    PathUnchecked,
    ImplicitMethods,
    PublicMappingUnavailable,
    DispatchDiscriminator,
}

impl Limit {
    pub fn code(self) -> &'static str {
        match self {
            Self::MethodUnchecked => "method_unchecked",
            Self::StarPolicyUnchecked => "star_policy_unchecked",
            Self::TailWidthUnchecked => "tail_width_unchecked",
            Self::OptionalSeparatorUnchecked => "optional_separator_unchecked",
            Self::RouterPolicyUnchecked => "router_policy_unchecked",
            Self::EntryOptionsUnchecked => "entry_options_unchecked",
            Self::MountUnchecked => "mount_unchecked",
            Self::PathUnchecked => "path_unchecked",
            Self::ImplicitMethods => "implicit_methods",
            Self::PublicMappingUnavailable => "public_mapping_unavailable",
            Self::DispatchDiscriminator => "dispatch_discriminator",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    pub id: OperationId,
    pub path: CanonicalPath,
    pub public: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Projection {
    pub declaration: RouteDeclaration,
    pub routes: Vec<Route>,
    pub limits: BTreeSet<Limit>,
}

pub fn project_route(declaration: &RouteDeclaration) -> Projection {
    let mut result = Projection {
        declaration: declaration.clone(),
        routes: Vec::new(),
        limits: BTreeSet::new(),
    };
    if declaration.visibility == Visibility::Detached {
        return result;
    }
    if let Some(withheld) = declaration.withheld {
        let limit = match withheld {
            Withheld::Method => Limit::MethodUnchecked,
            Withheld::Path => Limit::PathUnchecked,
            Withheld::RouterOptions => Limit::RouterPolicyUnchecked,
            Withheld::EntryOptions => Limit::EntryOptionsUnchecked,
            Withheld::Mount => Limit::MountUnchecked,
        };
        result.limits.insert(limit);
        if matches!(
            withheld,
            Withheld::Method | Withheld::Path | Withheld::RouterOptions
        ) {
            return result;
        }
    }
    if declaration.implicit_methods {
        result.limits.insert(Limit::ImplicitMethods);
    }
    if declaration.visibility == Visibility::Local {
        result.limits.insert(Limit::PublicMappingUnavailable);
    }
    if declaration.selector.is_some() {
        result.limits.insert(Limit::DispatchDiscriminator);
    }
    if (declaration.ascii_fold || declaration.strip_slash) && declaration.version.is_none() {
        result.limits.insert(Limit::RouterPolicyUnchecked);
        return result;
    }
    let methods = methods(declaration, &mut result.limits);
    let mut observed = BTreeMap::new();
    let no_prefixes = [Vec::new()];
    let alternatives =
        if declaration.prefixes.is_empty() || declaration.withheld == Some(Withheld::Mount) {
            &no_prefixes[..]
        } else {
            &declaration.prefixes
        };
    for prefixes in alternatives {
        for original in &declaration.paths {
            for path in path::paths(declaration, prefixes, original) {
                let path = match path {
                    Ok(path) => path,
                    Err(limit) => {
                        result.limits.insert(limit);
                        continue;
                    }
                };
                let constrained = path.captures().any(|capture| capture.constraint.is_some());
                if constrained {
                    result.limits.insert(Limit::DispatchDiscriminator);
                }
                let public = declaration.withheld.is_none()
                    && declaration.visibility == Visibility::Public
                    && declaration.selector.is_none()
                    && !constrained;
                for method in &methods {
                    let id = OperationId::new(method, &path);
                    // Retain alternatives whose spelling agrees but whose capture names differ.
                    let route = Route {
                        id: id.clone(),
                        path: path.clone(),
                        public,
                    };
                    let alternatives: &mut Vec<_> = observed.entry(id).or_default();
                    if !alternatives.contains(&route) {
                        alternatives.push(route);
                    }
                }
            }
        }
    }
    result.routes = observed.into_values().flatten().collect();
    result
}

fn methods(declaration: &RouteDeclaration, limits: &mut BTreeSet<Limit>) -> BTreeSet<String> {
    let mut methods = BTreeSet::new();
    for raw in &declaration.methods {
        match raw.as_deref() {
            Some("*") => match &declaration.star {
                StarPolicy::Closed(known) if !known.is_empty() => {
                    for method in known {
                        insert_method(declaration, method, &mut methods, limits);
                    }
                }
                StarPolicy::Open => {
                    methods.insert("*".into());
                }
                _ => {
                    limits.insert(Limit::StarPolicyUnchecked);
                }
            },
            Some(value) => insert_method(declaration, value, &mut methods, limits),
            None => {
                limits.insert(Limit::MethodUnchecked);
            }
        }
    }
    methods
}

fn insert_method(
    declaration: &RouteDeclaration,
    method: &str,
    methods: &mut BTreeSet<String>,
    limits: &mut BTreeSet<Limit>,
) {
    // RFC token punctuation, including extension methods. Star on its own is handled by the registrar policy.
    if method.is_empty()
        || method == "*"
        || !method
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
    {
        limits.insert(Limit::MethodUnchecked);
    } else {
        methods.insert(if declaration.uppercase {
            method.to_ascii_uppercase()
        } else {
            method.into()
        });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Drift {
    Method,
    Path,
    MethodAndPath,
}

impl Drift {
    pub fn code(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::Path => "path",
            Self::MethodAndPath => "method_and_path",
        }
    }
}

/// Compare identities only after the caller has paired both declarations to the same handler.
pub fn annotation_drift(registration: &OperationId, annotation: &OperationId) -> Option<Drift> {
    match (
        registration.method() != annotation.method(),
        registration.path() != annotation.path(),
    ) {
        (true, true) => Some(Drift::MethodAndPath),
        (true, false) => Some(Drift::Method),
        (false, true) => Some(Drift::Path),
        (false, false) => None,
    }
}
