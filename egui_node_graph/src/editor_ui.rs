use std::collections::{HashSet, HashMap};

use crate::color_hex_utils::*;
use crate::utils::ColorUtils;

use super::*;
use egui::epaint::{CubicBezierShape, RectShape};
use egui::*;

pub type HookLocations = HashMap<ConnectionId, Pos2>;

/// Ports communicate connection and disconnection events to the parent graph
/// when drawn.
#[derive(Clone, Debug)]
pub enum PortResponse<Node: NodeTrait> {
    /// The user is creating a new connection from the hook of ConnectionId
    ConnectEventStarted(ConnectionId),
    /// The user is moving the connection that used to be attached to ConnectionId
    MoveEvent(ConnectionId),
    /// A connection has been accepted by a port
    ConnectEventEnded {
        output: OutputId,
        input: InputId,
    },
    /// The value of a port has changed
    Value(ValueResponseOf<Node>)
}

impl<Node: NodeTrait> PortResponse<Node> {
    pub fn connect_event_ended(a: ConnectionId, b: ConnectionId) -> Option<Self> {
        if let Some(ConnectionId::Input(a), ConnectionId::Output(b)) = (a, b) {
            Some(PortResponse::ConnectEventEnded{output: b, input: a})
        }

        if let Some(ConnectionId::Output(a), ConnectionId::Input(b)) = (a, b) {
            Some(PortResponse::ConnectEventEnded{output: a, input: b})
        }

        None
    }
}

/// Nodes communicate certain events to the parent graph when drawn. There is
/// one special `User` variant which can be used by users as the return value
/// when executing some custom actions in the UI of the node.
#[derive(Clone, Debug)]
pub enum NodeResponse<Node: NodeTrait> {
    Port(PortResponse<Node>),
    CreatedNode(NodeId),
    SelectNode(NodeId),
    /// As a user of this library, prefer listening for `DeleteNodeFull` which
    /// will also contain the user data for the deleted node.
    DeleteNodeUi(NodeId),
    /// Emitted when a node is deleted. The node will no longer exist in the
    /// graph after this response is returned from the draw function, but its
    /// contents are passed along with the event.
    DeleteNodeFull {
        node_id: NodeId,
        node: Node,
    },
    /// Emitted when a node is interacted with, and should be raised
    RaiseNode(NodeId),
    Content(ContentResponseOf<Node>),
}

/// Automatically convert a Port Response into a NodeResponse
impl<N: NodeTrait> From<PortResponse<N>> for NodeResponse<N> {
    fn from(value: PortResponse<N>) -> Self {
        Self::Port(value)
    }
}

pub struct NodeUiState<'a, DataType> {
    pub pan: Pos2,
    pub hook_locations: &'a mut HookLocations,
    pub node_id: NodeId,
    pub ongoing_drag: Option<(ConnectionId, &'a DataType)>,
    pub selected: bool,
}

/// The return value of [`draw_graph_editor`]. This value can be used to make
/// user code react to specific events that happened when drawing the graph.
#[derive(Clone, Debug)]
pub struct GraphResponse<Node: NodeTrait> {
    pub node_responses: Vec<NodeResponse<Node>>,
}

impl<Context> GraphEditorState<Context>
where
    Context: GraphContext,
    Context::Node: NodeTrait,
    Context::NodeTemplate: NodeTemplateTrait<Context::Node>,
{
    #[must_use]
    pub fn draw_graph_editor(
        &mut self,
        ui: &mut Ui,
        all_kinds: impl NodeTemplateIter<Item = Context::NodeTemplate>,
    ) -> GraphResponse<Context::Node> {
        // This causes the graph editor to use as much free space as it can.
        // (so for windows it will use up to the resizeably set limit
        // and for a Panel it will fill it completely)
        let editor_rect = ui.max_rect();
        ui.allocate_rect(editor_rect, Sense::hover());

        let cursor_pos = ui.ctx().input().pointer.hover_pos().unwrap_or(Pos2::ZERO);
        let mut cursor_in_editor = editor_rect.contains(cursor_pos);
        let mut cursor_in_finder = false;

        // Gets filled with the port locations as nodes are drawn
        let mut hook_locations = HookLocations::new();

        // The responses returned from node drawing have side effects that are best
        // executed at the end of this function.
        let mut delayed_responses: Vec<NodeResponse<Context::Node>> = vec![];

        // Used to detect when the background was clicked, to dismiss certain selfs
        let mut click_on_background = false;

        debug_assert_eq!(
            self.node_order.iter().copied().collect::<HashSet<_>>(),
            self.graph.iter_nodes().collect::<HashSet<_>>(),
            "The node_order field of the GraphEditorself was left in an \
        inconsistent self. It has either more or less values than the graph."
        );

        /* Draw nodes */
        for node_id in self.node_order.iter().copied() {
            let responses = GraphNodeWidget {
                position: self.node_positions.get_mut(node_id).unwrap(),
                graph: &mut self.graph,
                port_locations: &mut port_locations,
                node_id,
                ongoing_drag: self.connection_in_progress,
                selected: self
                    .selected_node
                    .map(|selected| selected == node_id)
                    .unwrap_or(false),
                pan: self.pan_zoom.pan + editor_rect.min.to_vec2(),
            }
            .show(ui, &self.user_state);

            // Actions executed later
            delayed_responses.extend(responses);
        }

        let r = ui.allocate_rect(ui.min_rect(), Sense::click().union(Sense::drag()));
        if r.clicked() {
            click_on_background = true;
        }

        /* Draw the node finder, if open */
        let mut should_close_node_finder = false;
        if let Some(ref mut node_finder) = self.node_finder {
            let mut node_finder_area = Area::new("node_finder").order(Order::Foreground);
            if let Some(pos) = node_finder.position {
                node_finder_area = node_finder_area.current_pos(pos);
            }
            node_finder_area.show(ui.ctx(), |ui| {
                if let Some(node_kind) = node_finder.show(ui, all_kinds) {
                    let new_node = self.graph.add_node(
                        node_kind.node_graph_label(),
                        node_kind.user_data(),
                        |graph, node_id| node_kind.build_node(graph, &self.user_state, node_id),
                    );
                    self.node_positions.insert(
                        new_node,
                        cursor_pos - self.pan_zoom.pan - editor_rect.min.to_vec2(),
                    );
                    self.node_order.push(new_node);

                    should_close_node_finder = true;
                    delayed_responses.push(NodeResponse::CreatedNode(new_node));
                }
                let finder_rect = ui.max_rect();
                // If the cursor is not in the main editor, check if the cursor *is* in the finder
                // if the cursor is in the finder, then we can consider that also in the editor.
                if !cursor_in_editor && finder_rect.contains(cursor_pos) {
                    cursor_in_editor = true;
                    cursor_in_finder = true;
                }
            });
        }
        if should_close_node_finder {
            self.node_finder = None;
        }

        /* Draw connections */
        if let Some((_, ref locator)) = self.connection_in_progress {
            let port_type = self.graph.any_param_type(*locator).unwrap();
            let connection_color = port_type.data_type_color(&self.user_state);
            let start_pos = port_locations[locator];
            let (src_pos, dst_pos) = match locator {
                AnyParameterId::Output(_) => (start_pos, cursor_pos),
                AnyParameterId::Input(_) => (cursor_pos, start_pos),
            };
            draw_connection(ui.painter(), src_pos, dst_pos, connection_color);
        }

        for (input, output) in self.graph.iter_connections() {
            let port_type = self
                .graph
                .any_param_type(AnyParameterId::Output(output))
                .unwrap();
            let connection_color = port_type.data_type_color(&self.user_state);
            let src_pos = port_locations[&AnyParameterId::Output(output)];
            let dst_pos = port_locations[&AnyParameterId::Input(input)];
            draw_connection(ui.painter(), src_pos, dst_pos, connection_color);
        }

        /* Handle responses from drawing nodes */

        // Some responses generate additional responses when processed. These
        // are stored here to report them back to the user.
        let mut extra_responses: Vec<NodeResponse<UserResponse, NodeData>> = Vec::new();

        for response in delayed_responses.iter() {
            match response {
                NodeResponse::ConnectEventStarted(node_id, port) => {
                    self.connection_in_progress = Some((*node_id, *port));
                }
                NodeResponse::ConnectEventEnded { input, output } => {
                    self.graph.add_connection(*output, *input)
                }
                NodeResponse::CreatedNode(_) => {
                    //Convenience NodeResponse for users
                }
                NodeResponse::SelectNode(node_id) => {
                    self.selected_node = Some(*node_id);
                }
                NodeResponse::DeleteNodeUi(node_id) => {
                    let (node, disc_events) = self.graph.remove_node(*node_id);
                    // Pass the full node as a response so library users can
                    // listen for it and get their user data.
                    extra_responses.push(NodeResponse::DeleteNodeFull {
                        node_id: *node_id,
                        node,
                    });
                    extra_responses.extend(
                        disc_events
                            .into_iter()
                            .map(|(input, output)| NodeResponse::DisconnectEvent { input, output }),
                    );
                    self.node_positions.remove(*node_id);
                    // Make sure to not leave references to old nodes hanging
                    if self.selected_node.map(|x| x == *node_id).unwrap_or(false) {
                        self.selected_node = None;
                    }
                    self.node_order.retain(|id| *id != *node_id);
                }
                NodeResponse::DisconnectEvent { input, output } => {
                    let other_node = self.graph.get_input(*input).node();
                    self.graph.remove_connection(*input);
                    self.connection_in_progress =
                        Some((other_node, AnyParameterId::Output(*output)));
                }
                NodeResponse::RaiseNode(node_id) => {
                    let old_pos = self
                        .node_order
                        .iter()
                        .position(|id| *id == *node_id)
                        .expect("Node to be raised should be in `node_order`");
                    self.node_order.remove(old_pos);
                    self.node_order.push(*node_id);
                }
                NodeResponse::User(_) => {
                    // These are handled by the user code.
                }
                NodeResponse::DeleteNodeFull { .. } => {
                    unreachable!("The UI should never produce a DeleteNodeFull event.")
                }
            }
        }

        // Push any responses that were generated during response handling.
        // These are only informative for the end-user and need no special
        // treatment here.
        delayed_responses.extend(extra_responses);

        /* Mouse input handling */

        // This locks the context, so don't hold on to it for too long.
        let mouse = &ui.ctx().input().pointer;

        if mouse.any_released() && self.connection_in_progress.is_some() {
            self.connection_in_progress = None;
        }

        if mouse.secondary_down() && cursor_in_editor && !cursor_in_finder {
            self.node_finder = Some(NodeFinder::new_at(cursor_pos));
        }
        if ui.ctx().input().key_pressed(Key::Escape) {
            self.node_finder = None;
        }

        if r.dragged() && ui.ctx().input().pointer.middle_down() {
            self.pan_zoom.pan += ui.ctx().input().pointer.delta();
        }

        // Deselect and deactivate finder if the editor backround is clicked,
        // *or* if the the mouse clicks off the ui
        if click_on_background || (mouse.any_click() && !cursor_in_editor) {
            self.selected_node = None;
            self.node_finder = None;
        }

        GraphResponse {
            node_responses: delayed_responses,
        }
    }
}

fn draw_connection(painter: &Painter, src_pos: Pos2, dst_pos: Pos2, color: Color32) {
    let connection_stroke = egui::Stroke { width: 5.0, color };

    let control_scale = ((dst_pos.x - src_pos.x) / 2.0).max(30.0);
    let src_control = src_pos + Vec2::X * control_scale;
    let dst_control = dst_pos - Vec2::X * control_scale;

    let bezier = CubicBezierShape::from_points_stroke(
        [src_pos, src_control, dst_control, dst_pos],
        false,
        Color32::TRANSPARENT,
        connection_stroke,
    );

    painter.add(bezier);
}
