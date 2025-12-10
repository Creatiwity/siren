use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use super::super::utils::remote_file::RemoteFile;
use super::common::Action;
use crate::models::group_metadata::common::GroupType;
use crate::{connectors::Connectors, models::group_metadata};
use async_trait::async_trait;
use tracing::debug;

pub struct UpdateAction {
    pub force: bool,
}

#[async_trait]
impl Action for UpdateAction {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        debug!("Updating {:#?}", group_type);
        summary_delegate.start(connectors, None, 1)?;

        let metadata = group_metadata::get(connectors, group_type)?;

        let remote_file = RemoteFile::new(metadata.url.clone()).await?;
        let last_modified = remote_file.last_modified;

        debug!(
            "Will verify dates before importing, last imported metadata {:?}, last modified {}",
            metadata.last_imported_timestamp, last_modified
        );

        if !self.force {
            if let Some(last_imported_timestamp) = metadata.last_imported_timestamp
                && last_modified.le(&last_imported_timestamp)
            {
                debug!("{:#?} already imported", group_type);

                summary_delegate.finish(connectors, String::from("already imported"), 0, false)?;

                return Ok(());
            }

            if let Some(staging_imported_timestamp) = metadata.staging_imported_timestamp
                && last_modified.le(&staging_imported_timestamp)
            {
                debug!(
                    "{:#?} already downloaded, unzippped and inserted",
                    group_type
                );

                summary_delegate.finish(
                    connectors,
                    String::from("already downloaded, unzippped and inserted"),
                    0,
                    false,
                )?;

                return Ok(());
            }
        }

        let moved_connectors = connectors.clone();

        tokio::task::spawn_blocking(move || {
            group_type
                .get_updatable_model()
                .insert_remote_file_in_staging(&moved_connectors, remote_file)
        })
        .await??;

        group_metadata::set_staging_imported_timestamp(connectors, group_type, last_modified)?;

        debug!("Update of {:#?} finished", group_type);

        summary_delegate.finish(connectors, String::from("inserted"), 1, true)?;

        Ok(())
    }
}
