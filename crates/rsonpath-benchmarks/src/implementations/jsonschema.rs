use crate::framework::implementation::Implementation;
use jsonschema::{self, ValidationError, Validator};
use serde_json::Value;
use std::{
    fs,
    io::{self, BufReader},
};
use thiserror::Error;

pub struct JsonSchema;

impl Implementation for JsonSchema {
    type Query = Validator;

    type File = Value;

    type Error = JsonSchemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "jsonschema"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(JsonSchema {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::File::open(file_path)?;
        let reader = BufReader::new(file);
        let value: Value = serde_json::from_reader(reader)?;

        Ok(value)
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let schema_file = fs::File::open(schema_file_path)?;
        let reader = BufReader::new(schema_file);
        let schema: Value = serde_json::from_reader(reader)?;
        let validator = jsonschema::validator_for(&schema)?;

        Ok(validator)
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        let _ = query.is_valid(file);

        Ok("[validated]")
    }
}

#[derive(Error, Debug)]
pub enum JsonSchemaError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("error parsing JSON with serde: '{0}'")]
    SerdeError(#[from] serde_json::Error),
    #[error("error compiling JSON schema: '{0}'")]
    ValidationError(#[from] ValidationError<'static>),
}
