pub const QUERY: &str = r#"
    (class_definition
        name: (identifier) @name.definition.class) @definition.class

    (function_definition
        name: (identifier) @name.definition.function) @definition.function
"#;
