use crate::framework::implementation::Implementation;
use rsonpath::{input::OwnedBytes, validator::ValidatorEngine};
use std::{fs, io};
use thiserror::Error;

pub struct Rsonschema {}

impl Implementation for Rsonschema {
    type Query = ValidatorEngine;

    type File = OwnedBytes<Vec<u8>>;

    type Error = RsonschemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "rsonschema"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(Rsonschema {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::read_to_string(file_path)?;
        let input = OwnedBytes::new(file.into_bytes());

        Ok(input)
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let schema_str = fs::read_to_string(schema_file_path)?;
        let engine = ValidatorEngine::compile_schema(&schema_str)?;

        Ok(engine)
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        query.validate(file)?;
        Ok("[validated]")
    }
}

#[derive(Error, Debug)]
pub enum RsonschemaError {
    #[error(transparent)]
    SchemaParserError(#[from] rsonpath::validator::schema_parser::SchemaParseError),
    #[error(transparent)]
    ValidatorEngineError(#[from] rsonpath::validator::ValidatorEngineError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("something happened")]
    Unknown(),
}
