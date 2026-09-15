//! JSON Schema parser.
use crate::string_pattern::StringPattern;
use crate::validator::schema_automaton::{
    ArrayConstraints, JsonType, ObjectConstraints, SchemaAutomaton, SchemaNode, SchemaNodeId, TypeConstraints,
};
use rsonpath_syntax::{num::JsonUInt, str::JsonString};
use serde_json::Value;
use smallvec::SmallVec;
use std::collections::HashMap;
use thiserror::Error;

/// JSON Schema definition parser.
/// The parser assumes that the input schema is a well-formed JSON.
#[derive(Debug)]
pub struct SchemaParser {
    /// All nodes in the schema graph.
    nodes: Vec<SchemaNode>,
    /// Maps type names in $defs to their corresponding nodes.
    defs: HashMap<StringPattern, SchemaNodeId>,
    /// Node representing the `true` schema (accepts everything).
    true_schema: SchemaNodeId,
}

impl SchemaParser {
    const DEFAULT_TYPES: [&str; 6] = ["null", "boolean", "object", "array", "number", "string"];

    /// Create a new parser.
    #[inline]
    fn new() -> Self {
        let mut parser = Self {
            // Dummy node, so that all used node ids are non-zero.
            nodes: vec![SchemaNode::Null],
            defs: HashMap::new(),
            true_schema: SchemaNodeId::new(1), // temporary
        };

        parser.true_schema = parser.add_true_schema();
        parser
    }

    fn add_true_schema(&mut self) -> SchemaNodeId {
        let true_schema = self.reserve_node();
        let atomic_true_schema = self.add_node(SchemaNode::Null);
        let obj_true_schema = self.reserve_node();
        let arr_true_schema = self.reserve_node();

        let mut type_constr = [None; 6];
        type_constr[JsonType::Object as usize] = Some(obj_true_schema);
        type_constr[JsonType::Array as usize] = Some(arr_true_schema);
        // Atomic types (string, number, boolean, null) share the same node.
        for &t in &[JsonType::String, JsonType::Number, JsonType::Boolean, JsonType::Null] {
            type_constr[t as usize] = Some(atomic_true_schema);
        }
        self.nodes[true_schema.as_usize()] = SchemaNode::Type(TypeConstraints::new(type_constr));

        self.nodes[obj_true_schema.as_usize()] =
            SchemaNode::Object(ObjectConstraints::new(Box::new([]), Some(true_schema), None, None));

        self.nodes[arr_true_schema.as_usize()] = SchemaNode::Array(ArrayConstraints::new(true_schema, None, None));

        true_schema
    }

    /// Add a new schema node and return its id.
    #[inline]
    fn add_node(&mut self, node: SchemaNode) -> SchemaNodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        SchemaNodeId::new(id)
    }

    /// Insert a placeholder node and return its id.
    #[inline]
    fn reserve_node(&mut self) -> SchemaNodeId {
        let id = self.nodes.len();
        self.nodes.push(SchemaNode::Null);
        SchemaNodeId::new(id)
    }

    #[inline]
    fn copy_node(&mut self, dest: Option<SchemaNodeId>, source: SchemaNodeId) -> SchemaNodeId {
        if let Some(dest_id) = dest {
            self.nodes[dest_id.as_usize()] = self.nodes[source.as_usize()].clone();
            dest_id
        } else {
            source
        }
    }

    fn parse(&mut self, value: &Value, dest: Option<SchemaNodeId>) -> Result<SchemaNodeId, SchemaParseError> {
        if let Value::Bool(true) = value {
            return Ok(self.copy_node(dest, self.true_schema));
        } else if let Value::Bool(false) = value {
            return Err(SchemaParseError::UnsupportedKeyword("false"));
        }

        // Check for $ref.
        if let Some(ref_str) = value.get("$ref").and_then(|v| v.as_str()) {
            let ref_id = self.resolve_ref(ref_str)?;
            return Ok(self.copy_node(dest, ref_id));
        }

        let types: SmallVec<[&str; 7]> = match value.get("type") {
            Some(Value::String(s)) => SmallVec::from_slice(&[s.as_str()]),
            Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str()).collect(),
            _ => SmallVec::from_slice(&Self::DEFAULT_TYPES),
        };

        let mut constraints = [None; 6];

        for type_str in types {
            match type_str {
                "object" => constraints[JsonType::Object as usize] = Some(self.parse_object(value)?),
                "array" => constraints[JsonType::Array as usize] = Some(self.parse_array(value)?),
                "string" => constraints[JsonType::String as usize] = Some(self.parse_primitive_string()?),
                "number" | "integer" => constraints[JsonType::Number as usize] = Some(self.parse_primitive_number()?),
                "boolean" => constraints[JsonType::Boolean as usize] = Some(self.parse_primitive_boolean()?),
                _ => constraints[JsonType::Null as usize] = Some(self.parse_primitive_null()?),
            }
        }

        let node = SchemaNode::Type(TypeConstraints::new(constraints));
        Ok(if let Some(dest_id) = dest {
            self.nodes[dest_id.as_usize()] = node;
            dest_id
        } else {
            self.add_node(node)
        })
    }

    fn parse_object(&mut self, value: &Value) -> Result<SchemaNodeId, SchemaParseError> {
        let mut properties = Vec::new();

        // properties
        if let Some(props_map) = value.get("properties").and_then(|p| p.as_object()) {
            properties.reserve(props_map.len());
            for (key, val) in props_map {
                let node_id = self.parse(val, None)?;
                properties.push((make_key(key), node_id));
            }
        }

        let additional_properties = match value.get("additionalProperties") {
            Some(Value::Bool(true)) | None => Some(self.true_schema),
            Some(Value::Bool(false)) => None,
            Some(other) => Some(self.parse(other, None)?),
        };

        let min_properties = self.parse_uint_field(value, "minProperties");
        let max_properties = self.parse_uint_field(value, "maxProperties");

        let node_id = self.add_node(SchemaNode::Object(ObjectConstraints::new(
            properties.into_boxed_slice(),
            additional_properties,
            min_properties,
            max_properties,
        )));
        Ok(node_id)
    }

    fn parse_array(&mut self, value: &Value) -> Result<SchemaNodeId, SchemaParseError> {
        // Parse `items` if present, otherwise anything is accepted.
        let items = value
            .get("items")
            .map_or(Ok(self.true_schema), |i| self.parse(i, None))?;
        let min_items = self.parse_uint_field(value, "minItems");
        let max_items = self.parse_uint_field(value, "maxItems");

        Ok(self.add_node(SchemaNode::Array(ArrayConstraints::new(items, min_items, max_items))))
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

    #[inline]
    fn parse_uint_field(&self, value: &Value, key: &str) -> Option<JsonUInt> {
        value
            .get(key)
            .and_then(|v| v.as_u64())
            .and_then(|u| JsonUInt::try_from(u).ok())
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
        let parser = SchemaParser::new();
        return Ok(SchemaAutomaton::new(parser.nodes, parser.true_schema));
    } else if let Value::Bool(false) = schema {
        return Err(SchemaParseError::UnsupportedKeyword("false"));
    }

    let mut parser = SchemaParser::new();

    // First, parse all type definitions in $defs.
    if let Some(defs_obj) = schema.get("$defs").and_then(|d| d.as_object()) {
        // In the first pass, reserve ids for all $defs types to allow for references.
        let mut def_schemas_to_parse = Vec::with_capacity(defs_obj.len());
        parser.defs.reserve(defs_obj.len());

        for (type_name, def_schema) in defs_obj {
            // Insert a placeholder, will be replaced with actual node after parsing.
            let def_id = parser.reserve_node();
            if parser.defs.insert(make_key(type_name), def_id).is_some() {
                return Err(SchemaParseError::DuplicateTypeDef(type_name.clone()));
            }
            def_schemas_to_parse.push((def_schema, def_id));
        }

        // In the second pass, parse all types in $defs and update their nodes.
        for (def_schema, def_node_id) in def_schemas_to_parse {
            parser.parse(def_schema, Some(def_node_id))?;
        }
    }

    // Parse the root schema
    let root = parser.parse(&schema, None)?;

    Ok(SchemaAutomaton::new(parser.nodes, root))
}

/// Extract schema name from a reference string in $ref.
/// Works only for local references in format "#/$defs/TypeName".
fn get_schema_name_from_ref(ref_str: &str) -> Result<&str, SchemaParseError> {
    ref_str
        .strip_prefix("#/$defs/")
        .ok_or(SchemaParseError::UnsupportedKeyword(
            "Only local references in format '#/$defs/TypeName' are supported",
        ))
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
    UnsupportedKeyword(&'static str),
    /// Duplicate type name in $defs.
    #[error("duplicate type name in $defs: '{0}'")]
    DuplicateTypeDef(String),
    /// Referenced type could not be found in $defs.
    #[error("referenced type not found in $defs: {0}")]
    UndefinedType(String),
    /// Other invalid schema structure.
    #[error("{0}")]
    InvalidSchema(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_schema_ok(schema: &str) -> SchemaAutomaton {
        parse_schema(schema).expect("expected schema to parse")
    }

    fn get_property<'a>(properties: &'a [(StringPattern, SchemaNodeId)], name: &str) -> Option<&'a SchemaNodeId> {
        properties.iter().find(|(k, _)| k == &make_key(name)).map(|(_, v)| v)
    }

    fn get_type_node<'a>(definition: &'a SchemaAutomaton, id: SchemaNodeId, json_type: JsonType) -> SchemaNodeId {
        let SchemaNode::Type(type_constraints) = &definition[id] else {
            panic!("expected Type node, found {:?}", &definition[id]);
        };
        type_constraints[json_type].expect("expected type constraint")
    }

    fn get_object_node<'a>(definition: &'a SchemaAutomaton, id: SchemaNodeId) -> &'a ObjectConstraints {
        let SchemaNode::Object(object) = &definition[id] else {
            panic!("expected Object node, found {:?}", &definition[id]);
        };
        object
    }

    fn get_array_node<'a>(definition: &'a SchemaAutomaton, id: SchemaNodeId) -> &'a ArrayConstraints {
        let SchemaNode::Array(array) = &definition[id] else {
            panic!("expected Array node, found {:?}", &definition[id]);
        };
        array
    }

    #[test]
    fn parse_string_schema() {
        let definition = parse_schema_ok(r#"{"type":"string"}"#);

        let root_id = definition.root();
        let str_id = get_type_node(&definition, root_id, JsonType::String);
        assert!(matches!(definition[str_id], SchemaNode::Str));
    }

    #[test]
    fn parse_true_schema() {
        let definition = parse_schema_ok("true");

        let root_id = definition.root();
        assert!(matches!(definition[root_id], SchemaNode::Type(_)));
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

        let root_id = definition.root();
        let obj_id = get_type_node(&definition, root_id, JsonType::Object);
        let object = get_object_node(&definition, obj_id);

        let name_prop_id = *get_property(object.properties(), "name").unwrap();
        let name_str_id = get_type_node(&definition, name_prop_id, JsonType::String);
        assert!(matches!(definition[name_str_id], SchemaNode::Str));

        let age_prop_id = *get_property(object.properties(), "age").unwrap();
        let age_num_id = get_type_node(&definition, age_prop_id, JsonType::Number);
        assert!(matches!(definition[age_num_id], SchemaNode::Number));

        assert!(object.additional_properties().is_none());
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

        let root_id = definition.root();
        let obj_id = get_type_node(&definition, root_id, JsonType::Object);
        let object = get_object_node(&definition, obj_id);

        let name_prop_id = *get_property(object.properties(), "name").unwrap();
        let name_str_id = get_type_node(&definition, name_prop_id, JsonType::String);
        assert!(matches!(definition[name_str_id], SchemaNode::Str));

        let add_id = object.additional_properties().unwrap();
        let SchemaNode::Type(add_type) = &definition[add_id] else {
            panic!("expected type node")
        };
        let add_obj_id = add_type[JsonType::Object].expect("expected object type");
        get_object_node(&definition, add_obj_id);
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

        let root_id = definition.root();
        let obj_id = get_type_node(&definition, root_id, JsonType::Object);
        let object = get_object_node(&definition, obj_id);

        let next_prop_id = *get_property(object.properties(), "next").unwrap();
        assert_eq!(next_prop_id, root_id);

        assert!(object.additional_properties().is_none());
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

        let root_id = definition.root();
        let root_obj_id = get_type_node(&definition, root_id, JsonType::Object);
        let root_object = get_object_node(&definition, root_obj_id);
        assert!(root_object.additional_properties().is_none());

        let child_prop_id = *get_property(root_object.properties(), "child").unwrap();
        let child_obj_id = get_type_node(&definition, child_prop_id, JsonType::Object);
        let child_object = get_object_node(&definition, child_obj_id);
        assert!(child_object.additional_properties().is_none());

        let name_prop_id = *get_property(child_object.properties(), "name").unwrap();
        let name_str_id = get_type_node(&definition, name_prop_id, JsonType::String);
        assert!(matches!(definition[name_str_id], SchemaNode::Str));
    }

    #[test]
    fn parse_array_schema() {
        let definition = parse_schema_ok(r#"{"type":"array"}"#);

        let root_id = definition.root();
        let arr_id = get_type_node(&definition, root_id, JsonType::Array);
        let array = get_array_node(&definition, arr_id);

        assert!(matches!(definition[array.items()], SchemaNode::Type(_)));
    }

    #[test]
    fn parse_array_items_schema() {
        let definition = parse_schema_ok(r#"{"type":"array","items":{"type":"string"}}"#);

        let root_id = definition.root();
        let arr_id = get_type_node(&definition, root_id, JsonType::Array);
        let array = get_array_node(&definition, arr_id);

        let items_id = array.items();
        let str_id = get_type_node(&definition, items_id, JsonType::String);
        assert!(matches!(definition[str_id], SchemaNode::Str));
    }

    #[test]
    fn reject_undefined_reference() {
        let error = parse_schema(r##"{"$ref":"#/$defs/Node"}"##).expect_err("expected schema parsing to fail");

        assert!(matches!(error, SchemaParseError::UndefinedType(name) if name == "Node"));
    }

    #[test]
    fn parse_str_schema() {
        let definition = parse_schema_ok(r#"{"type":"string"}"#);

        let root_id = definition.root();
        let str_id = get_type_node(&definition, root_id, JsonType::String);
        assert!(matches!(definition[str_id], SchemaNode::Str));
    }

    #[test]
    fn parse_number_schema() {
        let definition = parse_schema_ok(r#"{"type":"number"}"#);

        let root_id = definition.root();
        let num_id = get_type_node(&definition, root_id, JsonType::Number);
        assert!(matches!(definition[num_id], SchemaNode::Number));
    }

    #[test]
    fn parse_boolean_schema() {
        let definition = parse_schema_ok(r#"{"type":"boolean"}"#);

        let root_id = definition.root();
        let bool_id = get_type_node(&definition, root_id, JsonType::Boolean);
        assert!(matches!(definition[bool_id], SchemaNode::Boolean));
    }

    #[test]
    fn parse_null_schema() {
        let definition = parse_schema_ok(r#"{"type":"null"}"#);

        let root_id = definition.root();
        let null_id = get_type_node(&definition, root_id, JsonType::Null);
        assert!(matches!(definition[null_id], SchemaNode::Null));
    }

    #[test]
    fn parse_multiple_types() {
        let definition = parse_schema_ok(r#"{"type": ["string", "number", "object"]}"#);

        let root_id = definition.root();
        let SchemaNode::Type(type_constraints) = &definition[root_id] else {
            panic!("expected type node");
        };

        let str_id = type_constraints[JsonType::String].unwrap();
        assert!(matches!(definition[str_id], SchemaNode::Str));

        let num_id = type_constraints[JsonType::Number].unwrap();
        assert!(matches!(definition[num_id], SchemaNode::Number));

        let obj_id = type_constraints[JsonType::Object].unwrap();
        assert!(matches!(definition[obj_id], SchemaNode::Object(_)));
    }
}
