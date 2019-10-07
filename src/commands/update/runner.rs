use crate::connectors::Connectors;
use crate::models;
use crate::models::metadata;
use crate::models::metadata::common::GroupType;
use chrono::{DateTime, Utc};
use reqwest::header::LAST_MODIFIED;
use std::fs::{create_dir_all, remove_file, set_permissions, File, Permissions};
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

pub fn update(groups: &Vec<GroupType>, config: Config, connectors: &Connectors) {
    println!("[Update] Starting");

    if !config.data_only {
        println!("[Update] Downloading files");
        let new_content = step_download_file(groups, &config.temp_folder, connectors);

        if new_content {
            println!("[Update] Unzipping files");
            step_unzip_file(groups, &config.temp_folder, &config.file_folder, connectors);
        } else {
            println!("[Update] No file downloaded, skipping unzip");
        }
    }

    println!("[Update] Inserting data");
    step_insert_data(groups, &config.db_folder, connectors);

    println!("[Update] Swaping data");
    step_swap_data(groups, config.force, connectors);

    if !config.data_only {
        println!("[Update] Cleaning files");
        step_clean_file(groups, &config.temp_folder, &config.file_folder, connectors);
    }

    println!("[Update] Finished");
}

pub fn step_download_file(
    groups: &Vec<GroupType>,
    temp_folder: &String,
    connectors: &Connectors,
) -> bool {
    groups.iter().fold(false, |acc, group| {
        let new_content = download_file(*group, temp_folder, connectors);
        new_content || acc
    })
}

pub fn step_unzip_file(
    groups: &Vec<GroupType>,
    temp_folder: &String,
    file_folder: &String,
    connectors: &Connectors,
) {
    for group in groups {
        unzip_file(*group, temp_folder, file_folder, connectors);
    }
}

pub fn step_insert_data(groups: &Vec<GroupType>, db_folder: &String, connectors: &Connectors) {
    for group in groups {
        insert_data(*group, db_folder, connectors);
    }
}

pub fn step_swap_data(groups: &Vec<GroupType>, force: bool, connectors: &Connectors) {
    for group in groups {
        swap_data(*group, force, connectors);
    }
}

pub fn step_clean_file(
    groups: &Vec<GroupType>,
    temp_folder: &String,
    file_folder: &String,
    connectors: &Connectors,
) {
    for group in groups {
        clean_file(*group, temp_folder, file_folder, connectors);
    }
}

fn download_file(group: GroupType, temp_folder: &String, connectors: &Connectors) -> bool {
    println!("[Download] Downloading {:#?}", group);

    let group_metadata = metadata::get(connectors, group).unwrap();

    // Create temp path
    create_dir_all(temp_folder).unwrap();

    // Get Zip path
    let mut zip_path = PathBuf::from(temp_folder);
    zip_path.push(group_metadata.file_name);
    zip_path.set_extension("zip");

    let mut resp =
        reqwest::get(group_metadata.url.as_str()).expect("Request failed while downloading");
    let last_modified_str = resp
        .headers()
        .get(LAST_MODIFIED)
        .expect("Missing Last-Modified header")
        .to_str()
        .unwrap();
    let last_modified = DateTime::parse_from_rfc2822(last_modified_str).unwrap();
    let last_modified = last_modified.with_timezone(&Utc);

    if let Some(staging_file_timestamp) = group_metadata.staging_file_timestamp {
        if last_modified.le(&staging_file_timestamp) {
            println!("[Download] {:#?} already downloaded", group);
            return false;
        }
    }

    let mut out = File::create(zip_path).expect("Failed to create file");
    io::copy(&mut resp, &mut out).expect("Failed to copy content");
    println!("[Download] Download of {:#?} finished", group);

    // Update staging file timestamp
    metadata::set_staging_file_timestamp(connectors, group, last_modified).unwrap();

    true
}

fn unzip_file(
    group: GroupType,
    temp_folder: &String,
    file_folder: &String,
    connectors: &Connectors,
) {
    println!("[Unzip] Unzipping {:#?}", group);

    let group_metadata = metadata::get(connectors, group).unwrap();

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
            create_dir_all(&p).unwrap();
        }
    }

    let zip_file = File::open(&zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();

    if archive.len() != 1 {
        panic!("Archive has more than one file inside it, you should review it before running it again")
    }

    let mut zipped_csv_file = archive.by_index(0).unwrap();

    println!(
        "[Unzip] Will unzip file {:#?} extracted to \"{}\" ({} bytes)",
        group,
        csv_path.as_path().display(),
        zipped_csv_file.size()
    );

    let mut csv_file = File::create(&csv_path).unwrap();
    io::copy(&mut zipped_csv_file, &mut csv_file).unwrap();

    // Get and Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = zipped_csv_file.unix_mode() {
            set_permissions(&csv_path, Permissions::from_mode(mode)).unwrap();
        }
    }

    println!("[Unzip] Unzip of {:#?} finished", group);
}

fn insert_data(group: GroupType, db_folder: &String, connectors: &Connectors) {
    println!("[Insert] Insert {:#?}", group);

    let group_metadata = metadata::get(connectors, group).unwrap();

    // Get CSV path
    let mut csv_path = PathBuf::from(db_folder);
    csv_path.push(group_metadata.file_name);
    csv_path.set_extension("csv");
    let csv_path_str = csv_path.into_os_string().into_string().unwrap();

    match group {
        GroupType::Etablissements => {
            models::etablissement::insert_in_staging(connectors, csv_path_str).unwrap();
        }
        GroupType::UnitesLegales => {
            models::unite_legale::insert_in_staging(connectors, csv_path_str).unwrap();
        }
    }

    println!("[Insert] Finished insert of {:#?}", group);
}

fn swap_data(group: GroupType, force: bool, connectors: &Connectors) {
    println!("[Insert] Swapping {:#?}", group);

    match group {
        GroupType::Etablissements => {
            if !force {
                let count = models::etablissement::count(connectors).unwrap() as f64;
                let count_staging =
                    models::etablissement::count_staging(connectors).unwrap() as f64;

                let max_count_staging = count * 1.01;
                let min_count_staging = count * 0.99;

                if count != 0.0
                    && (count_staging < min_count_staging || max_count_staging < count_staging)
                {
                    panic!("Swapping stopped on etablissement, more than 1% difference between the old values and the new ones. Use --force to override.");
                }
            }

            models::etablissement::swap(connectors).unwrap();
        }
        GroupType::UnitesLegales => {
            if !force {
                let count = models::unite_legale::count(connectors).unwrap() as f64;
                let count_staging = models::unite_legale::count_staging(connectors).unwrap() as f64;

                let max_count_staging = count * 1.01;
                let min_count_staging = count * 0.99;

                if count != 0.0
                    && (count_staging < min_count_staging || max_count_staging < count_staging)
                {
                    panic!("Swapping stopped on unite_legale, more than 1% difference between the old values and the new ones. Use --force to override.");
                }
            }

            models::unite_legale::swap(connectors).unwrap();
        }
    }

    println!("[Insert] Swap of {:#?} finished", group);
}

fn clean_file(
    group: GroupType,
    temp_folder: &String,
    file_folder: &String,
    connectors: &Connectors,
) {
    println!("[Clean] Cleaning {:#?}", group);

    let group_metadata = metadata::get(connectors, group).unwrap();

    // Get Zip path
    let mut zip_path = PathBuf::from(temp_folder);
    zip_path.push(group_metadata.file_name.clone());
    zip_path.set_extension("zip");

    // Get CSV path
    let mut csv_path = PathBuf::from(file_folder);
    csv_path.push(group_metadata.file_name);
    csv_path.set_extension("csv");

    let zip_result = remove_file(zip_path);
    let csv_result = remove_file(csv_path);

    // Panic at the end if at least one file was not deleted
    zip_result.unwrap();
    csv_result.unwrap();

    println!("[Clean] Finished cleaning of {:#?}", group);
}
