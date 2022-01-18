use custom_error::custom_error;

custom_error! { pub InseeTokenError
    Network { source: reqwest::Error } = "Unable to retrieve INSEE token (network error: {source})",
    Malformed {source: serde_json::Error} = "Unable to read INSEE token ({source})",
    Invalid {source: reqwest::header::InvalidHeaderValue} = "Unable to use INSEE token in header ({source})",
}

custom_error! { pub InseeUpdate
    Network {source: reqwest::Error} = "Unable to retrieve INSEE data (network error: {source})",
}
