use custom_error::custom_error;

pub mod insee;
pub mod local;

custom_error! { pub Error
    InseeError { source: insee::error::TokenError } = "{source}",
}

pub struct ConnectorsBuilders {
    local: local::ConnectorBuilder,
    insee: Option<insee::ConnectorBuilder>,
}

pub struct Connectors {
    pub local: local::Connector,
    pub insee: Option<insee::Connector>,
}

impl ConnectorsBuilders {
    pub fn new() -> Self {
        ConnectorsBuilders {
            local: local::ConnectorBuilder::new(),
            insee: insee::ConnectorBuilder::new(),
        }
    }

    pub fn create(&self) -> Connectors {
        Connectors {
            local: self.local.create(),
            insee: None,
        }
    }

    pub fn create_with_insee(&self) -> Result<Connectors, Error> {
        Ok(Connectors {
            local: self.local.create(),
            insee: match self.insee.as_ref() {
                Some(insee_builder) => Some(insee_builder.create()?),
                None => None,
            },
        })
    }
}
