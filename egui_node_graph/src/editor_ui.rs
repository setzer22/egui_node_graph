use std::collections::HashSet;

use crate::color_hex_utils::*;
use crate::utils::ColorUtils;

use super::*;
use egui::epaint::{CubicBezierShape, RectShape};
use egui::*;

pub type PortLocations = std::collections::HashMap<AnyParameterId, Pos2>;

/// Nodes communicate certain events to the parent graph when drawn. There is
/// one special `User` variant which can be used by users as the return value
/// when executing some custom actions in the UI of the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeResponse<UserResponse: UserResponseTrait> {
    ConnectEventStarted(NodeId, AnyParameterId),
    ConnectEventEnded(AnyParameterId),
    CreatedNode(NodeId),
    SelectNode(NodeId),
    DeleteNode(NodeId),
    DisconnectEvent(InputId),
    /// Emitted when a node is interacted with, and should be raised
    RaiseNode(NodeId),
    User(UserResponse),
}

/// The return value of [`draw_graph_editor`]. This value can be used to make
/// user code react to specific events that happened when drawing the graph.
#[derive(Clone, Debug)]
pub struct GraphResponse<UserResponse: UserResponseTrait> {
    pub node_responses: Vec<NodeResponse<UserResponse>>,
}

pub struct GraphNodeWidget<'a, NodeData, DataType, ValueType> {
    pub position: &'a mut Pos2,
    pub graph: &'a mut Graph<NodeData, DataType, ValueType>,
    pub port_locations: &'a mut PortLocations,
    pub node_id: NodeId,
    pub ongoing_drag: Option<(NodeId, AnyParameterId)>,
    pub selected: bool,
    pub pan: egui::Vec2,
}

impl<NodeData, DataType, ValueType, NodeTemplate, UserResponse, UserState>
    GraphEditorState<NodeData, DataType, ValueType, NodeTemplate, UserState>
where
    NodeData: NodeDataTrait<
        Response = UserResponse,
        UserState = UserState,
        DataType = DataType,
        ValueType = ValueType,
    >,
    UserResponse: UserResponseTrait,
    ValueType: WidgetValueTrait<Response = UserResponse>,
    NodeTemplate:
        NodeTemplateTrait<NodeData = NodeData, DataType = DataType, ValueType = ValueType>,
    DataType: DataTypeTrait<UserState>,
{
    #[must_use]
    pub fn draw_graph_editor(
        &mut self,
        ui: &mut Ui,
        all_kinds: impl NodeTemplateIter<Item = NodeTemplate>,
    ) -> GraphResponse<UserResponse> {
        // This causes the graph editor to use as much free space as it can.
        // (so for windows it will use up to the resizeably set limit
        // and for a Panel it will fill it completely)
        let editor_rect = ui.max_rect();
        ui.allocate_rect(editor_rect, Sense::hover());

        let cursor_pos = ui.ctx().input().pointer.hover_pos().unwrap_or(Pos2::ZERO);
        let mut cursor_in_editor = editor_rect.contains(cursor_pos);

        // Gets filled with the port locations as nodes are drawn
        let mut port_locations = PortLocations::new();

        // The responses returned from node drawing have side effects that are best
        // executed at the end of this function.
        let mut delayed_responses: Vec<NodeResponse<UserResponse>> = vec![];

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

        let r = ui.allocate_rect(ui.min_rect(), Sense::click());
        if r.clicked() {
            click_on_background = true;
        }

        /* Draw the node finder, if open */
        let mut should_close_node_finder = false;
        if let Some(ref mut node_finder) = self.node_finder {
            let mut node_finder_area = Area::new("node_finder");
            if let Some(pos) = node_finder.position {
                node_finder_area = node_finder_area.current_pos(pos);
            }
            node_finder_area.show(ui.ctx(), |ui| {
                if let Some(node_kind) = node_finder.show(ui, all_kinds) {
                    let new_node = self.graph.add_node(
                        node_kind.node_graph_label(),
                        node_kind.user_data(),
                        |graph, node_id| node_kind.build_node(graph, node_id),
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
        let mut extra_responses: Vec<NodeResponse<UserResponse>> = Vec::new();

        for response in delayed_responses.iter().copied() {
            match response {
                NodeResponse::ConnectEventStarted(node_id, port) => {
                    self.connection_in_progress = Some((node_id, port));
                }
                NodeResponse::ConnectEventEnded(locator) => {
                    let in_out = match (
                        self.connection_in_progress
                            .map(|(_node, param)| param)
                            .take()
                            .expect("Cannot end drag without in-progress connection."),
                        locator,
                    ) {
                        (AnyParameterId::Input(input), AnyParameterId::Output(output))
                        | (AnyParameterId::Output(output), AnyParameterId::Input(input)) => {
                            Some((input, output))
                        }
                        _ => None,
                    };

                    if let Some((input, output)) = in_out {
                        self.graph.add_connection(output, input)
                    }
                }
                NodeResponse::CreatedNode(_) => {
                    //Convenience NodeResponse for users
                }
                NodeResponse::SelectNode(node_id) => {
                    self.selected_node = Some(node_id);
                }
                NodeResponse::DeleteNode(node_id) => {
                    let removed = self.graph.remove_node(node_id);
                    extra_responses.extend(
                        removed
                            .into_iter()
                            .map(|x| x.0)
                            .into_iter()
                            .map(NodeResponse::DisconnectEvent),
                    );
                    self.node_positions.remove(node_id);
                    // Make sure to not leave references to old nodes hanging
                    if self.selected_node.map(|x| x == node_id).unwrap_or(false) {
                        self.selected_node = None;
                    }
                    self.node_order.retain(|id| *id != node_id);
                }
                NodeResponse::DisconnectEvent(input_id) => {
                    let corresp_output = self
                        .graph
                        .connection(input_id)
                        .expect("Connection data should be valid");
                    let other_node = self.graph.get_input(input_id).node();
                    self.graph.remove_connection(input_id);
                    self.connection_in_progress =
                        Some((other_node, AnyParameterId::Output(corresp_output)));
                }
                NodeResponse::RaiseNode(node_id) => {
                    let old_pos = self
                        .node_order
                        .iter()
                        .position(|id| *id == node_id)
                        .expect("Node to be raised should be in `node_order`");
                    self.node_order.remove(old_pos);
                    self.node_order.push(node_id);
                }
                NodeResponse::User(_) => {
                    // These are handled by the user code.
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

        if mouse.button_down(PointerButton::Secondary) && cursor_in_editor {
            self.node_finder = Some(NodeFinder::new_at(cursor_pos));
        }
        if ui.ctx().input().key_pressed(Key::Escape) {
            self.node_finder = None;
        }

        if ui.ctx().input().pointer.middle_down() {
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

impl<'a, NodeData, DataType, ValueType, UserResponse, UserState>
    GraphNodeWidget<'a, NodeData, DataType, ValueType>
where
    NodeData: NodeDataTrait<
        Response = UserResponse,
        UserState = UserState,
        DataType = DataType,
        ValueType = ValueType,
    >,
    UserResponse: UserResponseTrait,
    ValueType: WidgetValueTrait<Response = UserResponse>,
    DataType: DataTypeTrait<UserState>,
{
    pub const MAX_NODE_SIZE: [f32; 2] = [200.0, 200.0];

    pub fn show(self, ui: &mut Ui, user_state: &UserState) -> Vec<NodeResponse<UserResponse>> {
        let mut child_ui = ui.child_ui_with_id_source(
            Rect::from_min_size(*self.position + self.pan, Self::MAX_NODE_SIZE.into()),
            Layout::default(),
            self.node_id,
        );

        Self::show_graph_node(self, &mut child_ui, user_state)
    }

    /// Draws this node. Also fills in the list of port locations with all of its ports.
    /// Returns responses indicating multiple events.
    fn show_graph_node(
        self,
        ui: &mut Ui,
        user_state: &UserState,
    ) -> Vec<NodeResponse<UserResponse>> {
        let margin = egui::vec2(15.0, 5.0);
        let mut responses = Vec::new();

        let background_color;
        let text_color;
        if ui.visuals().dark_mode {
            background_color = color_from_hex("#3f3f3f").unwrap();
            text_color = color_from_hex("#fefefe").unwrap();
        } else {
            background_color = color_from_hex("#ffffff").unwrap();
            text_color = color_from_hex("#505050").unwrap();
        }

        ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.0, text_color);

        // Preallocate shapes to paint below contents
        let outline_shape = ui.painter().add(Shape::Noop);
        let background_shape = ui.painter().add(Shape::Noop);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let mut inner_rect = outer_rect_bounds.shrink2(margin);

        // Make sure we don't shrink to the negative:
        inner_rect.max.x = inner_rect.max.x.max(inner_rect.min.x);
        inner_rect.max.y = inner_rect.max.y.max(inner_rect.min.y);

        let mut child_ui = ui.child_ui(inner_rect, *ui.layout());
        let mut title_height = 0.0;

        let mut input_port_heights = vec![];
        let mut output_port_heights = vec![];

        child_ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(Label::new(
                    RichText::new(&self.graph[self.node_id].label)
                        .text_style(TextStyle::Button)
                        .color(text_color),
                ));
            });
            ui.add_space(margin.y);
            title_height = ui.min_size().y;

            // First pass: Draw the inner fields. Compute port heights
            let inputs = self.graph[self.node_id].inputs.clone();
            for (param_name, param_id) in inputs {
                if self.graph[param_id].shown_inline {
                    let height_before = ui.min_rect().bottom();
                    if self.graph.connection(param_id).is_some() {
                        ui.label(param_name);
                    } else {
                        responses.extend(
                            self.graph[param_id]
                                .value
                                .value_widget(&param_name, ui)
                                .into_iter()
                                .map(NodeResponse::User),
                        );
                    }
                    let height_after = ui.min_rect().bottom();
                    input_port_heights.push((height_before + height_after) / 2.0);
                }
            }

            let outputs = self.graph[self.node_id].outputs.clone();
            for (param_name, _param) in outputs {
                let height_before = ui.min_rect().bottom();
                ui.label(&param_name);
                let height_after = ui.min_rect().bottom();
                output_port_heights.push((height_before + height_after) / 2.0);
            }

            responses.extend(
                self.graph[self.node_id]
                    .user_data
                    .bottom_ui(ui, self.node_id, self.graph, user_state)
                    .into_iter(),
            );
        });

        // Second pass, iterate again to draw the ports. This happens outside
        // the child_ui because we want ports to overflow the node background.

        let outer_rect = child_ui.min_rect().expand2(margin);
        let port_left = outer_rect.left();
        let port_right = outer_rect.right();

        #[allow(clippy::too_many_arguments)]
        fn draw_port<NodeData, DataType, ValueType, UserResponse, UserState>(
            ui: &mut Ui,
            graph: &Graph<NodeData, DataType, ValueType>,
            node_id: NodeId,
            user_state: &UserState,
            port_pos: Pos2,
            responses: &mut Vec<NodeResponse<UserResponse>>,
            param_id: AnyParameterId,
            port_locations: &mut PortLocations,
            ongoing_drag: Option<(NodeId, AnyParameterId)>,
            is_connected_input: bool,
        ) where
            DataType: DataTypeTrait<UserState>,
            UserResponse: UserResponseTrait,
        {
            let port_type = graph.any_param_type(param_id).unwrap();

            let port_rect = Rect::from_center_size(port_pos, egui::vec2(10.0, 10.0));

            let sense = if ongoing_drag.is_some() {
                Sense::hover()
            } else {
                Sense::click_and_drag()
            };

            let resp = ui.allocate_rect(port_rect, sense);
            let port_color = if resp.hovered() {
                Color32::WHITE
            } else {
                port_type.data_type_color(user_state)
            };
            ui.painter()
                .circle(port_rect.center(), 5.0, port_color, Stroke::none());

            if resp.drag_started() {
                if is_connected_input {
                    responses.push(NodeResponse::DisconnectEvent(param_id.assume_input()));
                } else {
                    responses.push(NodeResponse::ConnectEventStarted(node_id, param_id));
                }
            }

            if let Some((origin_node, origin_param)) = ongoing_drag {
                if origin_node != node_id {
                    // Don't allow self-loops
                    if graph.any_param_type(origin_param).unwrap() == port_type
                        && resp.hovered()
                        && ui.input().pointer.any_released()
                    {
                        responses.push(NodeResponse::ConnectEventEnded(param_id));
                    }
                }
            }

            port_locations.insert(param_id, port_rect.center());
        }

        // Input ports
        for ((_, param), port_height) in self.graph[self.node_id]
            .inputs
            .iter()
            .zip(input_port_heights.into_iter())
        {
            let should_draw = match self.graph[*param].kind() {
                InputParamKind::ConnectionOnly => true,
                InputParamKind::ConstantOnly => false,
                InputParamKind::ConnectionOrConstant => true,
            };

            if should_draw {
                let pos_left = pos2(port_left, port_height);
                draw_port(
                    ui,
                    self.graph,
                    self.node_id,
                    user_state,
                    pos_left,
                    &mut responses,
                    AnyParameterId::Input(*param),
                    self.port_locations,
                    self.ongoing_drag,
                    self.graph.connection(*param).is_some(),
                );
            }
        }

        // Output ports
        for ((_, param), port_height) in self.graph[self.node_id]
            .outputs
            .iter()
            .zip(output_port_heights.into_iter())
        {
            let pos_right = pos2(port_right, port_height);
            draw_port(
                ui,
                self.graph,
                self.node_id,
                user_state,
                pos_right,
                &mut responses,
                AnyParameterId::Output(*param),
                self.port_locations,
                self.ongoing_drag,
                false,
            );
        }

        // Draw the background shape.
        // NOTE: This code is a bit more involve than it needs to be because egui
        // does not support drawing rectangles with asymmetrical round corners.

        let (shape, outline) = {
            let rounding_radius = 4.0;
            let rounding = Rounding::same(rounding_radius);

            let titlebar_height = title_height + margin.y;
            let titlebar_rect =
                Rect::from_min_size(outer_rect.min, vec2(outer_rect.width(), titlebar_height));
            let titlebar = Shape::Rect(RectShape {
                rect: titlebar_rect,
                rounding,
                fill: self.graph[self.node_id]
                    .user_data
                    .titlebar_color(ui, self.node_id, self.graph, user_state)
                    .unwrap_or_else(|| background_color.lighten(0.8)),
                stroke: Stroke::none(),
            });

            let body_rect = Rect::from_min_size(
                outer_rect.min + vec2(0.0, titlebar_height - rounding_radius),
                vec2(outer_rect.width(), outer_rect.height() - titlebar_height),
            );
            let body = Shape::Rect(RectShape {
                rect: body_rect,
                rounding: Rounding::none(),
                fill: background_color,
                stroke: Stroke::none(),
            });

            let bottom_body_rect = Rect::from_min_size(
                body_rect.min + vec2(0.0, body_rect.height() - titlebar_height * 0.5),
                vec2(outer_rect.width(), titlebar_height),
            );
            let bottom_body = Shape::Rect(RectShape {
                rect: bottom_body_rect,
                rounding,
                fill: background_color,
                stroke: Stroke::none(),
            });

            let outline = if self.selected {
                Shape::Rect(RectShape {
                    rect: titlebar_rect
                        .union(body_rect)
                        .union(bottom_body_rect)
                        .expand(1.0),
                    rounding,
                    fill: Color32::WHITE.lighten(0.8),
                    stroke: Stroke::none(),
                })
            } else {
                Shape::Noop
            };

            (Shape::Vec(vec![titlebar, body, bottom_body]), outline)
        };

        ui.painter().set(background_shape, shape);
        ui.painter().set(outline_shape, outline);

        // --- Interaction ---

        // Titlebar buttons
        if Self::close_button(ui, outer_rect).clicked() {
            responses.push(NodeResponse::DeleteNode(self.node_id));
        };

        let window_response = ui.interact(
            outer_rect,
            Id::new((self.node_id, "window")),
            Sense::click_and_drag(),
        );

        // Movement
        *self.position += window_response.drag_delta();
        if window_response.drag_delta().length_sq() > 0.0 {
            responses.push(NodeResponse::RaiseNode(self.node_id));
        }

        // Node selection
        //
        // HACK: Only set the select response when no other response is active.
        // This prevents some issues.
        if responses.is_empty() && window_response.clicked_by(PointerButton::Primary) {
            responses.push(NodeResponse::SelectNode(self.node_id));
            responses.push(NodeResponse::RaiseNode(self.node_id));
        }

        responses
    }

    fn close_button(ui: &mut Ui, node_rect: Rect) -> Response {
        // Measurements
        let margin = 8.0;
        let size = 10.0;
        let stroke_width = 2.0;
        let offs = margin + size / 2.0;

        let position = pos2(node_rect.right() - offs, node_rect.top() + offs);
        let rect = Rect::from_center_size(position, vec2(size, size));
        let resp = ui.allocate_rect(rect, Sense::click());

        let dark_mode = ui.visuals().dark_mode;
        let color = if resp.clicked() {
            if dark_mode {
                color_from_hex("#ffffff").unwrap()
            } else {
                color_from_hex("#000000").unwrap()
            }
        } else if resp.hovered() {
            if dark_mode {
                color_from_hex("#dddddd").unwrap()
            } else {
                color_from_hex("#222222").unwrap()
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if dark_mode {
                color_from_hex("#aaaaaa").unwrap()
            } else {
                color_from_hex("#555555").unwrap()
            }
        };
        let stroke = Stroke {
            width: stroke_width,
            color,
        };

        ui.painter()
            .line_segment([rect.left_top(), rect.right_bottom()], stroke);
        ui.painter()
            .line_segment([rect.right_top(), rect.left_bottom()], stroke);

        resp
    }
}
