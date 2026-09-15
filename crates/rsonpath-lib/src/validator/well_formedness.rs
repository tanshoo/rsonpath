//! Defines the [`WellFormednessCheck`] trait and its implementations for checking
//! the well-formedness of JSON input during validation.
use crate::classification::structural::{BracketType, Structural};
use crate::input::Input;
use crate::validator::engine::error::ValidatorEngineError;
use smallvec::{smallvec, SmallVec};

/// Trait that decides if the engine will additionally check if the JSON is structurally correct.
pub trait WellFormednessCheck: Clone {
    /// Whether this check requires all structural events to be emitted.
    const NEEDS_ALL_STRUCTURAL_EVENTS: bool;

    /// Handle the opening of a subtree with given `bracket_type` at index `idx`.
    fn on_opening<I: Input>(
        &mut self,
        bracket_type: BracketType,
        idx: usize,
        input: &I,
    ) -> Result<(), ValidatorEngineError>;

    /// Handle the closing of a subtree with given `bracket_type` at index `idx`.
    fn on_closing<I: Input>(
        &mut self,
        bracket_type: BracketType,
        idx: usize,
        input: &I,
    ) -> Result<(), ValidatorEngineError>;

    /// Handle a colon at index `idx`.
    fn on_colon<I: Input>(&mut self, idx: usize, input: &I) -> Result<(), ValidatorEngineError>;

    /// Handle a comma at index `idx`.
    fn on_comma<I: Input>(&mut self, idx: usize, input: &I) -> Result<(), ValidatorEngineError>;

    /// Update the index of an atomic value found by the engine to avoid doubling work
    /// when checking for atomic values.
    fn update_last_atomic(&mut self, idx: usize);
}

/// No well-formedness check is performed. All functions are no-ops and get compiled out.
/// Used when the input JSON is assumed to be well-formed.
#[derive(Debug, Clone)]
pub struct NoWellFormednessCheck;

impl WellFormednessCheck for NoWellFormednessCheck {
    const NEEDS_ALL_STRUCTURAL_EVENTS: bool = false;

    #[inline(always)]
    fn on_opening<I: Input>(
        &mut self,
        _bracket_type: BracketType,
        _idx: usize,
        _input: &I,
    ) -> Result<(), ValidatorEngineError> {
        Ok(())
    }

    #[inline(always)]
    fn on_closing<I: Input>(
        &mut self,
        _bracket_type: BracketType,
        _idx: usize,
        _input: &I,
    ) -> Result<(), ValidatorEngineError> {
        Ok(())
    }

    #[inline(always)]
    fn on_colon<I: Input>(&mut self, _idx: usize, _input: &I) -> Result<(), ValidatorEngineError> {
        Ok(())
    }

    #[inline(always)]
    fn on_comma<I: Input>(&mut self, _idx: usize, _input: &I) -> Result<(), ValidatorEngineError> {
        Ok(())
    }

    #[inline(always)]
    fn update_last_atomic(&mut self, _idx: usize) {}
}

/// Performs a well-formedness check on the JSON input along with the main engine execution.
#[derive(Debug, Clone)]
pub struct ActiveWellFormednessCheck {
    /// Last structural event encountered during validation.
    last_structural: Option<Structural>,
    /// Index of the last potentially atomic value found by the engine.
    last_atomic_idx: usize,
    /// Stack of currently open brackets.
    bracket_stack: SmallStack,
}

impl Default for ActiveWellFormednessCheck {
    fn default() -> Self {
        Self {
            last_structural: None,
            last_atomic_idx: usize::MAX,
            bracket_stack: SmallStack::new(),
        }
    }
}

impl ActiveWellFormednessCheck {
    /// Create a new [`ActiveWellFormednessCheck`] with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there is a non-whitespace character between the previous
    /// and the current structural character at index `idx`,
    /// which would indicate the presence of an atomic value.
    #[inline(always)]
    fn has_atomic<I: Input>(&self, idx: usize, input: &I) -> Result<bool, ValidatorEngineError> {
        let next_atomic_search_start = self.last_structural.map_or(0, |s| s.idx() + 1);

        if next_atomic_search_start == idx {
            return Ok(false);
        }

        if next_atomic_search_start <= self.last_atomic_idx && self.last_atomic_idx < idx {
            return Ok(true);
        }

        match input
            .seek_non_whitespace_forward(next_atomic_search_start)
            .map_err(|e| ValidatorEngineError::InputError(e.into()))?
        {
            Some((non_ws_idx, _)) => Ok(non_ws_idx < idx),
            None => Ok(false),
        }
    }

    #[inline(always)]
    fn update(&mut self, event: Structural) {
        self.last_structural = Some(event);
    }
}

impl WellFormednessCheck for ActiveWellFormednessCheck {
    const NEEDS_ALL_STRUCTURAL_EVENTS: bool = true;

    #[inline(always)]
    fn on_opening<I: Input>(
        &mut self,
        bracket_type: BracketType,
        idx: usize,
        input: &I,
    ) -> Result<(), ValidatorEngineError> {
        // There is no state where an atomic value is allowed before an opening bracket.
        if self.has_atomic(idx, input)? {
            return Err(ValidatorEngineError::UnexpectedStructural(idx));
        }

        match (self.bracket_stack.peek(), self.last_structural) {
            (Some(BracketType::Square), Some(Structural::Comma(_)))
            | (_, Some(Structural::Colon(_) | Structural::Opening(BracketType::Square, _)) | None) => {}
            _ => return Err(ValidatorEngineError::UnexpectedStructural(idx)),
        }

        self.update(Structural::Opening(bracket_type, idx));
        self.bracket_stack.push(bracket_type);
        Ok(())
    }

    #[inline(always)]
    fn on_closing<I: Input>(
        &mut self,
        bracket_type: BracketType,
        idx: usize,
        input: &I,
    ) -> Result<(), ValidatorEngineError> {
        if self.bracket_stack.peek() != Some(bracket_type) {
            return Err(ValidatorEngineError::UnexpectedStructural(idx));
        }

        let has_atomic_val = self.has_atomic(idx, input)?;
        match (bracket_type, self.last_structural) {
            // States that require an atomic value before the closing bracket.
            (BracketType::Curly, Some(Structural::Colon(_))) | (BracketType::Square, Some(Structural::Comma(_))) => {
                if !has_atomic_val {
                    return Err(ValidatorEngineError::UnexpectedStructural(idx));
                }
            }
            // States that forbid an atomic value before the closing bracket.
            (BracketType::Curly, Some(Structural::Opening(BracketType::Curly, _)))
            | (_, Some(Structural::Closing(_, _))) => {
                if has_atomic_val {
                    return Err(ValidatorEngineError::UnexpectedStructural(idx));
                }
            }
            // States where an atomic value is optional before the closing bracket.
            (BracketType::Square, Some(Structural::Opening(BracketType::Square, _))) => {}
            _ => return Err(ValidatorEngineError::UnexpectedStructural(idx)),
        }

        self.update(Structural::Closing(bracket_type, idx));
        self.bracket_stack.pop();
        Ok(())
    }

    #[inline(always)]
    fn on_colon<I: Input>(&mut self, idx: usize, _input: &I) -> Result<(), ValidatorEngineError> {
        match (self.bracket_stack.peek(), self.last_structural) {
            (Some(BracketType::Curly), Some(Structural::Opening(BracketType::Curly, _) | Structural::Comma(_))) => {}
            _ => return Err(ValidatorEngineError::UnexpectedStructural(idx)),
        }

        self.update(Structural::Colon(idx));
        Ok(())
    }

    #[inline(always)]
    fn on_comma<I: Input>(&mut self, idx: usize, input: &I) -> Result<(), ValidatorEngineError> {
        let last_open_bracket = self.bracket_stack.peek();

        let has_atomic_val = self.has_atomic(idx, input)?;
        match (last_open_bracket, self.last_structural) {
            // States that require an atomic value before the comma.
            (Some(BracketType::Curly), Some(Structural::Colon(_)))
            | (Some(BracketType::Square), Some(Structural::Opening(BracketType::Square, _) | Structural::Comma(_))) => {
                if !has_atomic_val {
                    return Err(ValidatorEngineError::UnexpectedStructural(idx));
                }
            }
            // States that forbid an atomic value before the comma.
            (_, Some(Structural::Closing(_, _))) => {
                if has_atomic_val {
                    return Err(ValidatorEngineError::UnexpectedStructural(idx));
                }
            }
            _ => return Err(ValidatorEngineError::UnexpectedStructural(idx)),
        }

        self.update(Structural::Comma(idx));
        Ok(())
    }

    #[inline(always)]
    fn update_last_atomic(&mut self, idx: usize) {
        self.last_atomic_idx = idx;
    }
}

#[derive(Debug, Clone)]
struct SmallStack {
    contents: SmallVec<[BracketType; 128]>,
}

impl SmallStack {
    fn new() -> Self {
        Self { contents: smallvec![] }
    }

    #[inline]
    fn peek(&self) -> Option<BracketType> {
        self.contents.last().copied()
    }

    #[inline]
    fn pop(&mut self) -> Option<BracketType> {
        self.contents.pop()
    }

    #[inline]
    fn push(&mut self, value: BracketType) {
        self.contents.push(value)
    }
}
