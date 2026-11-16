//! Address geocoding, backed by geocoder-core (feature `geocoding`).
//!
//! - [`sync`]: download the prebuilt index, signed, published by the
//!   geocoder-core CI (`update geocoding`, and `update all` unless
//!   disabled);
//! - [`handle`]: the index the API serves, reloaded when the file changes;
//! - [`address`]: turning geocoding results into API responses and search
//!   filters.

pub mod address;
pub mod handle;
pub mod sync;

pub use handle::GeocoderHandle;
