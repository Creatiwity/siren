#[derive(Debug)]
pub struct Config {
    pub force: bool,
    pub asynchronous: bool,
    pub crontab: Option<String>,
}
