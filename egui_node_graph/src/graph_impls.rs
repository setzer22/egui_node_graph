use std::num::NonZeroU32;

use super::*;

impl<NodeData, DataType, ValueType> Graph<NodeData, DataType, ValueType> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::default(),
            inputs: SlotMap::default(),
            outputs: SlotMap::default(),
            connections: SecondaryMap::default(),
        }
    }

    pub fn add_node(
        &mut self,
        label: String,
        user_data: NodeData,
        f: impl FnOnce(&mut Graph<NodeData, DataType, ValueType>, NodeId),
    ) -> NodeId {
        let node_id = self.nodes.insert_with_key(|node_id| {
            Node {
                id: node_id,
                label,
                // These get filled in later by the user function
                inputs: Vec::default(),
                outputs: Vec::default(),
                user_data,
            }
        });

        f(self, node_id);

        node_id
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_wide_input_param(
        &mut self,
        node_id: NodeId,
        name: String,
        typ: DataType,
        value: ValueType,
        kind: InputParamKind,
        max_connections: Option<NonZeroU32>,
        shown_inline: bool,
    ) -> InputId {
        let input_id = self.inputs.insert_with_key(|input_id| InputParam {
            id: input_id,
            typ,
            value,
            kind,
            node: node_id,
            max_connections,
            shown_inline,
        });
        self.nodes[node_id].inputs.push((name, input_id));
        input_id
    }

    pub fn add_input_param(
        &mut self,
        node_id: NodeId,
        name: String,
        typ: DataType,
        value: ValueType,
        kind: InputParamKind,
        shown_inline: bool,
    ) -> InputId {
        self.add_wide_input_param(
            node_id,
            name,
            typ,
            value,
            kind,
            NonZeroU32::new(1),
            shown_inline,
        )
    }

    pub fn remove_input_param(&mut self, param: InputId) {
        let node = self[param].node;
        self[node].inputs.retain(|(_, id)| *id != param);
        self.inputs.remove(param);
        self.connections.retain(|i, _| i != param);
    }

    pub fn remove_output_param(&mut self, param: OutputId) {
        let node = self[param].node;
        self[node].outputs.retain(|(_, id)| *id != param);
        self.outputs.remove(param);
        for (_, conns) in &mut self.connections {
            conns.retain(|o| *o != param);
        }
    }

    pub fn add_output_param(&mut self, node_id: NodeId, name: String, typ: DataType) -> OutputId {
        let output_id = self.outputs.insert_with_key(|output_id| OutputParam {
            id: output_id,
            node: node_id,
            typ,
        });
        self.nodes[node_id].outputs.push((name, output_id));
        output_id
    }

    /// Removes a node from the graph with given `node_id`. This also removes
    /// any incoming or outgoing connections from that node
    ///
    /// This function returns the list of connections that has been removed
    /// after deleting this node as input-output pairs. Note that one of the two
    /// ids in the pair (the one on `node_id`'s end) will be invalid after
    /// calling this function.
    pub fn remove_node(&mut self, node_id: NodeId) -> (Node<NodeData>, Vec<(InputId, OutputId)>) {
        let mut disconnect_events = vec![];

        for (i, conns) in &mut self.connections {
            conns.retain(|o| {
                if self.outputs[*o].node == node_id || self.inputs[i].node == node_id {
                    disconnect_events.push((i, *o));
                    false
                } else {
                    true
                }
            });
        }

        // NOTE: Collect is needed because we can't borrow the input ids while
        // we remove them inside the loop.
        for input in self[node_id].input_ids().collect::<SVec<_>>() {
            self.inputs.remove(input);
        }
        for output in self[node_id].output_ids().collect::<SVec<_>>() {
            self.outputs.remove(output);
        }
        let removed_node = self.nodes.remove(node_id).expect("Node should exist");

        (removed_node, disconnect_events)
    }

    pub fn remove_connection(&mut self, input_id: InputId, output_id: OutputId) -> bool {
        self.connections
            .get_mut(input_id)
            .map(|conns| {
                let old_size = conns.len();
                conns.retain(|id| id != &output_id);

                // connection removed if `conn` size changes
                old_size != conns.len()
            })
            .unwrap_or(false)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().map(|(id, _)| id)
    }

    pub fn add_connection(&mut self, output: OutputId, input: InputId, pos: usize) {
        if !self.connections.contains_key(input) {
            self.connections.insert(input, Vec::default());
        }

        let max_connections = self
            .get_input(input)
            .max_connections
            .map(NonZeroU32::get)
            .unwrap_or(std::u32::MAX) as usize;
        let already_in = self.connections[input].contains(&output);

        // connecting twice to the same port is a no-op
        // even for wide ports.
        if already_in {
            return;
        }

        if self.connections[input].len() == max_connections {
            // if full, replace the connected output
            self.connections[input][pos] = output;
        } else {
            // otherwise, insert at a selected position
            self.connections[input].insert(pos, output);
        }
    }

    pub fn iter_connection_groups(&self) -> impl Iterator<Item = (InputId, Vec<OutputId>)> + '_ {
        self.connections.iter().map(|(i, conns)| (i, conns.clone()))
    }

    pub fn iter_connections(&self) -> impl Iterator<Item = (InputId, OutputId)> + '_ {
        self.iter_connection_groups()
            .flat_map(|(i, conns)| conns.into_iter().map(move |o| (i, o)))
    }

    pub fn connections(&self, input: InputId) -> Vec<OutputId> {
        self.connections.get(input).cloned().unwrap_or_default()
    }

    pub fn connection(&self, input: InputId) -> Option<OutputId> {
        let is_limit_1 = self.get_input(input).max_connections == NonZeroU32::new(1);
        let connections = self.connections(input);

        if is_limit_1 && connections.len() == 1 {
            connections.into_iter().next()
        } else {
            None
        }
    }

    pub fn any_param_type(&self, param: AnyParameterId) -> Result<&DataType, EguiGraphError> {
        match param {
            AnyParameterId::Input(input) => self.inputs.get(input).map(|x| &x.typ),
            AnyParameterId::Output(output) => self.outputs.get(output).map(|x| &x.typ),
        }
        .ok_or(EguiGraphError::InvalidParameterId(param))
    }

    pub fn try_get_input(&self, input: InputId) -> Option<&InputParam<DataType, ValueType>> {
        self.inputs.get(input)
    }

    pub fn get_input(&self, input: InputId) -> &InputParam<DataType, ValueType> {
        &self.inputs[input]
    }

    pub fn try_get_output(&self, output: OutputId) -> Option<&OutputParam<DataType>> {
        self.outputs.get(output)
    }

    pub fn get_output(&self, output: OutputId) -> &OutputParam<DataType> {
        &self.outputs[output]
    }
}

impl<NodeData, DataType, ValueType> Default for Graph<NodeData, DataType, ValueType> {
    fn default() -> Self {
        Self::new()
    }
}

impl<NodeData> Node<NodeData> {
    pub fn inputs<'a, DataType, DataValue>(
        &'a self,
        graph: &'a Graph<NodeData, DataType, DataValue>,
    ) -> impl Iterator<Item = &InputParam<DataType, DataValue>> + 'a {
        self.input_ids().map(|id| graph.get_input(id))
    }

    pub fn outputs<'a, DataType, DataValue>(
        &'a self,
        graph: &'a Graph<NodeData, DataType, DataValue>,
    ) -> impl Iterator<Item = &OutputParam<DataType>> + 'a {
        self.output_ids().map(|id| graph.get_output(id))
    }

    pub fn input_ids(&self) -> impl Iterator<Item = InputId> + '_ {
        self.inputs.iter().map(|(_name, id)| *id)
    }

    pub fn output_ids(&self) -> impl Iterator<Item = OutputId> + '_ {
        self.outputs.iter().map(|(_name, id)| *id)
    }

    pub fn get_input(&self, name: &str) -> Result<InputId, EguiGraphError> {
        self.inputs
            .iter()
            .find(|(param_name, _id)| param_name == name)
            .map(|x| x.1)
            .ok_or_else(|| EguiGraphError::NoParameterNamed(self.id, name.into()))
    }

    pub fn get_output(&self, name: &str) -> Result<OutputId, EguiGraphError> {
        self.outputs
            .iter()
            .find(|(param_name, _id)| param_name == name)
            .map(|x| x.1)
            .ok_or_else(|| EguiGraphError::NoParameterNamed(self.id, name.into()))
    }
}

impl<DataType, ValueType> InputParam<DataType, ValueType> {
    pub fn value(&self) -> &ValueType {
        &self.value
    }

    pub fn kind(&self) -> InputParamKind {
        self.kind
    }

    pub fn node(&self) -> NodeId {
        self.node
    }
}
