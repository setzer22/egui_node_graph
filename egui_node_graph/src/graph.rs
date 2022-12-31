use super::*;
use std::{
    cell::RefCell,
    sync::{Mutex, Arc},
    collections::HashMap,
};
use thiserror::Error as ThisError;

#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "persistence")]
fn shown_inline_default() -> bool {
    true
}

/// The graph, containing nodes, input parameters and output parameters. Because
/// graphs are full of self-referential structures, this type uses the `slotmap`
/// crate to represent all the inner references in the data.
#[derive(Debug)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct Graph<Node> {
    /// The Nodes of the graph
    nodes: SlotMap<NodeId, Node>,
    /// Connects the input of a port to the output of its predecessor that
    /// produces it
    connections: HashMap<ConnectionId, ConnectionId>,
    /// Keep track of what connections have dropped while something in the graph
    /// was mutable
    #[cfg_attr(feature = "persistence", serde(skip))]
    dropped_connections: DroppedConnections,
    /// This field is used as a buffer inside of process_dropped_connections().
    /// We maintain it as a field so that we don't need to repeatedly reallocate
    /// the memory for this buffer.
    #[cfg_attr(feature = "persistence", serde(skip))]
    drop_buffer: Vec<ConnectionId>,
}

impl<Node: NodeTrait> Graph<Node> {
    pub fn new() -> Self {
        Self{
            nodes: SlotMap::default(),
            connections: Default::default(),
            dropped_connections: Default::default(),
            drop_buffer: Vec::new(),
        }
    }

    pub fn add_node(
        &mut self,
        node: Node,
    ) -> NodeId {
        self.nodes.insert(node)
    }

    pub fn node(&self, key: NodeId) -> Option<&Node> {
        self.nodes.get(key)
    }

    /// Operate on a mutable node. If any connections are dropped while mutating
    /// the node, the graph will automatically update itself. You may optionally
    /// have your function return a value.
    ///
    /// If the `node_id` is invalid then nothing will happen. To provide a fallback
    /// behavior when the node is missing use [`node_mut_or`]
    pub fn node_mut<F: FnOnce(&mut Node) -> T, T>(&mut self, node_id: NodeId, f: F) -> Result<T, ()> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            let output = f(node);

            // If the node dropped any connections while it was being mutated,
            // we need to handle that to maintain consistency in the graph.
            self.process_dropped_connections();
            Ok(output)
        } else {
            Err(())
        }
    }

    /// Operate on a mutable node or else perform some fallback behavior. If any
    /// connections are dropped while mutating the node, the graph will automatically
    /// update itself. You may optionally have your function return a value.
    pub fn node_mut_or<F: FnOnce(&mut Node) -> T, E: FnOnce() -> U, T, U>(
        &mut self,
        key: NodeId,
        found_f: F,
        else_f: E
    ) -> Result<T, U> {
        if let Some(node) = self.nodes.get_mut(key) {
            let output = found_f(node);
            self.process_dropped_connections();
            Ok(output)
        } else {
            Err(else_f())
        }
    }

    /// Removes a node from the graph with the given `node_id`. This also removes
    /// any incoming or outgoing connections from that node
    ///
    /// This function returns the list of connections that has been removed
    /// after deleting this node as input-output pairs. Note that one of the two
    /// ids in the pair (the one on `node_id`'s end) will be invalid after
    /// calling this function.
    pub fn remove_node(&mut self, node_id: NodeId) -> Option<(Node, Vec<(InputId, OutputId)>)> {
        if let Some(mut removed_node) = self.nodes.remove(node_id) {
            let dropped = removed_node.drop_all_connections().into_iter()
                .map(|(port_id, hook_id, connection)| {
                    match port_id {
                        PortId::Input(input_port_id) => {
                            let input_id = InputId(node_id, input_port_id, hook_id);
                            let output_id = connection.assume_output();
                            (input_id, output_id)
                        },
                        PortId::Output(output_port_id) => {
                            let output_id = OutputId(node_id, output_port_id, hook_id);
                            let input_id = connection.assume_input();
                            (input_id, output_id)
                        }
                    }
                }).collect();

            self.process_dropped_connections();
            return Some((removed_node, dropped));
        }

        return None;
    }

    /// Adds a connection to the graph from an output to an input.
    // TODO(@mxgrey): Should we test that the data types are compatible?
    pub fn add_connection(&mut self, output_id: OutputId, input_id: InputId) -> Result<(), GraphAddConnectionError> {
        self.connections.insert(input_id.into(), output_id.into());
        self.connections.insert(output_id.into(), input_id.into());

        let result = {
            // Note: We create the tokens inside this nested scope so that if
            // either of the nodes is non-existent, the token will drop before
            // we call [`process_dropped_connections`]

            // Also Note: The connection tokens intentionally contain the id of the
            // complementary ConnectionId
            let token_for_output_hook = ConnectionToken::new(
                input_id.into(),
                self.dropped_connections.clone(),
            );
            let token_for_input_hook = ConnectionToken::new(
                output_id.into(),
                self.dropped_connections.clone(),
            );

            // 1. Tell the output node about the connection
            // 2. If the output node connected successfully, tell the input node
            // If either node fails to connect, then its ConnectionToken will
            // drop once we exit this scope. When the token drops, it will let
            // the dropped_connections field know, and then self.process_dropped_connections()
            // will clean up any lingering traces of the connection.
            match self.node_mut_or(
                output_id.node(),
                |output_node| {
                    output_node.connect(
                        output_id.into(),
                        token_for_output_hook,
                    ).map_err(|err| GraphAddConnectionError::OutputNodeError{node: output_id.0, err})
                },
                || ()
            ) {
                Ok(result) => result,
                Err(()) => Err(GraphAddConnectionError::BadOutputNode(output_id.0)),
            }.and_then(|_| {
                match self.node_mut_or(
                    input_id.node(),
                    |input_node| {
                        input_node.connect(
                            input_id.into(),
                            token_for_input_hook,
                        ).map_err(|err| GraphAddConnectionError::InputNodeError{node: input_id.0, err})
                    },
                    || ()
                ) {
                    Ok(result) => result,
                    Err(()) => Err(GraphAddConnectionError::BadInputNode(input_id.0))
                }
            })
        };

        self.process_dropped_connections();
        return result;
    }

    pub fn drop_connection(&mut self, id: ConnectionId) -> Result<ConnectionId, GraphDropConnectionError> {
        match self.node_mut_or(
            id.node(),
            |node| {
                node.drop_connection(id.into())
                .map_err(|err| GraphDropConnectionError::NodeError{node: id.node(), err})
            },
            || ()
        ) {
            Ok(result) => result,
            Err(()) => Err(GraphDropConnectionError::BadNodeId(id.node()))
        }
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item=(NodeId, &Node)> + '_ {
        self.nodes.iter()
    }

    pub fn iter_connections(&self) -> impl Iterator<Item=(OutputId, InputId)> + '_ {
        self.connections.iter().filter_map(
            |(o, i)| o.as_output().map(|o| (o, i.assume_input()))
        )
    }

    pub fn connection(&self, id: &ConnectionId) -> Option<ConnectionId> {
        self.connections.get(id).copied()
    }

    /// This will be called automatically after each mutable graph function so
    /// users generally should not have to call this. However, if a Node
    /// implementation defies the recommended practice of only allowing
    /// connections to drop while mutable, then this function can be called to
    /// correct the graph.
    pub fn process_dropped_connections(&mut self) {
        // If we keep self.dropped_connections locked and iterate over it
        // directly to disconnect the complementary hooks, its mutex would
        // deadlock when the dropped ConnectionTokens of the complementary hook
        // try to lock it.
        //
        // So instead we temporarily lock the mutex of dropped_connections and
        // transfer its information into drop_buffer. Then iterate over drop_buffer,
        // telling the complementary hook to drop their connection. As those connections
        // drop, the ConnectionToken will lock self.dropped_connections and add their
        // value into it.
        //
        // self.drop_buffer is kept as a field so we don't need to dynamically
        // reallocate its memory every time we need to transfer the data into it.
        self.drop_buffer.extend(
            self.dropped_connections
            .lock().expect("the dropped_connections mutex is poisoned")
            .borrow_mut().drain(..)
        );

        let mut complements = Vec::new();
        for connection in self.drop_buffer.drain(..) {
            if let Some(complement) = self.connections.remove(&connection) {
                complements.push((complement, connection));
            }
        }

        for (complement, original) in complements {
            self.drop_connection(complement).ok();
            if let Some(connection) = self.connections.remove(&complement) {
                assert!(original == connection);
            }
            self.node_mut(
                complement.node(),
                |node| {
                    node.drop_connection(complement.into())
                }
            ).ok();
        }

        // Clear both buffers.
        // drop_buffer can be emptied because we've already processed all its contents
        self.drop_buffer.clear();
        // dropped_connections should be cleared even though it was drained
        // earlier because the complementary tokens will have filled it with
        // irrelevant dropped connection data
        self.dropped_connections
        .lock().expect("the dropped_connections mutex is poisoned")
        .borrow_mut().clear();
    }
}

#[derive(ThisError, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortAddConnectionError {
    #[error("there is no hook [{0:?}] for this port")]
    BadHook(HookId),
    #[error("hook [{0:?}] is already occupied")]
    HookOccupied(HookId),
}

#[derive(ThisError, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeAddConnectionError {
    #[error("there is no port [{0:?}] for this node")]
    BadPort(PortId),
    #[error("port [{port:?}] had a connection error: {err}")]
    PortError{port: PortId, err: PortAddConnectionError},
}

#[derive(ThisError, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphAddConnectionError {
    #[error("attempting to add a connection to a NodeId that doesn't exist: {0:?}")]
    BadOutputNode(NodeId),
    #[error("attempting to add a connection to a NodeId that doesn't exist: {0:?}")]
    BadInputNode(NodeId),
    #[error("error from output node {node:?} while attempting to add a connection: {err:?}")]
    OutputNodeError{node: NodeId, err: NodeAddConnectionError},
    #[error("error from input node {node:?} while attempting to add a connection: {err:?}")]
    InputNodeError{node: NodeId, err: NodeAddConnectionError},
}

#[derive(ThisError, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDropConnectionError {
    #[error("there is no hook [{0:?}] for this port")]
    BadHook(HookId),
    #[error("hook [{0:?}] does not have any connection")]
    NoConnection(HookId),
}

#[derive(ThisError, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeDropConnectionError {
    #[error("there is no port [{0:?}] for this node")]
    BadPort(PortId),
    #[error("port [{port:?}] had a drop connection error: {err}")]
    PortError{port: PortId, err: PortDropConnectionError},
}

#[derive(ThisError, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDropConnectionError {
    #[error("node [{0:?}] does not exist in the graph")]
    BadNodeId(NodeId),
    #[error("node [{node:?}] experienced an error: {err}")]
    NodeError{node: NodeId, err: NodeDropConnectionError},
}

impl<Node: NodeTrait> Default for Graph<Node> {
    fn default() -> Self {
        Self::new()
    }
}

// TODO(@mxgrey): Can this be safely replaced with Rc<RefCell<Vec<ConnectionId>>>?
// Should we use something like #[cfg(feature = "single_threaded")] to let the
// user choose the more efficient alternative?
pub type DroppedConnections = Arc<Mutex<RefCell<Vec<ConnectionId>>>>;

#[derive(Debug)]
pub struct ConnectionToken {
    connected_to: ConnectionId,
    drop_list: DroppedConnections,
}

impl ConnectionToken {
    /// Only the Graph class is allowed to create connection tokens
    fn new(
        connected_to: ConnectionId,
        drop_list: DroppedConnections,
    ) -> Self {
        Self{connected_to, drop_list}
    }

    pub fn connected_to(&self) -> ConnectionId {
        self.connected_to
    }
}

impl Drop for ConnectionToken {
    fn drop(&mut self) {
        self.drop_list.lock()
        .and_then(|list_cell| {
            list_cell.borrow_mut().push(self.connected_to);
            Ok(())
        }).ok();
    }
}
