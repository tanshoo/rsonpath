//! Main implementation of a JSON Schema validator engine.
use crate::classification::structural::BracketType;
use crate::error::DepthError;
use crate::validator::schema_automaton::{AdditionalProperties, SchemaAutomaton, SchemaNode, SchemaNodeId};
use crate::validator::schema_parser::{parse_schema, SchemaParseError};
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
    validator::engine::error::ValidatorEngineError,
    FallibleIterator as _, BLOCK_SIZE,
};
use rsonpath_syntax::num::JsonUInt;
use smallvec::{smallvec, SmallVec};

/// Main engine for a fixed JSON Schema schema.
///
/// The engine is stateless, meaning that it can be executed
/// on any number of separate inputs, even on separate threads.
#[derive(Clone, Debug)]
pub struct ValidatorEngine {
    schema: SchemaAutomaton,
    simd: SimdConfiguration,
}

impl ValidatorEngine {
    /// Compile a JSON Schema from a string into an [`ValidatorEngine`].
    pub fn compile_schema(schema_str: &str) -> Result<Self, SchemaParseError> {
        let schema = parse_schema(schema_str)?;
        let simd = simd::configure();
        Ok(Self { schema, simd })
    }

    /// Turn a compiled [`SchemaAutomaton`] into a [`ValidatorEngine`].
    pub fn from_compiled_schema(schema: SchemaAutomaton) -> Self {
        let simd = simd::configure();
        Self { schema, simd }
    }

    /// Validate the input against the schema.
    pub fn validate<I>(&self, input: &I) -> Result<(), ValidatorEngineError>
    where
        I: Input,
    {
        config_simd!(self.simd => |simd| {
            let executor = Executor::new(&self.schema, input, simd);
            executor.run_and_exit()
        })?;

        Ok(())
    }
}

// This is a convenience macro to hide the type of the classifier.
// It expects generic types `I` (the Input implementation) and `V` (the SIMD context).
macro_rules! Classifier {
    () => {
        <V as Simd>::StructuralClassifier<'i, <I as Input>::BlockIterator<'i, 'static, EmptyRecorder, BLOCK_SIZE>>
    };
}

/// This is the heart of an Engine run that holds the entire execution state.
struct Executor<'i, I, V> {
    /// Current schema node.
    state: SchemaNodeId,
    /// Next schema node.
    next_state: SchemaNodeId,
    /// Is current subtree an array
    is_array: bool,
    /// Count of seen properties/elements in the current subtree
    count: JsonUInt,
    /// Execution stack.
    stack: SmallStack,

    /// Read-only access to the schema automaton.
    schema: &'i SchemaAutomaton,
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
    fn new(schema: &'i SchemaAutomaton, input: &'i I, simd: V) -> Self {
        Self {
            state: schema.root(),
            next_state: schema.root(),
            is_array: false,
            count: JsonUInt::ZERO,
            stack: SmallStack::new(),
            schema,
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
        classifier.turn_colons_and_commas_on(0);

        if let Some((idx, c)) = self.input.seek_non_whitespace_forward(0).e()? {
            self.next_state = self.transition_on_type(idx, c, &self.schema[self.state])?;
        }

        self.run(&mut classifier)?;

        self.verify_subtree_closed()
    }

    /// Main loop of the engine.
    /// We loop through the document based on the `classifier`'s outputs.
    fn run(&mut self, classifier: &mut Classifier!()) -> Result<(), ValidatorEngineError> {
        dispatch_simd!(self.simd; self, classifier =>
        fn<'i, I, V>(
            eng: &mut Executor<'i, I, V>,
            classifier: &mut Classifier!()

        ) -> Result<(), ValidatorEngineError>
        where
            I: Input,
            V: Simd
        {
            loop {
                if let Some(event) = classifier.next().map_err(ValidatorEngineError::InputError)?.take() {
                    debug!("====================");
                    debug!("Event = {:?}", event);
                    debug!("Depth = {:?}", eng.stack.contents.len());
                    debug!("State = {:?}", eng.state);
                    debug!("====================");

                    match event {
                        Structural::Colon(idx) => eng.handle_colon(idx)?,
                        Structural::Comma(idx) => eng.handle_comma(idx)?,
                        Structural::Opening(b, idx) => eng.handle_opening(b, idx)?,
                        Structural::Closing(_, idx) => eng.handle_closing(idx)?,
                    }
                } else {
                    break;
                }
            }

            Ok(())
        })
    }

    /// Handle a colon at index `idx`.
    /// Finds the next state to transition to based on the object's properties.
    /// Transitions into objects and arrays are processed at their respective opening character.
    #[inline(always)]
    fn handle_colon(&mut self, idx: usize) -> Result<(), ValidatorEngineError> {
        debug!("Colon");

        // Check object properties to find a matching transition.
        self.next_state = self.find_transition_on_object_property(idx)?;
        if self.count.try_increment().is_err() {
            return Ok(());
        }
        // Check if the value following the colon matches the expected type.
        if let Some((new_idx, c)) = self.input.seek_non_whitespace_forward(idx + 1).e()? {
            self.next_state = self.transition_on_type(new_idx, c, &self.schema[self.next_state])?;
        }
        Ok(())
    }

    /// Handle a comma at index `idx`.
    /// Only used for arrays, where it finds the next state to transition to based on the array's items.
    /// Transitions into objects and arrays are processed at their respective opening character.
    #[inline(always)]
    fn handle_comma(&mut self, idx: usize) -> Result<(), ValidatorEngineError> {
        debug!("Comma");

        if self.is_array {
            if let SchemaNode::Array(arr) = &self.schema[self.state] {
                // The only possible transition from an array node is to its items schema node.
                self.next_state = arr.items();
                if self.count.try_increment().is_err() {
                    return Ok(());
                }
                // Check if the value following the comma matches the expected type.
                if let Some((new_idx, c)) = self.input.seek_non_whitespace_forward(idx + 1).e()? {
                    self.next_state = self.transition_on_type(new_idx, c, &self.schema[self.next_state])?;
                }
            }
        }
        // No need to process commas in objects, as they are used only to find where atomic values end.
        Ok(())
    }

    /// Handle the opening of a subtree with given `bracket_type` at index `idx`.
    #[inline(always)]
    fn handle_opening(&mut self, bracket_type: BracketType, idx: usize) -> Result<(), ValidatorEngineError> {
        debug!("Opening {bracket_type:?} and pushing stack.",);

        self.transition_to_next(bracket_type);

        // We need to validate the first element of the array here, as there is no comma before it.
        if self.is_array {
            if let SchemaNode::Array(arr) = &self.schema[self.state] {
                // The only possible transition from an array node is to its items schema node.
                self.next_state = arr.items();
                if let Some((new_idx, c)) = self.input.seek_non_whitespace_forward(idx + 1).e()? {
                    if c == b']' {
                        return Ok(());
                    }
                    if self.count.try_increment().is_err() {
                        return Ok(());
                    }
                    // Check if the value following the comma matches the expected type.
                    self.next_state = self.transition_on_type(new_idx, c, &self.schema[self.next_state])?;
                }
            }
        }
        Ok(())
    }

    /// Handle the closing of a subtree at index `idx`.
    #[inline(always)]
    fn handle_closing(&mut self, idx: usize) -> Result<(), ValidatorEngineError> {
        debug!("Closing and popping stack.");

        // Restore the state from the stack.
        let frame = self.stack.pop().ok_or_else(|| {
            ValidatorEngineError::RsonpathEngineError(RsonpathEngineError::DepthBelowZero(idx, DepthError::BelowZero))
        })?;
        self.state = frame.state;
        self.is_array = frame.is_array;
        self.count = frame.count;

        Ok(())
    }

    /// Find the schema transition for the object property whose name
    /// precedes the colon at index `idx`. Return the target state if found.
    ///
    /// Errors:
    /// - [`ValidatorEngineError::DisallowedProperty`] if the property
    ///   is not allowed by `properties` and `additionalProperties`.
    /// - [`ValidatorEngineError::TypeMismatch`] if the current schema node
    ///   is not an object.
    fn find_transition_on_object_property(&self, idx: usize) -> Result<SchemaNodeId, ValidatorEngineError> {
        let obj_state = &self.schema[self.state];

        // The colon can be preceded by whitespace before the actual label.
        let closing_quote_idx = match self.input.seek_backward(idx - 1, b'"') {
            Some(x) => x,
            None => {
                return Err(ValidatorEngineError::RsonpathEngineError(
                    RsonpathEngineError::MalformedStringQuotes(idx - 1),
                ))
            }
        };

        // Iterate over all property names in the current schema node.
        if let SchemaNode::Object(obj) = obj_state {
            for (property_name, target_state) in obj.properties() {
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
                    return Ok(*target_state);
                }
            }
            // Property name not found in `properties`, check `additionalProperties` policy.
            match obj.additional_properties() {
                AdditionalProperties::False => return Err(ValidatorEngineError::DisallowedProperty(idx)),
                AdditionalProperties::Schema(target_state) => {
                    return Ok(*target_state);
                }
                AdditionalProperties::True => {
                    return Err(ValidatorEngineError::UnsupportedFeature("additionalProperties: true"));
                }
            }
        }

        Err(ValidatorEngineError::TypeMismatch(idx))
    }

    /// Trigger the transition to the `next_state` into a new subtree.
    fn transition_to_next(&mut self, opening: BracketType) {
        self.stack.push(StackFrame {
            state: self.state,
            is_array: self.is_array,
            count: self.count,
        });
        self.state = self.next_state;
        self.is_array = opening == BracketType::Square;
        self.count = JsonUInt::ZERO;
    }

    /// Validate that the value starting at index `idx` with character `c`,
    /// matches the type expected by `node`.
    #[inline(always)]
    fn validate_type(&self, c: u8, node: &SchemaNode) -> bool {
        matches!(
            (node, c),
            (SchemaNode::Object(_), b'{')
                | (SchemaNode::Array(_), b'[')
                | (SchemaNode::Str, b'"')
                | (SchemaNode::Number, b'-' | b'0'..=b'9')
                | (SchemaNode::Boolean, b't' | b'f')
                | (SchemaNode::Null, b'n')
        )
    }

    #[inline(always)]
    fn transition_on_type(
        &mut self,
        idx: usize,
        c: u8,
        node: &SchemaNode,
    ) -> Result<SchemaNodeId, ValidatorEngineError> {
        if let SchemaNode::Type(types) = node {
            for &target_state in types.types() {
                let target_node = &self.schema[target_state];
                if self.validate_type(c, target_node) {
                    return Ok(target_state);
                }
            }
        }
        Err(ValidatorEngineError::TypeMismatch(idx))
    }

    /// Verify that we have reached zero depth, raise an error if not.
    fn verify_subtree_closed(&mut self) -> Result<(), ValidatorEngineError> {
        if self.stack.peek().is_some() {
            Err(ValidatorEngineError::RsonpathEngineError(
                RsonpathEngineError::MissingClosingCharacter(),
            ))
        } else {
            Ok(())
        }
    }
}

/// A single frame on the [`Executor`]'s stack enabling restoration
/// of the execution state after a subtree is processed.
#[derive(Clone, Copy, Debug)]
struct StackFrame {
    state: SchemaNodeId,
    is_array: bool,
    count: JsonUInt,
}

#[derive(Debug)]
struct SmallStack {
    contents: SmallVec<[StackFrame; 128]>,
}

impl SmallStack {
    fn new() -> Self {
        Self { contents: smallvec![] }
    }

    #[inline]
    fn peek(&self) -> Option<StackFrame> {
        self.contents.last().copied()
    }

    #[inline]
    fn pop(&mut self) -> Option<StackFrame> {
        self.contents.pop()
    }

    #[inline]
    fn push(&mut self, value: StackFrame) {
        self.contents.push(value)
    }
}
