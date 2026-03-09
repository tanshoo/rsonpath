use crate::engine::error::EngineError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorError {
    #[error(transparent)]
    Engine(#[from] EngineError),

    #[error("Invalid label at position {0}")]
    InvalidLabel(usize),
}
