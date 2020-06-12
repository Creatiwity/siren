use super::{Header, InseeResponse};
use crate::models::etablissement::common::Etablissement;
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeEtablissementResponse {
    pub header: Header,
    pub etablissements: Vec<InseeEtablissement>,
}

impl InseeResponse for InseeEtablissementResponse {
    fn header(&self) -> Header {
        self.header.clone()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseeEtablissementInner {
    pub siren: String,
    pub nic: String,
    pub siret: String,
    pub statut_diffusion_etablissement: String,
    pub date_creation_etablissement: Option<NaiveDate>,
    pub tranche_effectifs_etablissement: Option<String>,
    #[serde(deserialize_with = "super::from_str_optional")]
    pub annee_effectifs_etablissement: Option<i32>,
    pub activite_principale_registre_metiers_etablissement: Option<String>,
    pub date_dernier_traitement_etablissement: Option<NaiveDateTime>,
    pub etablissement_siege: bool,
    pub nombre_periodes_etablissement: Option<i32>,
    pub adresse_etablissement: InseeAdresseEtablissement,
    pub adresse2_etablissement: InseeAdresse2Etablissement,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeEtablissement {
    #[serde(flatten)]
    pub content: InseeEtablissementInner,
    pub periodes_etablissement: Vec<InseePeriodeEtablissement>,
}

#[derive(Debug)]
pub struct InseeEtablissementWithPeriode {
    pub content: InseeEtablissementInner,
    pub periode: InseePeriodeEtablissement,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseeAdresseEtablissement {
    pub complement_adresse_etablissement: Option<String>,
    pub numero_voie_etablissement: Option<String>,
    pub indice_repetition_etablissement: Option<String>,
    pub type_voie_etablissement: Option<String>,
    pub libelle_voie_etablissement: Option<String>,
    pub code_postal_etablissement: Option<String>,
    pub libelle_commune_etablissement: Option<String>,
    pub libelle_commune_etranger_etablissement: Option<String>,
    pub distribution_speciale_etablissement: Option<String>,
    pub code_commune_etablissement: Option<String>,
    pub code_cedex_etablissement: Option<String>,
    pub libelle_cedex_etablissement: Option<String>,
    pub code_pays_etranger_etablissement: Option<String>,
    pub libelle_pays_etranger_etablissement: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseeAdresse2Etablissement {
    complement_adresse2_etablissement: Option<String>,
    numero_voie2_etablissement: Option<String>,
    indice_repetition2_etablissement: Option<String>,
    type_voie2_etablissement: Option<String>,
    libelle_voie2_etablissement: Option<String>,
    code_postal2_etablissement: Option<String>,
    libelle_commune2_etablissement: Option<String>,
    libelle_commune_etranger2_etablissement: Option<String>,
    distribution_speciale2_etablissement: Option<String>,
    code_commune2_etablissement: Option<String>,
    code_cedex2_etablissement: Option<String>,
    libelle_cedex2_etablissement: Option<String>,
    code_pays_etranger2_etablissement: Option<String>,
    libelle_pays_etranger2_etablissement: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseePeriodeEtablissement {
    date_fin: Option<NaiveDate>,
    date_debut: Option<NaiveDate>,
    #[serde(deserialize_with = "deserialize_etat_administratif")]
    etat_administratif_etablissement: String,
    changement_etat_administratif_etablissement: bool,
    enseigne1_etablissement: Option<String>,
    enseigne2_etablissement: Option<String>,
    enseigne3_etablissement: Option<String>,
    changement_enseigne_etablissement: bool,
    denomination_usuelle_etablissement: Option<String>,
    changement_denomination_usuelle_etablissement: bool,
    activite_principale_etablissement: Option<String>,
    nomenclature_activite_principale_etablissement: Option<String>,
    changement_activite_principale_etablissement: bool,
    caractere_employeur_etablissement: Option<String>,
    changement_caractere_employeur_etablissement: bool,
}

impl From<&InseeEtablissement> for Option<Etablissement> {
    fn from(u: &InseeEtablissement) -> Self {
        match u
            .periodes_etablissement
            .iter()
            .find(|p| p.date_fin.is_none())
        {
            Some(periode) => {
                // Convert
                Some(
                    InseeEtablissementWithPeriode {
                        content: u.content.clone(),
                        periode: periode.clone(),
                    }
                    .into(),
                )
            }
            None => None,
        }
    }
}

impl From<InseeEtablissementWithPeriode> for Etablissement {
    fn from(e: InseeEtablissementWithPeriode) -> Self {
        let adresse = e.content.adresse_etablissement;
        let adresse2 = e.content.adresse2_etablissement;

        Etablissement {
            siret: e.content.siret,
            siren: e.content.siren,
            nic: e.content.nic,
            statut_diffusion: e.content.statut_diffusion_etablissement,
            date_creation: e.content.date_creation_etablissement,
            tranche_effectifs: e.content.tranche_effectifs_etablissement,
            annee_effectifs: e.content.annee_effectifs_etablissement,
            activite_principale_registre_metiers: e
                .content
                .activite_principale_registre_metiers_etablissement,
            date_dernier_traitement: e.content.date_dernier_traitement_etablissement,
            etablissement_siege: e.content.etablissement_siege,
            nombre_periodes: e.content.nombre_periodes_etablissement,
            complement_adresse: adresse.complement_adresse_etablissement,
            numero_voie: adresse.numero_voie_etablissement,
            indice_repetition: adresse.indice_repetition_etablissement,
            type_voie: adresse.type_voie_etablissement,
            libelle_voie: adresse.libelle_voie_etablissement,
            code_postal: adresse.code_postal_etablissement,
            libelle_commune: adresse.libelle_commune_etablissement,
            libelle_commune_etranger: adresse.libelle_commune_etranger_etablissement,
            distribution_speciale: adresse.distribution_speciale_etablissement,
            code_commune: adresse.code_commune_etablissement,
            code_cedex: adresse.code_cedex_etablissement,
            libelle_cedex: adresse.libelle_cedex_etablissement,
            code_pays_etranger: adresse.code_pays_etranger_etablissement,
            libelle_pays_etranger: adresse.libelle_pays_etranger_etablissement,
            complement_adresse2: adresse2.complement_adresse2_etablissement,
            numero_voie_2: adresse2.numero_voie2_etablissement,
            indice_repetition_2: adresse2.indice_repetition2_etablissement,
            type_voie_2: adresse2.type_voie2_etablissement,
            libelle_voie_2: adresse2.libelle_voie2_etablissement,
            code_postal_2: adresse2.code_postal2_etablissement,
            libelle_commune_2: adresse2.libelle_commune2_etablissement,
            libelle_commune_etranger_2: adresse2.libelle_commune_etranger2_etablissement,
            distribution_speciale_2: adresse2.distribution_speciale2_etablissement,
            code_commune_2: adresse2.code_commune2_etablissement,
            code_cedex_2: adresse2.code_cedex2_etablissement,
            libelle_cedex_2: adresse2.libelle_cedex2_etablissement,
            code_pays_etranger_2: adresse2.code_pays_etranger2_etablissement,
            libelle_pays_etranger_2: adresse2.libelle_pays_etranger2_etablissement,
            date_debut: e.periode.date_debut,
            etat_administratif: e.periode.etat_administratif_etablissement,
            enseigne_1: e.periode.enseigne1_etablissement,
            enseigne_2: e.periode.enseigne2_etablissement,
            enseigne_3: e.periode.enseigne3_etablissement,
            denomination_usuelle: e.periode.denomination_usuelle_etablissement,
            activite_principale: e.periode.activite_principale_etablissement,
            nomenclature_activite_principale: e
                .periode
                .nomenclature_activite_principale_etablissement,
            caractere_employeur: e.periode.caractere_employeur_etablissement,
        }
    }
}

fn deserialize_etat_administratif<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(String::from("F")))
}
