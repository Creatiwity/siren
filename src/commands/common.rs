use crate::models::update_metadata::common::SyntheticGroupType;
use serde::Deserialize;

#[derive(Clone)]
pub struct FolderOptions {
    pub temp: String,
    pub file: String,
    pub db: String,
}

#[derive(clap::ValueEnum, Debug, Deserialize, Clone, Copy)]
pub enum CmdGroupType {
    UnitesLegales,
    Etablissements,
    All,
}

impl From<CmdGroupType> for SyntheticGroupType {
    fn from(group: CmdGroupType) -> Self {
        match group {
            CmdGroupType::UnitesLegales => SyntheticGroupType::UnitesLegales,
            CmdGroupType::Etablissements => SyntheticGroupType::Etablissements,
            CmdGroupType::All => SyntheticGroupType::All,
        }
    }
}
