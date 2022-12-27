use super::*;
use crate::color_hex_utils::color_from_hex;

/// This trait must be implemented by the `ValueType` generic parameter of the
/// [`Graph`]. The trait allows drawing custom inline widgets for the different
/// types of the node graph.
pub trait ValueTrait {
    type Response;
    /// This method will be called for each input parameter with a widget.
    ///
    /// The return value is a tuple with the recommended size of the widget and
    /// a vector of custom response objects which can be used to implement
    /// handling of side effects. If unsure, the response Vec can be empty.
    fn show(
        &mut self,
        ui: &mut egui::Ui,
    ) -> (egui::Rect, Vec<Self::Response>);
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

    /// Return the ideal Rect that the port would like to use so that the parent
    /// Node widget can adjust its size if needed.
    fn show<Node>(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &NodeUiState<DataTypeOf<Node>>,
        context: &dyn GraphContext<Node=Node>,
    ) -> (egui::Rect, Vec<PortResponse<Node>>);
    // TODO(@mxgrey): All of these Vec return types should be changed to
    // impl IntoIterator<PortResponse> when type_alias_impl_trait is a
    // stable feature. That way we can avoid memory allocations in the return
    // value.
}

/// This trait must be implemented for the `Content` associated type of the
/// [`NodeTrait`]. This trait allows customizing some aspects of the node drawing.
pub trait NodeContentTrait: Sized {
    type Response;

    /// Additional UI elements to draw in the nodes, after the parameters.
    fn content_ui<Node>(
        &mut self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<Node>,
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
    ) -> Option<egui::Color32> {
        None
    }
}

pub trait NodeTrait {
    type DataType;
    type Content: NodeContentTrait;

    fn show<Node>(
        &mut self,
        ui: &mut egui::Ui,
        id: NodeId,
        state: NodeUiState<DataTypeOf<Node>>,
        graph: &Graph<Node>,
        context: &dyn GraphContext<Node=Node>,
    ) -> Vec<NodeResponse<Self>>;

    /// Drops the connection at the specified port and hook. Returns [`Ok`] with
    /// the connected ID if the drop was successful or [`Err`] if the hook does
    /// not exist or did not have a connection.
    fn drop_connection(
        &mut self,
        id: (PortId, HookId)
    ) -> Result<ConnectionId, NodeDropConnectionError>;

    /// Remove all connections that this Node is holding and report the ID
    /// information for them.
    fn drop_all_connections(&mut self) -> Vec<(PortId, HookId)>;

    /// Connect a hook in this node to another hook. This method can only be
    /// called by the [`Graph`] class because only the graph module can produce
    /// a ConnectionToken. To create a connection as a user, call [`Graph::add_connection`].
    fn connect(&mut self, from: (PortId, HookId), to: graph::ConnectionToken) -> Result<(), ()>;

    /// Get the user-defined content of this Node.
    fn content(&self) -> &Self::Content;
}

pub type ContentResponseOf<Node> = <<Node as NodeTrait>::Content as NodeContentTrait>::Response;
pub type DataTypeOf<Node> = <Node as NodeTrait>::DataType;
pub type ValueResponseOf<Node> = <<DataTypeOf<Node> as DataTypeTrait>::Value as ValueTrait>::Response;

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
pub trait NodeTemplateTrait<Node>: Clone {
    /// Returns a descriptive name for the node kind, used in the node finder.
    fn node_finder_label(&self) -> &str;

    /// Returns a descriptive name for the node kind, used in the graph.
    fn node_graph_label(&self) -> String;

    /// This function is run when this node kind gets added to the graph.
    fn build_node(&self) -> Node;
}

/// The custom user response types when drawing nodes in the graph must
/// implement this trait.
pub trait ContentResponseTrait: Clone + std::fmt::Debug {}

pub trait GraphContext {
    type Node;

    /// Recommend what color should be used for connections transmitting this data type
    fn recommend_data_type_color(&self, typ: &DataTypeOf<Self::Node>) -> egui::Color32;

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
        _node_id: NodeId,
    ) -> egui::Color32 {
        if ui.visuals().dark_mode {
            color_from_hex("#fefefe").unwrap()
        } else {
            color_from_hex("#505050").unwrap()
        }
    }

    /// Ports may choose to be highlighted with this color when a connection
    /// event is ongoing if their data type is compatible with the connection.
    fn recommend_compatible_port_color(
        &self,
        _ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        color_from_hex("#D9F8C4").unwrap()
    }

    /// Ports may choose to be highlighted with this color when a connection
    /// event is ongoing if their data type is incompatible with the connection.
    fn recommend_incompatible_port_color(
        &self,
        _ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        color_from_hex("#FFDEDE")
    }

    /// Ports may choose to be highlighted with this color when a compatible
    /// connection is hovering over it.
    fn recommend_port_accept_color(
        &self,
        _ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        color_from_hex("#00FFAB")
    }

    fn recommend_port_reject_color(
        &self,
        _ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        color_from_hex("#EB4747")
    }

    fn recommend_port_hover_color(
        &self,
        ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        if ui.visuals().dark_mode {
            color_from_hex("#F9F3EE")
        } else {
            color_from_hex("#C4DDFF")
        }
    }

    /// (stroke, background) colors for the close button of a node when it is passive
    fn recommend_close_button_passive_colors(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
    ) -> (egui::Color32, egui::Color32) {
        if ui.visuals().dark_mode {
            color_from_hex("#aaaaaa").unwrap()
        } else {
            color_from_hex("#555555").unwrap()
        }
    }

    /// (stroke, background) colors for the close button of a node when it is being hovered
    fn recommend_close_button_hover_colors(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
    ) -> (egui::Color32, egui::Color32) {
        if ui.visuals().dark_mode {
            color_from_hex("#dddddd").unwrap()
        } else {
            color_from_hex("#222222").unwrap()
        }
    }

    /// (stroke, background) colors for the close button of a node when it is being clicked
    fn recommend_close_button_clicked_colors(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
    ) -> (egui::Color32, egui::Color32) {
        if ui.visuals().dark_mode {
            color_from_hex("#ffffff").unwrap()
        } else {
            color_from_hex("#000000").unwrap()
        }
    }
}
