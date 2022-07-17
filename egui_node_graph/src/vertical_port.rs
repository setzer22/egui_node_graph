use super::*;

/// The three kinds of input params. These describe how the graph must behave
/// with respect to inline widgets and connections for this parameter.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub enum InputKind {
    /// No constant value can be set. Only incoming connections can produce it
    ConnectionOnly,
    /// Only a constant value can be set. No incoming connections accepted.
    ConstantOnly,
    /// Both incoming connections and constants are accepted. Connections take
    /// precedence over the constant values.
    ConnectionOrConstant,
}

pub enum Side {
    Left,
    Right,
}

/// A port that displays vertically.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "persistence", derive(Serialize, Deserialize))]
pub struct VerticalPort<DataType> {
    /// The data type of this node. Used to determine incoming connections. This
    /// should always match the type of the InputParamValue, but the property is
    /// not actually enforced.
    pub typ: DataType,
    /// The limit on number of connections this port allows. A None value means
    /// there is no limit.
    pub connection_limit: Option<usize>,
    /// What side of the block will this port be rendered on
    pub side: Side,
    /// When true, the node is shown inline inside the node graph.
    #[cfg_attr(feature = "persistence", serde(default = "shown_inline_default"))]
    pub shown_inline: bool,
}

pub struct VerticalInputPort<DataType: DataTypeTrait> {
    /// The input kind. See [`InputKind`]
    pub kind: InputKind,
    pub default_value: Vec<DataType::Value>,
    pub port: VerticalPort<DataType>,
}

