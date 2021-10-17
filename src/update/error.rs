use crate::connectors::insee::error::InseeUpdate;
use crate::models;
use crate::models::group_metadata::common::GroupType;
use crate::models::{group_metadata, update_metadata};
use custom_error::custom_error;
use std::process;
use tracing::error;

custom_error! { pub Error
    MetadataModel {source: group_metadata::error::Error} = "Error on Metadata model: {source}",
    UpdateMetadataModel {source: update_metadata::error::Error} = "Error on UpdateMetadata model: {source}",
    UpdatableModel {source: models::common::Error} = "Error on UpdatableModel model: {source}",
    TempFolderCreation {io_error: std::io::Error} = "Unable to create temporary folder ({io_error})",
    FileFolderCreation {io_error: std::io::Error} = "Unable to create data folder ({io_error})",
    FileCreation {io_error: std::io::Error} = "Unable to create file for download ({io_error})",
    FileCopy {io_error: std::io::Error} = "Unable to copy file from download ({io_error})",
    FileCSVCreation {io_error: std::io::Error} = "Unable to create CSV file for unzip ({io_error})",
    FileCSVCopy {io_error: std::io::Error} = "Unable to copy CSV file from archive ({io_error})",
    FileCSVPermission {io_error: std::io::Error} = "Unable to set permission for CSV file ({io_error})",
    Download {req_error: reqwest::Error} = "Unable to download data from remote server ({req_error})",
    ZipOpen {io_error: std::io::Error} = "Unable to open data zip file ({io_error})",
    ZipDecode {zip_error: zip::result::ZipError} = "Unable to decode zip file ({zip_error})",
    ZipFormat = "Archive has more than one file inside it, you should review it before running it again",
    ZipAccessFile {zip_error: zip::result::ZipError} = "Unable to open file in archive ({zip_error})",
    MissingLastModifiedHeader = "Needed header 'Last-Modified' is missing while downloading",
    InvalidLastModifiedHeader {head_error: reqwest::header::ToStrError} = "Needed header 'Last-Modified' is invalid while downloading ({head_error})",
    InvalidLastModifiedDate {date_error: chrono::format::ParseError} = "Needed header 'Last-Modified' is invalid while downloading ({date_error})",
    InvalidCSVPath = "Invalid CSV path, not UTF8 compatible",
    InvalidComponentInCSVPath {io_error: std::io::Error} = "Invalid component in CSV path ({io_error})",
    SwapStoppedTooMuchDifference {group_type: GroupType} = "Swapping stopped on {group_type}, more than 1% difference between the old values and the new ones. Use --force to override",
    SyncInsee {source: InseeUpdate} = "{source}",
    WaitThread {source: tokio::task::JoinError} = "Error while waiting for thread: {source}",
}

impl Error {
    pub fn exit(&self) -> ! {
        error!("{}", self);
        process::exit(1);
    }
}
