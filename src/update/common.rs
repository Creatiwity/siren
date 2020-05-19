#[derive(Debug)]
pub struct Config {
    pub force: bool,
    pub data_only: bool,
    pub temp_folder: String,
    pub file_folder: String,
    pub db_folder: String,
}
