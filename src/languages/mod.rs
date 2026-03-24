use tree_sitter::Node;

/// Supported programming languages for slicing analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
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
        }
    }

    /// Whether a node kind is an identifier/variable reference.
    pub fn is_identifier_node(&self, kind: &str) -> bool {
        matches!(
            kind,
            "identifier" | "shorthand_property_identifier" | "property_identifier"
                | "type_identifier" | "field_identifier"
        )
    }

    /// Whether a node is an assignment expression.
    pub fn is_assignment_node(&self, kind: &str) -> bool {
        match self {
            Self::Python => matches!(kind, "assignment" | "augmented_assignment"),
            Self::JavaScript | Self::TypeScript => {
                matches!(kind, "assignment_expression" | "augmented_assignment_expression")
            }
            Self::Go => matches!(kind, "assignment_statement" | "short_var_declaration"),
            Self::Java => matches!(kind, "assignment_expression"),
        }
    }

    /// Whether a node is a variable declaration.
    pub fn is_declaration_node(&self, kind: &str) -> bool {
        match self {
            Self::Python => false, // Python assignments are declarations
            Self::JavaScript | Self::TypeScript => {
                matches!(kind, "variable_declarator" | "lexical_declaration" | "variable_declaration")
            }
            Self::Go => matches!(kind, "var_declaration" | "short_var_declaration" | "const_declaration"),
            Self::Java => matches!(kind, "local_variable_declaration" | "field_declaration"),
        }
    }

    /// Get the assignment target (L-value) from an assignment node.
    pub fn assignment_target<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Self::Python => node.child_by_field_name("left"),
            Self::JavaScript | Self::TypeScript => node.child_by_field_name("left"),
            Self::Go => node.child_by_field_name("left"),
            Self::Java => node.child_by_field_name("left"),
        }
    }

    /// Get the assignment value (R-value) from an assignment node.
    pub fn assignment_value<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        match self {
            Self::Python => node.child_by_field_name("right"),
            Self::JavaScript | Self::TypeScript => node.child_by_field_name("right"),
            Self::Go => node.child_by_field_name("right"),
            Self::Java => node.child_by_field_name("right"),
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
                | "for_range_statement" // Go
                | "enhanced_for_statement" // Java
                | "try_statement"
        )
    }

    /// Get the condition expression from a control flow node.
    pub fn control_flow_condition<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        node.child_by_field_name("condition")
            .or_else(|| node.child_by_field_name("value"))
    }

    /// Whether a node is a function call.
    pub fn is_call_node(&self, kind: &str) -> bool {
        matches!(kind, "call_expression" | "call" | "method_invocation" | "object_creation_expression")
    }

    /// Get the function name node from a call.
    pub fn call_function_name<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        node.child_by_field_name("function")
            .or_else(|| node.child_by_field_name("name"))
            .or_else(|| node.child_by_field_name("object"))
    }

    /// Get the arguments node from a call.
    pub fn call_arguments<'a>(&self, node: &Node<'a>) -> Option<Node<'a>> {
        node.child_by_field_name("arguments")
            .or_else(|| node.child_by_field_name("argument_list"))
    }

    /// Whether a node is a return statement.
    pub fn is_return_node(&self, kind: &str) -> bool {
        matches!(kind, "return_statement")
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
        node.child_by_field_name("name")
    }
}
