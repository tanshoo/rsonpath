use crate::framework::implementation::Implementation;
use rsonpath::{
    input::{MmapInput, OwnedBytes},
    validator::{well_formedness::ActiveWellFormednessCheck, ValidatorEngine},
};
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

pub struct RsonschemaWithWellFormedness {}

impl Implementation for RsonschemaWithWellFormedness {
    type Query = ValidatorEngine<ActiveWellFormednessCheck>;

    type File = OwnedBytes<Vec<u8>>;

    type Error = RsonschemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "rsonschema_with_well_formedness"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(RsonschemaWithWellFormedness {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::read_to_string(file_path)?;
        let input = OwnedBytes::new(file.into_bytes());

        Ok(input)
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let schema_str = fs::read_to_string(schema_file_path)?;
        let engine =
            ValidatorEngine::compile_schema(&schema_str)?.with_well_formedness_check(ActiveWellFormednessCheck::new());

        Ok(engine)
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        query.validate(file)?;
        Ok("[validated]")
    }
}

pub struct RsonschemaWellFormednessOnly {}

impl Implementation for RsonschemaWellFormednessOnly {
    type Query = ValidatorEngine<ActiveWellFormednessCheck>;

    type File = OwnedBytes<Vec<u8>>;

    type Error = RsonschemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "rsonschema_well_formedness_only"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(RsonschemaWellFormednessOnly {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::read_to_string(file_path)?;
        let input = OwnedBytes::new(file.into_bytes());

        Ok(input)
    }

    fn compile_query(&self, _schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let engine =
            ValidatorEngine::compile_schema("true")?.with_well_formedness_check(ActiveWellFormednessCheck::new());

        Ok(engine)
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        query.validate(file)?;
        Ok("[validated]")
    }
}

pub struct RsonschemaMmap {}

impl Implementation for RsonschemaMmap {
    type Query = ValidatorEngine;

    type File = MmapInput;

    type Error = RsonschemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "rsonschema_mmap"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(RsonschemaMmap {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::File::open(file_path)?;
        let input = unsafe { MmapInput::map_file(&file)? };

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

pub struct RsonschemaMmapWithWellFormedness {}

impl Implementation for RsonschemaMmapWithWellFormedness {
    type Query = ValidatorEngine<ActiveWellFormednessCheck>;

    type File = MmapInput;

    type Error = RsonschemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "rsonschema_mmap_with_well_formedness"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(RsonschemaMmapWithWellFormedness {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::File::open(file_path)?;
        let input = unsafe { MmapInput::map_file(&file)? };

        Ok(input)
    }

    fn compile_query(&self, schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let schema_str = fs::read_to_string(schema_file_path)?;
        let engine =
            ValidatorEngine::compile_schema(&schema_str)?.with_well_formedness_check(ActiveWellFormednessCheck::new());

        Ok(engine)
    }

    fn run(&self, query: &Self::Query, file: &Self::File) -> Result<Self::Result<'_>, Self::Error> {
        query.validate(file)?;
        Ok("[validated]")
    }
}

pub struct RsonschemaMmapWellFormednessOnly {}

impl Implementation for RsonschemaMmapWellFormednessOnly {
    type Query = ValidatorEngine<ActiveWellFormednessCheck>;

    type File = MmapInput;

    type Error = RsonschemaError;

    type Result<'a> = &'static str;

    fn id() -> &'static str {
        "rsonschema_mmap_well_formedness_only"
    }

    fn new() -> Result<Self, Self::Error> {
        Ok(RsonschemaMmapWellFormednessOnly {})
    }

    fn load_file(&self, file_path: &str) -> Result<Self::File, Self::Error> {
        let file = fs::File::open(file_path)?;
        let input = unsafe { MmapInput::map_file(&file)? };

        Ok(input)
    }

    fn compile_query(&self, _schema_file_path: &str) -> Result<Self::Query, Self::Error> {
        let engine =
            ValidatorEngine::compile_schema("true")?.with_well_formedness_check(ActiveWellFormednessCheck::new());

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
    InputError(#[from] rsonpath::input::error::InputError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("something happened")]
    Unknown(),
}
