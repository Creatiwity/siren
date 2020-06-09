use custom_error::custom_error;

custom_error! { pub TokenError
    NetworkError { source: reqwest::Error } = "Unable to retrieve INSEE token (network error: {source})",
    ApiError = "Unable to retrieve INSEE token",
    MalformedError {source: serde_json::Error} = "Unable to read INSEE token ({source})",
}

custom_error! { pub InseeError
    NetworkError {source: reqwest::Error} = "Unable to retrieve INSEE data (network error: {source})",
    MissingPeriodeError = "Missing current unite legale periode",
}
