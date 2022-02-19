use std::marker::PhantomData;

use crate::{color_hex_utils::*, Graph, Node, NodeId};

use egui::*;

pub struct NodeFinder<NodeKind> {
    query: String,
    /// Reset every frame. When set, the node finder will be moved at that position
    pub position: Option<Pos2>,
    pub just_spawned: bool,
    _phantom: PhantomData<NodeKind>,
}

pub trait NodeKindIter {
    type Item;
    fn all_kinds(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_>;
}

pub trait NodeKindTrait: Clone {
    type NodeData;
    type DataType;
    type ValueType;

    /// Returns a descriptive name for the node kind, used in the node finder.
    fn node_finder_label(&self) -> &str;

    /// Returns a descriptive name for the node kind, used in the graph.
    fn node_graph_label(&self) -> String;

    /// Returns the user data for this node kind.
    fn user_data(&self) -> Self::NodeData;

    /// This function is run when this node kind gets added to the graph. The
    /// node will be empty by default, and this function can be used to fill its
    /// parameters.
    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: NodeId,
    );
}

impl<NodeKind, NodeData> NodeFinder<NodeKind>
where
    NodeKind: NodeKindTrait<NodeData = NodeData>,
{
    pub fn new_at(pos: Pos2) -> Self {
        NodeFinder {
            query: "".into(),
            position: Some(pos),
            just_spawned: true,
            _phantom: Default::default(),
        }
    }

    /// Shows the node selector panel with a search bar. Returns whether a node
    /// archetype was selected and, in that case, the finder should be hidden on
    /// the next frame.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        all_kinds: impl NodeKindIter<Item = NodeKind>,
    ) -> Option<NodeKind> {
        let background_color = color_from_hex("#3f3f3f").unwrap();
        let text_color = color_from_hex("#fefefe").unwrap();

        ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.0, text_color);

        let frame = Frame::dark_canvas(ui.style())
            .fill(background_color)
            .margin(vec2(5.0, 5.0));

        // The archetype that will be returned.
        let mut submitted_archetype = None;
        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                let resp = ui.text_edit_singleline(&mut self.query);
                if self.just_spawned {
                    resp.request_focus();
                    self.just_spawned = false;
                }

                let mut query_submit = resp.lost_focus() && ui.input().key_down(Key::Enter);

                Frame::default().margin(vec2(10.0, 10.0)).show(ui, |ui| {
                    for kind in all_kinds.all_kinds() {
                        let kind_name = kind.node_finder_label();
                        if kind_name.contains(self.query.as_str()) {
                            if query_submit {
                                submitted_archetype = Some(kind);
                                query_submit = false;
                            }
                            if ui.selectable_label(false, kind_name).clicked() {
                                submitted_archetype = Some(kind);
                            }
                        }
                    }
                });
            });
        });

        submitted_archetype.cloned()
    }
}
