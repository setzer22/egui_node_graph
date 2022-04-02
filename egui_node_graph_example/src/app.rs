use std::collections::HashMap;

use eframe::{
    egui::{self, DragValue, TextStyle},
    epi,
};
use egui_node_graph::*;

// ========= First, define your user data types =============

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
pub struct MyNodeData {
    template: MyNodeTemplate,
}

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
#[derive(Copy, Clone, Debug)]
pub enum MyValueType {
    Vec2 { value: egui::Vec2 },
    Scalar { value: f32 },
}

impl MyValueType {
    /// Tries to downcast this value type to a vector
    pub fn try_to_vec2(self) -> anyhow::Result<egui::Vec2> {
        if let MyValueType::Vec2 { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to vec2", self)
        }
    }

    /// Tries to downcast this value type to a scalar
    pub fn try_to_scalar(self) -> anyhow::Result<f32> {
        if let MyValueType::Scalar { value } = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to scalar", self)
        }
    }
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
#[derive(Clone, Copy)]
pub enum MyNodeTemplate {
    MakeVector,
    MakeScalar,
    AddScalar,
    SubtractScalar,
    VectorTimesScalar,
    AddVector,
    SubtractVector,
}

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MyResponse {
    SetActiveNode(NodeId),
    ClearActiveNode,
}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.
#[derive(Default)]
pub struct MyGraphState {
    pub active_node: Option<NodeId>,
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
impl NodeTemplateTrait for MyNodeTemplate {
    type NodeData = MyNodeData;
    type DataType = MyDataType;
    type ValueType = MyValueType;

    fn node_finder_label(&self) -> &str {
        match self {
            MyNodeTemplate::MakeVector => "New vector",
            MyNodeTemplate::MakeScalar => "New scalar",
            MyNodeTemplate::AddScalar => "Scalar add",
            MyNodeTemplate::SubtractScalar => "Scalar subtract",
            MyNodeTemplate::AddVector => "Vector add",
            MyNodeTemplate::SubtractVector => "Vector subtract",
            MyNodeTemplate::VectorTimesScalar => "Vector times scalar",
        }
    }

    fn node_graph_label(&self) -> String {
        // It's okay to delegate this to node_finder_label if you don't want to
        // show different names in the node finder and the node itself.
        self.node_finder_label().into()
    }

    fn user_data(&self) -> Self::NodeData {
        MyNodeData { template: *self }
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
            MyNodeTemplate::AddScalar => {
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
            MyNodeTemplate::SubtractScalar => {
                input!(scalar "A");
                input!(scalar "B");
                output!(scalar "out");
            }
            MyNodeTemplate::VectorTimesScalar => {
                input!(scalar "scalar");
                input!(vector "vector");
                output!(vector "out");
            }
            MyNodeTemplate::AddVector => {
                input!(vector "v1");
                input!(vector "v2");
                output!(vector "out");
            }
            MyNodeTemplate::SubtractVector => {
                input!(vector "v1");
                input!(vector "v2");
                output!(vector "out");
            }
            MyNodeTemplate::MakeVector => {
                input!(scalar "x");
                input!(scalar "y");
                output!(vector "out");
            }
            MyNodeTemplate::MakeScalar => {
                input!(scalar "value");
                output!(scalar "out");
            }
        }
    }
}

pub struct AllMyNodeTemplates;
impl NodeTemplateIter for AllMyNodeTemplates {
    type Item = MyNodeTemplate;

    fn all_kinds(&self) -> Vec<Self::Item> {
        // This function must return a list of node kinds, which the node finder
        // will use to display it to the user. Crates like strum can reduce the
        // boilerplate in enumerating all variants of an enum.
        vec![
            MyNodeTemplate::MakeScalar,
            MyNodeTemplate::MakeVector,
            MyNodeTemplate::AddScalar,
            MyNodeTemplate::SubtractScalar,
            MyNodeTemplate::AddVector,
            MyNodeTemplate::SubtractVector,
            MyNodeTemplate::VectorTimesScalar,
        ]
    }
}

impl WidgetValueTrait for MyValueType {
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

impl UserResponseTrait for MyResponse {}
impl NodeDataTrait for MyNodeData {
    type Response = MyResponse;
    type UserState = MyGraphState;
    type DataType = MyDataType;
    type ValueType = MyValueType;

    // This method will be called when drawing each node. This allows adding
    // extra ui elements inside the nodes. In this case, we create an "active"
    // button which introduces the concept of having an active node in the
    // graph. This is done entirely from user code with no modifications to the
    // node graph library.
    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        _graph: &Graph<MyNodeData, MyDataType, MyValueType>,
        user_state: &Self::UserState,
    ) -> Vec<NodeResponse<MyResponse>>
    where
        MyResponse: UserResponseTrait,
    {
        // This logic is entirely up to the user. In this case, we check if the
        // current node we're drawing is the active one, by comparing against
        // the value stored in the global user state, and draw different button
        // UIs based on that.

        let mut responses = vec![];
        let is_active = user_state
            .active_node
            .map(|id| id == node_id)
            .unwrap_or(false);

        // Pressing the button will emit a custom user response to either set,
        // or clear the active node. These responses do nothing by themselves,
        // the library only makes the responses available to you after the graph
        // has been drawn. See below at the update method for an example.
        if !is_active {
            if ui.button("👁 Set active").clicked() {
                responses.push(NodeResponse::User(MyResponse::SetActiveNode(node_id)));
            }
        } else {
            let button =
                egui::Button::new(egui::RichText::new("👁 Active").color(egui::Color32::BLACK))
                    .fill(egui::Color32::GOLD);
            if ui.add(button).clicked() {
                responses.push(NodeResponse::User(MyResponse::ClearActiveNode));
            }
        }

        responses
    }
}

type MyGraph = Graph<MyNodeData, MyDataType, MyValueType>;
type MyEditorState =
    GraphEditorState<MyNodeData, MyDataType, MyValueType, MyNodeTemplate, MyGraphState>;

pub struct NodeGraphExample {
    // The `GraphEditorState` is the top-level object. You "register" all your
    // custom types by specifying it as its generic parameters.
    state: MyEditorState,
}

impl Default for NodeGraphExample {
    fn default() -> Self {
        Self {
            state: GraphEditorState::new(1.0, MyGraphState::default()),
        }
    }
}

impl epi::App for NodeGraphExample {
    fn name(&self) -> &str {
        "Egui node graph example"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let graph_response = self.state.draw_graph_editor(ctx, AllMyNodeTemplates);
        for node_response in graph_response.node_responses {
            // Here, we ignore all other graph events. But you may find
            // some use for them. For example, by playing a sound when a new
            // connection is created
            if let NodeResponse::User(user_event) = node_response {
                match user_event {
                    MyResponse::SetActiveNode(node) => {
                        self.state.user_state.active_node = Some(node)
                    }
                    MyResponse::ClearActiveNode => self.state.user_state.active_node = None,
                }
            }
        }

        if let Some(node) = self.state.user_state.active_node {
            let text = match evaluate_node(&self.state.graph, node, &mut HashMap::new()) {
                Ok(value) => format!("The result is: {:?}", value),
                Err(err) => format!("Execution error: {}", err),
            };
            ctx.debug_painter().text(
                egui::pos2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                text,
                TextStyle::Button.resolve(&ctx.style()),
                egui::Color32::WHITE,
            );
        }
    }
}

type OutputsCache = HashMap<OutputId, MyValueType>;

/// Recursively evaluates all dependencies of this node, then evaluates the node itself.
pub fn evaluate_node(
    graph: &MyGraph,
    node_id: NodeId,
    outputs_cache: &mut OutputsCache,
) -> anyhow::Result<MyValueType> {
    // Similar to when creating node types above, we define two macros for
    // convenience. They may be overkill for this small example, but something
    // like this makes the code much more readable when the number of nodes
    // starts growing.
    macro_rules! input {
        (Vec2 $name:expr) => {
            evaluate_input(graph, node_id, $name, outputs_cache)?.try_to_vec2()?
        };
        (Scalar $name:expr) => {
            evaluate_input(graph, node_id, $name, outputs_cache)?.try_to_scalar()?
        };
    }

    macro_rules! output {
        (Vec2 $name:expr => $value:expr) => {{
            let out = MyValueType::Vec2 { value: $value };
            populate_output(graph, outputs_cache, node_id, $name, out)?;
            Ok(out)
        }};
        (Scalar $name:expr => $value:expr) => {{
            let out = MyValueType::Scalar { value: $value };
            populate_output(graph, outputs_cache, node_id, $name, out)?;
            Ok(out)
        }};
    }

    let node = &graph[node_id];
    match node.user_data.template {
        MyNodeTemplate::AddScalar => {
            // Calling `evaluate_input` recursively evaluates other nodes in the
            // graph until the input value for a paramater has been computed.
            // This first call doesn't use the `input!` macro to illustrate what
            // is going on underneath.
            let a = evaluate_input(graph, node_id, "A", outputs_cache)?.try_to_scalar()?;
            let b = evaluate_input(graph, node_id, "B", outputs_cache)?.try_to_scalar()?;

            // After computing an output, we don't just return it, but we also
            // populate the outputs cache with it. This ensures the evaluation
            // only ever computes an output once.
            //
            // The return value of the function is the "final" output of the
            // node, the thing we want to get from the evaluation. The example
            // would be slightly more contrived when we had multiple output
            // values, as we would need to choose which of the outputs is the
            // one we want to return. Other outputs could be used as
            // intermediate values.
            //
            // Note that this is just one possible semantic interpretation of
            // the graphs, you can come up with your own evaluation semantics!
            let out = MyValueType::Scalar { value: a + b };
            populate_output(graph, outputs_cache, node_id, "out", out)?;
            Ok(out)
        }
        MyNodeTemplate::SubtractScalar => {
            // Using the macros, the code gets as succint as it gets
            let a = input!(Scalar "A");
            let b = input!(Scalar "B");
            output!(Scalar "out" => a - b)
        }
        MyNodeTemplate::VectorTimesScalar => {
            let scalar = input!(Scalar "scalar");
            let vector = input!(Vec2 "vector");
            output!(Vec2 "out" => vector * scalar)
        }
        MyNodeTemplate::AddVector => {
            let v1 = input!(Vec2 "v1");
            let v2 = input!(Vec2 "v2");
            output!(Vec2 "out" => v1 + v2)
        }
        MyNodeTemplate::SubtractVector => {
            let v1 = input!(Vec2 "v1");
            let v2 = input!(Vec2 "v2");
            output!(Vec2 "out" => v1 - v2)
        }
        MyNodeTemplate::MakeVector => {
            let x = input!(Scalar "x");
            let y = input!(Scalar "y");
            output!(Vec2 "out" => egui::vec2(x, y))
        }
        MyNodeTemplate::MakeScalar => {
            let value = input!(Scalar "value");
            output!(Scalar "out" => value)
        }
    }
}

fn populate_output(
    graph: &MyGraph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
    value: MyValueType,
) -> anyhow::Result<()> {
    let output_id = graph[node_id].get_output(param_name)?;
    outputs_cache.insert(output_id, value);
    Ok(())
}

// Evaluates the input value of
fn evaluate_input(
    graph: &MyGraph,
    node_id: NodeId,
    param_name: &str,
    outputs_cache: &mut OutputsCache,
) -> anyhow::Result<MyValueType> {
    let input_id = graph[node_id].get_input(param_name)?;

    // The output of another node is connected.
    if let Some(other_output_id) = graph.connection(input_id) {
        // The value was already computed due to the evaluation of some other
        // node. We simply return value from the cache.
        if let Some(other_value) = outputs_cache.get(&other_output_id) {
            Ok(*other_value)
        }
        // This is the first time encountering this node, so we need to
        // recursively evaluate it.
        else {
            // Calling this will populate the cache
            evaluate_node(graph, graph[other_output_id].node, outputs_cache)?;

            // Now that we know the value is cached, return it
            Ok(*outputs_cache
                .get(&other_output_id)
                .expect("Cache should be populated"))
        }
    }
    // No existing connection, take the inline value instead.
    else {
        Ok(graph[input_id].value)
    }
}
