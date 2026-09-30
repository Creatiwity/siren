//! Geocoding results as exposed by the API, and how an address becomes a
//! search filter.

use geocoder_core::SearchResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Minimum geocoding score for `address` in searches.
pub const DEFAULT_MIN_SCORE: f32 = 0.5;

/// Search radius around a geocoded address, by precision.
pub const RADIUS_HOUSENUMBER: f64 = 100.0;
pub const RADIUS_STREET: f64 = 1000.0;
pub const RADIUS_MUNICIPALITY: f64 = 5000.0;

/// A geocoded address.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Adresse {
    /// "42 Rue de Rivoli 75004 Paris".
    pub label: String,
    /// Geocoding score, between 0 and 1.
    pub score: f32,
    /// `housenumber`, `street`, `locality` or `municipality`.
    #[serde(rename = "type")]
    pub kind: String,
    /// BAN identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// "42 Rue de Rivoli" for a housenumber, else the street, locality or
    /// municipality name.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub housenumber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// INSEE code of the municipality (`code_commune`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citycode: Option<String>,
    /// "75, Paris, Île-de-France".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub lat: f64,
    pub lng: f64,
}

fn first_str(doc: &Value, key: &str) -> Option<String> {
    let value = match doc.get(key)? {
        Value::Array(values) => values.first()?.clone(),
        value => value.clone(),
    };
    match value {
        Value::String(s) if !s.is_empty() => Some(s),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

impl From<&SearchResult> for Adresse {
    fn from(r: &SearchResult) -> Self {
        let doc = &r.doc;
        let street_name = first_str(doc, "name").unwrap_or_default();
        let (name, street) = match r.housenumber.as_deref() {
            Some(hn) => (format!("{hn} {street_name}"), Some(street_name)),
            None => {
                let street = (first_str(doc, "type").as_deref() == Some("street"))
                    .then(|| street_name.clone());
                (street_name, street)
            }
        };
        Adresse {
            label: r.label.clone(),
            score: r.score,
            kind: first_str(doc, "type").unwrap_or_default(),
            id: first_str(doc, "id"),
            name,
            housenumber: r.housenumber.clone(),
            street,
            postcode: first_str(doc, "postcode"),
            city: first_str(doc, "city"),
            citycode: first_str(doc, "citycode"),
            context: first_str(doc, "context"),
            lat: r.lat,
            lng: r.lon,
        }
    }
}

/// Which geocoding result an `address` search uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GeocodingMode {
    /// The best result if it reaches `geocoding_min_score`, else nothing is
    /// searched (empty results).
    Threshold,
    /// The best result if it reaches `geocoding_min_score`, else the best
    /// result anyway.
    #[default]
    ThresholdOrBest,
    /// Always the best result, whatever its score.
    Best,
}

/// The result to use, and whether it reached the minimum score.
pub fn select(
    results: &[SearchResult],
    mode: GeocodingMode,
    min_score: f32,
) -> Option<(&SearchResult, bool)> {
    let best = results.first()?;
    let meets = best.score >= min_score;
    match mode {
        GeocodingMode::Threshold if !meets => None,
        _ => Some((best, meets)),
    }
}

/// The address an `address` search was run with.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AddressMatch {
    #[serde(flatten)]
    pub adresse: Adresse,
    /// `geocoding_min_score` applied.
    pub min_score: f32,
    /// Whether the address reached `min_score`.
    pub meets_min_score: bool,
    /// How the search was narrowed. Absent when the address was rejected
    /// (`geocoding_mode=threshold` below `min_score`): nothing is returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<AddressFilter>,
}

/// How a geocoded address narrows an establishment search.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddressFilter {
    /// Within `radius` meters of the address.
    Radius { lat: f64, lng: f64, radius: f64 },
    /// In the municipality (its arrondissements for Paris, Lyon, Marseille).
    Commune { code_commune: Vec<String> },
}

/// Housenumber: 100 m. Street, locality: 1 km around its center. A
/// municipality: its INSEE code (SIRENE uses the arrondissement codes for
/// Paris, Lyon and Marseille). An explicit `radius` always wins.
pub fn filter_for(address: &Adresse, radius: Option<f64>) -> AddressFilter {
    let around = |radius| AddressFilter::Radius {
        lat: address.lat,
        lng: address.lng,
        radius,
    };
    if let Some(radius) = radius {
        return around(radius);
    }
    match address.kind.as_str() {
        "housenumber" => around(RADIUS_HOUSENUMBER),
        "municipality" => match address.citycode.as_deref() {
            Some(code) => AddressFilter::Commune {
                code_commune: sirene_communes(code),
            },
            None => around(RADIUS_MUNICIPALITY),
        },
        _ => around(RADIUS_STREET),
    }
}

/// SIRENE locates establishments of Paris, Lyon and Marseille by
/// arrondissement, never by the municipality code.
pub fn sirene_communes(citycode: &str) -> Vec<String> {
    let range = |first: u32, count: u32| (first..first + count).map(|c| c.to_string()).collect();
    match citycode {
        "75056" => range(75101, 20),
        "69123" => range(69381, 9),
        "13055" => range(13201, 16),
        code => vec![code.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn result(score: f32, doc: Value, housenumber: Option<&str>) -> SearchResult {
        SearchResult {
            label: "label".into(),
            score,
            housenumber: housenumber.map(str::to_string),
            lat: 48.85,
            lon: 2.35,
            distance_km: None,
            doc,
        }
    }

    fn street() -> Value {
        json!({"type": "street", "name": "Rue de Rivoli", "postcode": "75004", "city": ["Paris"], "citycode": "75104"})
    }

    #[test]
    fn modes() {
        let results = vec![result(0.4, street(), None), result(0.3, street(), None)];
        assert!(select(&results, GeocodingMode::Threshold, 0.5).is_none());
        assert_eq!(
            select(&results, GeocodingMode::ThresholdOrBest, 0.5).map(|(r, ok)| (r.score, ok)),
            Some((0.4, false))
        );
        assert_eq!(
            select(&results, GeocodingMode::Best, 0.5).map(|(_, ok)| ok),
            Some(false)
        );
        assert_eq!(
            select(&results, GeocodingMode::Threshold, 0.3).map(|(_, ok)| ok),
            Some(true)
        );
        assert!(select(&[], GeocodingMode::Best, 0.0).is_none());
    }

    #[test]
    fn address_of_a_housenumber() {
        let mut doc = street();
        doc["type"] = json!("housenumber");
        let a = Adresse::from(&result(0.97, doc, Some("42")));
        assert_eq!(a.name, "42 Rue de Rivoli");
        assert_eq!(a.street.as_deref(), Some("Rue de Rivoli"));
        assert_eq!(a.city.as_deref(), Some("Paris"));
        assert_eq!(
            filter_for(&a, None),
            AddressFilter::Radius {
                lat: 48.85,
                lng: 2.35,
                radius: 100.0
            }
        );
        assert_eq!(
            filter_for(&a, Some(30.0)),
            AddressFilter::Radius {
                lat: 48.85,
                lng: 2.35,
                radius: 30.0
            }
        );
    }

    #[test]
    fn municipalities_filter_by_code() {
        let paris = Adresse::from(&result(
            0.9,
            json!({"type": "municipality", "name": "Paris", "citycode": "75056"}),
            None,
        ));
        let AddressFilter::Commune { code_commune } = filter_for(&paris, None) else {
            panic!()
        };
        assert_eq!(code_commune.len(), 20);
        assert_eq!(code_commune[0], "75101");
        let nantes = Adresse::from(&result(
            0.9,
            json!({"type": "municipality", "name": "Nantes", "citycode": "44109"}),
            None,
        ));
        assert_eq!(
            filter_for(&nantes, None),
            AddressFilter::Commune {
                code_commune: vec!["44109".into()]
            }
        );
    }
}
