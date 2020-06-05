use serde::Deserialize;

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
pub struct InseeUniteLegaleResponse {
    header: Header,
    unites_legales: Vec<UniteLegale>,
}
