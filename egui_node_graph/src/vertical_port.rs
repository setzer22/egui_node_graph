use super::*;
use std::collections::HashMap;
use crate::utils::ColorUtils;

/// The three kinds of input params. These describe how the graph must behave
/// with respect to inline widgets and connections for this parameter.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum InputKind {
    /// No constant value can be set. Only incoming connections can produce it
    ConnectionOnly,
    /// Only a constant value can be set. No incoming connections accepted.
    ConstantOnly,
    /// Both incoming connections and constants are accepted. Connections take
    /// precedence over the constant values.
    ConnectionOrConstant,
}

pub enum Side {
    Left,
    Right,
}

/// A port that displays vertically.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct VerticalPort<DataType> {
    /// Name of the port. This will be displayed next to the port icon.
    pub name: String,
    /// The data type of this node. Used to determine incoming connections. This
    /// should always match the type of the InputParamValue, but the property is
    /// not actually enforced.
    pub data_type: DataType,
    /// The limit on number of connections this port allows. A None value means
    /// there is no limit.
    connection_limit: Option<usize>,
    /// What side of the block will this port be rendered on
    pub side: Side,
    /// List of existing hooks and whether or not they have a connection
    hooks: SlotMap<HookId, ()>,
    /// The next hook that's available for a connection
    available_hook: Option<HookId>,
    /// When true, the node is shown inline inside the node graph.
    #[cfg_attr(feature = "persistence", serde(default = "shown_inline_default"))]
    pub shown_inline: bool,
}

pub struct VerticalInputPort<DataType: DataTypeTrait> {
    /// The input kind. See [`InputKind`]
    pub kind: InputKind,
    pub default_value: Option<DataType::Value>,
    pub port: VerticalPort<DataType>,
}

impl<DataType: DataTypeTrait> VerticalPort<DataType> {
    pub fn show_impl<Node>(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &NodeUiState<DataTypeOf<Node>>,
        context: &dyn GraphContext<Node=Node>,
        show_value: Option<&DataType::Value>,
    ) -> (egui::Rect, Vec<PortResponse<Node>>)
    where
        DataType: Into<DataTypeOf<Node>>,
        DataType::Value: ValueTrait,
    {
        let value_rect_opt = None;
        let mut value_resp = Vec::<DataType::Value::Response>::default();
        let label_rect = ui.horizontal(|ui| {
            ui.add_space(15);
            ui.label(self.name);
            if let Some(value) = show_value {
                if self.hooks.len() == 0 || (self.hooks.len() == 1 && self.available_hook.is_some()) {
                    // There are no connections to this port, so we should show
                    // the value input widget.
                    let (value_rect, value_resp) = value.show(ui);
                    value_rect_opt = Some(value_rect);
                }
            }
        }).response.rect;

        let row_rect = if let Some(value_rect) = value_rect_opt {
            label_rect.union(value_rect)
        } else {
            label_rect
        };

        let (hook_x, port_edge_dx) = {
            if ui.layout().horizontal_align() == egui::Align::RIGHT {
                (row_rect.right(), -10.0)
            } else {
                (row_rect.left(), 10.0)
            }
        };

        let edge_width = 1_f32;
        let hook_spacing = 1_f32;
        let radius = 5_f32;
        let single_hook = self.connection_limit.is_some_with(|v| *v == 1);
        let hook_count = self.hooks.len();
        let height_for_hooks: f32 = 2.0*edge_width + (2.0*radius + hook_spacing)*hook_count + hook_spacing;
        let port_rect = {
            let height = label_rect.height().max(height_for_hooks);
            if port_edge_dx > 0.0 {
                egui::Rect::from_min_size(
                    egui::pos2(hook_x, label_rect.top()),
                    egui::vec2(port_edge_dx, height),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(hook_x + port_edge_dx, label_rect.top()),
                    egui::vec2(-port_edge_dx, height),
                )
            }
        };

        let top_hook_y = {
            if height_for_hooks >= label_rect.height() {
                // The top hook needs to be as high in the port as possible
                label_rect.min.y + edge_width + hook_spacing + radius
            }

            // The hooks should be centered in the port
            height_for_hooks/2.0 - edge_width - hook_spacing
        };

        let (port_color, default_hook_color, hook_color_map, port_response) = {
            // TODO(@mxgrey): It would be nice to move all this logic into its own
            // utility function that can be used by different types of ports.
            // That function would probably want to take in a port_rect and a
            // hook_rect_iterator argument.
            let ui_port_response = ui.allocate_rect(port_rect, egui::Sense::click_and_drag());
            if let Some((dragged_connection, dragged_data_type)) = state.ongoing_drag {
                let dragged_port: (NodeId, PortId) = dragged_connection.into();
                if dragged_port == id {
                    // The port that is being dragged is this one. We should use
                    // the acceptance color while it is being dragged
                    let accept_color = context.recommend_port_accept_color(ui, id);
                    (accept_color, accept_color, HashMap::default(), None)
                }

                if let Some(available_hook) = self.available_hook {
                    let connection_possible = PortResponse::connect_event_ended(
                        ConnectionId::new(id.0, id.1, available_hook),
                        dragged_connection,
                    );
                    if let Some(connection_possible) = connection_possible {
                        let dragged_data_type: DataType = dragged_data_type.into();
                        if dragged_data_type.is_compatible(&self.data_type) {
                            if ui_port_response.hovered() || ui_port_response.drag_released() {
                                let resp = if ui_port_response.drag_released() {
                                    if self.connection_limit <= Some(hook_count) {
                                        // This port cannot support any more connections
                                        self.available_hook = None;
                                    } else {
                                        // Create a new available port since the currently
                                        // available one is about to be consumed
                                        self.available_hook = Some(self.hooks.insert(()));
                                    }

                                    Some(connection_possible)
                                } else {
                                    None
                                };

                                let accept_color = context.recommend_port_accept_color(ui, id);

                                // The port can accept or has accepted the connection
                                (accept_color, accept_color, HashMap::default(), resp)
                            }

                            // The connection is compatible but the user needs to
                            // drag it over to the port
                            (
                                context.recommend_compatible_port_color(ui, id),
                                context.recommend_data_type_color(&self.data_type),
                                HashMap::default(),
                                None,
                            )
                        }
                    }

                }

                // A connection is not possible, either because all the hooks
                // are filled or because the data type that's being dragged
                // is incompatible
                (
                    context.recommend_incompatible_port_color(ui, id),
                    context.recommend_data_type_color(&self.data_type),
                    HashMap::default(),
                    None,
                )
            }

            let hook_selected: Option<(HookId, egui::Response)> = {
                let mut next_hook_y = top_hook_y;
                self.hooks.iter().find_map(|(hook_id, ())| {
                    let hook_y = next_hook_y;
                    next_hook_y += hook_spacing + 2.0*radius;
                    let resp = ui.allocate_rect(
                        egui::Rect::from_center_size(
                            egui::pos2(hook_x, hook_y),
                            egui::vec2(2.0*radius, 2.0*radius),
                        ),
                        egui::Sense::click_and_drag(),
                    );

                    if resp.hovered() || resp.drag_released() || resp.drag_started() {
                        Some((hook_id, resp))
                    }

                    None
                })
            };

            if let Some((hook_selected, hook_resp)) = hook_selected {
                if self.available_hook.is_some_with(|h| h == hook_selected) {
                    // The user is interacting with the available hook, so we
                    // should treat it as possibly creating a connection
                    if hook_resp.hovered() {
                        // The user is hovering over the available hook. Show
                        // the user that we see the hovering.
                        let hover_color = context.recommend_port_hover_color(ui, id);
                        (
                            hover_color,
                            context.recommend_data_type_color(&self.data_type),
                            HashMap::from_iter([(hook_selected, hover_color)]),
                            None,
                        )
                    }

                    if hook_resp.drag_started() {
                        let accept_color = context.recommend_port_accept_color(ui, id);
                        (
                            accept_color,
                            context.recommend_data_type_color(&self.data_type),
                            HashMap::from_iter([(hook_selected, accept_color)]),
                            Some(PortResponse::ConnectEventStarted(ConnectionId::new(id.0, id.1, hook_selected))),
                        )
                    }
                } else {
                    // The user is interacting with a hook that is part of a
                    // connection
                    if hook_resp.hovered() {
                        // Hovering over a connected hook. Show the user that we
                        // see the hovering.
                        let hover_color = context.recommend_port_hover_color(ui, id);
                        (
                            context.recommend_node_background_color(ui, id.0),
                            context.recommend_data_type_color(&self.data_type),
                            HashMap::from_iter([(hook_selected, hover_color)]),
                            None,
                        )
                    }

                    if hook_resp.drag_started() {
                        // Dragging from a connected hook. Begin the connection
                        // moving event.
                        let accept_color = context.recommend_port_accept_color(ui, id);
                        (
                            accept_color,
                            accept_color,
                            HashMap::default(),
                            Some(PortResponse::MoveEvent(ConnectionId::new(id.0, id.1, hook_selected))),
                        )
                    }
                }
            }

            if ui_port_response.hovered() {
                if let Some(available_hook) = self.available_hook {
                    // The user is hovering a port with an available hook.
                    // Show the user that we see the hovering.
                    let hover_color = context.recommend_port_hover_color(ui, id);
                    (
                        hover_color,
                        context.recommend_data_type_color(&self.data_type),
                        HashMap::from_iter([(available_hook, hover_color)]),
                        None,
                    )
                } else {
                    // The user is hovering over a port that does not have an
                    // available hook.
                    let hover_color = context.recommend_incompatible_port_color(ui, id);
                    (
                        hover_color,
                        context.recommend_data_type_color(&self.data_type),
                        HashMap::default(),
                        None,
                    )
                }
            }

            if ui_port_response.drag_started() {
                if let Some(available_hook) = self.available_hook {
                    // The user has started to drag a new connection from the
                    // port.
                    let accept_color = context.recommend_port_accept_color(ui, id);
                    (
                        accept_color,
                        accept_color,
                        HashMap::default(),
                        Some(PortResponse::ConnectEventStarted(ConnectionId::new(id.0, id.1, available_hook))),
                    )
                }
            }

            if ui_port_response.drag_started() || ui_port_response.dragged() {
                if self.available_hook.is_none() {
                    // The user is trying to drag on a port that has no available hook
                    let reject_color = context.recommend_port_reject_color(ui, id);
                    (
                        reject_color,
                        context.recommend_data_type_color(&self.data_type),
                        HashMap::default(),
                        None,
                    )
                }
            }

            // Nothing special is happening with this port
            (
                context.recommend_node_background_color(ui, id.0),
                context.recommend_data_type_color(&self.data_type),
                HashMap::default(),
                None,
            )
        };

        if !single_hook {
            // The port has multiple hooks so we'll draw the port rect
            ui.painter().rect(port_rect, Default::default(), port_color, None);
            let node_color = context.recommend_node_background_color(ui, id.0);
            let dark_stroke = (edge_width/2.0, node_color.lighten(0.8));
            let light_stroke = (edge_width/2.0, node_color.lighten(1.2));

            ui.painter().line_segment(
                [
                    egui::pos2(hook_x, port_rect.top()),
                    egui::pos2(hook_x + port_edge_dx, port_rect.top()),
                ], dark_stroke
            );
            ui.painter().line_segment(
                [
                    egui::pos2(hook_x, port_rect.bottom() + edge_width/2.0),
                    egui::pos2(hook_x + port_edge_dx, port_rect.bottom() + edge_width/2.0),
                ], dark_stroke
            );

            ui.painter().line_segment(
                [
                    egui::pos2(hook_x, port_rect.top() + edge_width/2.0),
                    egui::pos2(hook_x + port_edge_dx, port_rect.top() + edge_width/2.0),
                ], light_stroke
            );
            ui.painter().line_segment(
                [
                    egui::pos2(hook_x, port_rect.bottom()),
                    egui::pos2(hook_x + port_edge_dx, port_rect.bottom()),
                ], light_stroke
            );

            let (outer_stroke, inner_stroke, half_edge_dx) = if port_edge_dx > 0.0 {
                (light_stroke, dark_stroke, -edge_width/2.0)
            } else {
                (dark_stroke, light_stroke, edge_width/2.0)
            };

            ui.painter().line_segment(
                [
                    egui::pos2(hook_x + port_edge_dx, port_rect.top()),
                    egui::pos2(hook_x + port_edge_dx, port_rect.bottom()),
                ], outer_stroke,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(hook_x + port_edge_dx + half_edge_dx, port_rect.top() - edge_width/2.0),
                    egui::pos2(hook_x + port_edge_dx + half_edge_dx, port_rect.bottom() + edge_width/2.0),
                ], inner_stroke
            );
        }

        // Now draw the hooks and save their locations
        let mut next_hook_y = top_hook_y;
        for (hook_id, _) in &self.hooks {
            let color = hook_color_map.get(&hook_id).unwrap_or(&default_hook_color);
            let p = egui::pos2(hook_x, next_hook_y);
            ui.painter().circle(p, radius, color, None);
            state.hook_locations.insert(ConnectionId::new(id.0, id.1, hook_id), p);
            next_hook_y += hook_spacing + 2.0*radius;
        }

        let responses = value_resp.into_iter().map(PortResponse::Value)
            .chain([port_response].into_iter().filter_map(|r| r)).collect();
        return (row_rect.union(port_rect), responses);
    }
}

impl<DataType: DataTypeTrait> PortTrait for VerticalPort<DataType> {
    fn show<Node>(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &NodeUiState<DataTypeOf<Node>>,
        context: &dyn GraphContext<Node=Node>,
    ) -> (egui::Rect, Vec<PortResponse<Node>>)
    where
        DataType: Into<DataTypeOf<Node>>,
        DataType::Value: ValueTrait,
    {
        self.show_impl(ui, id, state, context, None)
    }
}

impl<DataType: DataTypeTrait> PortTrait for VerticalInputPort<DataType> {
    fn show<Node>(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &NodeUiState<DataTypeOf<Node>>,
        context: &dyn GraphContext<Node=Node>,
    ) -> (egui::Rect, Vec<PortResponse<Node>>)
    where
        DataType: Into<DataTypeOf<Node>>,
        DataType::Value: ValueTrait,
    {
        match self.kind {
            InputKind::ConnectionOnly => {
                self.port.show_impl(ui, id, state, context, None)
            },
            InputKind::ConstantOnly => {
                let label_rect = ui.label(self.port.name);
                if let Some(default_value) = &self.default_value {
                    let (value_rect, value_resp) = default_value.show(ui);
                    (
                        label_rect.union(value_rect),
                        [value_resp].into_iter().mapP(PortResponse::Value).collect(),
                    )
                }

                (label_rect, Vec::new())
            },
            InputKind::ConnectionOrConstant => {
                self.port.show_impl(ui, id, state, context, self.default_value)
            }
        }
    }
}
