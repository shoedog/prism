macro_rules! placeholder_owners {
    ($language:ident, $predicate:ident) => {
        &[
            (($language::Java, "formal_parameters", None::<$predicate>, "e0a-x-java-formal_parameters"), "Task 20"),
            (($language::Java, "spread_parameter", None::<$predicate>, "e0a-x-java-spread_parameter"), "Task 20"),
            (($language::C, "parameter_declaration", None::<$predicate>, "e0a-x-c-parameter_declaration"), "Task 22"),
            (($language::C, "parameter_list", None::<$predicate>, "e0a-x-c-parameter_list"), "Task 22"),
            (($language::Cpp, "parameter_declaration", None::<$predicate>, "e0a-x-cpp-parameter_declaration"), "Task 23"),
            (($language::Cpp, "parameter_list", None::<$predicate>, "e0a-x-cpp-parameter_list"), "Task 23"),
            (($language::Lua, "parameters", None::<$predicate>, "e0a-x-lua-parameters"), "Task 26"),
        ]
    };
}
