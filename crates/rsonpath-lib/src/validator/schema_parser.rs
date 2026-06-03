//! JSON Schema parser.
use crate::string_pattern::StringPattern;
use crate::validator::schema_automaton::{
    AdditionalProperties, ArrayConstraints, ObjectConstraints, SchemaAutomaton, SchemaNode, SchemaNodeId,
    TypeConstraints,
};
use rsonpath_syntax::str::JsonString;
use serde_json::Value;
use smallvec::SmallVec;

use std::collections::HashMap;
use thiserror::Error;

/// JSON Schema definition parser.
#[derive(Debug)]
pub struct SchemaParser {
    /// All nodes in the schema graph.
    nodes: Vec<SchemaNode>,
    /// Maps type names in $defs to their corresponding nodes.
    defs: HashMap<StringPattern, SchemaNodeId>,
}

impl SchemaParser {
    const DEFAULT_TYPES: [&str; 6] = ["null", "boolean", "object", "array", "number", "string"];

    /// Create a new parser.
    #[inline]
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            defs: HashMap::new(),
        }
    }

    /// Add a new schema node and return its id.
    #[inline]
    fn add_node(&mut self, node: SchemaNode) -> SchemaNodeId {
        let id = SchemaNodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    fn parse(&mut self, value: &Value) -> Result<SchemaNodeId, SchemaParseError> {
        if let Value::Bool(true) = value {
            return Ok(self.add_node(SchemaNode::Any));
        }

        if let Value::Bool(false) = value {
            return Err(SchemaParseError::UnsupportedKeyword("false".into()));
        }

        // Check for $ref.
        if let Some(ref_str) = value.get("$ref").and_then(|v| v.as_str()) {
            return self.resolve_ref(ref_str);
        }

        let types: SmallVec<[&str; 7]> = match value.get("type") {
            None => SmallVec::from_slice(&Self::DEFAULT_TYPES),
            Some(Value::String(s)) => SmallVec::from_slice(&[s.as_str()]),
            Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
            Some(_) => {
                return Err(SchemaParseError::InvalidSchema(
                    "The 'type' keyword must be a string or an array of strings.".into(),
                ))
            }
        };

        let mut constraints = Vec::new();
        for type_str in types {
            match type_str {
                "object" => constraints.push(self.parse_object(value)?),
                "array" => constraints.push(self.parse_array(value)?),
                "string" => constraints.push(self.parse_primitive_string()?),
                "number" | "integer" => constraints.push(self.parse_primitive_number()?),
                "boolean" => constraints.push(self.parse_primitive_boolean()?),
                "null" => constraints.push(self.parse_primitive_null()?),
                _ => return Err(SchemaParseError::InvalidSchema("Unsupported type.".into())),
            }
        }

        if constraints.len() == 1 {
            Ok(constraints[0])
        } else {
            Ok(self.add_node(SchemaNode::Type(TypeConstraints::new(constraints))))
        }
    }

    fn parse_object(&mut self, value: &Value) -> Result<SchemaNodeId, SchemaParseError> {
        let mut properties = Vec::new();
        let mut additional_properties = AdditionalProperties::default();

        // properties
        if let Some(props_map) = value.get("properties").and_then(|p| p.as_object()) {
            properties.reserve(props_map.len());
            for (key, val) in props_map {
                let node_id = self.parse(val)?;
                properties.push((make_key(key), node_id));
            }
        }

        // additionalProperties
        if let Some(additional) = value.get("additionalProperties") {
            additional_properties = match additional {
                Value::Bool(true) => AdditionalProperties::True,
                Value::Bool(false) => AdditionalProperties::False,
                _ => AdditionalProperties::Schema(self.parse(additional)?),
            };
        }

        let node_id = self.add_node(SchemaNode::Object(ObjectConstraints::new(
            properties.into_boxed_slice(),
            additional_properties,
        )));
        Ok(node_id)
    }

    fn parse_array(&mut self, value: &Value) -> Result<SchemaNodeId, SchemaParseError> {
        // Parse `items` if present, otherwise anything is accepted.
        let items = if let Some(items_schema) = value.get("items") {
            self.parse(items_schema)?
        } else {
            self.add_node(SchemaNode::Any)
        };
        Ok(self.add_node(SchemaNode::Array(ArrayConstraints::new(items))))
    }

    fn parse_primitive_string(&mut self) -> Result<SchemaNodeId, SchemaParseError> {
        Ok(self.add_node(SchemaNode::Str))
    }

    fn parse_primitive_number(&mut self) -> Result<SchemaNodeId, SchemaParseError> {
        Ok(self.add_node(SchemaNode::Number))
    }

    fn parse_primitive_boolean(&mut self) -> Result<SchemaNodeId, SchemaParseError> {
        Ok(self.add_node(SchemaNode::Boolean))
    }

    fn parse_primitive_null(&mut self) -> Result<SchemaNodeId, SchemaParseError> {
        Ok(self.add_node(SchemaNode::Null))
    }

    fn resolve_ref(&mut self, ref_str: &str) -> Result<SchemaNodeId, SchemaParseError> {
        let ref_name = get_schema_name_from_ref(ref_str)?;
        let def_id = self
            .defs
            .get(&make_key(ref_name))
            .ok_or_else(|| SchemaParseError::UndefinedType(ref_name.to_string()))?;
        Ok(*def_id)
    }
}

/// Parse a JSON Schema definition from string. Ignores or fails on unsupported keywords.
///
/// ## Errors
/// Fails if the string is not a valid JSON, or is not a valid JSON Schema.
pub fn parse_schema(schema_str: &str) -> Result<SchemaAutomaton, SchemaParseError> {
    let schema: Value = serde_json::from_str(schema_str).map_err(SchemaParseError::SerdeError)?;

    // Schema can be `true` (accepts everything), `false` (accepts nothing) or an object.
    if let Value::Bool(true) = schema {
        return Ok(SchemaAutomaton::new(vec![SchemaNode::Any], SchemaNodeId(0)));
    } else if let Value::Bool(false) = schema {
        return Err(SchemaParseError::UnsupportedKeyword("false".into()));
    }

    if !matches!(schema, Value::Object(_)) {
        return Err(SchemaParseError::InvalidSchema(
            "Schema must be an object or a boolean".into(),
        ));
    }

    let mut parser = SchemaParser::new();

    // First, parse all type definitions in $defs.
    if let Some(defs_obj) = schema.get("$defs").and_then(|d| d.as_object()) {
        // In the first pass, reserve ids for all $defs types to allow for references.
        parser.defs.reserve(defs_obj.len());
        for type_name in defs_obj.keys() {
            let key = make_key(type_name);
            if parser.defs.contains_key(&key) {
                Err(SchemaParseError::DuplicateTypeDef(type_name.clone()))?
            }
            // Insert a placeholder, will be replaced with actual node after parsing.
            let def_id = parser.add_node(SchemaNode::Null);
            parser.defs.insert(key, def_id);
        }

        // In the second pass, parse all types in $defs and update their nodes.
        for (def_name, def_schema) in defs_obj {
            let node_id = parser.parse(def_schema)?;
            let def_node_id = *parser.defs.get(&make_key(def_name)).expect("Definition id must exist.");
            parser.nodes[def_node_id.0 as usize] = parser.nodes.swap_remove(node_id.0 as usize);
        }
    }

    // Parse the root schema
    let root = parser.parse(&schema)?;

    Ok(SchemaAutomaton::new(parser.nodes, root))
}

/// Extract schema name from a reference string in $ref.
/// Works only for local references in format "#/$defs/TypeName".
fn get_schema_name_from_ref(ref_str: &str) -> Result<&str, SchemaParseError> {
    if ref_str.starts_with("#/$defs/") {
        Ok(&ref_str["#/$defs/".len()..])
    } else {
        Err(SchemaParseError::UnsupportedKeyword(format!(
            "Only local references in format '#/$defs/TypeName' are supported, got '{}'",
            ref_str
        )))
    }
}

/// Create a StringPattern from a string literal.
fn make_key(s: &str) -> StringPattern {
    StringPattern::from(JsonString::new(s))
}

/// Errors raised by the schema parser.
#[derive(Debug, Error)]
pub enum SchemaParseError {
    /// Error parsing JSON with serde.
    #[error("error parsing JSON with serde: {0}")]
    SerdeError(#[from] serde_json::Error),
    /// Unsupported JSON Schema keyword.
    #[error("unsupported JSON Schema keyword: {0}")]
    UnsupportedKeyword(String),
    /// Duplicate type name in $defs.
    #[error("duplicate type name in $defs: '{0}'")]
    DuplicateTypeDef(String),
    /// Referenced type could not be found in $defs.
    #[error("referenced type not found in $defs: {0}")]
    UndefinedType(String),
    /// Other invalid schema structure.
    #[error("{0}")]
    InvalidSchema(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_schema_ok(schema: &str) -> SchemaAutomaton {
        parse_schema(schema).expect("expected schema to parse")
    }

    #[test]
    fn parse_string_schema() {
        let definition = parse_schema_ok(r#"{"type":"string"}"#);

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);
        assert!(matches!(definition.nodes()[0], SchemaNode::Str));
    }

    #[test]
    fn parse_true_schema() {
        let definition = parse_schema_ok("true");

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);
        assert!(matches!(definition.nodes()[0], SchemaNode::Any));
    }

    #[test]
    fn parse_object_schema() {
        let definition = parse_schema_ok(
            r#"{
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "age": {"type": "number"}
                },
                "additionalProperties": false
            }"#,
        );

        assert_eq!(definition.root(), SchemaNodeId(2));
        assert_eq!(definition.nodes().len(), 3);

        assert!(matches!(definition.nodes()[0], SchemaNode::Number));
        assert!(matches!(definition.nodes()[1], SchemaNode::Str));

        let SchemaNode::Object(object) = &definition.nodes()[2] else {
            panic!("expected object node");
        };
        assert_eq!(object.properties().get(&make_key("name")), Some(&SchemaNodeId(1)));
        assert_eq!(object.properties().get(&make_key("age")), Some(&SchemaNodeId(0)));
        assert!(matches!(object.additional_properties(), AdditionalProperties::False));
    }

    #[test]
    fn parse_additional_properties_schema() {
        let definition = parse_schema_ok(
            r#"{
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                },
                "additionalProperties": {"type": "object"}
            }"#,
        );

        assert_eq!(definition.root(), SchemaNodeId(2));
        assert_eq!(definition.nodes().len(), 3);

        assert!(matches!(definition.nodes()[0], SchemaNode::Str));
        assert!(matches!(definition.nodes()[1], SchemaNode::Object(_)));

        let SchemaNode::Object(object) = &definition.nodes()[2] else {
            panic!("expected object node");
        };

        assert_eq!(object.properties().get(&make_key("name")), Some(&SchemaNodeId(0)));
        assert!(matches!(
            object.additional_properties(),
            AdditionalProperties::Schema(SchemaNodeId(1))
        ));
    }

    #[test]
    fn parse_recursive_definition() {
        let definition = parse_schema_ok(
            r##"{
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
            }"##,
        );

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);

        let SchemaNode::Object(object) = &definition.nodes()[0] else {
            panic!("expected object node");
        };
        assert_eq!(object.properties().get(&make_key("next")), Some(&SchemaNodeId(0)));
        assert!(matches!(object.additional_properties(), AdditionalProperties::False));
    }

    #[test]
    fn parse_nested_objects() {
        let definition = parse_schema_ok(
            r#"{
                "type": "object",
                "properties": {
                    "child": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"}
                        },
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }"#,
        );

        assert_eq!(definition.root(), SchemaNodeId(2));
        assert_eq!(definition.nodes().len(), 3);

        let SchemaNode::Object(root_object) = &definition.nodes()[2] else {
            panic!("expected root to be an object node");
        };
        assert_eq!(root_object.properties().get(&make_key("child")), Some(&SchemaNodeId(1)));
        assert!(matches!(
            root_object.additional_properties(),
            AdditionalProperties::False
        ));

        let SchemaNode::Object(child_object) = &definition.nodes()[1] else {
            panic!("expected child to be an object node");
        };
        assert_eq!(child_object.properties().get(&make_key("name")), Some(&SchemaNodeId(0)));
        assert!(matches!(
            child_object.additional_properties(),
            AdditionalProperties::False
        ));
    }

    #[test]
    fn parse_array_schema() {
        let definition = parse_schema_ok(r#"{"type":"array"}"#);

        // `items` is absent so any value is accepted for elements.
        assert_eq!(definition.root(), SchemaNodeId(1));
        assert_eq!(definition.nodes().len(), 2);
        assert!(matches!(definition.nodes()[0], SchemaNode::Any));
        let SchemaNode::Array(array) = &definition.nodes()[1] else {
            panic!("expected array node");
        };
        assert_eq!(array.items(), SchemaNodeId(0));
    }

    #[test]
    fn parse_array_items_schema() {
        let definition = parse_schema_ok(r#"{"type":"array","items":{"type":"string"}}"#);

        assert_eq!(definition.root(), SchemaNodeId(1));
        assert_eq!(definition.nodes().len(), 2);
        assert!(matches!(definition.nodes()[0], SchemaNode::Str));
        let SchemaNode::Array(array) = &definition.nodes()[1] else {
            panic!("expected array node");
        };
        assert_eq!(array.items(), SchemaNodeId(0));
    }

    #[test]
    fn reject_undefined_reference() {
        let error = parse_schema(r##"{"$ref":"#/$defs/Node"}"##).expect_err("expected schema parsing to fail");

        assert!(matches!(error, SchemaParseError::UndefinedType(name) if name == "Node"));
    }

    #[test]
    fn parse_str_schema() {
        let definition = parse_schema_ok(r#"{"type":"string"}"#);

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);
        assert!(matches!(definition.nodes()[0], SchemaNode::Str));
    }

    #[test]
    fn parse_number_schema() {
        let definition = parse_schema_ok(r#"{"type":"number"}"#);

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);
        assert!(matches!(definition.nodes()[0], SchemaNode::Number));
    }

    #[test]
    fn parse_boolean_schema() {
        let definition = parse_schema_ok(r#"{"type":"boolean"}"#);

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);
        assert!(matches!(definition.nodes()[0], SchemaNode::Boolean));
    }

    #[test]
    fn parse_null_schema() {
        let definition = parse_schema_ok(r#"{"type":"null"}"#);

        assert_eq!(definition.root(), SchemaNodeId(0));
        assert_eq!(definition.nodes().len(), 1);
        assert!(matches!(definition.nodes()[0], SchemaNode::Null));
    }

    #[test]
    fn parse_multiple_types() {
        let definition = parse_schema_ok(r#"{"type": ["string", "number", "object"]}"#);

        assert_eq!(definition.root(), SchemaNodeId(3));
        assert_eq!(definition.nodes().len(), 4);
        assert!(matches!(definition.nodes()[0], SchemaNode::Str));
        assert!(matches!(definition.nodes()[1], SchemaNode::Number));
        assert!(matches!(definition.nodes()[2], SchemaNode::Object(_)));
        let SchemaNode::Type(type_constraints) = &definition.nodes()[3] else {
            panic!("expected type node");
        };
        assert_eq!(
            type_constraints.types(),
            &[SchemaNodeId(0), SchemaNodeId(1), SchemaNodeId(2)]
        );
    }
}
