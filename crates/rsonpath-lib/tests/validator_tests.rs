#[cfg(test)]
mod validator_tests {
    use rsonpath::{input::BorrowedBytes, validator::ValidatorEngine, StringPattern};
    use rsonpath_syntax::str::JsonString;

    fn build_properties(label_names: &[&str]) -> Vec<StringPattern> {
        label_names
            .iter()
            .map(|&s| StringPattern::new(&JsonString::new(s)))
            .collect()
    }

    fn build_engine(label_names: &[&str]) -> ValidatorEngine {
        ValidatorEngine::new(build_properties(label_names))
    }

    #[test]
    fn test_all_in_properties() {
        let property_names = ["name", "age", "city", "aaaa"];
        let json_input = r#"{"name": "Alice", "age": 30, "city": "New York"}"#;
        let input = BorrowedBytes::new(json_input.as_bytes());
        let engine = build_engine(&property_names);
        assert!(engine.validate(&input).is_ok());
    }

    #[test]
    fn test_not_in_properties() {
        let property_names = ["name", "age", "city"];
        let json_input = r#"{"name": "Alice", "age": 30, "country": "USA"}"#;
        let input = BorrowedBytes::new(json_input.as_bytes());
        let engine = build_engine(&property_names);
        assert!(engine.validate(&input).is_err());
    }

    #[test]
    fn test_empty_json() {
        let property_names = ["name", "age", "city"];
        let json_input = r#"{}"#;
        let input = BorrowedBytes::new(json_input.as_bytes());
        let engine = build_engine(&property_names);
        assert!(engine.validate(&input).is_ok());
    }
}
