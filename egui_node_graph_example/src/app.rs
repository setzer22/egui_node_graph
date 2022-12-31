use std::{borrow::Cow, collections::HashMap};

use eframe::egui::{self, DragValue, TextStyle};
use egui_node_graph::*;

// ========= First, define your user data types =============

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
pub struct MyNodeContent {
    template: MyNodeTemplate,
}

impl MyNodeContent {
    pub fn new(template: MyNodeTemplate) -> Self {
        Self { template }
    }
}

/// `DataType`s are what defines the possible range of connections when
/// attaching two ports together. The graph UI will make sure to not allow
/// attaching incompatible datatypes.
#[derive(PartialEq, Eq, Debug, Clone)]
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
    Vec2(egui::Vec2),
    Scalar(f32),
}

impl MyValueType {
    /// Tries to downcast this value type to a vector
    pub fn try_to_vec2(self) -> anyhow::Result<egui::Vec2> {
        if let MyValueType::Vec2(value) = self {
            Ok(value)
        } else {
            anyhow::bail!("Invalid cast from {:?} to vec2", self)
        }
    }

    /// Tries to downcast this value type to a scalar
    pub fn try_to_scalar(self) -> anyhow::Result<f32> {
        if let MyValueType::Scalar(value) = self {
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
pub struct MyAppState {
    pub active_node: Option<NodeId>,
}

// =========== Then, you need to implement some traits ============

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait for MyDataType {
    type Value = MyValueType;

    fn is_compatible(&self, other: &Self) -> bool {
        *self == *other
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            MyDataType::Scalar => Cow::Borrowed("scalar"),
            MyDataType::Vec2 => Cow::Borrowed("2d vector"),
        }
    }
}

type MyNode = SimpleColumnNode<MyNodeContent, MyDataType>;

// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for MyNodeTemplate {
    type Node = MyNode;

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

    fn build_node(
        &self,
        position: egui::Pos2,
        app_state: &mut MyAppState,
    ) -> Self::Node {
        let mut node = MyNode::new(position, self.node_graph_label(), MyNodeContent::new(*self));
        match self {
            MyNodeTemplate::AddScalar => {
                node
                .with_input(VerticalInputPort::new("in".to_owned(), MyDataType::Scalar, None, InputKind::ConnectionOnly))
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Scalar, None))
            }
            MyNodeTemplate::AddVector => {
                node
                .with_input(VerticalInputPort::new("in".to_owned(), MyDataType::Vec2, None, InputKind::ConnectionOnly))
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Vec2, None))
            }
            MyNodeTemplate::MakeScalar => {
                node
                .with_input(
                    VerticalInputPort::new("value".to_owned(), MyDataType::Scalar, Some(1), InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Scalar(0.0))
                )
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Scalar, None))
            }
            MyNodeTemplate::MakeVector => {
                node
                .with_input(
                    VerticalInputPort::new("x".to_owned(), MyDataType::Scalar, Some(1), InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Scalar(0.0))
                )
                .with_input(
                    VerticalInputPort::new("y".to_owned(), MyDataType::Scalar, Some(1), InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Scalar(0.0))
                )
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Vec2, None))
            }
            MyNodeTemplate::SubtractScalar => {
                node
                .with_input(
                    VerticalInputPort::new("value".to_owned(), MyDataType::Scalar, Some(1), InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Scalar(0.0))
                )
                .with_input(
                    VerticalInputPort::new("minus".to_owned(), MyDataType::Scalar, None, InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Scalar(0.0))
                )
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Scalar, None))
            }
            MyNodeTemplate::SubtractVector => {
                node
                .with_input(
                    VerticalInputPort::new("value".to_owned(), MyDataType::Vec2, Some(1), InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Vec2(egui::vec2(0.0, 0.0)))
                )
                .with_input(
                    VerticalInputPort::new("minus".to_owned(), MyDataType::Vec2, None, InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Vec2(egui::vec2(0.0, 0.0)))
                )
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Vec2, None))
            }
            MyNodeTemplate::VectorTimesScalar => {
                node
                .with_input(
                    VerticalInputPort::new("scalar".to_owned(), MyDataType::Scalar, Some(1), InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Scalar(1.0))
                )
                .with_input(
                    VerticalInputPort::new("vec".to_owned(), MyDataType::Vec2, None, InputKind::ConnectionOrConstant)
                    .with_default_value(MyValueType::Vec2(egui::vec2(1.0, 1.0)))
                )
                .with_output(VerticalOutputPort::new("out".to_owned(), MyDataType::Vec2, None))
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

impl ValueTrait for MyValueType {
    type Response = MyResponse;
    fn show(&mut self, ui: &mut egui::Ui) -> (egui::Rect, Vec<MyResponse>) {
        // This trait is used to tell the library which UI to display for the
        // inline parameter widgets.
        let rect = match self {
            MyValueType::Vec2(value) => {
                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(DragValue::new(&mut value.x));
                    ui.label("y");
                    ui.add(DragValue::new(&mut value.y));
                })
            }
            MyValueType::Scalar(value) => {
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(value));
                })
            }
        }.response.rect;
        // This allows you to return your responses from the inline widgets.
        (rect, Vec::new())
    }
}

impl NodeContentTrait for MyNodeContent {
    type AppState = MyAppState;
    type Response = MyResponse;

    // This method will be called when drawing each node. This allows adding
    // extra ui elements inside the nodes. In this case, we create an "active"
    // button which introduces the concept of having an active node in the
    // graph. This is done entirely from user code with no modifications to the
    // node graph library.
    fn content_ui(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &Self::AppState,
        node_id: NodeId,
    ) -> Vec<Self::Response> {
        // This logic is entirely up to the user. In this case, we check if the
        // current node we're drawing is the active one, by comparing against
        // the value stored in the global user state, and draw different button
        // UIs based on that.

        let mut responses = vec![];
        let is_active = app_state
            .active_node
            .map(|id| id == node_id)
            .unwrap_or(false);

        // Pressing the button will emit a custom user response to either set,
        // or clear the active node. These responses do nothing by themselves,
        // the library only makes the responses available to you after the graph
        // has been drawn. See below at the update method for an example.
        if !is_active {
            if ui.button("👁 Set active").clicked() {
                responses.push(MyResponse::SetActiveNode(node_id));
            }
        } else {
            let button =
                egui::Button::new(egui::RichText::new("👁 Active").color(egui::Color32::BLACK))
                    .fill(egui::Color32::GOLD);
            if ui.add(button).clicked() {
                responses.push(MyResponse::ClearActiveNode);
            }
        }

        responses
    }
}

type MyGraph = Graph<MyNode>;

#[derive(Default)]
struct MyGraphContext;
impl GraphStyleTrait for MyGraphContext {
    type DataType = MyDataType;
    fn recommend_data_type_color(&self, data_type: &MyDataType) -> egui::Color32 {
        match data_type {
            MyDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
            MyDataType::Vec2 => egui::Color32::from_rgb(238, 207, 109),
        }
    }
}
impl GraphContextTrait for MyGraphContext {
    type Node = MyNode;
    type NodeTemplate = MyNodeTemplate;
}

type MyEditorState = GraphEditorState<MyGraphContext>;

pub struct NodeGraphExample {
    // The `GraphEditorState` is the top-level object. You "register" all your
    // custom types by specifying it as its generic parameters.
    editor: MyEditorState,
    app_state: MyAppState,
}

impl Default for NodeGraphExample {
    fn default() -> Self {
        Self {
            editor: GraphEditorState::new(1.0, MyGraphContext::default()),
            app_state: MyAppState { active_node: None },
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
                self.editor.draw_graph_editor(
                    ui, AllMyNodeTemplates, &mut self.app_state
                )
            })
            .inner;
        for node_response in graph_response.node_responses {
            // Here, we ignore all other graph events. But you may find
            // some use for them. For example, by playing a sound when a new
            // connection is created
            if let NodeResponse::Content(user_event) = node_response {
                match user_event {
                    MyResponse::SetActiveNode(node) => self.app_state.active_node = Some(node),
                    MyResponse::ClearActiveNode => self.app_state.active_node = None,
                }
            }
        }

        if let Some(node_id) = self.app_state.active_node {
            if self.editor.graph.node(node_id).is_some() {
                let text = match evaluate_node(&self.editor.graph, node_id, &mut HashMap::new()) {
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
                self.app_state.active_node = None;
            }
        }
    }
}

type OutputsCache = HashMap<OutputId, MyValueType>;

pub fn evaluate(
    graph: &MyGraph,
    node_id: NodeId,
) -> anyhow::Result<MyValueType> {
    enum InputType {
        Connections(Vec<InputId>),
        Constant(MyValueType),
    }

    impl InputType {
        fn dependencies(&self) -> Vec<InputId> {
            match self {
                InputType::Connections(v) => v.clone(),
                InputType::Constant(_) => vec![],
            }
        }
    }

    enum NodeInput {
        AddScalar(InputType),
        AddVector(InputType),
        MakeScalar(InputType),
        MakeVector {
            x: InputType,
            y: InputType,
        },
        SubtractScalar {
            value: InputType,
            minus: InputType,
        },
        SubtractVector {
            value: InputType,
            minus: InputType,
        },
        VectorTimesScalar {
            scalar: InputType,
            vec: InputType,
        },
    }

    impl NodeInput {
        fn dependencies(&self) -> Vec<InputId> {
            match self {
                NodeInput::AddScalar(v) => v.dependencies(),
                NodeInput::AddVector(v) => v.dependencies(),
                NodeInput::MakeScalar(v) => v.dependencies(),
                NodeInput::MakeVector { x, y } => x.dependencies().iter().cloned().chain(y.dependencies().iter().cloned()).collect(),
                NodeInput::SubtractScalar { value, minus } => value.dependencies().iter().cloned().chain(minus.dependencies().iter().cloned()).collect(),
                NodeInput::SubtractVector { value, minus } => value.dependencies().iter().cloned().chain(minus.dependencies().iter().cloned()).collect(),
                NodeInput::VectorTimesScalar { scalar, vec } => scalar.dependencies().iter().cloned().chain(vec.dependencies().iter().cloned()).collect(),
            }
        }
    }

    struct Evaluatee {
        node_id: NodeId,
        input: NodeInput,
    }

    fn collect_inputs(port: &VerticalInputPort<MyDataType>) -> InputType {
        if let Some(constant) = port.using_default_value() {
            InputType::Constant(constant)
        } else {
            InputType::Connections(port.iter_hooks().filter_map(|(_, c)| c.map(|c| c.as_input()).flatten()).collect())
        }
    }

    impl Evaluatee {
        fn new(node_id: NodeId, graph: &MyGraph) -> Evaluatee {
            let node = graph.node(node_id).unwrap();
            let input = match node.content.template {
                MyNodeTemplate::AddScalar => {
                    NodeInput::AddScalar(collect_inputs(node.inputs.iter().next().unwrap().1))
                }
                MyNodeTemplate::AddVector => {
                    NodeInput::AddVector(collect_inputs(node.inputs.iter().next().unwrap().1))
                }
                MyNodeTemplate::MakeScalar => {
                    NodeInput::MakeScalar(collect_inputs(node.inputs.iter().next().unwrap().1))
                }
                MyNodeTemplate::MakeVector => {
                    NodeInput::MakeVector {
                        x: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "x").unwrap().1),
                        y: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "y").unwrap().1),
                    }
                }
                MyNodeTemplate::SubtractScalar => {
                    NodeInput::SubtractScalar {
                        value: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "value").unwrap().1),
                        minus: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "minus").unwrap().1),
                    }
                }
                MyNodeTemplate::SubtractVector => {
                    NodeInput::SubtractVector {
                        value: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "value").unwrap().1),
                        minus: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "minus").unwrap().1),
                    }
                }
            };
            Evaluatee { node_id, input }
        }
    }

    let mut node_queue = Vec::new();
    node_queue.push(Evaluatee::new(node_id, graph));




    anyhow::bail!("unfinished")
}

/// Recursively evaluates all dependencies of this node, then evaluates the node itself.
pub fn evaluate_node(
    graph: &MyGraph,
    node_id: NodeId,
    outputs_cache: &mut OutputsCache,
) -> anyhow::Result<MyValueType> {
    // To solve a similar problem as creating node types above, we define an
    // Evaluator as a convenience. It may be overkill for this small example,
    // but something like this makes the code much more readable when the
    // number of nodes starts growing.

    struct Evaluator<'a> {
        graph: &'a MyGraph,
        outputs_cache: &'a mut OutputsCache,
        node_id: NodeId,
    }
    impl<'a> Evaluator<'a> {
        fn new(graph: &'a MyGraph, outputs_cache: &'a mut OutputsCache, node_id: NodeId) -> Self {
            Self {
                graph,
                outputs_cache,
                node_id,
            }
        }
        fn evaluate_input(&mut self, name: &str) -> anyhow::Result<MyValueType> {
            // Calling `evaluate_input` recursively evaluates other nodes in the
            // graph until the input value for a paramater has been computed.
            evaluate_input(self.graph, self.node_id, name, self.outputs_cache)
        }
        fn populate_output(
            &mut self,
            name: &str,
            value: MyValueType,
        ) -> anyhow::Result<MyValueType> {
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
            populate_output(self.graph, self.outputs_cache, self.node_id, name, value)
        }
        fn input_vector(&mut self, name: &str) -> anyhow::Result<egui::Vec2> {
            self.evaluate_input(name)?.try_to_vec2()
        }
        fn input_scalar(&mut self, name: &str) -> anyhow::Result<f32> {
            self.evaluate_input(name)?.try_to_scalar()
        }
        fn output_vector(&mut self, name: &str, value: egui::Vec2) -> anyhow::Result<MyValueType> {
            self.populate_output(name, MyValueType::Vec2(value))
        }
        fn output_scalar(&mut self, name: &str, value: f32) -> anyhow::Result<MyValueType> {
            self.populate_output(name, MyValueType::Scalar(value))
        }
    }

    let node = match graph.node(node_id) {
        Some(node) => node,
        None => anyhow::bail!("Missing node {node_id:?}"),
    };

    let mut evaluator = Evaluator::new(graph, outputs_cache, node_id);
    match node.content.template {
        MyNodeTemplate::AddScalar => {
            let a = evaluator.input_scalar("A")?;
            let b = evaluator.input_scalar("B")?;
            evaluator.output_scalar("out", a + b)
        }
        MyNodeTemplate::SubtractScalar => {
            let a = evaluator.input_scalar("A")?;
            let b = evaluator.input_scalar("B")?;
            evaluator.output_scalar("out", a - b)
        }
        MyNodeTemplate::VectorTimesScalar => {
            let scalar = evaluator.input_scalar("scalar")?;
            let vector = evaluator.input_vector("vector")?;
            evaluator.output_vector("out", vector * scalar)
        }
        MyNodeTemplate::AddVector => {
            let v1 = evaluator.input_vector("v1")?;
            let v2 = evaluator.input_vector("v2")?;
            evaluator.output_vector("out", v1 + v2)
        }
        MyNodeTemplate::SubtractVector => {
            let v1 = evaluator.input_vector("v1")?;
            let v2 = evaluator.input_vector("v2")?;
            evaluator.output_vector("out", v1 - v2)
        }
        MyNodeTemplate::MakeVector => {
            let x = evaluator.input_scalar("x")?;
            let y = evaluator.input_scalar("y")?;
            evaluator.output_vector("out", egui::vec2(x, y))
        }
        MyNodeTemplate::MakeScalar => {
            let value = evaluator.input_scalar("value")?;
            evaluator.output_scalar("out", value)
        }
    }
}

fn populate_output(
    graph: &MyGraph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
    value: MyValueType,
) -> anyhow::Result<MyValueType> {
    let output_id = graph[node_id].get_output(param_name)?;
    outputs_cache.insert(output_id, value);
    Ok(value)
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
