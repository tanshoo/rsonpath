use crate::framework::implementation::Implementation;
use std::{
    io,
    process::{Command, Stdio},
};
use thiserror::Error;

// Measures process spawn overhead.
// Used as a baseline to subtract from timings of CLI utilities (like the DJA benchmark),
// to better estimate the actual process execution time.
pub struct SpawnBaseline {}

impl Implementation for SpawnBaseline {
    type Query = String;

    type File = String;

    type Error = SpawnBaselineError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "spawn-baseline"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(SpawnBaseline {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        Ok(file_path.to_string())
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        Ok(schema_file_path.to_string())
    }

    fn run(&self, _query: &Self::Query, _file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        let status = Command::new("true")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if status.success() {
            Ok("[]")
        } else {
            Err(SpawnBaselineError::ProcessError(format!(
                "Spawn baseline (`true`) failed with status: {}",
                status
            )))
        }
    }
}

#[derive(Error, Debug)]
pub enum SpawnBaselineError {
    #[error("spawn baseline process error: {0}")]
    ProcessError(String),
    #[error(transparent)]
    IoError(#[from] io::Error),
}
