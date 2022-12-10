use super::*;

/// This trait must be implemented by the `ValueType` generic parameter of the
/// [`Graph`]. The trait allows drawing custom inline widgets for the different
/// types of the node graph.
///
/// The [`Default`] trait bound is required to circumvent borrow checker issues
/// using `std::mem::take` Otherwise, it would be impossible to pass the
/// `node_data` parameter during `value_widget`. The default value is never
/// used, so the implementation is not important, but it should be reasonably
/// cheap to construct.
pub trait WidgetValueTrait: Default {
    type Response;
    type UserState;
    type NodeData;
    /// This method will be called for each input parameter with a widget. The
    /// return value is a vector of custom response objects which can be used
    /// to implement handling of side effects. If unsure, the response Vec can
    /// be empty.
    fn value_widget(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut egui::Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response>;
}

/// This trait must be implemented by the `DataType` generic parameter of the
/// [`Graph`]. This trait tells the library how to visually expose data types
/// to the user.
pub trait DataTypeTrait<UserState>: PartialEq + Eq {
    /// The associated port color of this datatype
    fn data_type_color(&self, user_state: &mut UserState) -> egui::Color32;

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
        user_state: &mut Self::UserState,
    ) -> Vec<NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait;

    /// Set background color on titlebar
    /// If the return value is None, the default color is set.
    fn titlebar_color(
        &self,
        _ui: &egui::Ui,
        _node_id: NodeId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) -> Option<egui::Color32> {
        None
    }

    fn can_delete(
        &self,
        _node_id: NodeId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) -> bool {
        true
    }
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
    /// Must be set to the custom user `UserState` type
    type UserState;

    /// Returns a descriptive name for the node kind, used in the node finder.
    ///
    /// The return type is Cow<str> to allow returning owned or borrowed values
    /// more flexibly. Refer to the documentation for `DataTypeTrait::name` for
    /// more information
    fn node_finder_label(&self, user_state: &mut Self::UserState) -> std::borrow::Cow<str>;

    /// Returns a descriptive name for the node kind, used in the graph.
    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String;

    /// Returns the user data for this node kind.
    fn user_data(&self, user_state: &mut Self::UserState) -> Self::NodeData;

    /// This function is run when this node kind gets added to the graph. The
    /// node will be empty by default, and this function can be used to fill its
    /// parameters.
    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    );
}

/// The custom user response types when drawing nodes in the graph must
/// implement this trait.
pub trait UserResponseTrait: Clone + std::fmt::Debug {}
