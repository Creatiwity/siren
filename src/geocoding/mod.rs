//! Address geocoding, backed by geocoder-core (feature `geocoding`).
//!
//! - [`sync`]: build or refresh the index from the BAN export
//!   (`update geocoding`, and `update all` unless disabled);
//! - [`handle`]: the index the API serves, reloaded when the file changes;
//! - [`address`]: turning geocoding results into API responses and search
//!   filters.

pub mod address;
pub mod handle;
pub mod sync;

pub use handle::GeocoderHandle;

/// The BAN export in addok's format, the one the BAN API is built from.
pub const DEFAULT_SOURCE_URL: &str =
    "https://adresse.data.gouv.fr/data/ban/adresses/latest/addok/adresses-addok-france.ndjson.gz";
