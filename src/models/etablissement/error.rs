use custom_error::custom_error;
use diesel_async::pooled_connection::deadpool::PoolError;

custom_error! { pub Error
    LocalConnectionFailed{source: PoolError} = "Unable to connect to local database ({source}).",
    EtablissementNotFound = "Etablissement not found.",
    Database{diesel_error: diesel::result::Error} = "Unable to run some operations on etablissement ({diesel_error}).",
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Error::EtablissementNotFound,
            _ => Error::Database {
                diesel_error: error,
            },
        }
    }
}
