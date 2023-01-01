use std::{borrow::Cow, collections::{HashMap, HashSet}};

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

impl From<f32> for MyValueType {
    fn from(value: f32) -> Self {
        MyValueType::Scalar(value)
    }
}

impl From<egui::Vec2> for MyValueType {
    fn from(value: egui::Vec2) -> Self {
        MyValueType::Vec2(value)
    }
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
        _app_state: &mut MyAppState,
    ) -> Self::Node {
        let node = MyNode::new(position, self.node_graph_label(), MyNodeContent::new(*self));
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
                let text = match evaluate(&self.editor.graph, node_id) {
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

pub fn evaluate(
    graph: &MyGraph,
    node_id: NodeId,
) -> anyhow::Result<MyValueType> {

    #[derive(Debug)]
    enum InputType {
        Connections(Vec<OutputId>),
        Constant(MyValueType),
    }

    impl InputType {
        fn dependencies(&self) -> Vec<OutputId> {
            match self {
                InputType::Connections(v) => v.clone(),
                InputType::Constant(_) => vec![],
            }
        }

        fn values(&self, evaluations: &HashMap<(NodeId, PortId), MyValueType>) -> Vec<MyValueType> {
            match self {
                InputType::Connections(inputs) => {
                    inputs.iter().map(|OutputId(node, port, _)| {
                        evaluations.get(&(*node, (*port).into())).unwrap().clone()
                    }).collect()
                }
                InputType::Constant(value) => vec![value.clone()],
            }
        }

        fn sum_scalar_values(&self, evaluations: &HashMap<(NodeId, PortId), MyValueType>) -> f32 {
            self.values(evaluations).iter().map(|v| v.try_to_scalar().unwrap()).fold(0_f32, |a, b| a + b)
        }

        fn sum_vector_values(&self, evaluations: &HashMap<(NodeId, PortId), MyValueType>) -> egui::Vec2 {
            self.values(evaluations).iter().map(|v| v.try_to_vec2().unwrap()).fold(egui::vec2(0.0, 0.0), |a, b| a + b)
        }
    }

    #[derive(Debug)]
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
        fn new(node_id: NodeId, graph: &MyGraph) -> Self {
            let node = graph.node(node_id).unwrap();
            match &node.content.template {
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
                MyNodeTemplate::VectorTimesScalar => {
                    NodeInput::VectorTimesScalar {
                        scalar: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "scalar").unwrap().1),
                        vec: collect_inputs(node.inputs.iter().find(|p| p.1.base.label == "vec").unwrap().1),
                    }
                }
            }
        }

        fn find_rank(&self, evaluatees: &HashMap<NodeId, (usize, NodeInput)>) -> Option<usize> {
            let mut rank = 0;
            for dep in self.dependencies() {
                if let Some((dep_rank, _)) = evaluatees.get(&dep.node()) {
                    // The rank of this node is the highest rank of its
                    // dependencies, plus one.
                    rank = rank.max(*dep_rank+1);
                } else {
                    // This node has an unranked dependency, so we cannot rank
                    // it yet.
                    return None;
                }
            }

            Some(rank)
        }

        fn dependencies(&self) -> Vec<OutputId> {
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

        fn evaluate(&self, evaluations: &HashMap<(NodeId, PortId), MyValueType>) -> MyValueType {
            match &self {
                NodeInput::AddScalar(input) => {
                    input.sum_scalar_values(evaluations).into()
                }
                NodeInput::AddVector(input) => {
                    input.sum_vector_values(evaluations).into()
                }
                NodeInput::MakeScalar(input) => {
                    // To gracefully handle cases where there are no connections
                    // and no constant value set, we implement this the same way
                    // as AddScalar
                    input.sum_scalar_values(evaluations).into()
                }
                NodeInput::MakeVector { x, y } => {
                    // To gracefully handle cases where there are no connections
                    // and no constant value set, we implement this similarly to
                    // AddScalar
                    let x = x.sum_scalar_values(evaluations);
                    let y = y.sum_scalar_values(evaluations);
                    egui::vec2(x, y).into()
                }
                NodeInput::SubtractScalar { value, minus } => {
                    let value = value.sum_scalar_values(evaluations);
                    let minus = minus.sum_scalar_values(evaluations);
                    (value - minus).into()
                }
                NodeInput::SubtractVector { value, minus } => {
                    let value = value.sum_vector_values(evaluations);
                    let minus = minus.sum_vector_values(evaluations);
                    (value - minus).into()
                }
                NodeInput::VectorTimesScalar { scalar, vec } => {
                    let scalar = scalar.sum_scalar_values(evaluations);
                    let vec = vec.sum_vector_values(evaluations);
                    (scalar * vec).into()
                }
            }
        }
    }

    fn collect_inputs(port: &VerticalInputPort<MyDataType>) -> InputType {
        if let Some(constant) = port.using_default_value() {
            InputType::Constant(constant)
        } else {
            let connections = port.iter_hooks().filter_map(|(_, c)| c.map(|c| c.as_output()).flatten()).collect();
            InputType::Connections(connections)
        }
    }

    let mut ranking_queue = HashMap::<NodeId, NodeInput>::new();
    let mut evaluatees = HashMap::<NodeId, (usize, NodeInput)>::new();
    ranking_queue.insert(node_id, NodeInput::new(node_id, graph));
    while !ranking_queue.is_empty() {
        let mut next_queue = HashMap::<NodeId, NodeInput>::new();
        next_queue.reserve(ranking_queue.len());
        let original_queue_size = ranking_queue.len();
        let mut entries_added = false;

        for (node_id, node_input) in ranking_queue {
            if let Some(rank) = node_input.find_rank(&evaluatees) {
                evaluatees.insert(node_id, (rank, node_input));
            } else {
                for dep in node_input.dependencies() {
                    entries_added = true;
                    next_queue.insert(dep.node(), NodeInput::new(dep.node(), graph));
                }
                next_queue.insert(node_id, node_input);
            }
        }

        if original_queue_size == next_queue.len() && !entries_added {
            anyhow::bail!("circular dependency!");
        }

        ranking_queue = next_queue;
    }

    let mut evaluation_queue: Vec<(usize, NodeId, NodeInput)> = evaluatees.into_iter().map(|(id, (r, e))| (r, id, e)).collect();
    evaluation_queue.sort_by(|(r_a, _, _), (r_b, _, _)| r_a.cmp(r_b));

    let mut evaluations = HashMap::<(NodeId, PortId), MyValueType>::new();
    for (_, node_id, evaluatee) in evaluation_queue {
        if let Some((output_port, _)) = graph.node(node_id).unwrap().outputs.iter().next() {
            let evaluation = evaluatee.evaluate(&evaluations);
            evaluations.insert((node_id, output_port.into()), evaluation);
        } else {
            anyhow::bail!("missing output port for node {:?}", node_id);
        }
    }

    if let Some((output_port, _)) = graph.node(node_id).unwrap().outputs.iter().next() {
        return evaluations.get(&(node_id, output_port.into())).cloned().ok_or_else(
            || anyhow::format_err!("failed to include active node {:?} in evaluation", node_id)
        );
    } else {
        anyhow::bail!("missing output port for active node {:?}", node_id);
    }
}
