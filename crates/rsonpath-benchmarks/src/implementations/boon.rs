use crate::framework::implementation::Implementation;
use boon::{Compiler, SchemaIndex, Schemas};
use serde_json::Value;
use std::{
    fs,
    io::{self, BufReader},
};
use thiserror::Error;

pub struct Boon;

pub struct BoonSchema {
    schema: Schemas,
    index: SchemaIndex,
}

impl Implementation for Boon {
    type Query = BoonSchema;

    type File = Value;

    type Error = BoonError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "boon"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(Boon {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::File::open(file_path)?;
        let reader = BufReader::new(file);
        let value: Value = serde_json::from_reader(reader)?;

        Ok(value)
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let mut schemas = Schemas::new();
        let mut compiler = Compiler::new();
        let schema_index = compiler
            .compile(schema_file_path, &mut schemas)
            .map_err(|e| BoonError::BoonCompileError(e.to_string()))?;

        Ok(BoonSchema {
            schema: schemas,
            index: schema_index,
        })
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        query
            .schema
            .validate(file, query.index)
            .map_err(|e| BoonError::BoonValidationError(e.to_string()))?;

        Ok("[validated]")
    }
}

#[derive(Error, Debug)]
pub enum BoonError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("error parsing JSON with serde: '{0}'")]
    SerdeError(#[from] serde_json::Error),
    #[error("BoonCompileError: {0}")]
    BoonCompileError(String),
    #[error("BoonValidationError: {0}")]
    BoonValidationError(String),
}
