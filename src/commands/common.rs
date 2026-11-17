use crate::models::update_metadata::common::SyntheticGroupType;
use serde::Deserialize;

#[derive(clap::ValueEnum, Debug, Deserialize, Clone, Copy)]
pub enum CmdGroupType {
    UnitesLegales,
    Etablissements,
    LiensSuccession,
    SirenDoublons,
    All,
    /// The address geocoding index (a file, not database tables).
    #[cfg(feature = "geocoding")]
    Geocoding,
}

impl From<CmdGroupType> for SyntheticGroupType {
    fn from(group: CmdGroupType) -> Self {
        match group {
            CmdGroupType::UnitesLegales => SyntheticGroupType::UnitesLegales,
            CmdGroupType::Etablissements => SyntheticGroupType::Etablissements,
            CmdGroupType::LiensSuccession => SyntheticGroupType::LiensSuccession,
            CmdGroupType::SirenDoublons => SyntheticGroupType::SirenDoublons,
            CmdGroupType::All => SyntheticGroupType::All,
            #[cfg(feature = "geocoding")]
            CmdGroupType::Geocoding => unreachable!("the geocoding index has no database workflow"),
        }
    }
}
