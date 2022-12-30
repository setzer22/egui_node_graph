use std::collections::{HashSet, HashMap};

use crate::color_hex_utils::*;
use crate::utils::ColorUtils;

use super::*;
use egui::epaint::{CubicBezierShape, RectShape};
use egui::*;

/// For each hook, this specifies the (location, tangent vector) for any curve
/// rendering a connection out of it.
pub type HookGeometry = HashMap<ConnectionId, (Pos2, Vec2)>;

/// Ports communicate connection and disconnection events to the parent graph
/// when drawn.
#[derive(Clone, Debug)]
pub enum PortResponse<DataType: DataTypeTrait> {
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
    Value(<DataType::Value as ValueTrait>::Response)
}

impl<DataType: DataTypeTrait> PortResponse<DataType> {
    pub fn connect_event_ended(a: ConnectionId, b: ConnectionId) -> Option<Self> {
        if let (Some(input), Some(output)) = (a.as_input(), b.as_output()) {
            Some(PortResponse::ConnectEventEnded{output, input})
        } else if let (Some(output), Some(input)) = (a.as_output(), b.as_input()) {
            Some(PortResponse::ConnectEventEnded{output, input})
        } else {
            None
        }
    }
}

/// Nodes communicate certain events to the parent graph when drawn. There is
/// one special `User` variant which can be used by users as the return value
/// when executing some custom actions in the UI of the node.
#[derive(Clone, Debug)]
pub enum NodeResponse<Node: NodeTrait> {
    Port(PortResponse<Node::DataType>),
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
    /// Emitted for each disconnection that has occurred in the graph
    DisconnectEvent{ input: InputId, output: OutputId },
    /// Emitted when a node is interacted with, and should be raised
    RaiseNode(NodeId),
    Content(ContentResponseOf<Node>),
}

impl<Node: NodeTrait> NodeResponse<Node> {
    fn disconnect(a: ConnectionId, b: ConnectionId) -> Result<Self, ()> {
        let (input, output) = match a.port() {
            PortId::Input(_) => {
                match b.as_output() {
                    Some(output) => (a.assume_input(), output),
                    None => return Err(()),
                }
            }
            PortId::Output(_) => {
                match b.as_input() {
                    Some(input) => (input, a.assume_output()),
                    None => return Err(()),
                }
            }
        };

        Ok(Self::DisconnectEvent { input, output })
    }
}

/// Automatically convert a Port Response into a NodeResponse
impl<N: NodeTrait> From<PortResponse<N::DataType>> for NodeResponse<N> {
    fn from(value: PortResponse<N::DataType>) -> Self {
        Self::Port(value)
    }
}

pub struct EditorUiState<'a, DataType> {
    pub pan: Vec2,
    pub hook_geometry: &'a mut HookGeometry,
    pub ongoing_drag: Option<(ConnectionId, DataType)>,
    pub selected_nodes: &'a Vec<NodeId>,
}

/// The return value of [`draw_graph_editor`]. This value can be used to make
/// user code react to specific events that happened when drawing the graph.
#[derive(Clone, Debug)]
pub struct GraphResponse<Node: NodeTrait> {
    pub node_responses: Vec<NodeResponse<Node>>,
}

impl<Context: GraphContextTrait> GraphEditorState<Context> {
    #[must_use]
    pub fn draw_graph_editor(
        &mut self,
        ui: &mut Ui,
        all_kinds: impl NodeTemplateIter<Item = Context::NodeTemplate>,
        app_state: &mut AppStateOf<Context::Node>,
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
        let mut hook_geometry = HookGeometry::new();

        // The responses returned from node drawing have side effects that are best
        // executed at the end of this function.
        let mut delayed_responses: Vec<NodeResponse<Context::Node>> = vec![];

        // Used to detect when the background was clicked, to dismiss certain selfs
        let mut click_on_background = false;

        debug_assert_eq!(
            self.node_order.iter().copied().collect::<HashSet<_>>(),
            self.graph.iter_nodes().map(|(id, _)| id).collect::<HashSet<_>>(),
            "The node_order field of the GraphEditorself was left in an \
            inconsistent state. It has either more or less values than the graph."
        );

        let ongoing_drag = self.connection_in_progress
            .and_then(|connection| self.graph.node(connection.node()).map(|n| (connection, n)))
            .and_then(|(connection, n)| n.port_data_type(connection.port()).map(|d| (connection, d)));

        let mut state = EditorUiState {
            pan: self.pan_zoom.pan + editor_rect.min.to_vec2(),
            hook_geometry: &mut hook_geometry,
            ongoing_drag: ongoing_drag.clone(),
            selected_nodes: &self.selected_nodes,
        };

        /* Draw nodes */
        for node_id in self.node_order.iter().copied() {
            let responses = match self.graph.node_mut(node_id, |node| {
                node.show(ui, app_state, node_id, &mut state, &self.context)
            }) {
                Ok(responses) => responses,
                // TODO(MXG): Should we alert the user to this error? It really
                // shouldn't happen...
                Err(()) => continue,
            };

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
                        node_kind.build_node(cursor_pos - state.pan, app_state),
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
        if let Some((connection, data_type)) = &ongoing_drag {
            let connection_color = self.context.recommend_data_type_color(&data_type);
            let hook_geom = hook_geometry[connection];
            let cursor_geom = (cursor_pos, (hook_geom.0 - cursor_pos).normalized());
            let (start_geom, end_geom) = match connection.port() {
                PortId::Input(_) => (cursor_geom, hook_geom),
                PortId::Output(_) => (hook_geom, cursor_geom),
            };
            draw_connection(ui.painter(), start_geom, end_geom, connection_color);
        }

        for (input, output) in self.graph.iter_connections() {
            let data_type = self
                .graph
                .node(input.node()).expect("node missing for a connection")
                .port_data_type(input.port().into()).expect("port missing for a connection");
            let connection_color = self.context.recommend_data_type_color(&data_type);
            let start_geom = hook_geometry[&output.into()];
            let end_geom = hook_geometry[&input.into()];
            draw_connection(ui.painter(), start_geom, end_geom, connection_color);
        }

        /* Handle responses from drawing nodes */

        // Some responses generate additional responses when processed. These
        // are stored here to report them back to the user.
        let mut extra_responses = Vec::new();

        for response in delayed_responses.iter() {
            match response {
                NodeResponse::Port(port_response) => {
                    match port_response {
                        PortResponse::ConnectEventStarted(connection) => {
                            self.connection_in_progress = Some(*connection);
                        }
                        PortResponse::MoveEvent(connection) => {
                            if let Ok(complement) = self.graph.drop_connection(*connection) {
                                extra_responses.push(
                                    NodeResponse::disconnect(*connection, complement)
                                    .expect("invalid input/output pair for connection")
                                );
                                if let Some(available_hook) = self.graph.node(complement.node())
                                    .and_then(|n| n.available_hook(complement.port()))
                                {
                                    self.connection_in_progress = Some(
                                        ConnectionId(complement.node(), complement.port(), available_hook)
                                    );
                                }
                            }
                        }
                        PortResponse::ConnectEventEnded { output, input } => {
                            self.graph.add_connection(*output, *input);
                        }
                        PortResponse::Value(_) => {
                            // User-defined response type
                        }
                    }
                }
                NodeResponse::CreatedNode(_) => {
                    // Convenience NodeResponse for users
                }
                NodeResponse::SelectNode(node_id) => {
                    if !ui.input().modifiers.shift {
                        self.selected_nodes.clear();
                    }
                    if !self.selected_nodes.contains(node_id) {
                        self.selected_nodes.push(*node_id);
                    }
                }
                NodeResponse::DisconnectEvent { input, .. } => {
                    self.graph.drop_connection(input.clone().into());
                }
                NodeResponse::DeleteNodeUi(node_id) => {
                    if let Some((node, disc_events)) = self.graph.remove_node(*node_id) {
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
                    }
                    // Make sure to not leave references to old nodes hanging
                    self.selected_nodes.retain(|id| *id != *node_id);
                    self.node_order.retain(|id| *id != *node_id);
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
                NodeResponse::Content(_) => {
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

        if click_on_background && !ui.input().modifiers.shift {
            // Clear the selected nodes if the background is clicked when shift
            // is not selected.
            self.selected_nodes.clear();
        }

        if click_on_background || (mouse.any_click() && !cursor_in_editor) {
            // Deactivate finder if the editor backround is clicked,
            // *or* if the the mouse clicks off the ui
            self.node_finder = None;
        }

        GraphResponse {
            node_responses: delayed_responses,
        }
    }
}

fn calculate_control(
    a_pos: Pos2,
    b_pos: Pos2,
    tangent: Vec2,
) -> Pos2 {
    let delta = ((a_pos - b_pos).dot(tangent).abs()/2.0).max(30.0);
    a_pos + delta*tangent
}

fn draw_connection(
    painter: &Painter,
    (start_pos, start_tangent): (Pos2, Vec2),
    (end_pos, end_tangent): (Pos2, Vec2),
    color: Color32
) {
    let connection_stroke = egui::Stroke { width: 5.0, color };

    let start_control = calculate_control(start_pos, end_pos, start_tangent);
    let end_control = calculate_control(end_pos, start_pos, end_tangent);

    let bezier = CubicBezierShape::from_points_stroke(
        [start_pos, start_control, end_control, end_pos],
        false,
        Color32::TRANSPARENT,
        connection_stroke,
    );

    painter.add(bezier);
}
