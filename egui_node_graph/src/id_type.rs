slotmap::new_key_type! { pub struct NodeId; }
slotmap::new_key_type! { pub struct InputId; }
slotmap::new_key_type! { pub struct OutputId; }

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AnyParameterId {
    Input(InputId),
    Output(OutputId),
}

impl AnyParameterId {
    pub fn assume_input(&self) -> InputId {
        match self {
            AnyParameterId::Input(input) => *input,
            AnyParameterId::Output(output) => panic!("{:?} is not an InputId", output),
        }
    }
    pub fn assume_output(&self) -> OutputId {
        match self {
            AnyParameterId::Output(output) => *output,
            AnyParameterId::Input(input) => panic!("{:?} is not an OutputId", input),
        }
    }
}

impl From<OutputId> for AnyParameterId {
    fn from(output: OutputId) -> Self {
        Self::Output(output)
    }
}

impl From<InputId> for AnyParameterId {
    fn from(input: InputId) -> Self {
        Self::Input(input)
    }
}
