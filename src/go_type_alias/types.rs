use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AliasUnresolvedReason {
    DefinedVariant,
    ProfileUncertain,
    Cycle,
    Arity,
    Unresolvable,
}

impl AliasUnresolvedReason {
    pub(crate) fn telemetry_key(self) -> &'static str {
        match self {
            Self::DefinedVariant => "defined_variant",
            Self::ProfileUncertain => "profile_uncertain",
            Self::Cycle => "cycle",
            Self::Arity => "arity",
            Self::Unresolvable => "unresolvable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanonTypeError {
    Generic,
    AnonymousInterface,
    QualifiedTypeIdentity,
    UnknownCanonType,
    Alias(AliasUnresolvedReason),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GoAliasTelemetry {
    pub expanded: usize,
    pub unresolved: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NameIdentity {
    Bare,
    Path { path: String, qualified: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum OwnerHint {
    Local { package_dir: String, clause: String },
    ImportPath(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NamedType {
    pub identity: NameIdentity,
    pub owner_hint: OwnerHint,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AliasExpr {
    Atom(String),
    Param(String),
    Named(NamedType),
    Pointer(Box<Self>),
    Slice(Box<Self>),
    Array(String, Box<Self>),
    Map(Box<Self>, Box<Self>),
    Channel(String, Box<Self>),
    Variadic(Box<Self>),
    Function(Vec<Self>, Vec<Self>),
    Generic(Box<Self>, Vec<Self>),
}

impl AliasExpr {
    pub(super) fn render(&self, normalize_path_marker: bool) -> String {
        match self {
            Self::Atom(value) | Self::Param(value) => value.clone(),
            Self::Named(named) => match &named.identity {
                NameIdentity::Bare => named.name.clone(),
                NameIdentity::Path { path, qualified } => {
                    let marker = if normalize_path_marker || *qualified {
                        '@'
                    } else {
                        '~'
                    };
                    format!("{marker}{path}::{}", named.name)
                }
            },
            Self::Pointer(inner) => format!("*{}", inner.render(normalize_path_marker)),
            Self::Slice(inner) => format!("[]{}", inner.render(normalize_path_marker)),
            Self::Array(length, inner) => {
                format!("[{length}]{}", inner.render(normalize_path_marker))
            }
            Self::Map(key, value) => format!(
                "map[{}]{}",
                key.render(normalize_path_marker),
                value.render(normalize_path_marker)
            ),
            Self::Channel(direction, inner) => {
                format!("{direction} {}", inner.render(normalize_path_marker))
            }
            Self::Variadic(inner) => format!("...{}", inner.render(normalize_path_marker)),
            Self::Function(params, results) => format!(
                "func({})({})",
                render_list(params, normalize_path_marker),
                render_list(results, normalize_path_marker)
            ),
            Self::Generic(base, args) => format!(
                "{}[{}]",
                base.render(normalize_path_marker),
                render_list(args, normalize_path_marker)
            ),
        }
    }

    pub(super) fn substitute(
        &self,
        bindings: &BTreeMap<String, AliasExpr>,
    ) -> Result<Self, AliasUnresolvedReason> {
        match self {
            Self::Param(name) => bindings
                .get(name)
                .cloned()
                .ok_or(AliasUnresolvedReason::Unresolvable),
            Self::Atom(_) | Self::Named(_) => Ok(self.clone()),
            Self::Pointer(inner) => Ok(Self::Pointer(Box::new(inner.substitute(bindings)?))),
            Self::Slice(inner) => Ok(Self::Slice(Box::new(inner.substitute(bindings)?))),
            Self::Array(length, inner) => Ok(Self::Array(
                length.clone(),
                Box::new(inner.substitute(bindings)?),
            )),
            Self::Map(key, value) => Ok(Self::Map(
                Box::new(key.substitute(bindings)?),
                Box::new(value.substitute(bindings)?),
            )),
            Self::Channel(direction, inner) => Ok(Self::Channel(
                direction.clone(),
                Box::new(inner.substitute(bindings)?),
            )),
            Self::Variadic(inner) => Ok(Self::Variadic(Box::new(inner.substitute(bindings)?))),
            Self::Function(params, results) => Ok(Self::Function(
                substitute_list(params, bindings)?,
                substitute_list(results, bindings)?,
            )),
            Self::Generic(base, args) => Ok(Self::Generic(
                Box::new(base.substitute(bindings)?),
                substitute_list(args, bindings)?,
            )),
        }
    }
}

fn render_list(values: &[AliasExpr], normalize: bool) -> String {
    values
        .iter()
        .map(|value| value.render(normalize))
        .collect::<Vec<_>>()
        .join(",")
}

fn substitute_list(
    values: &[AliasExpr],
    bindings: &BTreeMap<String, AliasExpr>,
) -> Result<Vec<AliasExpr>, AliasUnresolvedReason> {
    values
        .iter()
        .map(|value| value.substitute(bindings))
        .collect()
}

#[derive(Debug, Clone)]
pub(super) enum AliasDeclarationKind {
    Alias {
        params: Vec<String>,
        rhs: Result<AliasExpr, AliasUnresolvedReason>,
    },
    Defined,
}

#[derive(Debug, Clone)]
pub(super) struct AliasDeclaration {
    pub defining_file: String,
    pub kind: AliasDeclarationKind,
}
