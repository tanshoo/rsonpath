use crate::framework::implementation::Implementation;
use std::{
    io,
    process::{Command, Stdio},
};
use thiserror::Error;

pub struct Dja {}

impl Dja {
    const VALIDATOR_PATH: &'static str = "./src/implementations/dja-bin/bench";
}

impl Implementation for Dja {
    // Path to the JSON Schema file
    type Query = String;
    // Path to the JSON document file
    type File = String;

    type Error = DjaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "DJA"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(Dja {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        Ok(file_path.to_string())
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        Ok(schema_file_path.to_string())
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        // ./bench <schema> <document> tokenizer
        let status = Command::new(Self::VALIDATOR_PATH)
            .arg(query)
            .arg(file)
            .arg("tokenizer")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if status.success() {
            Ok("[validated]")
        } else {
            Err(DjaError::DJAProcessError(format!(
                "DJA `bench` binary (validator) failed with status: {}",
                status
            )))
        }
    }
}

#[derive(Error, Debug)]
pub enum DjaError {
    #[error("DJA process error: {0}")]
    DJAProcessError(String),
    #[error(transparent)]
    IoError(#[from] io::Error),
}
