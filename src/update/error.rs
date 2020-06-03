use crate::models;
use crate::models::group_metadata::common::GroupType;
use crate::models::{group_metadata, update_metadata};
use custom_error::custom_error;
use std::process;

custom_error! { pub Error
    MetadataModelError {source: group_metadata::error::Error} = "Error on Metadata model: {source}.",
    UpdateMetadataModelError {source: update_metadata::error::Error} = "Error on UpdateMetadata model: {source}.",
    UpdatableModelError {source: models::common::Error} = "Error on UpdatableModel model: {source}.",
    TempFolderCreationError {io_error: std::io::Error} = "Unable to create temporary folder ({io_error}).",
    FileFolderCreationError {io_error: std::io::Error} = "Unable to create data folder ({io_error}).",
    FileCreationError {io_error: std::io::Error} = "Unable to create file for download ({io_error}).",
    FileCopyError {io_error: std::io::Error} = "Unable to copy file from download ({io_error}).",
    FileCSVCreationError {io_error: std::io::Error} = "Unable to create CSV file for unzip ({io_error}).",
    FileCSVCopyError {io_error: std::io::Error} = "Unable to copy CSV file from archive ({io_error}).",
    FileCSVPermissionError {io_error: std::io::Error} = "Unable to set permission for CSV file ({io_error}).",
    DownloadError {req_error: reqwest::Error} = "Unable to download data from remote server ({req_error}).",
    ZipOpenError {io_error: std::io::Error} = "Unable to open data zip file ({io_error}).",
    ZipDecodeError {zip_error: zip::result::ZipError} = "Unable to decode zip file ({zip_error}).",
    ZipFormatError = "Archive has more than one file inside it, you should review it before running it again.",
    ZipAccessFileError {zip_error: zip::result::ZipError} = "Unable to open file in archive ({zip_error}).",
    MissingLastModifiedHeader = "Needed header 'Last-Modified' is missing while downloading.",
    InvalidLastModifiedHeader {head_error: reqwest::header::ToStrError} = "Needed header 'Last-Modified' is invalid while downloading ({head_error}).",
    InvalidLastModifiedDate {date_error: chrono::format::ParseError} = "Needed header 'Last-Modified' is invalid while downloading ({date_error}).",
    InvalidCSVPath = "Invalid CSV path, not UTF8 compatible.",
    InvalidComponentInCSVPath {io_error: std::io::Error} = "Invalid component in CSV path ({io_error}).",
    SwapStoppedTooMuchDifference {group_type: GroupType} = "Swapping stopped on {group_type}, more than 1% difference between the old values and the new ones. Use --force to override.",
    SyncInseeConnectorError = "Missing insee connector",
}

impl Error {
    pub fn exit(&self) -> ! {
        eprintln!("{}", self);
        process::exit(1);
    }
}
