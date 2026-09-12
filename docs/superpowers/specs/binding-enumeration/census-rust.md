# Binding census: rust

Grammar SHA-256: `4b73a1248978340336100db455bf0731c23f9190568c9ae62265fa4a80a327d5`

| Kind | Named | Heuristic fields | Existing curated row | Candidate | Grammar-only | Corpus occurrences |
|---|---:|---|---:|---:|---:|---:|
| `abstract_type` | true |  | false | false | false | 0 |
| `arguments` | true |  | false | false | false | 66 |
| `array_expression` | true |  | false | false | false | 0 |
| `array_type` | true |  | false | false | false | 0 |
| `assignment_expression` | true |  | false | true | false | 8 |
| `associated_type` | true |  | false | false | false | 0 |
| `async_block` | true |  | false | true | false | 0 |
| `attribute` | true |  | false | false | false | 0 |
| `attribute_item` | true |  | false | false | false | 0 |
| `await_expression` | true |  | false | false | false | 0 |
| `base_field_initializer` | true |  | false | false | false | 0 |
| `binary_expression` | true |  | false | false | false | 5 |
| `block` | true |  | true | true | false | 89 |
| `block_comment` | true |  | false | true | false | 0 |
| `boolean_literal` | true |  | false | false | false | 0 |
| `bounded_type` | true |  | false | false | false | 0 |
| `bracketed_type` | true |  | false | false | false | 0 |
| `break_expression` | true |  | false | false | false | 0 |
| `call_expression` | true |  | false | false | false | 66 |
| `captured_pattern` | true |  | false | true | false | 0 |
| `closure_expression` | true | parameters | false | true | false | 2 |
| `closure_parameters` | true |  | false | true | false | 2 |
| `compound_assignment_expr` | true |  | false | true | false | 0 |
| `const_block` | true |  | false | true | false | 0 |
| `const_item` | true | name | true | false | false | 0 |
| `const_parameter` | true | name | false | true | false | 0 |
| `continue_expression` | true |  | false | false | false | 0 |
| `declaration_list` | true |  | false | true | false | 0 |
| `dynamic_type` | true |  | false | false | false | 1 |
| `else_clause` | true |  | false | false | false | 1 |
| `empty_statement` | true |  | false | false | false | 0 |
| `enum_item` | true | name | false | false | false | 0 |
| `enum_variant` | true |  | false | false | false | 0 |
| `enum_variant_list` | true |  | false | false | false | 0 |
| `expression_statement` | true |  | false | false | false | 53 |
| `extern_crate_declaration` | true | alias, name | false | true | false | 0 |
| `extern_modifier` | true |  | false | false | false | 0 |
| `field_declaration` | true | name | false | true | false | 0 |
| `field_declaration_list` | true |  | false | true | false | 0 |
| `field_expression` | true |  | false | false | false | 23 |
| `field_initializer` | true |  | false | false | false | 0 |
| `field_initializer_list` | true |  | false | false | false | 0 |
| `field_pattern` | true | name, pattern | false | true | false | 0 |
| `for_expression` | true | pattern | false | true | false | 0 |
| `for_lifetimes` | true |  | false | true | false | 0 |
| `foreign_mod_item` | true |  | false | false | false | 0 |
| `fragment_specifier` | true |  | false | false | false | 0 |
| `function_item` | true | name, parameters | false | true | false | 83 |
| `function_modifiers` | true |  | false | true | false | 0 |
| `function_signature_item` | true | name, parameters | false | true | false | 0 |
| `function_type` | true | parameters | false | true | false | 0 |
| `gen_block` | true |  | false | true | false | 0 |
| `generic_function` | true |  | false | true | false | 0 |
| `generic_pattern` | true |  | false | true | false | 0 |
| `generic_type` | true |  | false | false | false | 1 |
| `generic_type_with_turbofish` | true |  | false | true | false | 0 |
| `higher_ranked_trait_bound` | true |  | false | false | false | 0 |
| `if_expression` | true |  | false | false | false | 1 |
| `impl_item` | true |  | false | false | false | 0 |
| `index_expression` | true |  | false | false | false | 0 |
| `inner_attribute_item` | true |  | false | false | false | 0 |
| `inner_doc_comment_marker` | true |  | false | false | false | 0 |
| `label` | true |  | false | false | false | 0 |
| `let_chain` | true |  | false | true | false | 0 |
| `let_condition` | true | pattern | false | true | false | 0 |
| `let_declaration` | true | pattern | true | true | false | 19 |
| `lifetime` | true |  | false | false | false | 0 |
| `lifetime_parameter` | true |  | false | true | false | 0 |
| `line_comment` | true |  | false | false | false | 0 |
| `loop_expression` | true |  | false | false | false | 2 |
| `macro_definition` | true |  | false | false | false | 0 |
| `macro_invocation` | true |  | false | false | false | 6 |
| `macro_rule` | true |  | false | false | false | 0 |
| `match_arm` | true | pattern | false | true | false | 0 |
| `match_block` | true |  | false | true | false | 0 |
| `match_expression` | true |  | false | true | false | 0 |
| `match_pattern` | true |  | false | true | false | 0 |
| `mod_item` | true | name | false | false | false | 0 |
| `mut_pattern` | true |  | false | true | false | 0 |
| `negative_literal` | true |  | false | false | false | 0 |
| `never_type` | true |  | false | false | false | 0 |
| `or_pattern` | true |  | false | true | false | 0 |
| `ordered_field_declaration_list` | true |  | false | true | false | 0 |
| `outer_doc_comment_marker` | true |  | false | false | false | 0 |
| `parameter` | true | pattern | false | true | false | 20 |
| `parameters` | true |  | true | true | false | 83 |
| `parenthesized_expression` | true |  | false | false | false | 0 |
| `pointer_type` | true |  | false | false | false | 0 |
| `qualified_type` | true |  | false | false | false | 0 |
| `range_expression` | true |  | false | true | false | 0 |
| `range_pattern` | true | left | false | true | false | 0 |
| `raw_string_literal` | true |  | false | false | false | 0 |
| `ref_pattern` | true |  | false | true | false | 0 |
| `reference_expression` | true |  | false | false | false | 0 |
| `reference_pattern` | true |  | false | true | false | 0 |
| `reference_type` | true |  | false | false | false | 2 |
| `remaining_field_pattern` | true |  | false | true | false | 0 |
| `removed_trait_bound` | true |  | false | false | false | 0 |
| `return_expression` | true |  | false | false | false | 0 |
| `scoped_identifier` | true |  | false | false | false | 7 |
| `scoped_type_identifier` | true |  | false | false | false | 3 |
| `scoped_use_list` | true |  | false | false | false | 0 |
| `self_parameter` | true |  | false | true | false | 24 |
| `shorthand_field_initializer` | true |  | false | false | false | 0 |
| `slice_pattern` | true |  | false | true | false | 0 |
| `source_file` | true |  | false | false | false | 0 |
| `static_item` | true | name | true | false | false | 0 |
| `string_literal` | true |  | false | false | false | 0 |
| `struct_expression` | true |  | false | false | false | 0 |
| `struct_item` | true | name | false | false | false | 0 |
| `struct_pattern` | true |  | false | true | false | 0 |
| `token_binding_pattern` | true |  | false | true | false | 0 |
| `token_repetition` | true |  | false | false | false | 0 |
| `token_repetition_pattern` | true |  | false | true | false | 0 |
| `token_tree` | true |  | false | false | false | 13 |
| `token_tree_pattern` | true |  | false | true | false | 0 |
| `trait_bounds` | true |  | false | false | false | 0 |
| `trait_item` | true | name | false | false | false | 0 |
| `try_block` | true |  | false | true | false | 0 |
| `try_expression` | true |  | false | false | false | 0 |
| `tuple_expression` | true |  | false | false | false | 0 |
| `tuple_pattern` | true |  | false | true | false | 0 |
| `tuple_struct_pattern` | true |  | false | true | false | 0 |
| `tuple_type` | true |  | false | false | false | 0 |
| `type_arguments` | true |  | false | false | false | 1 |
| `type_binding` | true | name | false | true | false | 0 |
| `type_cast_expression` | true |  | false | false | false | 0 |
| `type_item` | true | name | false | false | false | 0 |
| `type_parameter` | true | name | false | true | false | 0 |
| `type_parameters` | true |  | false | true | false | 0 |
| `unary_expression` | true |  | false | false | false | 0 |
| `union_item` | true | name | false | false | false | 0 |
| `unit_expression` | true |  | false | false | false | 0 |
| `unit_type` | true |  | false | false | false | 0 |
| `unsafe_block` | true |  | false | true | false | 0 |
| `use_as_clause` | true | alias | false | false | false | 0 |
| `use_bounds` | true |  | false | false | false | 0 |
| `use_declaration` | true |  | false | true | false | 0 |
| `use_list` | true |  | false | false | false | 0 |
| `use_wildcard` | true |  | false | false | false | 0 |
| `variadic_parameter` | true | pattern | false | true | false | 0 |
| `visibility_modifier` | true |  | false | false | false | 27 |
| `where_clause` | true |  | false | false | false | 0 |
| `where_predicate` | true |  | false | false | false | 0 |
| `while_expression` | true |  | false | false | false | 0 |
| `yield_expression` | true |  | false | false | false | 0 |
| `char_literal` | true |  | false | false | false | 0 |
| `crate` | true |  | false | false | false | 0 |
| `doc_comment` | true |  | false | false | false | 0 |
| `escape_sequence` | true |  | false | false | false | 0 |
| `field_identifier` | true |  | false | false | false | 23 |
| `float_literal` | true |  | false | false | false | 0 |
| `identifier` | true |  | false | false | false | 239 |
| `integer_literal` | true |  | false | false | false | 21 |
| `metavariable` | true |  | false | true | false | 0 |
| `mutable_specifier` | true |  | false | false | false | 6 |
| `primitive_type` | true |  | false | false | false | 20 |
| `self` | true |  | false | false | false | 24 |
| `shebang` | true |  | false | false | false | 0 |
| `shorthand_field_identifier` | true |  | false | false | false | 0 |
| `string_content` | true |  | false | false | false | 0 |
| `super` | true |  | false | false | false | 0 |
| `type_identifier` | true |  | false | false | false | 17 |
