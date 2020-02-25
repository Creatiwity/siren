use custom_error::custom_error;

custom_error! { pub Error
    AlreadyLaunched = "Unable to launch update while another update is already running.",
    UpdateNotRegistered = "Unable to register this update in database.",
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    MetadataNotFound = "Metadata not found.",
    DatabaseError{diesel_error: diesel::result::Error} = "Unable to run some operations on metadata ({diesel_error}).",
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Error::MetadataNotFound,
            _ => Error::DatabaseError {
                diesel_error: error,
            },
        }
    }
}
