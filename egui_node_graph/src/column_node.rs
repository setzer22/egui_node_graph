use super::*;
use crate::utils::ColorUtils;
use egui::*;
use epaint::RectShape;

pub type SimpleColumnNode<Content, DataType> =
    ColumnNode<Content, VerticalInputPort<DataType>, VerticalOutputPort<DataType>>;

/// A node inside the [`Graph`]. Nodes have input and output parameters, stored
/// as ids. They also contain a custom `NodeData` struct with whatever data the
/// user wants to store per-node.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct ColumnNode<Content, InputPort, OutputPort> {
    pub position: Pos2,
    pub label: String,
    pub content: Content,
    /// The input ports of the graph
    pub inputs: SlotMap<InputPortId, InputPort>,
    /// The output ports of the graph
    pub outputs: SlotMap<OutputPortId, OutputPort>,

    /// The size hint is used to automatically scale the widget to a desirable
    /// size while still allowing right-side ports to be justified to the right
    /// size of the node widget. If the desired size of a widget inside of the
    /// node's frame changes then the node size should be fixed after one bad
    /// rendering cycle.
    pub size_hint: f32,
}

impl<Content, InputPort, OutputPort> ColumnNode<Content, InputPort, OutputPort> {
    pub fn new(position: Pos2, label: String, content: Content) -> Self {
        Self {
            position,
            label,
            content,
            inputs: SlotMap::with_key(),
            outputs: SlotMap::with_key(),
            size_hint: 80.0,
        }
    }

    pub fn with_input(mut self, input: InputPort) -> Self {
        self.inputs.insert(input);
        self
    }

    pub fn with_output(mut self, output: OutputPort) -> Self {
        self.outputs.insert(output);
        self
    }

    pub fn with_size_hint(mut self, size_hint: f32) -> Self {
        self.size_hint = size_hint;
        self
    }
}

impl<Content, InputPort, OutputPort> NodeTrait for ColumnNode<Content, InputPort, OutputPort>
where
    Content: NodeContentTrait,
    InputPort: PortTrait,
    OutputPort: PortTrait<DataType=InputPort::DataType>,
{
    type DataType = InputPort::DataType;
    type Content = Content;

    fn show(
        &mut self,
        parent_ui: &mut egui::Ui,
        app: &<Self::Content as NodeContentTrait>::AppState,
        node_id: NodeId,
        state: &mut EditorUiState<Self::DataType>,
        style: &dyn GraphStyleTrait<DataType=Self::DataType>,
    ) -> Vec<NodeResponse<Self>> {
        let mut ui = parent_ui.child_ui_with_id_source(
            Rect::from_min_size(self.position + state.pan, [self.size_hint, 0.0].into()),
            Layout::default(),
            node_id,
        );

        let margin = egui::vec2(15.0, 5.0);
        let mut responses = Vec::<NodeResponse<Self>>::new();

        let background_color = style.recommend_node_background_color(&ui, node_id);
        let text_color = style.recommend_node_text_color(&ui, node_id);

        ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.0, text_color);

        // Forward declare shapes to paint below contents
        let outline_shape = ui.painter().add(Shape::Noop);
        let background_shape = ui.painter().add(Shape::Noop);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = {
            let mut inner_rect = outer_rect_bounds.shrink2(margin);

            // Try to use the size hint, unless our outer limits are smaller
            inner_rect.max.x = inner_rect.max.x.min(self.size_hint + inner_rect.min.x);

            // Make sure we don't shrink to the negative
            inner_rect.max.x = inner_rect.max.x.max(inner_rect.min.x);
            inner_rect.max.y = inner_rect.max.y.max(inner_rect.min.y);

            inner_rect
        };

        let mut title_height = 0.0;
        let mut child_ui = ui.child_ui(inner_rect, *ui.layout());
        child_ui.vertical(|ui| {
            let title_rect = ui.horizontal(|ui| {
                ui.add(Label::new(
                    RichText::new(&self.label)
                        .text_style(TextStyle::Button)
                        .color(style.recommend_node_text_color(ui, node_id)),
                ));
                ui.add_space(8.0); // The size of the little cross icon
            }).response.rect;
            self.size_hint = title_rect.width();
            ui.add_space(margin.y);
            title_height = ui.min_size().y;

            for (input_id, port) in &mut self.inputs {
                ui.horizontal(|ui| {
                    let (rect, port_responses): (egui::Rect, Vec<PortResponse<Self::DataType>>) = port.show(
                        ui, (node_id, PortId::Input(input_id)), state, style
                    );
                    responses.extend(port_responses.into_iter().map(NodeResponse::Port));
                    self.size_hint = self.size_hint.max(rect.width());
                });
            }

            for (output_id, port) in &mut self.outputs {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, port_responses): (egui::Rect, Vec<PortResponse<Self::DataType>>) = port.show(
                        ui, (node_id, PortId::Output(output_id)), state, style
                    );
                    responses.extend(port_responses.into_iter().map(NodeResponse::Port));
                    self.size_hint = self.size_hint.max(rect.width());
                });
            }

            let (rect, resp) = self.content.content_ui(ui, app, node_id);
            self.size_hint = self.size_hint.max(rect.width() + 3.0*margin.x);
            responses.extend(resp.into_iter().map(NodeResponse::Content));
        });

        let (shape, outline, outer_rect) = {
            let rounding_radius = 4.0;
            let rounding = Rounding::same(rounding_radius);

            let outer_rect = child_ui.min_rect().expand2(margin);
            let titlebar_height = title_height + margin.y;
            let titlebar_rect = Rect::from_min_size(
                outer_rect.min, vec2(outer_rect.width(), titlebar_height)
            );
            let titlebar = Shape::Rect(RectShape{
                rect: titlebar_rect,
                rounding,
                fill: self.content.titlebar_color(
                    &ui, app, node_id,
                ).unwrap_or_else(|| style.recommend_node_background_color(
                    &ui, node_id).lighten(0.8)
                ),
                stroke: Stroke::none(),
            });

            let body_rect = Rect::from_min_size(
                outer_rect.min + vec2(0.0, titlebar_height - rounding_radius),
                vec2(outer_rect.width(), outer_rect.height() - titlebar_height),
            );
            let body = Shape::Rect(RectShape{
                rect: body_rect,
                rounding: Rounding::none(),
                fill: background_color,
                stroke: Stroke::none(),
            });

            let bottom_body_rect = Rect::from_min_size(
                body_rect.min + vec2(0.0, body_rect.height() - titlebar_height * 0.5),
                vec2(outer_rect.width(), title_height),
            );
            let bottom_body = Shape::Rect(RectShape {
                rect: bottom_body_rect,
                rounding,
                fill: background_color,
                stroke: Stroke::none(),
            });

            let outline = if state.selected_nodes.contains(&node_id) {
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

            (Shape::Vec(vec![titlebar, body, bottom_body]), outline, outer_rect)
        };

        ui.painter().set(background_shape, shape);
        ui.painter().set(outline_shape, outline);

        // Make close button
        if {
            let margin = 8.0;
            let size = 10.0;
            let x_size = 8.0;
            let stroke_width = 2.0;
            let offset = margin + size / 2.0;

            let position = pos2(outer_rect.right() - offset, outer_rect.top() + offset);
            let rect = Rect::from_center_size(position, vec2(size, size));
            let x_rect = Rect::from_center_size(position, vec2(x_size, x_size));
            let resp = ui.allocate_rect(rect, Sense::click());

            let (stroke, fill) = if resp.dragged() {
                style.recommend_close_button_clicked_colors(&ui, node_id)
            } else if resp.hovered() {
                style.recommend_close_button_hover_colors(&ui, node_id)
            } else {
                style.recommend_close_button_passive_colors(&ui, node_id)
            };

            ui.painter().rect(rect, 0.5, fill, (0_f32, fill));

            let stroke = Stroke {
                width: stroke_width,
                color: stroke,
            };
            ui.painter().line_segment([x_rect.left_top(), x_rect.right_bottom()], stroke);
            ui.painter().line_segment([x_rect.right_top(), x_rect.left_bottom()], stroke);

            resp
        }.clicked() {
            responses.push(NodeResponse::DeleteNodeUi(node_id));
        }

        let window_response = ui.interact(
            outer_rect,
            Id::new((node_id, "window")),
            Sense::click_and_drag(),
        );

        // Movement
        self.position += window_response.drag_delta();
        if window_response.drag_delta().length_sq() > 0.0 {
            responses.push(NodeResponse::RaiseNode(node_id));
        }

        // Node selection
        if responses.is_empty() && window_response.clicked_by(PointerButton::Primary) {
            responses.push(NodeResponse::SelectNode(node_id));
            responses.push(NodeResponse::RaiseNode(node_id));
        }

        responses
    }

    fn port_data_type(&self, port_id: PortId) -> Option<Self::DataType> {
        match port_id {
            PortId::Input(port_id) => self.inputs.get(port_id).map(|p| p.data_type()),
            PortId::Output(port_id) => self.outputs.get(port_id).map(|p| p.data_type()),
        }
    }

    fn available_hook(&self, port_id: PortId) -> Option<HookId> {
        match port_id {
            PortId::Input(port_id) => self.inputs.get(port_id).map(|p| p.available_hook()).flatten(),
            PortId::Output(port_id) => self.outputs.get(port_id).map(|p| p.available_hook()).flatten(),
        }
    }

    fn drop_connection(
        &mut self,
        (port, hook): (PortId, HookId)
    ) -> Result<ConnectionId, NodeDropConnectionError> {
        match port {
            PortId::Input(input_port) => {
                match self.inputs.get_mut(input_port) {
                    Some(input_port) => {
                        input_port.drop_connection(hook).map_err(
                            |err| NodeDropConnectionError::PortError { port, err }
                        )
                    }
                    None => {
                        Err(NodeDropConnectionError::BadPort(port))
                    }
                }
            }
            PortId::Output(output_port) => {
                match self.outputs.get_mut(output_port) {
                    Some(output_port) => {
                        output_port.drop_connection(hook).map_err(
                            |err| NodeDropConnectionError::PortError { port, err }
                        )
                    }
                    None => {
                        Err(NodeDropConnectionError::BadPort(port))
                    }
                }
            }
        }
    }

    fn drop_all_connections(&mut self) -> Vec<(PortId, HookId, ConnectionId)> {
        let mut dropped = Vec::new();
        for (id, port) in &mut self.inputs {
            dropped.extend(port.drop_all_connections().into_iter().map(
                |(hook, connection)| (PortId::Input(id), hook, connection)
            ));
        }

        for (id, port) in &mut self.outputs {
            dropped.extend(port.drop_all_connections().into_iter().map(
                |(hook, connection)| (PortId::Output(id), hook, connection)
            ));
        }

        dropped
    }

    fn connect(&mut self, (port, hook): (PortId, HookId), to: graph::ConnectionToken) -> Result<(), NodeAddConnectionError> {
        match port {
            PortId::Input(input_port) => {
                match self.inputs.get_mut(input_port) {
                    Some(input_port) => {
                        input_port.connect(hook, to).map_err(
                            |err| NodeAddConnectionError::PortError { port, err }
                        )
                    }
                    None => Err(NodeAddConnectionError::BadPort(port))
                }
            }
            PortId::Output(output_port) => {
                match self.outputs.get_mut(output_port) {
                    Some(output_port) => {
                        output_port.connect(hook, to).map_err(
                            |err| NodeAddConnectionError::PortError { port, err }
                        )
                    }
                    None => Err(NodeAddConnectionError::BadPort(port))
                }
            }
        }
    }
}
