use custom_error::custom_error;

custom_error! { pub InseeTokenError
    NetworkError { source: reqwest::Error } = "Unable to retrieve INSEE token (network error: {source})",
    ApiError = "Unable to retrieve INSEE token",
    MalformedError {source: serde_json::Error} = "Unable to read INSEE token ({source})",
    InvalidError {source: reqwest::header::InvalidHeaderValue} = "Unable to use INSEE token in header ({source})",
}

custom_error! { pub InseeUpdateError
    NetworkError {source: reqwest::Error} = "Unable to retrieve INSEE data (network error: {source})",
}
