use super::*;
use crate::color_hex_utils::color_from_hex;

/// This trait must be implemented by the `ValueType` generic parameter of the
/// [`Graph`]. The trait allows drawing custom inline widgets for the different
/// types of the node graph.
pub trait WidgetValueTrait {
    type Response;
    /// This method will be called for each input parameter with a widget. The
    /// return value is a vector of custom response objects which can be used
    /// to implement handling of side effects. If unsure, the response Vec can
    /// be empty.
    fn value_widget(&mut self, param_name: &str, ui: &mut egui::Ui) -> Vec<Self::Response>;
}

/// This trait must be implemented by the `DataType` associated type of any
/// [`NodeTrait`]. This trait tells the library how to visually expose data types
/// to the user.
pub trait DataTypeTrait {

    /// This associated type gives the type of a raw value
    type Value;

    fn is_compatible(&self, other: &Self) -> bool;

    /// The name of this datatype. Return type is specified as Cow<str> because
    /// some implementations will need to allocate a new string to provide an
    /// answer while others won't.
    ///
    /// ## Example (borrowed value)
    /// Use this when you can get the name of the datatype from its fields or as
    /// a &'static str. Prefer this method when possible.
    /// ```ignore
    /// pub struct DataType { name: String }
    ///
    /// impl DataTypeTrait<()> for DataType {
    ///     fn name(&self) -> std::borrow::Cow<str> {
    ///         Cow::Borrowed(&self.name)
    ///     }
    /// }
    /// ```
    ///
    /// ## Example (owned value)
    /// Use this when you can't derive the name of the datatype from its fields.
    /// ```ignore
    /// pub struct DataType { some_tag: i32 }
    ///
    /// impl DataTypeTrait<()> for DataType {
    ///     fn name(&self) -> std::borrow::Cow<str> {
    ///         Cow::Owned(format!("Super amazing type #{}", self.some_tag))
    ///     }
    /// }
    /// ```
    fn name(&self) -> std::borrow::Cow<str>;
}

/// Trait to be implemented by port types. Ports belong to nodes and define the
/// behavior for connecting inputs and outputs for its node.
pub trait PortTrait {
    fn show<Node>(
        &self,
        ui: &mut egui::Ui,
        responses: &mut Vec<NodeResponse<Node>>,
        state: &NodeUiState<Node>,
        context: &dyn GraphContext,
    );
}

/// This trait must be implemented for the `Content` associated type of the
/// [`NodeTrait`]. This trait allows customizing some aspects of the node drawing.
pub trait NodeContentTrait
where
    Self: Sized,
{
    type Response;
    type ContentState;
    type DataType;

    /// Additional UI elements to draw in the nodes, after the parameters.
    fn content_ui<Node>(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<Node>,
        content_state: &Self::ContentState,
    ) -> Vec<NodeResponse<Node>>
    where
        Self::Response: ContentResponseTrait;

    /// Set background color on titlebar
    /// If the return value is None, the default color is set.
    fn titlebar_color<Node>(
        &self,
        _ui: &egui::Ui,
        _node_id: NodeId,
        _graph: &Graph<Node>,
        _content_state: &Self::ContentState,
    ) -> Option<egui::Color32> {
        None
    }
}

pub trait NodeTrait {
    type Content: NodeContentTrait;

    fn show(
        &self,
        ui: &mut egui::Ui,
        state: NodeUiState<Self>,
        context: &dyn GraphContext,
    ) -> Vec<NodeResponse<Self>>;

    fn content(&self) -> &Self::Content;
}

pub type ResponseOf<Node> = <<Node as NodeTrait>::Content as NodeContentTrait>::Response;
pub type DataTypeOf<Node> = <<Node as NodeTrait>::Content as NodeContentTrait>::DataType;

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
    type Node;
    /// Must be set to the custom user `DataType` type
    type DataType;
    /// Must be set to the custom user `ValueType` type
    type ValueType;
    /// Must be set to the custom user `UserState` type
    type UserState;

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
        user_state: &Self::UserState,
        node_id: NodeId,
    );
}

/// The custom user response types when drawing nodes in the graph must
/// implement this trait.
pub trait ContentResponseTrait: Clone + std::fmt::Debug {}

pub trait GraphContext {
    type DataType;
    type NodeTemplate;

    /// Recommend what color should be used for connections transmitting this data type
    fn recommend_data_type_color(&self, typ: &Self::DataType) -> egui::Color32;

    /// Recommend what color should be used for the background of a node
    fn recommend_node_background_color(
        &self,
        ui: &egui::Ui,
        node_id: NodeId,
    ) -> egui::Color32 {
        if ui.visuals().dark_mode {
            color_from_hex("#3f3f3f").unwrap()
        } else {
            color_from_hex("#ffffff").unwrap()
        }
    }

    /// Recommend what color should be used for the text in a node
    fn recommend_node_text_color(
        &self,
        ui: &egui::Ui,
        node_id: NodeId,
    ) -> egui::Color32 {
        if ui.visuals().dark_mode {
            color_from_hex("#fefefe").unwrap()
        } else {
            color_from_hex("#505050").unwrap()
        }
    }
}
