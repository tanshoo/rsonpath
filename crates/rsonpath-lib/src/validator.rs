//! JSON Schema validator.
pub mod engine;
pub mod schema_automaton;
pub mod schema_parser;
pub mod well_formedness;

pub use engine::error::ValidatorEngineError;
pub use engine::ValidatorEngine;
pub use schema_parser::SchemaParser;
