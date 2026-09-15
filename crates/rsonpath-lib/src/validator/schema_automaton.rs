//! Automaton representation of a JSON Schema.
use crate::string_pattern::StringPattern;
use rsonpath_syntax::num::JsonUInt;
use std::{fmt::Display, num::NonZeroU32, ops::Index};

/// Identifier of a [`SchemaNode`].
/// it is a non-zero type, so that niche optimization
/// can be used for [`Option<SchemaNodeId>`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SchemaNodeId(pub(crate) NonZeroU32);

impl SchemaNodeId {
    #[inline(always)]
    pub(crate) fn new(id: usize) -> Self {
        Self(NonZeroU32::new(id as u32).expect("SchemaNodeId cannot be 0"))
    }

    #[inline(always)]
    pub(crate) fn as_usize(self) -> usize {
        self.0.get() as usize
    }
}

impl Display for SchemaNodeId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SchemaNodeId({})", self.as_usize())
    }
}

/// JSON Schema constraints node.
///
/// Every schema is a graph of these nodes,
/// each representing validator/applicator keywords
/// applicable at a given point.
#[derive(Debug, Clone)]
pub enum SchemaNode {
    /// Object type.
    Object(ObjectConstraints),
    /// Array type.
    Array(ArrayConstraints),
    /// String type.
    Str,
    /// Number type (currently merged with int).
    Number,
    /// Boolean type.
    Boolean,
    /// Null type.
    Null,
    /// Dispatch for `type` keyword with multiple types allowed.
    /// It can be assumed that every SchemaNodeId in the vector
    /// represents a different type.
    Type(TypeConstraints),
}

/// Represents a primitive JSON type.
/// Used for type dispatch in [`SchemaNode::Type`].
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum JsonType {
    /// Object: '{'
    Object,
    /// Array: '['
    Array,
    /// String: '"'
    String,
    /// Number: '-', '0'..'9'
    Number,
    /// Boolean: 't', 'f'
    Boolean,
    /// Null: 'n'
    Null,
}

impl JsonType {
    /// Determine the JSON type based on the first byte of the value.
    /// The value is assumed to be of a valid primitive JSON type.
    #[inline(always)]
    pub fn from_byte(b: u8) -> Self {
        match b {
            b'{' => JsonType::Object,
            b'[' => JsonType::Array,
            b'"' => JsonType::String,
            b't' | b'f' => JsonType::Boolean,
            b'n' => JsonType::Null,
            _ => JsonType::Number,
        }
    }
}

/// JSON Schema object constraints.
#[derive(Debug, Clone)]
pub struct ObjectConstraints {
    /// Maps property names to their corresponding schema nodes.
    properties: Box<[(StringPattern, SchemaNodeId)]>,
    additional_properties: Option<SchemaNodeId>,
    min_properties: JsonUInt,
    max_properties: JsonUInt,
}

impl ObjectConstraints {
    pub(crate) fn new(
        properties: Box<[(StringPattern, SchemaNodeId)]>,
        additional_properties: Option<SchemaNodeId>,
        min_properties: Option<JsonUInt>,
        max_properties: Option<JsonUInt>,
    ) -> Self {
        Self {
            properties,
            additional_properties,
            min_properties: min_properties.unwrap_or(JsonUInt::ZERO),
            max_properties: max_properties.unwrap_or(JsonUInt::MAX),
        }
    }

    #[inline]
    pub(crate) fn properties(&self) -> &[(StringPattern, SchemaNodeId)] {
        &self.properties
    }

    #[inline]
    pub(crate) fn additional_properties(&self) -> Option<SchemaNodeId> {
        self.additional_properties
    }

    #[inline]
    pub(crate) fn min_properties(&self) -> JsonUInt {
        self.min_properties
    }

    #[inline]
    pub(crate) fn max_properties(&self) -> JsonUInt {
        self.max_properties
    }
}

/// JSON Schema array constraints.
#[derive(Debug, Clone)]
pub struct ArrayConstraints {
    /// "items" applies its subschema to all instance array elements.
    items: SchemaNodeId,
    /// An array instance is valid against "minItems" if its size is greater than,
    /// or equal to, the value of this keyword.
    /// Omitting this keyword has the same behavior as a value of 0.
    min_items: JsonUInt,
    /// An array instance is valid against "maxItems" if its size is less than,
    /// or equal to, the value of this keyword.
    max_items: JsonUInt,
}

impl ArrayConstraints {
    pub(crate) fn new(items: SchemaNodeId, min_items: Option<JsonUInt>, max_items: Option<JsonUInt>) -> Self {
        Self {
            items,
            min_items: min_items.unwrap_or(JsonUInt::ZERO),
            max_items: max_items.unwrap_or(JsonUInt::MAX),
        }
    }

    #[inline]
    pub(crate) fn items(&self) -> SchemaNodeId {
        self.items
    }

    #[inline]
    pub(crate) fn min_items(&self) -> JsonUInt {
        self.min_items
    }

    #[inline]
    pub(crate) fn max_items(&self) -> JsonUInt {
        self.max_items
    }
}

/// Node representation of a `type` keyword with multiple types allowed.
/// Transitions to the corresponding schema node based on the value type.
#[derive(Debug, Clone)]
pub struct TypeConstraints {
    types: [Option<SchemaNodeId>; 6],
}

impl TypeConstraints {
    pub(crate) fn new(types: [Option<SchemaNodeId>; 6]) -> Self {
        Self { types }
    }
}

impl Index<JsonType> for TypeConstraints {
    type Output = Option<SchemaNodeId>;

    #[inline(always)]
    fn index(&self, index: JsonType) -> &Self::Output {
        &self.types[index as usize]
    }
}

/// Automaton representation of a JSON Schema.
#[derive(Debug, Clone)]
pub struct SchemaAutomaton {
    /// All nodes in the schema graph.
    nodes: Vec<SchemaNode>,
    /// Root node index.
    root: SchemaNodeId,
}

impl SchemaAutomaton {
    pub(crate) fn new(nodes: Vec<SchemaNode>, root: SchemaNodeId) -> Self {
        Self { nodes, root }
    }

    /// Get root node index.
    #[inline]
    pub(crate) fn root(&self) -> SchemaNodeId {
        self.root
    }
}

impl Index<SchemaNodeId> for SchemaAutomaton {
    type Output = SchemaNode;

    #[inline(always)]
    fn index(&self, index: SchemaNodeId) -> &Self::Output {
        &self.nodes[index.as_usize()]
    }
}
