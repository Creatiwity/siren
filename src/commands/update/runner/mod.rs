pub mod error;

use crate::connectors::Connectors;
use crate::models;
use crate::models::metadata;
use crate::models::metadata::common::GroupType;
use chrono::{DateTime, Utc};
use error::Error;
use reqwest::header::LAST_MODIFIED;
use std::fs::{canonicalize, create_dir_all, remove_file, set_permissions, File, Permissions};
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub force: bool,
    pub data_only: bool,
    pub temp_folder: String,
    pub file_folder: String,
    pub db_folder: String,
}

pub fn update(
    groups: &Vec<GroupType>,
    config: Config,
    connectors: &Connectors,
) -> Result<(), Error> {
    println!("[Update] Starting");

    if !config.data_only {
        println!("[Update] Downloading files");
        step_download_file(groups, &config.temp_folder, config.force, connectors)?;

        println!("[Update] Unzipping files");
        step_unzip_file(
            groups,
            &config.temp_folder,
            &config.file_folder,
            config.force,
            connectors,
        )?;
    }

    println!("[Update] Inserting data");
    step_insert_data(groups, &config.db_folder, config.force, connectors)?;

    println!("[Update] Swaping data");
    step_swap_data(groups, config.force, connectors)?;

    if !config.data_only {
        println!("[Update] Cleaning files");
        step_clean_file(groups, &config.temp_folder, &config.file_folder, connectors)?;
    }

    println!("[Update] Finished");

    Ok(())
}

pub fn step_download_file(
    groups: &Vec<GroupType>,
    temp_folder: &String,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    for group in groups {
        download_file(*group, temp_folder, force, connectors)?;
    }
    Ok(())
}

pub fn step_unzip_file(
    groups: &Vec<GroupType>,
    temp_folder: &String,
    file_folder: &String,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    for group in groups {
        unzip_file(*group, temp_folder, file_folder, force, connectors)?;
    }
    Ok(())
}

pub fn step_insert_data(
    groups: &Vec<GroupType>,
    db_folder: &String,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    for group in groups {
        insert_data(*group, db_folder, force, connectors)?;
    }
    Ok(())
}

pub fn step_swap_data(
    groups: &Vec<GroupType>,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    for group in groups {
        swap_data(*group, force, connectors)?;
    }
    Ok(())
}

pub fn step_clean_file(
    groups: &Vec<GroupType>,
    temp_folder: &String,
    file_folder: &String,
    connectors: &Connectors,
) -> Result<(), Error> {
    for group in groups {
        clean_file(*group, temp_folder, file_folder, connectors)?;
    }
    Ok(())
}

fn download_file(
    group: GroupType,
    temp_folder: &String,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    println!("[Download] Downloading {:#?}", group);

    let group_metadata = metadata::get(connectors, group)?;

    // Create temp path
    create_dir_all(temp_folder).map_err(|io_error| Error::TempFolderCreationError { io_error })?;

    // Get Zip path
    let mut zip_path = PathBuf::from(temp_folder);
    zip_path.push(group_metadata.file_name);
    zip_path.set_extension("zip");

    // Prepare file download
    let mut resp = reqwest::blocking::get(group_metadata.url.as_str())
        .map_err(|req_error| Error::DownloadError { req_error })?;

    // Decode Last-Modified header
    let last_modified_str = resp
        .headers()
        .get(LAST_MODIFIED)
        .ok_or(Error::MissingLastModifiedHeader)?
        .to_str()
        .map_err(|head_error| Error::InvalidLastModifiedHeader { head_error })?;
    let last_modified = DateTime::parse_from_rfc2822(last_modified_str)
        .map_err(|date_error| Error::InvalidLastModifiedDate { date_error })?;
    let last_modified = last_modified.with_timezone(&Utc);

    // Test if not already imported or downloaded
    if !force {
        if let Some(last_imported_timestamp) = group_metadata.last_imported_timestamp {
            if last_modified.le(&last_imported_timestamp) {
                println!("[Download] {:#?} already imported", group);
                return Ok(());
            }
        }

        if let Some(staging_file_timestamp) = group_metadata.staging_file_timestamp {
            if last_modified.le(&staging_file_timestamp) {
                println!("[Download] {:#?} already downloaded", group);
                return Ok(());
            }
        }
    }

    // Download data and store it on filesystem
    let mut out =
        File::create(zip_path).map_err(|io_error| Error::FileCreationError { io_error })?;
    io::copy(&mut resp, &mut out).map_err(|io_error| Error::FileCopyError { io_error })?;
    println!("[Download] Download of {:#?} finished", group);

    // Update staging file timestamp
    metadata::set_staging_file_timestamp(connectors, group, last_modified)?;

    Ok(())
}

fn unzip_file(
    group: GroupType,
    temp_folder: &String,
    file_folder: &String,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    println!("[Unzip] Unzipping {:#?}", group);

    let group_metadata = metadata::get(connectors, group)?;

    // Unzip only if zip file is referenced in database
    let staging_file_timestamp = match group_metadata.staging_file_timestamp {
        Some(staging_file_timestamp) => staging_file_timestamp,
        None => {
            println!("[Unzip] Nothing to unzip for {:#?}", group);
            return Ok(());
        }
    };

    // Test if not already imported or unzipped
    if !force {
        if let Some(staging_csv_file_timestamp) = group_metadata.staging_csv_file_timestamp {
            if let Some(last_imported_timestamp) = group_metadata.last_imported_timestamp {
                if staging_csv_file_timestamp.le(&last_imported_timestamp) {
                    println!("[Unzip] {:#?} already imported", group);
                    return Ok(());
                }
            }

            if staging_file_timestamp.le(&staging_csv_file_timestamp) {
                println!("[Unzip] {:#?} already unzipped", group);
                return Ok(());
            }
        }
    }

    // Get Zip path
    let mut zip_path = PathBuf::from(temp_folder);
    zip_path.push(group_metadata.file_name.clone());
    zip_path.set_extension("zip");

    // Get CSV path
    let mut csv_path = PathBuf::from(file_folder);
    csv_path.push(group_metadata.file_name);
    csv_path.set_extension("csv");

    if let Some(p) = csv_path.parent() {
        if !p.exists() {
            create_dir_all(&p).map_err(|io_error| Error::FileFolderCreationError { io_error })?;
        }
    }

    let zip_file = File::open(&zip_path).map_err(|io_error| Error::ZipOpenError { io_error })?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|zip_error| Error::ZipDecodeError { zip_error })?;

    if archive.len() != 1 {
        return Err(Error::ZipFormatError);
    }

    let mut zipped_csv_file = archive
        .by_index(0)
        .map_err(|zip_error| Error::ZipAccessFileError { zip_error })?;

    println!(
        "[Unzip] Unzipping file {:#?} extracted to \"{}\" ({} bytes)",
        group,
        csv_path.as_path().display(),
        zipped_csv_file.size()
    );

    let mut csv_file =
        File::create(&csv_path).map_err(|io_error| Error::FileCSVCreationError { io_error })?;
    io::copy(&mut zipped_csv_file, &mut csv_file)
        .map_err(|io_error| Error::FileCSVCopyError { io_error })?;

    // Get and Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = zipped_csv_file.unix_mode() {
            set_permissions(&csv_path, Permissions::from_mode(mode))
                .map_err(|io_error| Error::FileCSVPermissionError { io_error })?;
        }
    }

    models::metadata::set_staging_csv_file_timestamp(connectors, group, staging_file_timestamp)?;

    println!("[Unzip] Unzip of {:#?} finished", group);

    Ok(())
}

fn insert_data(
    group: GroupType,
    db_folder: &String,
    force: bool,
    connectors: &Connectors,
) -> Result<(), Error> {
    println!("[Insert] Insert {:#?}", group);

    let group_metadata = metadata::get(connectors, group)?;

    // Insert only if csv file is referenced in database
    let staging_csv_file_timestamp = match group_metadata.staging_csv_file_timestamp {
        Some(staging_csv_file_timestamp) => staging_csv_file_timestamp,
        None => {
            println!("[Insert] Nothing to insert for {:#?}", group);
            return Ok(());
        }
    };

    // Test if not already inserted
    if !force {
        if let Some(staging_imported_timestamp) = group_metadata.staging_imported_timestamp {
            if let Some(last_imported_timestamp) = group_metadata.last_imported_timestamp {
                if staging_imported_timestamp.le(&last_imported_timestamp) {
                    println!("[Insert] {:#?} already imported", group);
                    return Ok(());
                }
            }

            if staging_csv_file_timestamp.le(&staging_imported_timestamp) {
                println!("[Insert] {:#?} already inserted", group);
                return Ok(());
            }
        }
    }

    // Get CSV path
    let mut csv_path = PathBuf::from(db_folder);
    csv_path.push(group_metadata.file_name);
    csv_path.set_extension("csv");
    let absolute_csv_path =
        canonicalize(csv_path).map_err(|io_error| Error::InvalidComponentInCSVPath { io_error })?;
    let csv_path_str = absolute_csv_path
        .into_os_string()
        .into_string()
        .map_err(|_| Error::InvalidCSVPath)?;

    match group {
        GroupType::Etablissements => {
            models::etablissement::insert_in_staging(connectors, csv_path_str)?;
        }
        GroupType::UnitesLegales => {
            models::unite_legale::insert_in_staging(connectors, csv_path_str)?;
        }
    }

    models::metadata::set_staging_imported_timestamp(
        connectors,
        group,
        staging_csv_file_timestamp,
    )?;

    println!("[Insert] Finished insert of {:#?}", group);

    Ok(())
}

fn swap_data(group_type: GroupType, force: bool, connectors: &Connectors) -> Result<(), Error> {
    println!("[Insert] Swapping {:#?}", group_type);

    let group_metadata = metadata::get(connectors, group_type)?;

    // Swap only if inserted data are referenced in database
    let staging_imported_timestamp = match group_metadata.staging_imported_timestamp {
        Some(staging_imported_timestamp) => staging_imported_timestamp,
        None => {
            println!("[Swap] Nothing to swap for {:#?}", group_type);
            return Ok(());
        }
    };

    // Test if not already swapped
    if !force {
        if let Some(last_imported_timestamp) = group_metadata.last_imported_timestamp {
            if staging_imported_timestamp.le(&last_imported_timestamp) {
                println!("[Swap] {:#?} already imported", group_type);
                return Ok(());
            }
        }
    }

    match group_type {
        GroupType::Etablissements => {
            if !force {
                let count = models::etablissement::count(connectors)? as f64;
                let count_staging = models::etablissement::count_staging(connectors)? as f64;

                let max_count_staging = count * 1.01;
                let min_count_staging = count * 0.99;

                if count != 0.0
                    && (count_staging < min_count_staging || max_count_staging < count_staging)
                {
                    return Err(Error::SwapStoppedTooMuchDifference { group_type });
                }
            }

            models::etablissement::swap(connectors)?;
        }
        GroupType::UnitesLegales => {
            if !force {
                let count = models::unite_legale::count(connectors)? as f64;
                let count_staging = models::unite_legale::count_staging(connectors)? as f64;

                let max_count_staging = count * 1.01;
                let min_count_staging = count * 0.99;

                if count != 0.0
                    && (count_staging < min_count_staging || max_count_staging < count_staging)
                {
                    return Err(Error::SwapStoppedTooMuchDifference { group_type });
                }
            }

            models::unite_legale::swap(connectors)?;
        }
    }

    models::metadata::set_last_imported_timestamp(
        connectors,
        group_type,
        staging_imported_timestamp,
    )?;

    println!("[Insert] Swap of {:#?} finished", group_type);

    Ok(())
}

fn clean_file(
    group: GroupType,
    temp_folder: &String,
    file_folder: &String,
    connectors: &Connectors,
) -> Result<(), Error> {
    println!("[Clean] Cleaning {:#?}", group);

    let group_metadata = metadata::get(connectors, group)?;

    // Get Zip path
    let mut zip_path = PathBuf::from(temp_folder);
    zip_path.push(group_metadata.file_name.clone());
    zip_path.set_extension("zip");

    // Get CSV path
    let mut csv_path = PathBuf::from(file_folder);
    csv_path.push(group_metadata.file_name);
    csv_path.set_extension("csv");

    if let Err(error) = remove_file(zip_path) {
        println!("[Clean] Zip not deleted ({})", error);
    }

    if let Err(error) = remove_file(csv_path) {
        println!("[Clean] CSV not deleted ({})", error);
    }

    models::metadata::reset_staging_timestamps(connectors, group)?;

    println!("[Clean] Finished cleaning of {:#?}", group);

    Ok(())
}
