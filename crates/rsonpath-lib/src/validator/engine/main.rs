//! Main implementation of a JSON Schema validator engine.

use crate::{
    classification::{
        simd::{self, config_simd, dispatch_simd, Simd, SimdConfiguration},
        structural::{Structural, StructuralIterator as _},
    },
    debug,
    engine::error::EngineError as RsonpathEngineError,
    input::error::InputErrorConvertible as _,
    input::Input,
    result::empty::EmptyRecorder,
    string_pattern::StringPattern,
    validator::engine::error::ValidatorEngineError,
    FallibleIterator as _, BLOCK_SIZE,
};

/// Main engine for a fixed JSON Schema schema.
///
/// The engine is stateless, meaning that it can be executed
/// on any number of separate inputs, even on separate threads.
#[derive(Clone, Debug)]
pub struct ValidatorEngine {
    property_names: Vec<StringPattern>,
    simd: SimdConfiguration,
}

impl ValidatorEngine {
    /// Creates a new engine.
    pub fn new(property_names: Vec<StringPattern>) -> Self {
        Self {
            property_names,
            simd: simd::configure(),
        }
    }

    /// Validate the input against the schema.
    /// Currently only validates that all property names in the input
    /// belong to the allowed property names.
    pub fn validate<I>(&self, input: &I) -> Result<(), ValidatorEngineError>
    where
        I: Input,
    {
        config_simd!(self.simd => |simd| {
            let executor = Executor::new(&self.property_names, input, simd);
            executor.run_and_exit()
        })?;

        Ok(())
    }
}

type Classifier<'i, I, V> =
    <V as Simd>::StructuralClassifier<'i, <I as Input>::BlockIterator<'i, 'static, EmptyRecorder, BLOCK_SIZE>>;

/// This is the heart of an Engine run that holds the entire execution state.
struct Executor<'i, I, V> {
    /// Allowed property names.
    property_names: &'i Vec<StringPattern>,
    /// Handle to the input.
    input: &'i I,
    /// Resolved SIMD context.
    simd: V,
}

impl<'i, I, V> Executor<'i, I, V>
where
    I: Input,
    V: Simd,
{
    fn new(property_names: &'i Vec<StringPattern>, input: &'i I, simd: V) -> Self {
        Self {
            property_names,
            input,
            simd,
        }
    }

    /// One-shot run of the engine on whatever JSON tree starts at the current input.
    /// The engine exits when input ends.
    fn run_and_exit(mut self) -> Result<(), ValidatorEngineError> {
        let iter = self.input.iter_blocks(&EmptyRecorder);
        let quote_classifier = self.simd.classify_quoted_sequences(iter);
        let structural_classifier = self.simd.classify_structural_characters(quote_classifier);
        let mut classifier = structural_classifier;
        classifier.turn_colons_on(0);

        self.run(&mut classifier)
    }

    /// Main loop of the engine.
    /// We loop through the document based on the `classifier`'s outputs.
    /// Currently, the only event we handle is the colon (which indicates a property name).
    /// Once the input ends, the engine exits.
    fn run(&mut self, classifier: &mut Classifier<'i, I, V>) -> Result<(), ValidatorEngineError> {
        dispatch_simd!(self.simd; self, classifier =>
        fn<'i, I, V>(
            eng: &mut Executor<'i, I, V>,
            classifier: &mut Classifier<'i, I, V>
        ) -> Result<(), ValidatorEngineError>
        where
            I: Input,
            V: Simd
        {
            loop {
                let mut next_event = match classifier.next() {
                    Ok(e) => e,
                    Err(err) => return Err(ValidatorEngineError::InputError(err)),
                };
                if let Some(event) = next_event.take() {
                    debug!("Event: {:?}", event);

                    match event {
                        Structural::Colon(idx) => eng.handle_colon(idx)?,
                        _ => {}
                    }
                } else {
                    break;
                }
            }

            Ok(())
        })
    }

    /// Handle a colon at index `idx`.
    /// Validates the property name that precedes the colon.
    #[inline(always)]
    fn handle_colon(&mut self, idx: usize) -> Result<(), ValidatorEngineError> {
        debug!("Colon");

        // Check if property name is valid
        match self.is_property_name_allowed(idx) {
            Ok(true) => Ok(()),
            Ok(false) => Err(ValidatorEngineError::DisallowedProperty(idx)),
            Err(err) => Err(err),
        }
    }

    /// Check if the property name that precedes the colon at index `idx`
    /// belongs to the allowed property names.
    #[inline(always)]
    fn is_property_name_allowed(&self, idx: usize) -> Result<bool, ValidatorEngineError> {
        // The colon can be preceded by whitespace before the actual label.
        let closing_quote_idx = match self.input.seek_backward(idx - 1, b'"') {
            Some(x) => x,
            None => {
                return Err(ValidatorEngineError::RsonpathEngineError(
                    RsonpathEngineError::MalformedStringQuotes(idx - 1),
                ))
            }
        };

        for property_name in self.property_names {
            let len = property_name.quoted().len();

            // First check if the length matches.
            if closing_quote_idx + 1 < len {
                continue;
            }

            // Do the expensive memcmp.
            let start_idx = closing_quote_idx + 1 - len;
            if self
                .input
                .is_member_match(start_idx, closing_quote_idx + 1, property_name)
                .e()?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
