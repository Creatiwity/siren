use custom_error::custom_error;
use warp::{reject::Reject, Rejection};

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    UniteLegaleNotFound = "Unite Legale not found.",
    DatabaseError{diesel_error: diesel::result::Error} = "Unable to run some operations on unite_legale ({diesel_error}).",
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Error::UniteLegaleNotFound,
            _ => Error::DatabaseError {
                diesel_error: error,
            },
        }
    }
}

impl Reject for Error {}

impl From<Error> for Rejection {
    fn from(e: Error) -> Self {
        warp::reject::custom(e)
    }
}
