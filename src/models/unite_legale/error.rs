use custom_error::custom_error;
use diesel_async::pooled_connection::deadpool::PoolError;

custom_error! { pub Error
    LocalConnectionFailed{source: PoolError} = "Unable to connect to local database ({source}).",
    UniteLegaleNotFound = "Unite Legale not found.",
    Database{diesel_error: diesel::result::Error} = "Unable to run some operations on unite_legale ({diesel_error}).",
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Error::UniteLegaleNotFound,
            _ => Error::Database {
                diesel_error: error,
            },
        }
    }
}
