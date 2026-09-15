use rsonpath::input::{Input, OwnedBytes};
use rsonpath::validator::well_formedness::ActiveWellFormednessCheck;
use rsonpath::validator::ValidatorEngine;

fn get_engine(schema: &str) -> ValidatorEngine<ActiveWellFormednessCheck> {
    let schema_auto = rsonpath::validator::schema_parser::parse_schema(schema).unwrap();
    ValidatorEngine::from_compiled_schema(schema_auto).with_well_formedness_check(ActiveWellFormednessCheck::new())
}

fn validate(
    engine: &ValidatorEngine<ActiveWellFormednessCheck>,
    json: &str,
) -> Result<(), rsonpath::validator::ValidatorEngineError> {
    let input = OwnedBytes::from(json.to_string());
    engine.validate(&input)
}

#[test]
fn test_valid_json() {
    let schema = r#"{
        "type": "object",
        "properties": {
            "a": {
                "type": ["integer", "array"],
                "items": {"type": "integer"}
            }
        }
    }"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{}").is_ok());
    validate(&engine, "{\"a\": 1}").unwrap();
    validate(&engine, "{\"a\": [1, 2, 3]}").unwrap();
}

#[test]
fn test_invalid_json_trailing_comma() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{\"a\": 1,}").is_err());
    let schema_arr = r#"{"type": "array", "items": {"type": "integer"}}"#;
    let engine_arr = get_engine(schema_arr);
    assert!(validate(&engine_arr, "[1,]").is_err());
}

#[test]
fn test_invalid_json_extra_comma() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{,\"a\": 1}").is_err());
    let schema_arr = r#"{"type": "array", "items": {"type": "integer"}}"#;
    let engine_arr = get_engine(schema_arr);
    assert!(validate(&engine_arr, "[,1]").is_err());
}

#[test]
fn test_invalid_json_missing_colon() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{\"a\" 1}").is_err());
}

#[test]
fn test_invalid_json_colon_in_array() {
    let schema_arr = r#"{"type": "array", "items": {"type": "integer"}}"#;
    let engine_arr = get_engine(schema_arr);
    assert!(validate(&engine_arr, "[1 : 2]").is_err());
}

#[test]
fn test_invalid_json_missing_closing_bracket() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{\"a\": 1").is_err());
    let schema_arr = r#"{"type": "array", "items": {"type": "integer"}}"#;
    let engine_arr = get_engine(schema_arr);
    assert!(validate(&engine_arr, "[").is_err());
}

#[test]
fn test_invalid_json_multiple_colons() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{\"a\":: 1}").is_err());
}

#[test]
fn test_invalid_json_multiple_commas() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{\"a\": 1,, \"b\": 2}").is_err());
    let schema_arr = r#"{"type": "array", "items": {"type": "integer"}}"#;
    let engine_arr = get_engine(schema_arr);
    assert!(validate(&engine_arr, "[1,, 2]").is_err());
}

#[test]
fn test_invalid_json_bracket_mismatch() {
    let schema = r#"{"type": "object", "properties": {"a": {"type": "integer"}}}"#;
    let engine = get_engine(schema);
    assert!(validate(&engine, "{\"a\": 1]").is_err());
    let schema_arr = r#"{"type": "array", "items": {"type": "integer"}}"#;
    let engine_arr = get_engine(schema_arr);
    assert!(validate(&engine_arr, "[1, 2}").is_err());
}
