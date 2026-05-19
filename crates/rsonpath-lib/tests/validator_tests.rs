#[cfg(test)]
mod validator_tests {
    use rsonpath::{input::BorrowedBytes, validator::ValidatorEngine};

    fn sample_schema_name_age() -> &'static str {
        r#"{"type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}}"#
    }

    fn compile_schema_engine(schema_str: &str) -> ValidatorEngine {
        ValidatorEngine::compile_schema(schema_str).expect("failed to compile schema")
    }

    // Basic object validation tests
    #[test]
    fn test_object_with_all_properties() {
        let schema = sample_schema_name_age();
        let json = r#"{"name": "Alice", "age": 30}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
    }

    #[test]
    fn test_object_with_subset_of_properties() {
        let schema = sample_schema_name_age();
        let json = r#"{"name": "Alice"}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
    }

    #[test]
    fn test_empty_object() {
        let schema = sample_schema_name_age();
        let json = r#"{}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
    }

    // additionalProperties tests
    #[test]
    fn test_additional_properties_false() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}, "additionalProperties": false}"#;
        let json = r#"{"name": "Alice", "country": "USA"}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_err());
    }

    #[test]
    fn test_additional_properties_schema_valid() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "additionalProperties": {"type": "integer"}}"#;
        let json = r#"{"name": "Alice", "age": 30}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
    }

    #[test]
    fn test_additional_properties_schema_invalid() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "additionalProperties": {"type": "object"}}"#;
        let json = r#"{"name": "Alice", "age": 30}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_err());
    }

    // Nested object validation tests
    #[test]
    fn test_nested_objects() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2_a": {
                            "type": "object",
                            "properties": {
                                "level3": {"type": "string"}
                            },
                            "additionalProperties": false
                        },
                        "level2_b": {
                            "type": "object",
                            "properties": {
                                "level3": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        }"#;
        let json = r#"{"level1": {"level2_a": {"level3": "value"}}}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
        let json = r#"{"level1": {"level2_b": {"level3": "value"}}}"#;
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
    }

    #[test]
    fn test_nested_object_with_disallowed_property() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            }
        }"#;
        let json = r#"{"user": {"name": "Alice", "age": 30}}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_err());
    }

    #[test]
    fn test_recursive_def() {
        let schema = r##"{
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "next": {"$ref": "#/$defs/Node"}
                    },
                    "additionalProperties": false
                }
            },
            "$ref": "#/$defs/Node"
        }"##;
        let json = r#"{"next": {"next": {"next": {}}}}"#;
        let engine = compile_schema_engine(schema);
        let input = BorrowedBytes::new(json.as_bytes());
        assert!(engine.validate(&input).is_ok());
    }
}
