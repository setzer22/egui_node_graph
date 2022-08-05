use std::borrow::Cow;

use eframe::egui::{self, DragValue, TextStyle};
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

    pub infinity_loop: Option<(OutputId, InputId)>,
}

// =========== Then, you need to implement some traits ============

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<MyGraphState> for MyDataType {
    fn data_type_color(
        &self,
        user_state: &MyGraphState,
        port_connection: PortConnection,
    ) -> egui::Color32 {
        // Turns the color of connections in infinite loops red
        if let Some((inf_input, inf_output)) = user_state.infinity_loop {
            if let PortConnection::Connection(input_id, output_id) = port_connection {
                if input_id == inf_input && output_id == inf_output {
                    return egui::Color32::from_rgb(255, 0, 0);
                }
            }
        }
        match self {
            MyDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
            MyDataType::Vec2 => egui::Color32::from_rgb(238, 207, 109),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            MyDataType::Scalar => Cow::Borrowed("scalar"),
            MyDataType::Vec2 => Cow::Borrowed("2d vector"),
        }
    }
}

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for MyNodeTemplate {
    type NodeData = MyNodeData;
    type DataType = MyDataType;
    type ValueType = MyValueType;
    type UserState = MyGraphState;

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
        _user_state: &Self::UserState,
        node_id: NodeId,
    ) {
        // The nodes are created empty by default. This function needs to take
        // care of creating the desired inputs and outputs based on the template

        // We define some closures here to avoid boilerplate. Note that this is
        // entirely optional.
        let input_scalar = |graph: &mut MyGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                MyDataType::Scalar,
                MyValueType::Scalar { value: 0.0 },
                InputParamKind::ConnectionOrConstant,
                true,
            );
        };
        let input_vector = |graph: &mut MyGraph, name: &str| {
            graph.add_input_param(
                node_id,
                name.to_string(),
                MyDataType::Vec2,
                MyValueType::Vec2 {
                    value: egui::vec2(0.0, 0.0),
                },
                InputParamKind::ConnectionOrConstant,
                true,
            );
        };

        let output_scalar = |graph: &mut MyGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), MyDataType::Scalar);
        };
        let output_vector = |graph: &mut MyGraph, name: &str| {
            graph.add_output_param(node_id, name.to_string(), MyDataType::Vec2);
        };

        match self {
            MyNodeTemplate::AddScalar => {
                // The first input param doesn't use the closure so we can comment
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
                input_scalar(graph, "B");
                output_scalar(graph, "out");
            }
            MyNodeTemplate::SubtractScalar => {
                input_scalar(graph, "A");
                input_scalar(graph, "B");
                output_scalar(graph, "out");
            }
            MyNodeTemplate::VectorTimesScalar => {
                input_scalar(graph, "scalar");
                input_vector(graph, "vector");
                output_vector(graph, "out");
            }
            MyNodeTemplate::AddVector => {
                input_vector(graph, "v1");
                input_vector(graph, "v2");
                output_vector(graph, "out");
            }
            MyNodeTemplate::SubtractVector => {
                input_vector(graph, "v1");
                input_vector(graph, "v2");
                output_vector(graph, "out");
            }
            MyNodeTemplate::MakeVector => {
                input_scalar(graph, "x");
                input_scalar(graph, "y");
                output_vector(graph, "out");
            }
            MyNodeTemplate::MakeScalar => {
                input_scalar(graph, "value");
                output_scalar(graph, "out");
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
    type Response = MyResponse;
    fn value_widget(&mut self, param_name: &str, ui: &mut egui::Ui) -> Vec<MyResponse> {
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
        // This allows you to return your responses from the inline widgets.
        Vec::new()
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
    ) -> Vec<NodeResponse<MyResponse, MyNodeData>>
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
    evaluator: DfsEvaluator,
}

impl Default for NodeGraphExample {
    fn default() -> Self {
        Self {
            state: GraphEditorState::new(1.0, MyGraphState::default()),
            evaluator: DfsEvaluator::default(),
        }
    }
}

impl eframe::App for NodeGraphExample {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });
        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state.draw_graph_editor(ui, AllMyNodeTemplates)
            })
            .inner;
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
            if self.state.graph.nodes.contains_key(node) {
                let text = match self.evaluator.compute(
                    &self.state.graph,
                    node,
                    &mut self.state.user_state,
                ) {
                    Ok(value) => format!("The result is: {:?}", value),
                    Err(err) => format!("Execution error: {}", err),
                };
                ctx.debug_painter().text(
                    egui::pos2(10.0, 35.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    TextStyle::Button.resolve(&ctx.style()),
                    egui::Color32::WHITE,
                );
            } else {
                self.state.user_state.active_node = None;
            }
        }
    }
}

use egui_node_graph::{EguiGraphError, NodeId, OutputId};
use std::collections::{BTreeMap, BTreeSet};
type OutputsCache = BTreeMap<OutputId, MyValueType>;

#[derive(Default)]
struct DfsEvaluator {
    evaluated: BTreeSet<NodeId>,
    stack: Vec<NodeId>,

    // Reverse order of calculation
    sequence: Vec<NodeId>,
    outputs_cache: OutputsCache,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputPortId(pub usize);
/// Evaluate all dependencies of this node by BFS
impl DfsEvaluator {
    /// Explore the nodes, evaluate, and return the result of the evaluation.
    pub fn compute(
        &mut self,
        graph: &MyGraph,
        node_id: NodeId,
        user_state: &mut MyGraphState,
    ) -> anyhow::Result<MyValueType> {
        user_state.infinity_loop = None;
        self.clear_cache();
        self.explore(graph, node_id, user_state)?;
        self.evaluate_all(graph, user_state)?;
        let ans = self.get_output(graph, node_id)?;
        Ok(ans)
    }

    /// Return the result of the evaluation. This example returns the first output.
    pub fn get_output(&self, graph: &MyGraph, node_id: NodeId) -> anyhow::Result<MyValueType> {
        let output_id = graph[node_id].output_ids().next().unwrap();
        // If there are multiple outputs:
        // let output_id = graph[node_id].get_output(param_name);
        let output = self
            .outputs_cache
            .get(&output_id)
            .ok_or_else(|| anyhow::format_err!("It may be in an infinite loop in get_output."))?;
        Ok(*output)
    }

    /// Always run before exploring.
    pub fn clear_cache(&mut self) {
        self.stack.clear();
        self.evaluated.clear();
        self.outputs_cache.clear();
        self.sequence.clear();
    }

    /// Explore all dependencies of this node by BFS to find the order of computation
    fn explore(
        &mut self,
        graph: &MyGraph,
        node_id: NodeId,
        user_state: &mut MyGraphState,
    ) -> anyhow::Result<()> {
        self.evaluated.clear();
        self.sequence.push(node_id);
        self.stack.push(node_id);

        while let Some(current_node_id) = self.stack.pop() {
            if graph[current_node_id]
                .input_ids()
                .flat_map(|input_id| graph.connection(input_id))
                .count()
                != 0
            {
                // Proceed to the endpoint(= the node with no input)
                let connectoins = graph[current_node_id].input_ids().flat_map(|input_id| {
                    graph
                        .connection(input_id)
                        .map(|output_id| (output_id, input_id))
                });
                for (prev_output_id, current_input_id) in connectoins {
                    let prev_node_id = graph[prev_output_id].node;
                    if self.evaluated.insert(prev_node_id) {
                        // Stores the destination node in the stack and the sequence.
                        self.sequence.push(prev_node_id);
                        self.stack.push(prev_node_id);
                    } else {
                        // If it hits a node that has been explored before reaching the endpoint, it is looping.
                        user_state.infinity_loop = Some((prev_output_id, current_input_id));
                        return Err(anyhow::format_err!(
                            "It may be in an infinite loop in explore."
                        ));
                    }
                }
            } else {
                // When get to the end, clear evaluated.
                self.evaluated.clear();
            }
        }
        Ok(())
    }

    /// Evaluate based on the calculation sequence
    /// Note that the order of computation is stored in reverse
    fn evaluate_all(
        &mut self,
        graph: &MyGraph,
        user_state: &mut MyGraphState,
    ) -> anyhow::Result<()> {
        self.evaluated.clear();
        for node_id in self.sequence.iter().rev().copied() {
            // If it's already evaluated, ignore it.
            if self.evaluated.insert(node_id) {
                let mut evaluator = NodeEvaluator {
                    graph,
                    node_id,
                    outputs_cache: &mut self.outputs_cache,
                    user_state,
                };
                evaluator.evaluate(graph)?;
            }
        }
        Ok(())
    }
}

/// Evaluate a node
struct NodeEvaluator<'a> {
    pub outputs_cache: &'a mut OutputsCache,
    pub graph: &'a MyGraph,
    pub node_id: NodeId,

    pub user_state: &'a mut MyGraphState,
}
impl NodeEvaluator<'_> {
    fn input_vector(&mut self, param_name: &str) -> anyhow::Result<egui::Vec2> {
        self.get_input(param_name)?.try_to_vec2()
    }
    fn input_scalar(&mut self, param_name: &str) -> anyhow::Result<f32> {
        self.get_input(param_name)?.try_to_scalar()
    }
    /// When this node's is evaluated, it should have already finished computing its dependencies.
    /// If they are not in the cache, an infinite loop may have occurred.
    fn get_input(&mut self, param_name: &str) -> anyhow::Result<MyValueType> {
        let input_id = self.graph[self.node_id].get_input(param_name)?;
        // The output of another node is connected.
        if let Some(output_id) = self.graph.connection(input_id) {
            // Now that we know the value is cached, return it
            if let Some(value) = self.outputs_cache.get(&output_id) {
                Ok(*value)
            } else {
                self.user_state.infinity_loop = Some((output_id, input_id));
                Err(anyhow::format_err!(
                    "It may be in an infinite loop in get_input."
                ))
            }
        } else {
            // No existing connection, take the inline value instead.
            Ok(self.graph[input_id].value)
        }
    }

    fn output_vector(&mut self, name: &str, value: egui::Vec2) -> Result<(), EguiGraphError> {
        self.populate_output(name, MyValueType::Vec2 { value })
    }
    fn output_scalar(&mut self, name: &str, value: f32) -> Result<(), EguiGraphError> {
        self.populate_output(name, MyValueType::Scalar { value })
    }
    /// After computing an output, populate the outputs cache with it.
    /// This ensures the evaluation only ever computes an output once.
    fn populate_output(
        &mut self,
        param_name: &str,
        value: MyValueType,
    ) -> Result<(), EguiGraphError> {
        let output_id = self.graph[self.node_id].get_output(param_name)?;
        self.outputs_cache.insert(output_id, value);
        Ok(())
    }

    pub fn evaluate(&mut self, graph: &MyGraph) -> anyhow::Result<()> {
        match graph[self.node_id].user_data.template {
            MyNodeTemplate::AddScalar => {
                let a = self.input_scalar("A")?;
                let b = self.input_scalar("B")?;
                self.output_scalar("out", a + b)?;
            }
            MyNodeTemplate::SubtractScalar => {
                let a = self.input_scalar("A")?;
                let b = self.input_scalar("B")?;
                self.output_scalar("out", a - b)?;
            }
            MyNodeTemplate::VectorTimesScalar => {
                let scalar = self.input_scalar("scalar")?;
                let vector = self.input_scalar("vector")?;
                self.output_scalar("out", vector * scalar)?;
            }
            MyNodeTemplate::AddVector => {
                let v1 = self.input_vector("v1")?;
                let v2 = self.input_vector("v2")?;
                self.output_vector("out", v1 + v2)?;
            }
            MyNodeTemplate::SubtractVector => {
                let v1 = self.input_vector("v1")?;
                let v2 = self.input_vector("v2")?;
                self.output_vector("out", v1 - v2)?;
            }
            MyNodeTemplate::MakeVector => {
                let x = self.input_scalar("x")?;
                let y = self.input_scalar("y")?;
                self.output_vector("out", egui::vec2(x, y))?;
            }
            MyNodeTemplate::MakeScalar => {
                let value = self.input_scalar("value")?;
                self.output_scalar("out", value)?;
            }
        }
        Ok(())
    }
}
