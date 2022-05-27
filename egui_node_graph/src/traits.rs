use super::*;

/// This trait must be implemented by the `ValueType` generic parameter of the
/// [`Graph`]. The trait allows drawing custom inline widgets for the different
/// types of the node graph.
pub trait WidgetValueTrait {
    type Response;
    fn value_widget(&mut self, param_name: &str, ui: &mut egui::Ui) -> Vec<Self::Response>;
}

/// This trait must be implemented by the `DataType` generic parameter of the
/// [`Graph`]. This trait tells the library how to visually expose data types
/// to the user.
pub trait DataTypeTrait: PartialEq + Eq {
    // The associated port color of this datatype
    fn data_type_color(&self) -> egui::Color32;

    // The name of this datatype
    fn name(&self) -> &str;
}

/// This trait must be implemented for the `NodeData` generic parameter of the
/// [`Graph`]. This trait allows customizing some aspects of the node drawing.
pub trait NodeDataTrait
where
    Self: Sized,
{
    /// Must be set to the custom user `NodeResponse` type
    type Response;
    /// Must be set to the custom user `UserState` type
    type UserState;
    /// Must be set to the custom user `DataType` type
    type DataType;
    /// Must be set to the custom user `ValueType` type
    type ValueType;

    /// Additional UI elements to draw in the nodes, after the parameters.
    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &Self::UserState,
    ) -> Vec<NodeResponse<Self::Response>>
    where
        Self::Response: UserResponseTrait;
}

/// This trait can be implemented by any user type. The trait tells the library
/// how to enumerate the node templates it will present to the user as part of
/// the node finder.
pub trait NodeTemplateIter {
    type Item;
    fn all_kinds(&self) -> Vec<Self::Item>;
}

/// This trait must be implemented by the `NodeTemplate` generic parameter of
/// the [`GraphEditorState`]. It allows the customization of node templates. A
/// node template is what describes what kinds of nodes can be added to the
/// graph, what is their name, and what are their input / output parameters.
pub trait NodeTemplateTrait: Clone {
    /// Must be set to the custom user `NodeData` type
    type NodeData;
    /// Must be set to the custom user `DataType` type
    type DataType;
    /// Must be set to the custom user `ValueType` type
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

/// The custom user response types when drawing nodes in the graph must
/// implement this trait.
pub trait UserResponseTrait: Clone + Copy + std::fmt::Debug + PartialEq + Eq {}
