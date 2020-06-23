use super::super::error::Error;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::Step;
use super::super::summary::SummaryGroupDelegate;
use async_trait::async_trait;

#[async_trait]
pub trait Action: Sync + Send {
    fn step(&self) -> Step;
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error>;
}
