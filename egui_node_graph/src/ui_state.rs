use super::*;
use egui::{Rect, Style, Ui, Vec2};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::scale::Scale;
#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};

const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 2.0;

#[derive(Clone)]
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
    pub selected_nodes: Vec<NodeId>,
    /// The mouse drag start position for an ongoing box selection.
    pub ongoing_box_selection: Option<egui::Pos2>,
    /// The position of each node.
    pub node_positions: SecondaryMap<NodeId, egui::Pos2>,
    /// The node finder is used to create new nodes.
    pub node_finder: Option<NodeFinder<NodeTemplate>>,
    /// The panning of the graph viewport.
    pub pan_zoom: PanZoom,
    pub _user_state: PhantomData<fn() -> UserState>,
}

impl<NodeData, DataType, ValueType, NodeKind, UserState>
    GraphEditorState<NodeData, DataType, ValueType, NodeKind, UserState>
{
    pub fn new(default_zoom: f32) -> Self {
        Self {
            pan_zoom: PanZoom::new(default_zoom),
            ..Default::default()
        }
    }
}
impl<NodeData, DataType, ValueType, NodeKind, UserState> Default
    for GraphEditorState<NodeData, DataType, ValueType, NodeKind, UserState>
{
    fn default() -> Self {
        Self {
            graph: Default::default(),
            node_order: Default::default(),
            connection_in_progress: Default::default(),
            selected_nodes: Default::default(),
            ongoing_box_selection: Default::default(),
            node_positions: Default::default(),
            node_finder: Default::default(),
            pan_zoom: Default::default(),
            _user_state: Default::default(),
        }
    }
}

#[cfg(feature = "persistence")]
fn _default_clip_rect() -> Rect {
    Rect::NOTHING
}

#[derive(Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct PanZoom {
    pub pan: Vec2,
    pub zoom: f32,
    #[cfg_attr(feature = "persistence", serde(skip, default = "_default_clip_rect"))]
    pub clip_rect: Rect,
    #[cfg_attr(feature = "persistence", serde(skip, default))]
    pub zoomed_style: Arc<Style>,
    #[cfg_attr(feature = "persistence", serde(skip, default))]
    pub started: bool,
}

impl Default for PanZoom {
    fn default() -> Self {
        PanZoom {
            pan: Vec2::ZERO,
            zoom: 1.0,
            clip_rect: Rect::NOTHING,
            zoomed_style: Default::default(),
            started: false,
        }
    }
}

impl PanZoom {
    pub fn new(zoom: f32) -> PanZoom {
        let style: Style = Default::default();
        PanZoom {
            pan: Vec2::ZERO,
            zoom,
            clip_rect: Rect::NOTHING,
            zoomed_style: Arc::new(style.scaled(1.0)),
            started: false,
        }
    }

    pub fn zoom(&mut self, clip_rect: Rect, style: &Arc<Style>, zoom_delta: f32) {
        self.clip_rect = clip_rect;
        let new_zoom = (self.zoom * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoomed_style = Arc::new(style.scaled(new_zoom));
        self.zoom = new_zoom;
    }
}

pub fn show_zoomed<R, F>(
    default_style: Arc<Style>,
    zoomed_style: Arc<Style>,
    ui: &mut Ui,
    add_content: F,
) -> R
where
    F: FnOnce(&mut Ui) -> R,
{
    *ui.style_mut() = (*zoomed_style).clone();
    let response = add_content(ui);
    *ui.style_mut() = (*default_style).clone();

    response
}
