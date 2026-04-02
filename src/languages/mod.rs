use tree_sitter::Node;

/// Supported programming languages for slicing analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    Rust,
    Lua,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "py" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => Some(Self::Cpp),
            "rs" => Some(Self::Rust),
            "lua" => Some(Self::Lua),
            _ => None,
        }
    }

    /// Detect language from file path.
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?;
        Self::from_extension(ext)
    }

    /// Get the tree-sitter language for parsing.
    pub fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
        }
    }

    /// Node types that represent function/method definitions.
    pub fn function_node_types(&self) -> Vec<&'static str> {
        match self {
            Self::Python => vec!["function_definition", "decorated_definition"],
            Self::JavaScript => vec![
                "function_declaration",
                "method_definition",
                "arrow_function",
                "function_expression",
                "generator_function_declaration",
            ],
            Self::TypeScript => vec![
                "function_declaration",
                "method_definition",
                "arrow_function",
                "function_expression",
                "generator_function_declaration",
            ],
            Self::Go => vec!["function_declaration", "method_declaration"],
            Self::Java => vec!["method_declaration", "constructor_declaration"],
            Self::C => vec!["function_definition"],
            Self::Cpp => vec!["function_definition", "template_declaration"],
            Self::Rust => vec!["function_item"],
            Self::Lua => vec!["function_declaration", "local_function"],
        }
    }

    /// Whether a node kind is an identifier/variable reference.
    pub fn is_identifier_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "identifier"
                | "shorthand_property_identifier"
                | "property_identifier"
                | "type_identifier"
                | "field_identifier"
                | "field_expression"
                | "qualified_identifier"
                | "namespace_identifier"
        )
    }

    /// Whether a node is an assignment expression.
    pub fn is_assignment_node(&self, kind: &str) -> bool {
        match self {
            Self::Python => {
                matches!(
                    kind,
                    "assignment" | "augmented_assignment" | "named_expression"
                )
            }
            Self::JavaScript | Self::TypeScript => {
                matches!(
                    kind,
                    "assignment_expression" | "augmented_assignment_expression"
                )
            }
            Self::Go => matches!(kind, "assignment_statement" | "short_var_declaration"),
            Self::Java => matches!(kind, "assignment_expression"),
            Self::C | Self::Cpp => {
                matches!(
                    kind,
                    "assignment_expression"
                        | "augmented_assignment_expression"
                        | "update_expression"
                )
            }
            Self::Rust => matches!(kind, "assignment_expression" | "compound_assignment_expr"),
            Self::Lua => matches!(kind, "assignment_statement"),
        }
    }

    /// Whether a node is a variable declaration.
    pub fn is_declaration_node(&self, kind: &str) -> bool {
        match self {
            Self::Python => false, // Python assignments are declarations
            Self::JavaScript | Self::TypeScript => {
                matches!(
                    kind,
                    "variable_declarator" | "lexical_declaration" | "variable_declaration"
                )
            }
            Self::Go => matches!(
                kind,
                "var_declaration" | "short_var_declaration" | "const_declaration"
            ),
            Self::Java => matches!(kind, "local_variable_declaration" | "field_declaration"),
            Self::C | Self::Cpp => {
                matches!(
                    kind,
                    "declaration" | "init_declarator" | "field_declaration"
                )
            }
            Self::Rust => matches!(kind, "let_declaration" | "const_item" | "static_item"),
            Self::Lua => matches!(kind, "local_variable_declaration"),
        }
    }

    /// Get the assignment target (L-value) from an assignment node.
    pub fn assignment_target<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Self::Python => {
                // named_expression (walrus :=) uses "name" field, not "left".
                // Only fall back to "name" for named_expression to avoid matching
                // unintended nodes if grammar changes add "name" to other node types.
                node.child_by_field_name("left").or_else(|| {
                    if node.kind() == "named_expression" {
                        node.child_by_field_name("name")
                    } else {
                        None
                    }
                })
            }
            Self::JavaScript | Self::TypeScript => node.child_by_field_name("left"),
            Self::Go => node.child_by_field_name("left"),
            Self::Java => node.child_by_field_name("left"),
            Self::C | Self::Cpp => node.child_by_field_name("left"),
            Self::Rust => node.child_by_field_name("left"),
            Self::Lua => {
                // assignment_statement -> variable_list (first child with identifiers)
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_list" {
                        return child.child(0);
                    }
                }
                None
            }
        }
    }

    /// Get the assignment value (R-value) from an assignment node.
    pub fn assignment_value<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Self::Python => {
                // named_expression (walrus :=) uses "value" field, not "right"
                node.child_by_field_name("right").or_else(|| {
                    if node.kind() == "named_expression" {
                        node.child_by_field_name("value")
                    } else {
                        None
                    }
                })
            }
            Self::JavaScript | Self::TypeScript => node.child_by_field_name("right"),
            Self::Go => node.child_by_field_name("right"),
            Self::Java => node.child_by_field_name("right"),
            Self::C | Self::Cpp => node.child_by_field_name("right"),
            Self::Rust => node.child_by_field_name("right"),
            Self::Lua => {
                // assignment_statement -> expression_list (RHS values)
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "expression_list" {
                        return child.child(0);
                    }
                }
                None
            }
        }
    }

    /// Get the variable name from a declaration node.
    pub fn declaration_name<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Self::Python => None,
            Self::JavaScript | Self::TypeScript => {
                if node.kind() == "variable_declarator" {
                    node.child_by_field_name("name")
                } else {
                    // lexical_declaration -> variable_declarator -> name
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declarator" {
                            return child.child_by_field_name("name");
                        }
                    }
                    None
                }
            }
            Self::Go => {
                // var_declaration -> var_spec -> name
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "var_spec" || child.kind() == "const_spec" {
                        return child.child_by_field_name("name");
                    }
                }
                node.child_by_field_name("left")
            }
            Self::Java => {
                // local_variable_declaration -> variable_declarator -> name
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator" {
                        return child.child_by_field_name("name");
                    }
                }
                None
            }
            Self::Rust => {
                // let_declaration -> pattern -> identifier
                // const_item -> name
                // static_item -> name
                if node.kind() == "let_declaration" {
                    if let Some(p) = node.child_by_field_name("pattern") {
                        if p.kind() == "identifier" {
                            return Some(p);
                        }
                        // Destructuring: try first identifier child
                        let mut c = p.walk();
                        for child in p.children(&mut c) {
                            if child.kind() == "identifier" {
                                return Some(child);
                            }
                        }
                    }
                    return None;
                }
                node.child_by_field_name("name")
            }
            Self::C | Self::Cpp => {
                // declaration -> init_declarator -> declarator -> identifier
                // OR declaration -> declarator -> identifier (no init)
                if node.kind() == "init_declarator" {
                    return find_identifier_in_c_declarator(node);
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "init_declarator" {
                        return find_identifier_in_c_declarator(&child);
                    }
                    // Direct declarator without initializer
                    if child.kind() == "identifier" || child.kind() == "field_identifier" {
                        return Some(child);
                    }
                }
                None
            }
            Self::Lua => {
                // local_variable_declaration -> variable_list -> identifier
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_list" || child.kind() == "identifier" {
                        if child.kind() == "identifier" {
                            return Some(child);
                        }
                        // variable_list -> first identifier
                        let mut c2 = child.walk();
                        for gc in child.children(&mut c2) {
                            if gc.kind() == "identifier" {
                                return Some(gc);
                            }
                        }
                    }
                }
                None
            }
        }
    }

    /// Get the initializer value from a declaration node.
    /// Used by Phase 3 must-alias tracking to detect `type *ptr = dev`.
    pub fn declaration_value<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Self::Python => None,
            Self::JavaScript | Self::TypeScript => {
                if node.kind() == "variable_declarator" {
                    node.child_by_field_name("value")
                } else {
                    // lexical_declaration -> variable_declarator -> value
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declarator" {
                            return child.child_by_field_name("value");
                        }
                    }
                    None
                }
            }
            Self::Go => {
                // var_declaration -> var_spec -> value
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "var_spec" || child.kind() == "const_spec" {
                        return child.child_by_field_name("value");
                    }
                }
                // short_var_declaration uses "right"
                node.child_by_field_name("right")
            }
            Self::Java => {
                // local_variable_declaration -> variable_declarator -> value
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator" {
                        return child.child_by_field_name("value");
                    }
                }
                None
            }
            Self::Rust => {
                // let_declaration -> value
                node.child_by_field_name("value")
            }
            Self::C | Self::Cpp => {
                // declaration -> init_declarator -> value
                if node.kind() == "init_declarator" {
                    return node.child_by_field_name("value");
                }
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "init_declarator" {
                        return child.child_by_field_name("value");
                    }
                }
                None
            }
            Self::Lua => {
                // local_variable_declaration -> expression_list -> first expr
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "expression_list" {
                        return child.child(0);
                    }
                }
                None
            }
        }
    }

    /// Whether a node is a control flow statement.
    pub fn is_control_flow_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "if_statement"
                | "if_expression"
                | "for_statement"
                | "for_in_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "match_statement"
                | "for_range_statement"    // Go
                | "for_range_loop"         // C++ range-based for
                | "enhanced_for_statement" // Java
                | "try_statement"
                | "goto_statement"         // C/C++
                | "match_expression"       // Rust
                | "if_let_expression"      // Rust
                | "while_let_expression"   // Rust
                | "for_expression"         // Rust
                | "loop_expression"   // Rust
                | "repeat_statement" // Lua repeat..until
        )
    }

    /// Get the condition expression from a control flow node.
    pub fn control_flow_condition<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        node.child_by_field_name("condition")
            .or_else(|| node.child_by_field_name("value"))
    }

    /// Whether a node is a function call.
    pub fn is_call_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "call_expression"
                | "call"
                | "method_invocation"
                | "object_creation_expression"
                | "new_expression"
                | "function_call" // Lua
        )
    }

    /// Get the function name node from a call.
    pub fn call_function_name<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        let func_node = node
            .child_by_field_name("function")
            .or_else(|| node.child_by_field_name("name"))
            .or_else(|| node.child_by_field_name("object"))
            .or_else(|| {
                // Lua function_call: first child is the function expression
                if node.kind() == "function_call" {
                    node.child(0)
                } else {
                    None
                }
            })?;

        // For field-access calls (timer->callback(...), obj.method(...)),
        // extract the field identifier so it can match function definitions.
        // Without this, the full text "timer->callback" would never match
        // a function named "callback" in the call graph.
        // Lua dot_index_expression: io.open -> extract "open"
        // Lua method_index_expression: obj:close -> extract "close"
        if func_node.kind() == "dot_index_expression"
            || func_node.kind() == "method_index_expression"
        {
            if let Some(field) = func_node.child_by_field_name("field") {
                return Some(field);
            }
            // method_index_expression may use "method" field name
            if let Some(method) = func_node.child_by_field_name("method") {
                return Some(method);
            }
        }

        if func_node.kind() == "field_expression" || func_node.kind() == "member_expression" {
            if let Some(field) = func_node.child_by_field_name("field") {
                return Some(field);
            }
            // JS/TS member_expression uses "property" instead of "field"
            if let Some(prop) = func_node.child_by_field_name("property") {
                return Some(prop);
            }
        }

        Some(func_node)
    }

    /// Get the arguments node from a call.
    pub fn call_arguments<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        node.child_by_field_name("arguments")
            .or_else(|| node.child_by_field_name("argument_list"))
    }

    /// Whether a node kind represents a lexical scope block that introduces variable shadowing.
    pub fn is_scope_block(&self, kind: &str) -> bool {
        matches!(
            kind,
            "block"              // Go, Java
                | "compound_statement" // C, C++
                | "statement_block" // JavaScript, TypeScript
        )
    }

    /// Whether a node is a return statement.
    pub fn is_return_node(&self, kind: &str) -> bool {
        matches!(kind, "return_statement")
    }

    /// Whether a node is a statement-level construct suitable for CFG construction.
    ///
    /// Returns true for assignments, declarations, calls, branches, loops,
    /// return/break/continue, gotos, labels — anything that represents an
    /// executable step within a function body.
    pub fn is_statement_node(&self, kind: &str) -> bool {
        self.is_assignment_node(kind)
            || self.is_declaration_node(kind)
            || self.is_control_flow_node(kind)
            || self.is_return_node(kind)
            || self.is_call_node(kind)
            || matches!(
                kind,
                // C/C++
                "expression_statement"
                    | "labeled_statement"
                    | "break_statement"
                    | "continue_statement"
                    | "throw_statement"   // JS/TS, Java, C++
                    | "raise_statement"   // Python
                    | "yield_statement"   // Python
                    | "assert_statement"  // Python
                    | "delete_statement"  // Python
                    // Go
                    | "go_statement"
                    | "defer_statement"
                    | "send_statement"
                    | "select_statement"
                    | "fallthrough_statement"
                    // Rust
                    | "macro_invocation"
            )
    }

    /// Whether a node is a loop construct (subset of control flow).
    pub fn is_loop_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "for_statement"
                | "for_in_statement"
                | "while_statement"
                | "do_statement"
                | "for_range_statement"    // Go
                | "for_range_loop"         // C++ range-based for
                | "enhanced_for_statement" // Java
                | "for_expression"         // Rust
                | "loop_expression"        // Rust
                | "while_let_expression"   // Rust
                | "repeat_statement" // Lua repeat..until
        )
    }

    /// Whether a statement is a terminator — no fall-through to the next statement.
    pub fn is_terminator(&self, kind: &str) -> bool {
        matches!(
            kind,
            "return_statement"
                | "break_statement"
                | "continue_statement"
                | "goto_statement"
                | "throw_statement"
                | "raise_statement"
        )
    }

    /// Whether a switch/match has fall-through semantics between cases.
    ///
    /// C/C++/JS/Java: yes (unless `break`). Go/Rust: no.
    pub fn switch_has_fallthrough(&self) -> bool {
        matches!(
            self,
            Self::C | Self::Cpp | Self::JavaScript | Self::TypeScript | Self::Java
        )
    }

    /// Get the function name node from a function definition.
    pub fn function_name<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        // Handle decorated definitions (Python)
        if node.kind() == "decorated_definition" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    return child.child_by_field_name("name");
                }
            }
            return None;
        }

        // Handle C/C++ template declarations
        if node.kind() == "template_declaration" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    return find_identifier_in_c_function_def(&child);
                }
            }
            return None;
        }

        // C/C++ function definitions have the name nested in a declarator chain
        if matches!(self, Self::C | Self::Cpp) && node.kind() == "function_definition" {
            return find_identifier_in_c_function_def(node);
        }

        // Rust impl blocks: extract the type name
        if node.kind() == "impl_item" {
            return node.child_by_field_name("type");
        }

        // Lua function_declaration: name is in "name" field
        // local_function: name is in "name" field
        node.child_by_field_name("name")
    }
}

/// Navigate the C/C++ declarator chain to find the function name identifier.
///
/// C function definitions have a complex declarator structure:
///   function_definition -> declarator -> (pointer_declarator ->) function_declarator -> identifier
///
/// Examples:
///   `void foo(int x)` -> declarator is `foo(int x)` -> function_declarator -> identifier `foo`
///   `void *bar(void)` -> declarator is `*bar(void)` -> pointer_declarator -> function_declarator -> identifier `bar`
///   `static int baz(int a, int b)` -> same pattern through declarator field
fn find_identifier_in_c_function_def<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let declarator = node.child_by_field_name("declarator")?;
    find_identifier_in_c_declarator(&declarator)
}

/// Recursively navigate a C/C++ declarator to find the innermost identifier.
///
/// Declarators can be nested: pointer_declarator -> function_declarator -> identifier
/// Or: array_declarator -> identifier
/// Or just: identifier
fn find_identifier_in_c_declarator<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    match node.kind() {
        "identifier" | "field_identifier" => Some(*node),
        "qualified_identifier" => {
            // C++: namespace::function_name — get the rightmost identifier
            let mut cursor = node.walk();
            let mut last_id = None;
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "template_function" {
                    last_id = Some(child);
                }
            }
            last_id
        }
        "destructor_name" => {
            // C++: ~ClassName — get the identifier after ~
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    return Some(child);
                }
            }
            None
        }
        _ => {
            // Navigate through declarator chain: check "declarator" field first,
            // then iterate children looking for nested declarators or identifiers
            if let Some(inner) = node.child_by_field_name("declarator") {
                return find_identifier_in_c_declarator(&inner);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "identifier" | "field_identifier" => return Some(child),
                    "pointer_declarator"
                    | "function_declarator"
                    | "array_declarator"
                    | "parenthesized_declarator"
                    | "reference_declarator" => {
                        if let Some(id) = find_identifier_in_c_declarator(&child) {
                            return Some(id);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
    }
}
