use super::*;
use crate::color_hex_utils::color_from_hex;

/// This trait must be implemented by the `ValueType` generic parameter of the
/// [`Graph`]. The trait allows drawing custom inline widgets for the different
/// types of the node graph.
pub trait ValueTrait: Clone + std::fmt::Debug {
    // TODO(MXG): We require the bounds Clone + Debug for ValueTrait in order to
    // use the derive macro to implement Clone and Debug for PortResponse, but
    // ValueTrait should not actually need Clone and Debug for that to work. We
    // can remove this bound if we manually implement Clone and Debug or if this
    // bug is ever fixed: https://github.com/rust-lang/rust/issues/26925

    type Response: ResponseTrait;
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
pub trait DataTypeTrait: Clone + std::fmt::Debug {

    /// This associated type gives the type of a raw value
    type Value: ValueTrait;

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
    type DataType: DataTypeTrait;

    /// Return the ideal Rect that the port would like to use so that the parent
    /// Node widget can adjust its size if needed.
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: (NodeId, PortId),
        state: &mut EditorUiState<Self::DataType>,
        style: &dyn GraphStyleTrait<DataType=Self::DataType>,
    ) -> (egui::Rect, Vec<PortResponse<Self::DataType>>);
    // TODO(@mxgrey): All of these Vec return types should be changed to
    // impl IntoIterator<PortResponse> when type_alias_impl_trait is a
    // stable feature. That way we can avoid memory allocations in the return
    // value.

    /// Get the data type information for this port.
    fn data_type(&self) -> Self::DataType;

    /// Get the ID of an available hook, if one exists.
    fn available_hook(&self) -> Option<HookId>;

    /// Drops the connection at the specified hook. Returns the ID of the
    /// other side of the dropped connection if the drop was successful or
    /// [`Err`] if the hook does not exist or did not have a connection.
    fn drop_connection(&mut self, id: HookId) -> Result<ConnectionId, PortDropConnectionError>;

    /// Remove all connections that this Port is holding and report the ID
    /// information for them along with what they were connected to.
    fn drop_all_connections(&mut self) -> Vec<(HookId, ConnectionId)>;

    /// Connect a hook in this port to another hook. This method should only be
    /// called by a [`NodeTrait`] implementation. To create a connection as a
    /// user, call [`Graph::add_connection`].
    fn connect(&mut self, from: HookId, to: graph::ConnectionToken) -> Result<(), PortAddConnectionError>;
}

/// This trait must be implemented for the `Content` associated type of the
/// [`NodeTrait`]. This trait allows customizing some aspects of the node drawing.
pub trait NodeContentTrait: Sized {
    type AppState;
    type Response: ResponseTrait;

    /// Additional UI elements to draw in the nodes, after the parameters.
    fn content_ui(
        &mut self,
        ui: &mut egui::Ui,
        app: &Self::AppState,
        node_id: NodeId,
    ) -> Vec<Self::Response>;

    /// Set background color on titlebar
    /// If the return value is None, the default color is set.
    fn titlebar_color(
        &self,
        _ui: &egui::Ui,
        _app: &Self::AppState,
        _node_id: NodeId,
    ) -> Option<egui::Color32> {
        None
    }
}

pub trait NodeTrait {
    type DataType: DataTypeTrait;
    type Content: NodeContentTrait;

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        app: &<Self::Content as NodeContentTrait>::AppState,
        id: NodeId,
        state: &mut EditorUiState<Self::DataType>,
        style: &dyn GraphStyleTrait<DataType=Self::DataType>,
    ) -> Vec<NodeResponse<Self>> where Self: Sized;

    /// Get the data type of the specified port if it exists, or None if the
    /// port does not exist.
    fn port_data_type(&self, port_id: PortId) -> Option<Self::DataType>;

    /// Get the ID of an available hook, if one exists.
    fn available_hook(&self, port_id: PortId) -> Option<HookId>;

    /// Drops the connection at the specified port and hook. Returns [`Ok`] if
    /// the drop was successful or [`Err`] if the hook does not exist or did not
    /// have a connection.
    fn drop_connection(
        &mut self,
        id: (PortId, HookId)
    ) -> Result<ConnectionId, NodeDropConnectionError>;

    /// Remove all connections that this Node is holding and report the ID
    /// information for them.
    fn drop_all_connections(&mut self) -> Vec<(PortId, HookId, ConnectionId)>;

    /// Connect a hook in this node to another hook. This method can only be
    /// called by the [`Graph`] class because only the graph module can produce
    /// a ConnectionToken. To create a connection as a user, call [`Graph::add_connection`].
    fn connect(&mut self, from: (PortId, HookId), to: graph::ConnectionToken) -> Result<(), NodeAddConnectionError>;
}

pub type ContentResponseOf<Node> = <<Node as NodeTrait>::Content as NodeContentTrait>::Response;
pub type DataTypeOf<Node> = <Node as NodeTrait>::DataType;
pub type ValueResponseOf<Node> = <<DataTypeOf<Node> as DataTypeTrait>::Value as ValueTrait>::Response;
pub type AppStateOf<Node> = <<Node as NodeTrait>::Content as NodeContentTrait>::AppState;

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
    /// What kind of node can be produced by this template
    type Node: NodeTrait;

    /// Returns a descriptive name for the node kind, used in the node finder.
    fn node_finder_label(&self) -> &str;

    /// Returns a descriptive name for the node kind, used in the graph.
    fn node_graph_label(&self) -> String;

    /// This function is run when this node kind gets added to the graph.
    fn build_node(
        &self,
        position: egui::Pos2,
        app_state: &mut AppStateOf<Self::Node>
    ) -> Self::Node;
}

/// The custom user response types when drawing nodes in the graph must
/// implement this trait.
pub trait ResponseTrait: Clone + std::fmt::Debug {}
impl<T: Clone + std::fmt::Debug> ResponseTrait for T {}

pub trait GraphStyleTrait {
    type DataType: DataTypeTrait;

    /// Recommend what color should be used for connections transmitting this data type
    fn recommend_data_type_color(&self, typ: &Self::DataType) -> egui::Color32;

    /// Recommend what color should be used for the background of a node
    fn recommend_node_background_color(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
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
        color_from_hex("#FFDEDE").unwrap()
    }

    /// Ports may choose to be highlighted with this color when a compatible
    /// connection is hovering over it.
    fn recommend_port_accept_color(
        &self,
        _ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        color_from_hex("#00FFAB").unwrap()
    }

    fn recommend_port_reject_color(
        &self,
        _ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        color_from_hex("#EB4747").unwrap()
    }

    fn recommend_port_hover_color(
        &self,
        ui: &egui::Ui,
        _port: (NodeId, PortId),
    ) -> egui::Color32 {
        if ui.visuals().dark_mode {
            color_from_hex("#F9F3EE").unwrap()
        } else {
            color_from_hex("#C4DDFF").unwrap()
        }
    }

    /// (stroke, background) colors for the close button of a node when it is passive
    fn recommend_close_button_passive_colors(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
    ) -> (egui::Color32, egui::Color32) {
        let dark = color_from_hex("#aaaaaa").unwrap();
        let light = color_from_hex("#555555").unwrap();
        if ui.visuals().dark_mode {
            (light, dark)
        } else {
            (dark, light)
        }
    }

    /// (stroke, background) colors for the close button of a node when it is being hovered
    fn recommend_close_button_hover_colors(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
    ) -> (egui::Color32, egui::Color32) {
        let dark = color_from_hex("#dddddd").unwrap();
        let light = color_from_hex("#222222").unwrap();
        if ui.visuals().dark_mode {
            (light, dark)
        } else {
            (dark, light)
        }
    }

    /// (stroke, background) colors for the close button of a node when it is being clicked
    fn recommend_close_button_clicked_colors(
        &self,
        ui: &egui::Ui,
        _node_id: NodeId,
    ) -> (egui::Color32, egui::Color32) {
        let dark = color_from_hex("#ffffff").unwrap();
        let light = color_from_hex("#000000").unwrap();
        if ui.visuals().dark_mode {
            (light, dark)
        } else {
            (dark, light)
        }
    }
}

pub trait GraphContextTrait: GraphStyleTrait {
    type Node: NodeTrait<DataType=Self::DataType>;
    type NodeTemplate: NodeTemplateTrait<Node=Self::Node>;
}
