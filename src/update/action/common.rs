use super::super::error::Error;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};

pub trait Action {
    fn step(&self) -> Step;
    fn execute(
        &self,
        group_type: GroupType,
        connectors: &Connectors,
    ) -> Result<UpdateGroupSummary, Error>;
}
