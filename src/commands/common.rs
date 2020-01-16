use crate::models::metadata::common::GroupType;
use serde::Deserialize;

pub struct FolderOptions {
    pub temp: String,
    pub file: String,
    pub db: String,
}

arg_enum! {
    #[derive(Debug, Deserialize, Clone, Copy)]
    pub enum CmdGroupType {
        UnitesLegales,
        Etablissements,
        All
    }
}

impl From<CmdGroupType> for Vec<GroupType> {
    fn from(group: CmdGroupType) -> Self {
        match group {
            CmdGroupType::UnitesLegales => vec![GroupType::UnitesLegales],
            CmdGroupType::Etablissements => vec![GroupType::Etablissements],
            CmdGroupType::All => vec![GroupType::UnitesLegales, GroupType::Etablissements],
        }
    }
}
