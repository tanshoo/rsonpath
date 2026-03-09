use crate::classification::structural::StructuralIterator;
use crate::{
    classification::{
        simd::{self, config_simd, dispatch_simd, Simd, SimdConfiguration},
        structural::Structural,
    },
    debug,
    engine::error::EngineError,
    input::Input,
    result::empty::EmptyRecorder,
    string_pattern::StringPattern,
    validator::error::ValidatorError,
    FallibleIterator, BLOCK_SIZE,
};

#[derive(Clone, Debug)]
pub struct ValidatorEngine {
    properties: Vec<StringPattern>,
    simd: SimdConfiguration,
}

impl ValidatorEngine {
    pub fn new(properties: Vec<StringPattern>) -> Self {
        Self {
            properties,
            simd: simd::configure(),
        }
    }

    pub fn validate<I>(&self, input: &I) -> Result<(), ValidatorError>
    where
        I: Input,
    {
        config_simd!(self.simd => |simd| {
            let executor = Executor::new(&self.properties, input, simd);
            executor.run_and_exit()
        })?;

        Ok(())
    }
}

type Classifier<'i, I, V> =
    <V as Simd>::StructuralClassifier<'i, <I as Input>::BlockIterator<'i, 'static, EmptyRecorder, BLOCK_SIZE>>;

struct Executor<'i, I, V> {
    properties: &'i [StringPattern],
    input: &'i I,
    simd: V,
}

impl<'i, I, V> Executor<'i, I, V>
where
    I: Input,
    V: Simd,
{
    fn new(properties: &'i [StringPattern], input: &'i I, simd: V) -> Self {
        Self {
            properties,
            input,
            simd,
        }
    }

    fn run_and_exit(mut self) -> Result<(), ValidatorError> {
        let iter = self.input.iter_blocks(&EmptyRecorder);
        let quote_classifier = self.simd.classify_quoted_sequences(iter);
        let structural_classifier = self.simd.classify_structural_characters(quote_classifier);
        let mut classifier = structural_classifier;
        classifier.turn_colons_on(0);

        self.run(&mut classifier)
    }

    fn run(&mut self, classifier: &mut Classifier<'i, I, V>) -> Result<(), ValidatorError> {
        dispatch_simd!(self.simd; self, classifier =>
        fn<'i, I, V>(
            eng: &mut Executor<'i, I, V>,
            classifier: &mut Classifier<'i, I, V>
        ) -> Result<(), ValidatorError>
        where
            I: Input,
            V: Simd
        {
            loop {
                let mut next_event = match classifier.next() {
                    Ok(e) => e,
                    Err(err) => return Err(EngineError::InputError(err).into()),
                };
                if let Some(event) = next_event.take() {
                    debug!("Event: {:?}", event);

                    match event {
                        Structural::Colon(idx) => {
                            eng.handle_colon(idx)?;
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }

            Ok(())
        })
    }

    #[inline(always)]
    fn handle_colon(&mut self, idx: usize) -> Result<(), ValidatorError> {
        debug!("Colon");

        // Check if label is valid
        match self.is_in_properties(idx) {
            Ok(true) => Ok(()),
            Ok(false) => Err(ValidatorError::InvalidLabel(idx)),
            Err(e) => Err(e.into()),
        }
    }

    #[inline(always)]
    fn is_in_properties(&self, idx: usize) -> Result<bool, EngineError> {
        // The colon can be preceded by whitespace before the actual label.
        let closing_quote_idx = match self.input.seek_backward(idx - 1, b'"') {
            Some(x) => x,
            None => return Err(EngineError::MalformedStringQuotes(idx - 1)),
        };

        for label in self.properties {
            let len = label.quoted().len();

            // First check if the length matches.
            if closing_quote_idx + 1 < len {
                continue;
            }

            // Do the expensive memcmp.
            let start_idx = closing_quote_idx + 1 - len;
            match self.input.is_member_match(start_idx, closing_quote_idx + 1, label) {
                Ok(true) => {
                    return Ok(true);
                }
                Ok(false) => continue,
                Err(e) => return Err(e.into().into()),
            }
        }
        Ok(false)
    }
}
