use custom_error::custom_error;
use diesel_async::pooled_connection::deadpool::PoolError;

custom_error! { pub Error
    LocalConnectionFailed{source: PoolError} = "Unable to connect to local database ({source}).",
    Database{source: diesel::result::Error} = "Unable to run some operations on liens_succession ({source}).",
}
