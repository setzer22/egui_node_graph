use slotmap::{SecondaryMap, SlotMap};

pub mod id_type;
pub use id_type::*;

pub mod index_impls;

pub mod graph_impls;

pub mod error;
pub use error::*;

pub mod ui_state;
pub use ui_state::*;

pub mod node_finder;
pub use node_finder::*;

pub mod editor_ui;
pub use editor_ui::*;

mod utils;

mod color_hex_utils;

#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};

pub type SVec<T> = smallvec::SmallVec<[T; 4]>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Node<NodeData> {
    pub id: NodeId,
    pub label: String,
    pub inputs: Vec<(String, InputId)>,
    pub outputs: Vec<(String, OutputId)>,
    pub user_data: NodeData,
}

/// There are three kinds of input params
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
    /// When true, the node is shown inline inside the node graph.
    #[cfg_attr(feature = "persistence", serde(default = "shown_inline_default"))]
    pub shown_inline: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct OutputParam<DataType> {
    pub id: OutputId,
    /// Back-reference to the node containing this parameter.
    pub node: NodeId,
    pub typ: DataType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Graph<NodeData, DataType, ValueType> {
    pub nodes: SlotMap<NodeId, Node<NodeData>>,
    pub inputs: SlotMap<InputId, InputParam<DataType, ValueType>>,
    pub outputs: SlotMap<OutputId, OutputParam<DataType>>,
    // Connects the input of a node, to the output of its predecessor that
    // produces it
    connections: SecondaryMap<InputId, OutputId>,
}
