use super::{BindingRow, DeclarationKind, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) static LUA_ROWS: &[BindingRow] = &[BindingRow {
    language: Language::Lua,
    kind: "parameters",
    variant: None,
    roles: RoleSet::of(&[Role::Binding]),
    fields: &["name"],
    declaration: Some(DeclarationKind::Parameter),
    visibility: Visibility::WholeScope,
    ruling: Ruling::Classified,
    regression: "e0a-x-lua-parameters",
}];
