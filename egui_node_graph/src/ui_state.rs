use super::*;

#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct PanZoom {
    pub pan: egui::Vec2,
    pub zoom: f32,
}

#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct GraphEditorState<NodeData, DataType, ValueType, NodeTemplate, UserState> {
    pub graph: Graph<NodeData, DataType, ValueType>,
    /// Nodes are drawn in this order. Draw order is important because nodes
    /// that are drawn last are on top.
    pub node_order: Vec<NodeId>,
    /// An ongoing connection interaction: The mouse has dragged away from a
    /// port and the user is holding the click
    pub connection_in_progress: Option<(NodeId, AnyParameterId)>,
    /// The currently selected node. Some interface actions depend on the
    /// currently selected node.
    pub selected_node: Option<NodeId>,
    /// The position of each node.
    pub node_positions: SecondaryMap<NodeId, egui::Pos2>,
    /// The node finder is used to create new nodes.
    pub node_finder: Option<NodeFinder<NodeTemplate>>,
    /// The panning of the graph viewport.
    pub pan_zoom: PanZoom,
    pub user_state: UserState,
}

impl<NodeData, DataType: DataTypeTrait, ValueType, NodeKind, UserState>
    GraphEditorState<NodeData, DataType, ValueType, NodeKind, UserState>
{
    pub fn new(default_zoom: f32, user_state: UserState) -> Self {
        Self {
            graph: Graph::new(),
            node_order: Vec::new(),
            connection_in_progress: None,
            selected_node: None,
            node_positions: SecondaryMap::new(),
            node_finder: None,
            pan_zoom: PanZoom {
                pan: egui::Vec2::ZERO,
                zoom: default_zoom,
            },
            user_state,
        }
    }
}

impl PanZoom {
    pub fn adjust_zoom(
        &mut self,
        zoom_delta: f32,
        point: egui::Vec2,
        zoom_min: f32,
        zoom_max: f32,
    ) {
        let zoom_clamped = (self.zoom + zoom_delta).clamp(zoom_min, zoom_max);
        let zoom_delta = zoom_clamped - self.zoom;

        self.zoom += zoom_delta;
        self.pan += point * zoom_delta;
    }
}
