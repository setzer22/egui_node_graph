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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// A port that displays vertically.
#[derive(Debug)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct VerticalPort<DataType> {
    /// Name of the port. This will be displayed next to the port icon.
    pub label: String,
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
    hooks: SlotMap<HookId, Option<ConnectionToken>>,
    /// The next hook that's available for a connection
    available_hook: Option<HookId>,
}

pub struct VerticalOutputPort<DataType: DataTypeTrait> {
    pub base: VerticalPort<DataType>,
}

impl<DataType: DataTypeTrait> VerticalOutputPort<DataType> {
    pub fn new(
        label: String,
        data_type: DataType,
        connection_limit: Option<usize>,
    ) -> Self {
        let mut result = Self {
            base: VerticalPort {
                label,
                data_type,
                connection_limit,
                side: Side::Right,
                hooks: SlotMap::with_key(),
                available_hook: None
            }
        };
        result.base.consider_new_available_hook();
        result
    }

    pub fn iter_hooks(&self) -> impl Iterator<Item=(HookId, Option<ConnectionId>)> + '_ {
        self.base.iter_hooks()
    }
}

pub struct VerticalInputPort<DataType: DataTypeTrait> {
    /// The input kind. See [`InputKind`]
    pub kind: InputKind,
    pub default_value: Option<DataType::Value>,
    pub base: VerticalPort<DataType>,
}

impl<DataType: DataTypeTrait> VerticalInputPort<DataType> {
    pub fn new(
        label: String,
        data_type: DataType,
        connection_limit: Option<usize>,
        kind: InputKind,
    ) -> Self {
        let mut result = Self {
            kind,
            default_value: None,
            base: VerticalPort {
                label,
                data_type,
                connection_limit,
                side: Side::Left,
                hooks: SlotMap::with_key(),
                available_hook: None
            }
        };
        result.base.consider_new_available_hook();
        result
    }

    pub fn with_default_value(mut self, default_value: DataType::Value) -> Self {
        self.default_value = Some(default_value);
        self
    }

    pub fn iter_hooks(&self) -> impl Iterator<Item=(HookId, Option<ConnectionId>)> + '_ {
        self.base.iter_hooks()
    }

    pub fn using_default_value(&self) -> Option<DataType::Value> {
        match self.kind {
            InputKind::ConnectionOnly => {
                None
            }
            InputKind::ConnectionOrConstant => {
                if self.base.hooks.is_empty() {
                    self.default_value.clone()
                } else {
                    None
                }
            }
            InputKind::ConstantOnly => {
                self.default_value.clone()
            }
        }
    }
}

impl<DataType: DataTypeTrait> VerticalPort<DataType> {

    pub fn iter_hooks(&self) -> impl Iterator<Item=(HookId, Option<ConnectionId>)> + '_ {
        self.hooks.iter().map(|(id, token)| (id, token.as_ref().map(|t| t.connected_to())))
    }

    fn tangent(&self) -> egui::Vec2 {
        match self.side {
            Side::Left => egui::vec2(-1.0, 0.0),
            Side::Right => egui::vec2(1.0, 0.0),
        }
    }

    pub fn show_impl(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &mut EditorUiState<DataType>,
        context: &dyn GraphStyleTrait<DataType=DataType>,
        show_value: Option<&mut DataType::Value>,
    ) -> (egui::Rect, Vec<PortResponse<DataType>>) {
        let (outer_left, outer_right) = (ui.min_rect().left(), ui.min_rect().right());
        let mut value_rect_opt = None;
        let mut value_responses = Vec::<<DataType::Value as ValueTrait>::Response>::default();
        let label_rect = ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(&self.label);
            if let Some(value) = show_value {
                if self.hooks.len() == 0 || (self.hooks.len() == 1 && self.available_hook.is_some()) {
                    // There are no connections to this port, so we should show
                    // the value input widget.
                    let (value_rect, value_resp) = value.show(ui);
                    value_rect_opt = Some(value_rect);
                    value_responses = value_resp;
                }
            }
        }).response.rect;

        let row_rect = if let Some(value_rect) = value_rect_opt {
            label_rect.union(value_rect)
        } else {
            label_rect
        };

        let (hook_x, port_edge_dx) = {
            match self.side {
                Side::Right => {
                    (outer_right - 6.0, -10.0)
                }
                Side::Left => {
                    (outer_left + 6.0, 10.0)
                }
            }
        };

        let edge_width = 1_f32;
        let hook_spacing = 1_f32;
        let radius = 5_f32;
        let single_hook = self.connection_limit.filter(|v| *v == 1).is_some();
        let hook_count = self.hooks.len();
        let height_for_hooks: f32 = 2.0*edge_width + (2.0*radius + hook_spacing)*hook_count as f32 + hook_spacing;
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
                label_rect.y_range().start() + edge_width + hook_spacing + radius
            } else {
                // The hooks should be centered in the port
                (label_rect.y_range().start() + label_rect.y_range().end())/2.0 - height_for_hooks/2.0 + edge_width + hook_spacing + radius
            }
        };

        let (port_color, default_hook_color, hook_color_map, port_response): (_, _, HashMap<HookId, egui::Color32>, _) = 'port: {
            // TODO(@mxgrey): It would be nice to move all this logic into its own
            // utility function that can be used by different types of ports.
            // That function would probably want to take in a port_rect and a
            // hook_rect_iterator argument.

            // NOTE: We must allocate the hook rectangles before allocating the
            // full port rectangles so that egui gives priority to the hooks
            // over the port. For some reason the UI prioritizes sensing for
            // the rectangles that are allocated sooner.
            let hook_selected: Option<(HookId, egui::Response)> = {
                let mut next_hook_y = top_hook_y;
                self.hooks.iter().find_map(|(hook_id, _)| {
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
                    } else {
                        None
                    }
                })
            };

            let ui_port_response = ui.allocate_rect(port_rect, egui::Sense::click_and_drag());
            if let Some((dragged_connection, dragged_data_type)) = &state.ongoing_drag {
                let dragged_port: (NodeId, PortId) = dragged_connection.clone().into();
                if dragged_port == id {
                    // The port that is being dragged is this one. We should use
                    // the acceptance color while it is being dragged
                    let accept_color = context.recommend_port_accept_color(ui, id);
                    break 'port (accept_color, accept_color, HashMap::default(), None);
                }

                if let Some(available_hook) = self.available_hook {
                    let connection_possible = PortResponse::connect_event_ended(
                        ConnectionId(id.0, id.1, available_hook),
                        *dragged_connection,
                    );
                    if let Some(connection_possible) = connection_possible {
                        if dragged_data_type.is_compatible(&self.data_type) && dragged_port.0 != id.0 {
                            if ui_port_response.hovered() {
                                let resp = if ui.input().pointer.any_released() {
                                    Some(connection_possible)
                                } else {
                                    None
                                };

                                let accept_color = context.recommend_port_accept_color(ui, id);

                                // The port can accept or has accepted the connection
                                break 'port (accept_color, accept_color, HashMap::default(), resp);
                            }

                            // The connection is compatible but the user needs to
                            // drag it over to the port
                            break 'port (
                                context.recommend_compatible_port_color(ui, id),
                                context.recommend_data_type_color(&self.data_type.clone().into()),
                                HashMap::default(),
                                None,
                            );
                        }
                    }
                }

                // A connection is not possible, either because all the hooks
                // are filled or because the data type that's being dragged
                // is incompatible
                break 'port (
                    context.recommend_incompatible_port_color(ui, id),
                    context.recommend_data_type_color(&self.data_type.clone().into()),
                    HashMap::default(),
                    None,
                );
            }

            if let Some((hook_selected, hook_resp)) = hook_selected {
                if self.available_hook.filter(|h| *h == hook_selected).is_some() {
                    if hook_resp.drag_started() {
                        let accept_color = context.recommend_port_accept_color(ui, id);
                        break 'port (
                            accept_color,
                            context.recommend_data_type_color(&self.data_type),
                            HashMap::from_iter([(hook_selected, accept_color)]),
                            Some(PortResponse::ConnectEventStarted(ConnectionId(id.0, id.1, hook_selected))),
                        );
                    }

                    if hook_resp.hovered() {
                        // The user is hovering over the available hook. Show
                        // the user that we see the hovering.
                        let hover_color = context.recommend_port_hover_color(ui, id);
                        break 'port (
                            hover_color,
                            context.recommend_data_type_color(&self.data_type),
                            HashMap::from_iter([(hook_selected, hover_color)]),
                            None,
                        );
                    }
                } else {
                    // The user is interacting with a hook that is part of a
                    // connection
                    if hook_resp.drag_started() {
                        // Dragging from a connected hook. Begin the connection
                        // moving event.
                        let accept_color = context.recommend_port_accept_color(ui, id);
                        break 'port (
                            accept_color,
                            accept_color,
                            HashMap::default(),
                            Some(PortResponse::MoveEvent(ConnectionId(id.0, id.1, hook_selected))),
                        );
                    }

                    if hook_resp.hovered() {
                        // Hovering over a connected hook. Show the user that we
                        // see the hovering.
                        let hover_color = context.recommend_port_hover_color(ui, id);
                        break 'port (
                            context.recommend_node_background_color(ui, id.0),
                            context.recommend_data_type_color(&self.data_type),
                            HashMap::from_iter([(hook_selected, hover_color)]),
                            None,
                        );
                    }
                }
            }

            if ui_port_response.hovered() {
                if let Some(available_hook) = self.available_hook {
                    // The user is hovering a port with an available hook.
                    // Show the user that we see the hovering.
                    let hover_color = context.recommend_port_hover_color(ui, id);
                    break 'port (
                        hover_color,
                        context.recommend_data_type_color(&self.data_type),
                        HashMap::from_iter([(available_hook, hover_color)]),
                        None,
                    );
                } else {
                    // The user is hovering over a port that does not have an
                    // available hook.
                    let hover_color = context.recommend_incompatible_port_color(ui, id);
                    break 'port (
                        hover_color,
                        context.recommend_data_type_color(&self.data_type),
                        HashMap::default(),
                        None,
                    );
                }
            }

            if ui_port_response.drag_started() {
                if let Some(available_hook) = self.available_hook {
                    // The user has started to drag a new connection from the
                    // port.
                    let accept_color = context.recommend_port_accept_color(ui, id);
                    break 'port (
                        accept_color,
                        accept_color,
                        HashMap::default(),
                        Some(PortResponse::ConnectEventStarted(ConnectionId(id.0, id.1, available_hook))),
                    );
                }
            }

            if ui_port_response.drag_started() || ui_port_response.dragged() {
                if self.available_hook.is_none() {
                    // The user is trying to drag on a port that has no available hook
                    let reject_color = context.recommend_port_reject_color(ui, id);
                    break 'port (
                        reject_color,
                        context.recommend_data_type_color(&self.data_type),
                        HashMap::default(),
                        None,
                    );
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
            let node_color = context.recommend_node_background_color(ui, id.0);
            ui.painter().rect(port_rect, egui::Rounding::default(), port_color, (0_f32, node_color));
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
            ui.painter().circle(p, radius, *color, (0_f32, *color));
            state.hook_geometry.insert(ConnectionId(id.0, id.1, hook_id), (p, self.tangent()));
            next_hook_y += hook_spacing + 2.0*radius;
        }

        let responses = value_responses.into_iter().map(PortResponse::Value)
            .chain([port_response].into_iter().filter_map(|r| r)).collect();
        return (row_rect.union(port_rect), responses);
    }

    pub fn consider_new_available_hook(&mut self) {
        if self.available_hook.is_none() {
            if self.connection_limit.filter(|limit| *limit <= self.hooks.len()).is_none() {
                self.available_hook = Some(self.hooks.insert(None));
            }
        }
    }
}

impl<DataType: DataTypeTrait> PortTrait for VerticalPort<DataType> {
    type DataType = DataType;

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &mut EditorUiState<Self::DataType>,
        style: &dyn GraphStyleTrait<DataType=Self::DataType>,
    ) -> (egui::Rect, Vec<PortResponse<DataType>>) {
        self.show_impl(ui, id, state, style, None)
    }

    fn data_type(&self) -> Self::DataType {
        self.data_type.clone()
    }

    fn available_hook(&self) -> Option<HookId> {
        self.available_hook
    }

    fn connect(&mut self, from: HookId, to: graph::ConnectionToken) -> Result<(), PortAddConnectionError> {
        let connection = match self.hooks.get_mut(from) {
            Some(connection) => connection,
            None => return Err(PortAddConnectionError::BadHook(from)),
        };

        *connection = Some(to);
        if self.available_hook == Some(from) {
            // We are now using up the available hook, so we should decide
            // whether to clear it or replace it.
            self.available_hook = None;
            self.consider_new_available_hook();
        }

        Ok(())
    }

    fn drop_connection(&mut self, id: HookId) -> Result<ConnectionId, PortDropConnectionError> {
        let connection = match self.hooks.get(id) {
            Some(Some(connection)) => connection,
            Some(None) => return Err(PortDropConnectionError::NoConnection(id)),
            None => return Err(PortDropConnectionError::BadHook(id)),
        }.connected_to();

        self.hooks.remove(id);
        self.consider_new_available_hook();

        Ok(connection)
    }

    fn drop_all_connections(&mut self) -> Vec<(HookId, ConnectionId)> {
        let mut dropped = Vec::new();
        for (id, connection) in &self.hooks {
            if let Some(connection) = connection {
                dropped.push((id, connection.connected_to()));
            }
        }

        self.hooks.clear();
        self.consider_new_available_hook();

        dropped
    }
}

impl<DataType: DataTypeTrait> PortTrait for VerticalInputPort<DataType> {
    type DataType = DataType;

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &mut EditorUiState<Self::DataType>,
        style: &dyn GraphStyleTrait<DataType=Self::DataType>,
    ) -> (egui::Rect, Vec<PortResponse<DataType>>) {
        match self.kind {
            InputKind::ConnectionOnly => {
                self.base.show_impl(ui, id, state, style, None)
            },
            InputKind::ConstantOnly => {
                let label_rect = ui.label(&self.base.label).rect;
                if let Some(default_value) = &mut self.default_value {
                    let (value_rect, value_resp) = default_value.show(ui);
                    (
                        label_rect.union(value_rect),
                        value_resp.into_iter().map(PortResponse::Value).collect(),
                    )
                } else {
                    (label_rect, Vec::new())
                }
            },
            InputKind::ConnectionOrConstant => {
                self.base.show_impl(ui, id, state, style, self.default_value.as_mut())
            }
        }
    }

    fn data_type(&self) -> Self::DataType {
        self.base.data_type.clone()
    }

    fn available_hook(&self) -> Option<HookId> {
        self.base.available_hook
    }

    fn connect(&mut self, from: HookId, to: graph::ConnectionToken) -> Result<(), PortAddConnectionError> {
        self.base.connect(from, to)
    }

    fn drop_all_connections(&mut self) -> Vec<(HookId, ConnectionId)> {
        self.base.drop_all_connections()
    }

    fn drop_connection(&mut self, id: HookId) -> Result<ConnectionId, PortDropConnectionError> {
        self.base.drop_connection(id)
    }
}

impl<DataType: DataTypeTrait> PortTrait for VerticalOutputPort<DataType> {
    type DataType = DataType;

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &mut EditorUiState<Self::DataType>,
        style: &dyn GraphStyleTrait<DataType=Self::DataType>,
    ) -> (egui::Rect, Vec<PortResponse<Self::DataType>>) {
        self.base.show(ui, id, state, style)
    }

    fn available_hook(&self) -> Option<HookId> {
        self.base.available_hook()
    }

    fn connect(&mut self, from: HookId, to: graph::ConnectionToken) -> Result<(), PortAddConnectionError> {
        self.base.connect(from, to)
    }

    fn data_type(&self) -> Self::DataType {
        self.base.data_type()
    }

    fn drop_all_connections(&mut self) -> Vec<(HookId, ConnectionId)> {
        self.base.drop_all_connections()
    }

    fn drop_connection(&mut self, id: HookId) -> Result<ConnectionId, PortDropConnectionError> {
        self.base.drop_connection(id)
    }
}
