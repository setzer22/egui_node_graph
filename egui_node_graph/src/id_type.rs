slotmap::new_key_type! { pub struct NodeId; }
slotmap::new_key_type! { pub struct HookId; }
slotmap::new_key_type! { pub struct InputPortId; }
slotmap::new_key_type! { pub struct OutputPortId; }

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InputId(pub NodeId, pub InputPortId, pub HookId);

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OutputId(pub NodeId, pub OutputPortId, pub HookId);

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ConnectionId(pub NodeId, pub PortId, pub HookId);

impl ConnectionId {
    pub fn as_input(&self) -> Option<InputId> {
        match self.1 {
            PortId::Input(port) => Some(InputId(self.0, port, self.2)),
            _ => None,
        }
    }

    pub fn as_output(&self) -> Option<OutputId> {
        match self.1 {
            PortId::Output(port) => Some(OutputId(self.0, port, self.2)),
            _ => None,
        }
    }

    pub fn assume_input(&self) -> InputId {
        match self.as_input() {
            Some(input) => input,
            None => panic!("{:?} is not an InputId", self),
        }
    }
    pub fn assume_output(&self) -> OutputId {
        match self.as_output() {
            Some(output) => output,
            None => panic!("{:?} is not an OutputId", self),
        }
    }

    pub fn node(&self) -> NodeId {
        self.0
    }

    pub fn port(&self) -> PortId {
        self.1
    }

    pub fn hook(&self) -> HookId {
        self.2
    }
}

impl InputId {
    pub fn node(&self) -> NodeId {
        self.0
    }

    pub fn port(&self) -> InputPortId {
        self.1
    }

    pub fn hook(&self) -> HookId {
        self.2
    }
}

impl OutputId {
    pub fn node(&self) -> NodeId {
        self.0
    }

    pub fn port(&self) -> OutputPortId {
        self.1
    }

    pub fn hook(&self) -> HookId {
        self.2
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

    pub fn is_complementary(&self, other: &Self) -> bool {
        if let (PortId::Input(_), PortId::Output(_)) = (self, other) {
            return true;
        }

        if let (PortId::Output(_), PortId::Input(_)) = (self, other) {
            return true;
        }

        return false;
    }
}

impl From<InputId> for ConnectionId {
    fn from(c: InputId) -> Self {
        ConnectionId(c.0, c.into(), c.2)
    }
}

impl From<OutputId> for ConnectionId {
    fn from(c: OutputId) -> Self {
        ConnectionId(c.0, c.into(), c.2)
    }
}

impl From<ConnectionId> for NodeId {
    fn from(c: ConnectionId) -> Self {
        c.node()
    }
}

impl From<ConnectionId> for (NodeId, PortId) {
    fn from(c: ConnectionId) -> Self {
        (c.0, c.1)
    }
}

impl From<ConnectionId> for (PortId, HookId) {
    fn from(c: ConnectionId) -> Self {
        (c.1, c.2)
    }
}

impl From<InputId> for PortId {
    fn from(c: InputId) -> Self {
        PortId::Input(c.1)
    }
}

impl From<OutputId> for PortId {
    fn from(c: OutputId) -> Self {
        PortId::Output(c.port())
    }
}

impl From<OutputPortId> for PortId {
    fn from(value: OutputPortId) -> Self {
        PortId::Output(value)
    }
}

impl From<InputPortId> for PortId {
    fn from(value: InputPortId) -> Self {
        PortId::Input(value)
    }
}

impl From<OutputId> for (PortId, HookId) {
    fn from(c: OutputId) -> Self {
        (PortId::Output(c.1), c.2)
    }
}

impl From<InputId> for (PortId, HookId) {
    fn from(c: InputId) -> Self {
        (PortId::Input(c.1), c.2)
    }
}
