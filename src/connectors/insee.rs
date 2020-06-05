use custom_error::custom_error;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::env;

custom_error! { pub Error
    TokenNetworkError { source: reqwest::Error } = "Unable to retrieve INSEE token (network error: {source})",
    TokenApiError = "Unable to retrieve INSEE token",
    TokenMalformedError {source: serde_json::Error} = "Unable to read INSEE token ({source})",
}

pub struct Connector {
    pub token: String,
}

pub struct ConnectorBuilder {
    pub credentials: String,
}

#[derive(Serialize)]
struct InseeTokenParameters {
    pub grant_type: String,
    pub validity_period: u32,
}

#[derive(Deserialize)]
struct InseeTokenResponse {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl ConnectorBuilder {
    pub fn new() -> Option<ConnectorBuilder> {
        let credentials = env::var("INSEE_CREDENTIALS").ok();

        if let Some(credentials) = credentials {
            Some(ConnectorBuilder { credentials })
        } else {
            None
        }
    }

    pub fn create(&self) -> Result<Connector, Error> {
        self.generate_token().map(|token| Connector { token })
    }

    fn generate_token(&self) -> Result<String, Error> {
        let client = reqwest::blocking::Client::new();
        let response: InseeTokenResponse = client
            .post("https://api.insee.fr/token")
            .header(AUTHORIZATION, format!("Basic {}", self.credentials))
            .form(&InseeTokenParameters {
                grant_type: String::from("client_credentials"),
                validity_period: 86400,
            })
            .send()?
            .json()?;

        Ok(response.access_token)
    }
}

custom_error! { pub InseeError
    NetworkError {source: reqwest::Error} = "Unable to retrieve INSEE data (network error: {source})",
}

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Header {
    total: u32,
    debut: u32,
    nombre: u32,
    curseur: String,
    curseur_suivant: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PeriodeUniteLegale {
    date_fin: Option<String>,
    date_debut: String,
    etat_administratif_unite_legale: String,
    changement_etat_administratif_unite_legale: bool,
    nom_unite_legale: Option<String>,
    changement_nom_unite_legale: bool,
    nom_usage_unite_legale: Option<String>,
    changement_nom_usage_unite_legale: bool,
    denomination_unite_legale: Option<String>,
    changement_denomination_unite_legale: bool,
    denomination_usuelle1_unite_legale: Option<String>,
    denomination_usuelle2_unite_legale: Option<String>,
    denomination_usuelle3_unite_legale: Option<String>,
    changement_denomination_usuelle_unite_legale: bool,
    categorie_juridique_unite_legale: String,
    changement_categorie_juridique_unite_legale: bool,
    activite_principale_unite_legale: Option<String>,
    nomenclature_activite_principale_unite_legale: Option<String>,
    changement_activite_principale_unite_legale: bool,
    nic_siege_unite_legale: String,
    changement_nic_siege_unite_legale: bool,
    economie_sociale_solidaire_unite_legale: Option<String>,
    changement_economie_sociale_solidaire_unite_legale: bool,
    caractere_employeur_unite_legale: Option<String>,
    changement_caractere_employeur_unite_legale: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UniteLegale {
    siren: String,
    statut_diffusion_unite_legale: String,

    #[serde(default = "default_as_false")]
    unite_purgee_unite_legale: bool,

    date_creation_unite_legale: String,
    sigle_unite_legale: Option<String>,
    sexe_unite_legale: Option<String>,
    prenom1_unite_legale: Option<String>,
    prenom2_unite_legale: Option<String>,
    prenom3_unite_legale: Option<String>,
    prenom4_unite_legale: Option<String>,
    prenom_usuel_unite_legale: Option<String>,
    pseudonyme_unite_legale: Option<String>,
    identifiant_association_unite_legale: Option<String>,
    tranche_effectifs_unite_legale: String,
    annee_effectifs_unite_legale: Option<String>,
    date_dernier_traitement_unite_legale: String,
    nombre_periodes_unite_legale: u32,
    categorie_entreprise: String,
    annee_categorie_entreprise: String,
    periodes_unite_legale: Vec<PeriodeUniteLegale>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InseeUniteLegaleResponse {
    header: Header,
    unites_legales: Vec<UniteLegale>,
}

impl Connector {
    pub fn get_daily_unites_legales(&self) -> Result<String, InseeError> {
        let client = reqwest::blocking::Client::new();
        let response: InseeUniteLegaleResponse = client
            .get("https://api.insee.fr/entreprises/sirene/V3/siren?q=dateDernierTraitementUniteLegale:[2020-06-04 TO *]&nombre=3&curseur=*")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/json")
            .send()?
            .error_for_status()?
            .json()?;

        println!("{:#?}", response);

        Ok(String::from("UniteLegale"))
    }

    pub fn get_daily_etablissements(&self) -> Result<String, InseeError> {
        Ok(String::from("Etablissement"))
    }
}
