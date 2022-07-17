use super::*;
use crate::color_hex_utils::*;
use egui::*;

/// A node inside the [`Graph`]. Nodes have input and output parameters, stored
/// as ids. They also contain a custom `NodeData` struct with whatever data the
/// user wants to store per-node.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct ColumnNode<Content, InputPort, OutputPort> {
    pub label: String,
    pub content: Content,
    /// The input ports of the graph
    pub inputs: SlotMap<InputId, InputPort>,
    /// The [`OutputParam`]s of the graph
    pub outputs: SlotMap<OutputId, OutputPort>,
    pub input_port_names: Vec<(String, InputId)>,
    pub output_port_names: Vec<(String, OutputId)>,
}

pub type SimpleColumnNode<Content, DataType> =
    ColumnNode<Content, VerticalInputPort<DataType>, VerticalPort<DataType>>;

impl<Content, InputPort, OutputPort> NodeTrait for ColumnNode<Content, InputPort, OutputPort>
where
    InputPort: PortTrait,
    OutputPort: PortTrait,
{
    type Content = Content;

    fn show(
        &self,
        ui: &mut egui::Ui,
        state: NodeUiState<Self>,
        context: &dyn GraphContext,
    ) -> Vec<NodeResponse<Self>> {
        let margin = egui::vec2(15.0, 5.0);
        let mut responses = Vec::<NodeResponse<Self>>::new();

        let background_color = context.recommend_node_background_color(ui, state.node_id);
        let text_color = context.recommend_node_text_color(ui, state.node_id);

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
        ui.add(widget)
        let mut title_height = 0.0;
    }
}
