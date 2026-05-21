//! JSON Schema parser.
use crate::string_pattern::StringPattern;
use rsonpath_syntax::str::JsonString;
use serde_json::Value;
use smallvec::SmallVec;
use std::fmt::Display;
use std::{collections::HashMap, hash::Hash};
use thiserror::Error;

/// Identifier of a [`SchemaNode`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SchemaNodeId(pub(crate) u32);

impl Display for SchemaNodeId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SchemaNodeId({})", self.0)
    }
}

/// JSON Schema constraints node.
///
/// Every schema is a graph of these nodes,
/// each representing a constraint on the JSON structure.
#[derive(Debug, Clone)]
pub enum SchemaNode {
    /// Any value is accepted.
    Any,
    /// Primitive type (null, boolean, number, integer, string).
    Primitive,
    /// Object type.
    Object(ObjectConstraints),
    /// Array type.
    Array,
    /// Logical OR on multiple schema nodes.
    Or(Vec<SchemaNodeId>),
}

impl SchemaNode {
    /// Check if the node is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, SchemaNode::Primitive)
    }
}

/// JSON Schema object constraints.
#[derive(Debug, Clone)]
pub struct ObjectConstraints {
    /// Maps property names to their corresponding schema nodes.
    properties: HashMap<StringPattern, SchemaNodeId>,
    additional_properties: AdditionalProperties,
}

/// Possible values for "additionalProperties" in JSON Schema.
#[derive(Debug, Clone, Default)]
pub enum AdditionalProperties {
    /// Any additional properties allowed.
    /// Default value when "additionalProperties" is not specified.
    #[default]
    True,
    /// No additional properties allowed.
    False,
    /// Additional properties must validate against the given schema.
    Schema(SchemaNodeId),
}

impl ObjectConstraints {
    /// Get properties.
    #[inline]
    pub fn properties(&self) -> &HashMap<StringPattern, SchemaNodeId> {
        &self.properties
    }

    /// Get additional properties policy.
    #[inline]
    pub fn additional_properties(&self) -> &AdditionalProperties {
        &self.additional_properties
    }
}

/// JSON Schema definition representation.
#[derive(Debug, Clone)]
pub struct JsonSchemaDefinition {
    /// All nodes in the schema graph.
    nodes: Vec<SchemaNode>,
    /// Root node index.
    root: SchemaNodeId,
}

impl JsonSchemaDefinition {
    /// Get all nodes in the schema graph.
    #[inline]
    pub fn nodes(&self) -> &[SchemaNode] {
        &self.nodes
    }

    /// Get root node index.
    #[inline]
    pub fn root(&self) -> SchemaNodeId {
        self.root
    }

    /// Get node by id.
    #[inline]
    pub fn node(&self, id: SchemaNodeId) -> Option<&SchemaNode> {
        self.nodes.get(id.0 as usize)
    }
}

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
                "array" => constraints.push(self.parse_array()?),
                _ => constraints.push(self.parse_primitive()?),
            }
        }

        if constraints.len() == 1 {
            Ok(constraints[0])
        } else {
            Ok(self.add_node(SchemaNode::Or(constraints)))
        }
    }

    fn parse_object(&mut self, value: &Value) -> Result<SchemaNodeId, SchemaParseError> {
        let mut properties = HashMap::new();
        let mut additional_properties = AdditionalProperties::default();

        // properties
        if let Some(props_map) = value.get("properties").and_then(|p| p.as_object()) {
            properties.reserve(props_map.len());
            for (key, val) in props_map {
                let node_id = self.parse(val)?;
                properties.insert(make_key(key), node_id);
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

        let node_id = self.add_node(SchemaNode::Object(ObjectConstraints {
            properties,
            additional_properties,
        }));
        Ok(node_id)
    }

    fn parse_array(&mut self) -> Result<SchemaNodeId, SchemaParseError> {
        todo!()
    }

    fn parse_primitive(&mut self) -> Result<SchemaNodeId, SchemaParseError> {
        Ok(self.add_node(SchemaNode::Primitive))
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
pub fn parse_schema(schema_str: &str) -> Result<JsonSchemaDefinition, SchemaParseError> {
    let schema: Value = serde_json::from_str(schema_str).map_err(SchemaParseError::SerdeError)?;

    // Schema can be `true` (accepts everything), `false` (accepts nothing) or an object.
    if let Value::Bool(true) = schema {
        return Ok(JsonSchemaDefinition {
            nodes: vec![SchemaNode::Any],
            root: SchemaNodeId(0),
        });
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
            let def_id = parser.add_node(SchemaNode::Primitive);
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

    Ok(JsonSchemaDefinition {
        nodes: parser.nodes,
        root,
    })
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

    fn parse_schema_ok(schema: &str) -> JsonSchemaDefinition {
        parse_schema(schema).expect("expected schema to parse")
    }

    #[test]
    fn parse_string_schema() {
        let definition = parse_schema_ok(r#"{"type":"string"}"#);

        assert_eq!(definition.root, SchemaNodeId(0));
        assert_eq!(definition.nodes.len(), 1);
        assert!(matches!(definition.nodes[0], SchemaNode::Primitive));
    }

    #[test]
    fn parse_true_schema() {
        let definition = parse_schema_ok("true");

        assert_eq!(definition.root, SchemaNodeId(0));
        assert_eq!(definition.nodes.len(), 1);
        assert!(matches!(definition.nodes[0], SchemaNode::Any));
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

        assert_eq!(definition.root, SchemaNodeId(2));
        assert_eq!(definition.nodes.len(), 3);

        assert!(matches!(definition.nodes[0], SchemaNode::Primitive));
        assert!(matches!(definition.nodes[1], SchemaNode::Primitive));

        let SchemaNode::Object(object) = &definition.nodes[2] else {
            panic!("expected object node");
        };
        assert_eq!(object.properties.get(&make_key("name")), Some(&SchemaNodeId(1)));
        assert_eq!(object.properties.get(&make_key("age")), Some(&SchemaNodeId(0)));
        assert!(matches!(object.additional_properties, AdditionalProperties::False));
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

        assert_eq!(definition.root, SchemaNodeId(2));
        assert_eq!(definition.nodes.len(), 3);

        assert!(matches!(definition.nodes[0], SchemaNode::Primitive));
        assert!(matches!(definition.nodes[1], SchemaNode::Object(_)));

        let SchemaNode::Object(object) = &definition.nodes[2] else {
            panic!("expected object node");
        };

        assert_eq!(object.properties.get(&make_key("name")), Some(&SchemaNodeId(0)));
        assert!(matches!(
            object.additional_properties,
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

        assert_eq!(definition.root, SchemaNodeId(0));
        assert_eq!(definition.nodes.len(), 1);

        let SchemaNode::Object(object) = &definition.nodes[0] else {
            panic!("expected object node");
        };
        assert_eq!(object.properties.get(&make_key("next")), Some(&SchemaNodeId(0)));
        assert!(matches!(object.additional_properties, AdditionalProperties::False));
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

        assert_eq!(definition.root, SchemaNodeId(2));
        assert_eq!(definition.nodes.len(), 3);

        let SchemaNode::Object(root_object) = &definition.nodes[2] else {
            panic!("expected root to be an object node");
        };
        assert_eq!(root_object.properties.get(&make_key("child")), Some(&SchemaNodeId(1)));
        assert!(matches!(root_object.additional_properties, AdditionalProperties::False));

        let SchemaNode::Object(child_object) = &definition.nodes[1] else {
            panic!("expected child to be an object node");
        };
        assert_eq!(child_object.properties.get(&make_key("name")), Some(&SchemaNodeId(0)));
        assert!(matches!(
            child_object.additional_properties,
            AdditionalProperties::False
        ));
    }

    #[test]
    fn parse_multiple_types() {
        let definition = parse_schema_ok(r#"{"type": ["object", "string"]}"#);

        assert_eq!(definition.root, SchemaNodeId(2));
        assert_eq!(definition.nodes.len(), 3);
        assert!(matches!(definition.nodes[0], SchemaNode::Object(_)));
        assert!(matches!(definition.nodes[1], SchemaNode::Primitive));
        assert!(matches!(&definition.nodes[2], SchemaNode::Or(types) if types == &[SchemaNodeId(0), SchemaNodeId(1)]));
    }

    #[test]
    fn reject_undefined_reference() {
        let error = parse_schema(r##"{"$ref":"#/$defs/Node"}"##).expect_err("expected schema parsing to fail");

        assert!(matches!(error, SchemaParseError::UndefinedType(name) if name == "Node"));
    }
}
