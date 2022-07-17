use super::*;
use egui::Pos2;

#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "persistence")]
fn shown_inline_default() -> bool {
    true
}

/// The graph, containing nodes, input parameters and output parameters. Because
/// graphs are full of self-referential structures, this type uses the `slotmap`
/// crate to represent all the inner references in the data.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Graph<Node> {
    /// The Nodes of the graph
    pub nodes: SlotMap<NodeId, Node>,
    /// Connects the input of a port to the output of its predecessor that
    /// produces it
    pub input_to_output: SecondaryMap<InputId, OutputId>,
    /// Connects the output of a port to the input that it's funneling to
    pub output_to_input: SecondaryMap<OutputId, InputId>,
}

pub struct NodeUiState<'a, Node: NodeTrait> {
    pub connection_locations: &'a mut ConnectionLocations,
    pub node_id: NodeId,
    pub ongoing_drag: Option<(NodeId, PortId, &'a DataTypeOf<Node>)>,
    pub selected: bool,
}
