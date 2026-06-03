//! Automaton representation of a JSON Schema.
use crate::string_pattern::StringPattern;
use std::{fmt::Display, ops::Index};

/// Identifier of a [`SchemaNode`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SchemaNodeId(pub(crate) u32);

impl Index<SchemaNodeId> for SchemaAutomaton {
    type Output = SchemaNode;

    fn index(&self, index: SchemaNodeId) -> &Self::Output {
        &self.nodes[index.0 as usize]
    }
}

impl Display for SchemaNodeId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SchemaNodeId({})", self.0)
    }
}

/// JSON Schema constraints node.
///
/// Every schema is a graph of these nodes,
/// each representing validator/applicator keywords
/// applicable at a given point.
#[derive(Debug, Clone)]
pub enum SchemaNode {
    /// Any value is accepted.
    Any,
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

/// JSON Schema object constraints.
#[derive(Debug, Clone)]
pub struct ObjectConstraints {
    /// Maps property names to their corresponding schema nodes.
    properties: Box<[(StringPattern, SchemaNodeId)]>,
    additional_properties: AdditionalProperties,
}

/// Possible values for "additionalProperties" in JSON Schema.
#[derive(Debug, Clone, Default)]
pub enum AdditionalProperties {
    /// Any additional properties allowed.
    /// Default value when "additionalProperties" is not specified.
    #[default]
    True,
    /// No additional properties allowed.
    False,
    /// Additional properties must validate against the given schema.
    Schema(SchemaNodeId),
}

impl ObjectConstraints {
    pub(crate) fn new(
        properties: Box<[(StringPattern, SchemaNodeId)]>,
        additional_properties: AdditionalProperties,
    ) -> Self {
        Self {
            properties,
            additional_properties,
        }
    }

    #[inline]
    pub(crate) fn properties(&self) -> &[(StringPattern, SchemaNodeId)] {
        &self.properties
    }

    #[inline]
    pub(crate) fn additional_properties(&self) -> &AdditionalProperties {
        &self.additional_properties
    }
}

/// JSON Schema array constraints.
#[derive(Debug, Clone)]
pub struct ArrayConstraints {
    /// Schema node describing the items in the array.
    items: SchemaNodeId,
}

impl ArrayConstraints {
    pub(crate) fn new(items: SchemaNodeId) -> Self {
        Self { items }
    }

    #[inline]
    pub(crate) fn items(&self) -> SchemaNodeId {
        self.items
    }
}

/// Node representation of a `type` keyword with multiple types allowed.
/// Transitions to the corresponding schema node based on the value type.
#[derive(Debug, Clone)]
pub struct TypeConstraints {
    types: Vec<SchemaNodeId>,
}

impl TypeConstraints {
    pub(crate) fn new(types: Vec<SchemaNodeId>) -> Self {
        Self { types }
    }

    #[inline]
    pub(crate) fn types(&self) -> &[SchemaNodeId] {
        &self.types
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

    /// Get all nodes in the schema graph.
    #[cfg(test)]
    #[inline]
    pub(crate) fn nodes(&self) -> &[SchemaNode] {
        &self.nodes
    }

    /// Get root node index.
    #[inline]
    pub(crate) fn root(&self) -> SchemaNodeId {
        self.root
    }
}
