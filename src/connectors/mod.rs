pub mod local;

pub struct ConnectorsBuilders {
    local: local::ConnectorBuilder,
}

pub struct Connectors {
    pub local: local::Connector,
}

impl ConnectorsBuilders {
    pub fn new() -> Self {
        ConnectorsBuilders {
            local: local::ConnectorBuilder::new(),
        }
    }

    pub fn create(&self) -> Connectors {
        Connectors {
            local: self.local.create(),
        }
    }
}
