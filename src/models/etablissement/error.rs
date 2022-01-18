use custom_error::custom_error;

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
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
