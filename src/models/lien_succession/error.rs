use custom_error::custom_error;

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    Database{source: diesel::result::Error} = "Unable to run some operations on liens_succession ({source}).",
}
