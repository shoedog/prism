//! React hook detection and metadata extraction.
//!
//! Identifies React hook calls (`useState`, `useEffect`, `useMemo`, etc.) within
//! component functions and extracts semantic metadata: hook type, callback body
//! ranges, dependency array identifiers, and custom hook detection.

use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// Detected React hook call with semantic metadata.
#[derive(Debug, Clone)]
pub struct HookCall {
    pub file: String,
    pub function: String,
    pub line: usize,
    pub hook_type: HookType,
    pub callback: Option<CallbackInfo>,
    /// Dependency array info. `None` for hooks that don't take deps (useState, useRef, etc.).
    /// `Some(DepsInfo { is_missing: true, .. })` for hooks that accept deps but none were
    /// provided (e.g., `useEffect(() => {})` — runs every render).
    /// `Some(DepsInfo { is_empty: true, .. })` for empty deps (e.g., `useEffect(() => {}, [])` — mount only).
    pub deps: Option<DepsInfo>,
}

/// The type of React hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookType {
    UseState,
    UseReducer,
    UseEffect,
    UseLayoutEffect,
    UseMemo,
    UseCallback,
    UseRef,
    UseContext,
    UseId,
    UseTransition,
    UseDeferredValue,
    Use,
    Custom(String),
}

impl HookType {
    /// Classify a function call name as a hook type, if it matches.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "useState" => Some(Self::UseState),
            "useReducer" => Some(Self::UseReducer),
            "useEffect" => Some(Self::UseEffect),
            "useLayoutEffect" => Some(Self::UseLayoutEffect),
            "useMemo" => Some(Self::UseMemo),
            "useCallback" => Some(Self::UseCallback),
            "useRef" => Some(Self::UseRef),
            "useContext" => Some(Self::UseContext),
            "useId" => Some(Self::UseId),
            "useTransition" => Some(Self::UseTransition),
            "useDeferredValue" => Some(Self::UseDeferredValue),
            "use" => Some(Self::Use),
            _ => {
                // Custom hooks follow the useXxx pattern (capital letter after "use")
                if name.starts_with("use") && name.len() > 3 {
                    let next_char = name.as_bytes()[3];
                    if next_char.is_ascii_uppercase() {
                        return Some(Self::Custom(name.to_string()));
                    }
                }
                None
            }
        }
    }

    /// Whether this hook type takes a callback as its first argument.
    pub fn has_callback(&self) -> bool {
        matches!(
            self,
            Self::UseEffect | Self::UseLayoutEffect | Self::UseMemo | Self::UseCallback
        )
    }

    /// Whether this hook type takes a dependency array.
    pub fn has_deps(&self) -> bool {
        matches!(
            self,
            Self::UseEffect | Self::UseLayoutEffect | Self::UseMemo | Self::UseCallback
        )
    }
}

/// Information about a hook's callback argument.
#[derive(Debug, Clone)]
pub struct CallbackInfo {
    pub start_line: usize,
    pub end_line: usize,
    /// All identifiers referenced within the callback body (name, line).
    /// Includes both locally-defined and externally-captured identifiers.
    /// Layer 5 (dependency array analysis) will filter this to only identifiers
    /// defined in the enclosing component scope (true captures) using scope resolution.
    pub all_identifiers: Vec<(String, usize)>,
}

/// Information about a hook's dependency array.
#[derive(Debug, Clone)]
pub struct DepsInfo {
    pub line: usize,
    pub identifiers: Vec<(String, usize)>,
    pub is_empty: bool,
    pub is_missing: bool,
}

/// Detect all React hook calls in a set of parsed files.
///
/// Returns a map from file path to a list of detected hook calls.
pub fn detect_hooks(files: &BTreeMap<String, ParsedFile>) -> BTreeMap<String, Vec<HookCall>> {
    let mut result = BTreeMap::new();

    for (file_path, parsed) in files {
        // Only scan JS/TS/JSX/TSX files
        if !matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            continue;
        }

        let mut hooks = Vec::new();
        for func_node in parsed.all_functions() {
            let func_name = match parsed.language.function_name(&func_node) {
                Some(n) => parsed.node_text(&n).to_string(),
                None => continue,
            };

            collect_hooks_in_function(parsed, &func_node, file_path, &func_name, &mut hooks);
        }

        if !hooks.is_empty() {
            result.insert(file_path.clone(), hooks);
        }
    }

    result
}

fn collect_hooks_in_function(
    parsed: &ParsedFile,
    node: &Node<'_>,
    file: &str,
    func_name: &str,
    out: &mut Vec<HookCall>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            if let Some(hook) = try_extract_hook(parsed, &child, file, func_name) {
                out.push(hook);
            }
        }
        // Recurse into child nodes (but not into nested function definitions,
        // since those are separate functions with their own hook scope)
        let child_kind = child.kind();
        if child_kind != "arrow_function"
            && child_kind != "function_expression"
            && child_kind != "function_declaration"
        {
            collect_hooks_in_function(parsed, &child, file, func_name, out);
        }
    }
}

fn try_extract_hook(
    parsed: &ParsedFile,
    call_node: &Node<'_>,
    file: &str,
    func_name: &str,
) -> Option<HookCall> {
    let func_name_node = parsed.language.call_function_name(call_node)?;
    let callee_name = parsed.node_text(&func_name_node);

    let hook_type = HookType::from_name(callee_name)?;
    let line = call_node.start_position().row + 1;

    let args = parsed.language.call_arguments(call_node);

    let callback = if hook_type.has_callback() {
        args.as_ref().and_then(|a| extract_callback(parsed, a))
    } else {
        None
    };

    let deps = if hook_type.has_deps() {
        Some(extract_deps(parsed, args.as_ref()))
    } else {
        None
    };

    Some(HookCall {
        file: file.to_string(),
        function: func_name.to_string(),
        line,
        hook_type,
        callback,
        deps,
    })
}

fn extract_callback(parsed: &ParsedFile, args_node: &Node<'_>) -> Option<CallbackInfo> {
    // First argument should be an arrow_function or function_expression
    let mut cursor = args_node.walk();
    for child in args_node.children(&mut cursor) {
        if child.kind() == "arrow_function" || child.kind() == "function_expression" {
            let start_line = child.start_position().row + 1;
            let end_line = child.end_position().row + 1;

            let mut identifiers = Vec::new();
            collect_identifiers(parsed, &child, &mut identifiers);

            return Some(CallbackInfo {
                start_line,
                end_line,
                all_identifiers: identifiers,
            });
        }
    }
    None
}

fn extract_deps(parsed: &ParsedFile, args_node: Option<&Node<'_>>) -> DepsInfo {
    let args = match args_node {
        Some(a) => a,
        None => {
            return DepsInfo {
                line: 0,
                identifiers: Vec::new(),
                is_empty: false,
                is_missing: true,
            };
        }
    };

    // Find the second argument (skip the callback, find the array).
    // Note: this walks direct children of the arguments node, skipping punctuation.
    // This handles the common `useEffect(() => {}, [deps])` pattern correctly.
    // A more robust approach would use tree-sitter field access by index, but the
    // arguments node doesn't expose positional fields in most grammars.
    let mut cursor = args.walk();
    let mut arg_index = 0;
    for child in args.children(&mut cursor) {
        if child.kind() == "," || child.kind() == "(" || child.kind() == ")" {
            continue;
        }
        arg_index += 1;
        if arg_index == 2 && child.kind() == "array" {
            let line = child.start_position().row + 1;
            let mut identifiers = Vec::new();
            collect_identifiers(parsed, &child, &mut identifiers);
            let is_empty = identifiers.is_empty();

            return DepsInfo {
                line,
                identifiers,
                is_empty,
                is_missing: false,
            };
        }
    }

    // No second argument found — deps are missing
    DepsInfo {
        line: 0,
        identifiers: Vec::new(),
        is_empty: false,
        is_missing: true,
    }
}

fn collect_identifiers(parsed: &ParsedFile, node: &Node<'_>, out: &mut Vec<(String, usize)>) {
    if parsed.language.is_identifier_node(node.kind()) {
        let name = parsed.node_text(node).to_string();
        let line = node.start_position().row + 1;
        // Skip language keywords and well-known globals that aren't meaningful
        // for dependency tracking (not reactive values in a React component).
        if !is_filtered_global(&name) {
            out.push((name, line));
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_identifiers(parsed, &child, out);
    }
}

/// Filter out language keywords and well-known global objects that are not
/// reactive values in a React component context. These are excluded from
/// identifier collection because they cannot be stale closure captures.
fn is_filtered_global(name: &str) -> bool {
    matches!(
        name,
        // Language keywords
        "true"
            | "false"
            | "null"
            | "undefined"
            | "this"
            | "super"
            // Well-known global objects (stable, not reactive)
            | "console"
            | "window"
            | "document"
            | "globalThis"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_type_from_name() {
        assert_eq!(HookType::from_name("useState"), Some(HookType::UseState));
        assert_eq!(HookType::from_name("useEffect"), Some(HookType::UseEffect));
        assert_eq!(HookType::from_name("useRef"), Some(HookType::UseRef));
        assert_eq!(HookType::from_name("use"), Some(HookType::Use));
        assert_eq!(
            HookType::from_name("useCustomHook"),
            Some(HookType::Custom("useCustomHook".to_string()))
        );
        assert_eq!(HookType::from_name("user"), None);
        assert_eq!(HookType::from_name("used"), None);
        assert_eq!(HookType::from_name("notAHook"), None);
    }

    #[test]
    fn test_hook_has_callback() {
        assert!(HookType::UseEffect.has_callback());
        assert!(HookType::UseLayoutEffect.has_callback());
        assert!(HookType::UseMemo.has_callback());
        assert!(HookType::UseCallback.has_callback());
        assert!(!HookType::UseState.has_callback());
        assert!(!HookType::UseRef.has_callback());
    }
}
