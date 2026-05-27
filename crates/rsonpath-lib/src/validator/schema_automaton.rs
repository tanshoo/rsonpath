//! Automaton representation of a JSON Schema.
use crate::string_pattern::StringPattern;
use std::collections::HashMap;
use std::{fmt::Display, ops::Index};

/// Identifier of a [`SchemaNode`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct SchemaNodeId(pub(crate) u32);

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
pub(crate) enum SchemaNode {
    /// Any value is accepted.
    Any,
    /// Primitive type (null, boolean, number, integer, string).
    Primitive,
    /// Object type.
    Object(ObjectConstraints),
    /// Array type.
    Array(ArrayConstraints),
    /// Logical OR on multiple schema nodes.
    Or(Vec<SchemaNodeId>),
}

impl SchemaNode {
    /// Check if the node is a primitive type.
    pub(crate) fn is_primitive(&self) -> bool {
        matches!(self, SchemaNode::Primitive)
    }

    /// Check if the node is an array type.
    pub(crate) fn is_array(&self) -> bool {
        matches!(self, SchemaNode::Array(_))
    }
}

/// JSON Schema object constraints.
#[derive(Debug, Clone)]
pub(crate) struct ObjectConstraints {
    /// Maps property names to their corresponding schema nodes.
    properties: HashMap<StringPattern, SchemaNodeId>,
    additional_properties: AdditionalProperties,
}

/// Possible values for "additionalProperties" in JSON Schema.
#[derive(Debug, Clone, Default)]
pub(crate) enum AdditionalProperties {
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
        properties: HashMap<StringPattern, SchemaNodeId>,
        additional_properties: AdditionalProperties,
    ) -> Self {
        Self {
            properties,
            additional_properties,
        }
    }

    #[inline]
    pub(crate) fn properties(&self) -> &HashMap<StringPattern, SchemaNodeId> {
        &self.properties
    }

    #[inline]
    pub(crate) fn additional_properties(&self) -> &AdditionalProperties {
        &self.additional_properties
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ArrayConstraints {
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

/// Automaton representation of a JSON Schema.
#[derive(Debug, Clone)]
pub(crate) struct SchemaAutomaton {
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
