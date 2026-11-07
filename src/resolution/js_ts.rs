//! JS/TS resolution: tsconfig paths, anonymous owners, alias, CJS, namespace and returned-value routes.

pub(crate) mod alias;
pub(crate) mod anonymous;
pub(crate) mod cjs;
pub(crate) mod namespace;
pub(crate) mod paths;
pub(crate) mod paths_first_pass;
pub mod paths_snapshot;
pub(crate) mod paths_syntax;
pub(crate) mod returns;
