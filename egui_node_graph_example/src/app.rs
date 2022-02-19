use eframe::{
    egui::{self, DragValue},
    epi,
};
use egui_node_graph::*;

// ========= First, define your user data types =============

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// simple example we don't really want to store anything.
pub struct MyNodeData;

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(PartialEq, Eq)]
pub enum MyDataType {
    Scalar,
    Vec2,
}

/// In the graph, input parameters can optionally have a constant value. This
/// value can be directly edited in a widget inside the node itself.
///
/// There will usually be a correspondence between DataTypes and ValueTypes. But
/// this library makes no attempt to check this consistency. For instance, it is
/// up to the user code in this example to make sure no parameter is created
/// with a DataType of Scalar and a ValueType of Vec2.
pub enum MyValueType {
    Vec2 { value: egui::Vec2 },
    Scalar { value: f32 },
}

/// NodeKind is a mechanism to define node "templates". It's what the graph will
/// display in the "new node" popup. The user code needs to tell the library how
/// to convert a NodeKind into a Node.
#[derive(Clone, Copy)]
pub enum MyNodeKind {
    AddScalar,
    SubtractScalar,
    VectorTimesScalar,
    AddVector,
}

// =========== Then, you need to implement some traits ============

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait for MyDataType {
    fn data_type_color(&self) -> egui::Color32 {
        match self {
            MyDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
            MyDataType::Vec2 => egui::Color32::from_rgb(238, 207, 109),
        }
    }

    fn name(&self) -> &str {
        match self {
            MyDataType::Scalar => "scalar",
            MyDataType::Vec2 => "2d vector",
        }
    }
}

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeKindTrait for MyNodeKind {
    type NodeData = MyNodeData;
    type DataType = MyDataType;
    type ValueType = MyValueType;

    fn node_finder_label(&self) -> &str {
        match self {
            MyNodeKind::AddScalar => "Scalar add",
            MyNodeKind::SubtractScalar => "Scalar subtract",
            MyNodeKind::VectorTimesScalar => "Vector times scalar",
            MyNodeKind::AddVector => "Vector subtract",
        }
    }

    fn node_graph_label(&self) -> String {
        // It's okay to delegate this to node_finder_label if you don't want to
        // show different names in the node finder and the node itself.
        self.node_finder_label().into()
    }

    fn user_data(&self) -> Self::NodeData {
        MyNodeData
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        node_id: NodeId,
    ) {
        // The nodes are created empty by default. This function needs to take
        // care of creating the desired inputs and outputs based on the template

        // We define some macros here to avoid boilerplate. Note that this is
        // entirely optional.
        macro_rules! input {
            (scalar $name:expr) => {
                graph.add_input_param(
                    node_id,
                    $name.to_string(),
                    MyDataType::Scalar,
                    MyValueType::Scalar { value: 0.0 },
                    InputParamKind::ConnectionOrConstant,
                    true,
                );
            };
            (vector $name:expr) => {
                graph.add_input_param(
                    node_id,
                    $name.to_string(),
                    MyDataType::Vec2,
                    MyValueType::Vec2 {
                        value: egui::vec2(0.0, 0.0),
                    },
                    InputParamKind::ConnectionOrConstant,
                    true,
                );
            };
        }

        macro_rules! output {
            (scalar $name:expr) => {
                graph.add_output_param(node_id, $name.to_string(), MyDataType::Scalar);
            };
            (vector $name:expr) => {
                graph.add_output_param(node_id, $name.to_string(), MyDataType::Vec2);
            };
        }

        match self {
            MyNodeKind::AddScalar => {
                // The first input param doesn't use the macro so we can comment
                // it in more detail.
                graph.add_input_param(
                    node_id,
                    // This is the name of the parameter. Can be later used to
                    // retrieve the value. Parameter names should be unique.
                    "A".into(),
                    // The data type for this input. In this case, a scalar
                    MyDataType::Scalar,
                    // The value type for this input. We store zero as default
                    MyValueType::Scalar { value: 0.0 },
                    // The input parameter kind. This allows defining whether a
                    // parameter accepts input connections and/or an inline
                    // widget to set its value.
                    InputParamKind::ConnectionOrConstant,
                    true,
                );
                input!(scalar "B");
                output!(scalar "out");
            }
            MyNodeKind::SubtractScalar => {
                input!(scalar "A");
                input!(scalar "B");
                output!(scalar "out");
            }
            MyNodeKind::VectorTimesScalar => {
                input!(scalar "scalar");
                input!(vector "vector");
                output!(vector "out");
            }
            MyNodeKind::AddVector => {
                input!(vector "v1");
                input!(vector "v2");
                output!(vector "out");
            }
        }
    }
}

pub struct AllMyNodeKinds;
impl NodeKindIter for AllMyNodeKinds {
    type Item = MyNodeKind;

    fn all_kinds(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        // This function must return a list of node kinds, which the node finder
        // will use to display it to the user. Crates like strum can reduce the
        // boilerplate in enumerating all variants of an enum.
        //
        // The Box here is required because traits in Rust cannot be generic
        // over return parameters, so you can't return an iterator.
        Box::new(
            [
                MyNodeKind::AddScalar,
                MyNodeKind::SubtractScalar,
                MyNodeKind::VectorTimesScalar,
                MyNodeKind::AddVector,
            ]
            .iter(),
        )
    }
}

impl InputParamWidget for MyValueType {
    fn value_widget(&mut self, param_name: &str, ui: &mut egui::Ui) {
        // This trait is used to tell the library which UI to display for the
        // inline parameter widgets.
        match self {
            MyValueType::Vec2 { value } => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(DragValue::new(&mut value.x));
                    ui.label("y");
                    ui.add(DragValue::new(&mut value.y));
                });
            }
            MyValueType::Scalar { value } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(DragValue::new(value));
                });
            }
        }
    }
}

pub struct NodeGraphExample {
    state: GraphEditorState<MyNodeData, MyDataType, MyValueType, MyNodeKind>,
}

impl Default for NodeGraphExample {
    fn default() -> Self {
        Self {
            state: GraphEditorState::new(1.0),
        }
    }
}

impl epi::App for NodeGraphExample {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        self.state.draw_graph_editor(ctx, AllMyNodeKinds);
    }
}
