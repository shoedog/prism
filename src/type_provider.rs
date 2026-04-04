//! Type system traits, registry, and common types for multi-language type resolution.
//!
//! Defines the `TypeProvider` trait (core type info), `DispatchProvider` (method
//! dispatch resolution), and `StructuralTypingProvider` (structural typing for
//! TypeScript). The `TypeRegistry` collects per-language providers and routes
//! queries by `Language`.
//!
//! See `docs/E12-type-system.md` for the full design.

use crate::call_graph::FunctionId;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Common types
// ---------------------------------------------------------------------------

/// Resolved type information for a variable or expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    /// The canonical type name (e.g., "User", "io.Reader", "string").
    pub name: String,
    /// Classification of this type.
    pub kind: ResolvedTypeKind,
    /// Generic type parameters (e.g., `["string", "number"]` for `Map<string, number>`).
    pub type_params: Vec<String>,
}

/// Classification of a resolved type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTypeKind {
    /// Class, struct — can be instantiated.
    Concrete,
    /// Interface, protocol, trait — cannot be instantiated.
    Interface,
    /// Type alias, typedef.
    Alias,
    /// Go iota, Java enum, Rust enum, TS union.
    Enum,
    /// int, string, bool.
    Primitive,
    /// Couldn't determine.
    Unknown,
}

/// Result of a structural compatibility check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    /// Types are structurally compatible.
    Compatible,
    /// Types are not compatible, with explanation.
    Incompatible { reason: String },
    /// Can't determine (missing type info, complex expression).
    Unknown,
}

/// Field within a type definition (language-agnostic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeFieldInfo {
    /// Field name.
    pub name: String,
    /// Type as a string.
    pub type_str: String,
}

/// Target language version for version-aware analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageVersion {
    /// Language version string (e.g., "3.8", "1.21", "18", "ES2022").
    pub version: String,
    /// Parsed major version.
    pub major: u32,
    /// Parsed minor version.
    pub minor: u32,
}

impl LanguageVersion {
    /// Parse a version string like "3.8", "18", "1.21".
    pub fn parse(version: &str) -> Option<Self> {
        let parts: Vec<&str> = version.split('.').collect();
        let major = parts.first()?.parse::<u32>().ok()?;
        let minor = parts
            .get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        Some(LanguageVersion {
            version: version.to_string(),
            major,
            minor,
        })
    }
}

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

/// Core type information provider. Implemented per-language.
pub trait TypeProvider: Send + Sync {
    /// Resolve the type of a variable/expression at a source location.
    fn resolve_type(&self, file: &str, expr: &str, line: usize) -> Option<ResolvedType>;

    /// Get the field layout of a named type.
    fn field_layout(&self, type_name: &str) -> Option<Vec<TypeFieldInfo>>;

    /// Find all concrete types that satisfy a given type constraint.
    fn subtypes_of(&self, type_name: &str) -> Vec<String>;

    /// Resolve a type alias to its canonical name.
    fn resolve_alias(&self, type_name: &str) -> String;

    /// Which language(s) this provider covers.
    fn languages(&self) -> Vec<Language>;
}

// ---------------------------------------------------------------------------
// Dispatch capability
// ---------------------------------------------------------------------------

/// Languages with method dispatch resolution (Go, Java, C++, TS, Rust).
pub trait DispatchProvider: TypeProvider {
    /// Resolve a method call on a receiver type to concrete function targets.
    ///
    /// `live_types` is the set of types observed as instantiated (for RTA pruning).
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId>;
}

// ---------------------------------------------------------------------------
// Structural typing capability
// ---------------------------------------------------------------------------

/// Languages with structural typing (TypeScript).
pub trait StructuralTypingProvider: TypeProvider {
    /// Check if `value_type` is assignable to `target_type`.
    fn is_assignable_to(
        &self,
        value_type: &ResolvedType,
        target_type: &ResolvedType,
    ) -> Compatibility;

    /// Resolve a generic type with concrete type arguments.
    ///
    /// Returns `None` if the base type is unknown or generics aren't supported.
    fn resolve_generic(&self, base_type: &str, type_args: &[String]) -> Option<ResolvedType>;
}

// ---------------------------------------------------------------------------
// Type resolution mode
// ---------------------------------------------------------------------------

/// How type sources are loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeResolutionMode {
    /// Parse all type sources upfront (default for reviews).
    Eager,
    /// Parse only types in files relevant to the diff scope.
    Scoped,
}

impl Default for TypeResolutionMode {
    fn default() -> Self {
        TypeResolutionMode::Eager
    }
}

// ---------------------------------------------------------------------------
// TypeRegistry
// ---------------------------------------------------------------------------

/// Registry of per-language type providers.
///
/// Built once per review, held in `CpgContext`. Routes type queries to the
/// appropriate language-specific provider.
pub struct TypeRegistry {
    /// All registered type providers (boxed trait objects).
    providers: Vec<Box<dyn TypeProvider>>,
    /// Index into `providers` by language.
    language_map: BTreeMap<Language, usize>,
    /// Providers that also implement `DispatchProvider`.
    /// Maps language → index into `dispatch_providers`.
    dispatch_map: BTreeMap<Language, usize>,
    dispatch_providers: Vec<Box<dyn DispatchProvider>>,
    /// Providers that also implement `StructuralTypingProvider`.
    structural_map: BTreeMap<Language, usize>,
    structural_providers: Vec<Box<dyn StructuralTypingProvider>>,
    /// Target language versions for version-aware analysis.
    pub target_versions: BTreeMap<Language, LanguageVersion>,
    /// Type resolution mode.
    pub mode: TypeResolutionMode,
}

impl std::fmt::Debug for TypeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeRegistry")
            .field("languages", &self.language_map.keys().collect::<Vec<_>>())
            .field(
                "dispatch_languages",
                &self.dispatch_map.keys().collect::<Vec<_>>(),
            )
            .field(
                "structural_languages",
                &self.structural_map.keys().collect::<Vec<_>>(),
            )
            .field("target_versions", &self.target_versions)
            .field("mode", &self.mode)
            .finish()
    }
}

impl TypeRegistry {
    /// Create an empty registry with no providers.
    pub fn empty() -> Self {
        TypeRegistry {
            providers: Vec::new(),
            language_map: BTreeMap::new(),
            dispatch_map: BTreeMap::new(),
            dispatch_providers: Vec::new(),
            structural_map: BTreeMap::new(),
            structural_providers: Vec::new(),
            target_versions: BTreeMap::new(),
            mode: TypeResolutionMode::Eager,
        }
    }

    /// Register a type provider. The provider's `languages()` method determines
    /// which languages it handles.
    pub fn register_provider(&mut self, provider: Box<dyn TypeProvider>) {
        let idx = self.providers.len();
        for lang in provider.languages() {
            self.language_map.insert(lang, idx);
        }
        self.providers.push(provider);
    }

    /// Register a dispatch provider.
    pub fn register_dispatch_provider(&mut self, provider: Box<dyn DispatchProvider>) {
        let idx = self.dispatch_providers.len();
        for lang in provider.languages() {
            self.dispatch_map.insert(lang, idx);
        }
        self.dispatch_providers.push(provider);
    }

    /// Register a structural typing provider.
    pub fn register_structural_provider(&mut self, provider: Box<dyn StructuralTypingProvider>) {
        let idx = self.structural_providers.len();
        for lang in provider.languages() {
            self.structural_map.insert(lang, idx);
        }
        self.structural_providers.push(provider);
    }

    /// Get the type provider for a language.
    pub fn provider_for(&self, lang: Language) -> Option<&dyn TypeProvider> {
        self.language_map
            .get(&lang)
            .map(|&idx| self.providers[idx].as_ref())
    }

    /// Get the dispatch provider for a language.
    pub fn dispatch_for(&self, lang: Language) -> Option<&dyn DispatchProvider> {
        self.dispatch_map
            .get(&lang)
            .map(|&idx| self.dispatch_providers[idx].as_ref())
    }

    /// Get the structural typing provider for a language.
    pub fn structural_for(&self, lang: Language) -> Option<&dyn StructuralTypingProvider> {
        self.structural_map
            .get(&lang)
            .map(|&idx| self.structural_providers[idx].as_ref())
    }

    /// Get the target version for a language.
    pub fn target_version(&self, lang: Language) -> Option<&LanguageVersion> {
        self.target_versions.get(&lang)
    }

    /// Set the target version for a language.
    pub fn set_target_version(&mut self, lang: Language, version: LanguageVersion) {
        self.target_versions.insert(lang, version);
    }

    /// Whether any type providers are registered.
    pub fn has_providers(&self) -> bool {
        !self.providers.is_empty()
    }
}
