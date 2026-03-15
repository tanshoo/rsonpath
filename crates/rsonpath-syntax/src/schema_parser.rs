//! Basic JSON Schema parser
use crate::str::JsonString;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    io::{self, BufReader},
};
use thiserror::Error;

/// Errors raised during schema parsing.
#[derive(Debug, Error)]
pub enum SchemaParserError {
    /// Error parsing JSON with serde.
    #[error("error parsing JSON with serde: '{0}'")]
    SerdeError(#[from] serde_json::Error),
    /// Error when reading input from an underlying IO handle.
    #[error(transparent)]
    IoError(#[from] io::Error),
}

/// Get all unique property names from the schema.
fn get_property_names(schema_file_path: &str) -> Result<Vec<JsonString>, SchemaParserError> {
    let file = fs::File::open(schema_file_path)?;
    let reader = BufReader::new(file);
    let value: Value = serde_json::from_reader(reader)?;
    let mut property_names_set = HashSet::new();
    get_property_names_recursive(&value, &mut property_names_set);
    let mut result: Vec<_> = property_names_set.into_iter().map(|s| JsonString::new(&s)).collect();
    result.sort_by(|a, b| a.quoted().cmp(b.quoted()));
    Ok(result)
}

// Recurse into the schema to find all property names.
fn get_property_names_recursive(value: &Value, property_names: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                if key == "properties" {
                    if let Some(props) = val.as_object() {
                        for (prop_key, prop_val) in props {
                            property_names.insert(prop_key.clone());
                            get_property_names_recursive(prop_val, property_names);
                        }
                    }
                } else {
                    get_property_names_recursive(val, property_names);
                }
            }
        }
        Value::Array(arr) => {
            for val in arr {
                get_property_names_recursive(val, property_names);
            }
        }
        _ => {}
    }
}

/// Returns all unique property names from the schema.
pub fn parse(schema_file_path: &str) -> Result<Vec<JsonString>, SchemaParserError> {
    get_property_names(schema_file_path)
}
