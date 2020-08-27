use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use async_trait::async_trait;

#[async_trait]
pub trait Action: Sync + Send {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error>;
}
