use super::super::error::Error;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use async_trait::async_trait;

#[async_trait]
pub trait Action {
    fn step(&self) -> Step;
    async fn execute(
        &self,
        group_type: GroupType,
        connectors: &Connectors,
    ) -> Result<UpdateGroupSummary, Error>;
}
