slotmap::new_key_type! { pub struct NodeId; }
slotmap::new_key_type! { pub struct HookId; }
slotmap::new_key_type! { pub struct InputPortId; }
slotmap::new_key_type! { pub struct OutputPortId; }

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InputId(NodeId, InputPortId, HookId);

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OutputId(NodeId, OutputPortId, HookId);

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ConnectionId {
    Input(InputId),
    Output(OutputId),
}

impl ConnectionId {
    pub fn assume_input(&self) -> InputId {
        match self {
            ConnectionId::Input(input) => *input,
            ConnectionId::Output(output) => panic!("{:?} is not an InputId", output),
        }
    }
    pub fn assume_output(&self) -> OutputId {
        match self {
            ConnectionId::Output(output) => *output,
            ConnectionId::Input(input) => panic!("{:?} is not an OutputId", input),
        }
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PortId {
    Input(InputPortId),
    Output(OutputPortId),
}

impl PortId {
    pub fn assume_input(&self) -> InputPortId {
        match self {
            PortId::Input(input) => *input,
            PortId::Output(output) => panic!("{:?} is not an InputPortId", output),
        }
    }

    pub fn assume_output(&self) -> OutputPortId {
        match self {
            PortId::Output(output) => *output,
            PortId::Input(input) => panic!("{:?} is not an OutputPortId", input),
        }
    }
}
