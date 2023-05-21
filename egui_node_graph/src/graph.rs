use std::num::NonZeroU32;

use super::*;

#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};

/// A node inside the [`Graph`]. Nodes have input and output parameters, stored
/// as ids. They also contain a custom `NodeData` struct with whatever data the
/// user wants to store per-node.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Node<NodeData> {
    pub id: NodeId,
    pub label: String,
    pub inputs: Vec<(String, InputId)>,
    pub outputs: Vec<(String, OutputId)>,
    pub user_data: NodeData,
}

/// The three kinds of input params. These describe how the graph must behave
/// with respect to inline widgets and connections for this parameter.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum InputParamKind {
    /// No constant value can be set. Only incoming connections can produce it
    ConnectionOnly,
    /// Only a constant value can be set. No incoming connections accepted.
    ConstantOnly,
    /// Both incoming connections and constants are accepted. Connections take
    /// precedence over the constant values.
    ConnectionOrConstant,
}

#[cfg(feature = "persistence")]
fn shown_inline_default() -> bool {
    true
}

/// An input parameter. Input parameters are inside a node, and represent data
/// that this node receives. Unlike their [`OutputParam`] counterparts, input
/// parameters also display an inline widget which allows setting its "value".
/// The `DataType` generic parameter is used to restrict the range of input
/// connections for this parameter, and the `ValueType` is use to represent the
/// data for the inline widget (i.e. constant) value.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct InputParam<DataType, ValueType> {
    pub id: InputId,
    /// The data type of this node. Used to determine incoming connections. This
    /// should always match the type of the InputParamValue, but the property is
    /// not actually enforced.
    pub typ: DataType,
    /// The constant value stored in this parameter.
    pub value: ValueType,
    /// The input kind. See [`InputParamKind`]
    pub kind: InputParamKind,
    /// Back-reference to the node containing this parameter.
    pub node: NodeId,
    /// How many connections can be made with this input. `None` means no limit.
    pub max_connections: Option<NonZeroU32>,
    /// When true, the node is shown inline inside the node graph.
    #[cfg_attr(feature = "persistence", serde(default = "shown_inline_default"))]
    pub shown_inline: bool,
}

/// An output parameter. Output parameters are inside a node, and represent the
/// data that the node produces. Output parameters can be linked to the input
/// parameters of other nodes. Unlike an [`InputParam`], output parameters
/// cannot have a constant inline value.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct OutputParam<DataType> {
    pub id: OutputId,
    /// Back-reference to the node containing this parameter.
    pub node: NodeId,
    pub typ: DataType,
}

/// The graph, containing nodes, input parameters and output parameters. Because
/// graphs are full of self-referential structures, this type uses the `slotmap`
/// crate to represent all the inner references in the data.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Graph<NodeData, DataType, ValueType> {
    /// The [`Node`]s of the graph
    pub nodes: SlotMap<NodeId, Node<NodeData>>,
    /// The [`InputParam`]s of the graph
    pub inputs: SlotMap<InputId, InputParam<DataType, ValueType>>,
    /// The [`OutputParam`]s of the graph
    pub outputs: SlotMap<OutputId, OutputParam<DataType>>,
    // Connects the input of a node, to the output of its predecessor that
    // produces it
    pub connections: SecondaryMap<InputId, Vec<OutputId>>,
}
