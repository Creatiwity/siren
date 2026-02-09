use crate::models::update_metadata::common::SyntheticGroupType;
use serde::Deserialize;

#[derive(clap::ValueEnum, Debug, Deserialize, Clone, Copy)]
pub enum CmdGroupType {
    UnitesLegales,
    Etablissements,
    LiensSuccession,
    All,
}

impl From<CmdGroupType> for SyntheticGroupType {
    fn from(group: CmdGroupType) -> Self {
        match group {
            CmdGroupType::UnitesLegales => SyntheticGroupType::UnitesLegales,
            CmdGroupType::Etablissements => SyntheticGroupType::Etablissements,
            CmdGroupType::LiensSuccession => SyntheticGroupType::LiensSuccession,
            CmdGroupType::All => SyntheticGroupType::All,
        }
    }
}
