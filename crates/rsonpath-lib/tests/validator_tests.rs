#[cfg(test)]
mod validator_tests {
    use rsonpath::{input::BorrowedBytes, validator::ValidatorEngine};
    use rsonpath_syntax::str::JsonString;

    fn build_engine(property_names: &[&str]) -> ValidatorEngine {
        ValidatorEngine::new(property_names.iter().copied().map(JsonString::from))
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
