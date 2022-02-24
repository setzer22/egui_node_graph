use super::*;

#[derive(Debug, thiserror::Error)]
pub enum EguiGraphError {
    #[error("Node {0:?} has no parameter named {1}")]
    NoParameterNamed(NodeId, String),

    #[error("Parameter {0:?} was not found in the graph.")]
    InvalidParameterId(AnyParameterId),
}
