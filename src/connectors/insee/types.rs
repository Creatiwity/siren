use serde::Deserialize;

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    total: u32,
    debut: u32,
    nombre: u32,
    pub curseur: String,
    pub curseur_suivant: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PeriodeUniteLegale {
    pub date_fin: Option<String>,
    pub date_debut: String,
    pub etat_administratif_unite_legale: String,
    pub changement_etat_administratif_unite_legale: bool,
    pub nom_unite_legale: Option<String>,
    pub changement_nom_unite_legale: bool,
    pub nom_usage_unite_legale: Option<String>,
    pub changement_nom_usage_unite_legale: bool,
    pub denomination_unite_legale: Option<String>,
    pub changement_denomination_unite_legale: bool,
    pub denomination_usuelle1_unite_legale: Option<String>,
    pub denomination_usuelle2_unite_legale: Option<String>,
    pub denomination_usuelle3_unite_legale: Option<String>,
    pub changement_denomination_usuelle_unite_legale: bool,
    pub categorie_juridique_unite_legale: String,
    pub changement_categorie_juridique_unite_legale: bool,
    pub activite_principale_unite_legale: Option<String>,
    pub nomenclature_activite_principale_unite_legale: Option<String>,
    pub changement_activite_principale_unite_legale: bool,
    pub nic_siege_unite_legale: String,
    pub changement_nic_siege_unite_legale: bool,
    pub economie_sociale_solidaire_unite_legale: Option<String>,
    pub changement_economie_sociale_solidaire_unite_legale: bool,
    pub caractere_employeur_unite_legale: Option<String>,
    pub changement_caractere_employeur_unite_legale: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UniteLegale {
    pub siren: String,
    pub statut_diffusion_unite_legale: String,

    #[serde(default = "default_as_false")]
    pub unite_purgee_unite_legale: bool,

    pub date_creation_unite_legale: String,
    pub sigle_unite_legale: Option<String>,
    pub sexe_unite_legale: Option<String>,
    pub prenom1_unite_legale: Option<String>,
    pub prenom2_unite_legale: Option<String>,
    pub prenom3_unite_legale: Option<String>,
    pub prenom4_unite_legale: Option<String>,
    pub prenom_usuel_unite_legale: Option<String>,
    pub pseudonyme_unite_legale: Option<String>,
    pub identifiant_association_unite_legale: Option<String>,
    pub tranche_effectifs_unite_legale: String,
    pub annee_effectifs_unite_legale: Option<String>,
    pub date_dernier_traitement_unite_legale: String,
    pub nombre_periodes_unite_legale: i32,
    pub categorie_entreprise: String,
    pub annee_categorie_entreprise: String,
    pub periodes_unite_legale: Vec<PeriodeUniteLegale>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeUniteLegaleResponse {
    pub header: Header,
    pub unites_legales: Vec<UniteLegale>,
}
