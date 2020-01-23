use crate::connectors::Connectors;
use custom_error::custom_error;

pub trait UpdatableModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn count_staging(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn insert_in_staging(&self, connectors: &Connectors, file_path: String) -> Result<bool, Error>;
    fn swap(&self, connectors: &Connectors) -> Result<(), Error>;
}

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    DatabaseError{source: diesel::result::Error} = "Unable to run some operations on updatable model ({source}).",
}