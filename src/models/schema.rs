// @generated automatically by Diesel CLI.

diesel::table! {
    etablissement (siret) {
        #[max_length = 9]
        siren -> Varchar,
        nic -> Text,
        #[max_length = 14]
        siret -> Varchar,
        #[max_length = 1]
        statut_diffusion -> Varchar,
        date_creation -> Nullable<Date>,
        #[max_length = 3]
        tranche_effectifs -> Nullable<Varchar>,
        annee_effectifs -> Nullable<Int4>,
        activite_principale_registre_metiers -> Nullable<Text>,
        date_dernier_traitement -> Nullable<Timestamp>,
        etablissement_siege -> Bool,
        nombre_periodes -> Nullable<Int4>,
        complement_adresse -> Nullable<Text>,
        numero_voie -> Nullable<Text>,
        indice_repetition -> Nullable<Text>,
        dernier_numero_voie -> Nullable<Text>,
        indice_repetition_dernier_numero_voie -> Nullable<Text>,
        type_voie -> Nullable<Text>,
        libelle_voie -> Nullable<Text>,
        code_postal -> Nullable<Text>,
        libelle_commune -> Nullable<Text>,
        libelle_commune_etranger -> Nullable<Text>,
        distribution_speciale -> Nullable<Text>,
        code_commune -> Nullable<Text>,
        code_cedex -> Nullable<Text>,
        libelle_cedex -> Nullable<Text>,
        code_pays_etranger -> Nullable<Text>,
        libelle_pays_etranger -> Nullable<Text>,
        identifiant_adresse -> Nullable<Text>,
        coordonnee_lambert_x -> Nullable<Text>,
        coordonnee_lambert_y -> Nullable<Text>,
        complement_adresse2 -> Nullable<Text>,
        numero_voie_2 -> Nullable<Text>,
        indice_repetition_2 -> Nullable<Text>,
        type_voie_2 -> Nullable<Text>,
        libelle_voie_2 -> Nullable<Text>,
        code_postal_2 -> Nullable<Text>,
        libelle_commune_2 -> Nullable<Text>,
        libelle_commune_etranger_2 -> Nullable<Text>,
        distribution_speciale_2 -> Nullable<Text>,
        code_commune_2 -> Nullable<Text>,
        code_cedex_2 -> Nullable<Text>,
        libelle_cedex_2 -> Nullable<Text>,
        code_pays_etranger_2 -> Nullable<Text>,
        libelle_pays_etranger_2 -> Nullable<Text>,
        date_debut -> Nullable<Date>,
        #[max_length = 1]
        etat_administratif -> Varchar,
        enseigne_1 -> Nullable<Text>,
        enseigne_2 -> Nullable<Text>,
        enseigne_3 -> Nullable<Text>,
        denomination_usuelle -> Nullable<Text>,
        activite_principale -> Nullable<Text>,
        nomenclature_activite_principale -> Nullable<Text>,
        caractere_employeur -> Nullable<Text>,
    }
}

diesel::table! {
    etablissement_staging (siret) {
        #[max_length = 9]
        siren -> Varchar,
        nic -> Text,
        #[max_length = 14]
        siret -> Varchar,
        #[max_length = 1]
        statut_diffusion -> Varchar,
        date_creation -> Nullable<Date>,
        #[max_length = 3]
        tranche_effectifs -> Nullable<Varchar>,
        annee_effectifs -> Nullable<Int4>,
        activite_principale_registre_metiers -> Nullable<Text>,
        date_dernier_traitement -> Nullable<Timestamp>,
        etablissement_siege -> Bool,
        nombre_periodes -> Nullable<Int4>,
        complement_adresse -> Nullable<Text>,
        numero_voie -> Nullable<Text>,
        indice_repetition -> Nullable<Text>,
        dernier_numero_voie -> Nullable<Text>,
        indice_repetition_dernier_numero_voie -> Nullable<Text>,
        type_voie -> Nullable<Text>,
        libelle_voie -> Nullable<Text>,
        code_postal -> Nullable<Text>,
        libelle_commune -> Nullable<Text>,
        libelle_commune_etranger -> Nullable<Text>,
        distribution_speciale -> Nullable<Text>,
        code_commune -> Nullable<Text>,
        code_cedex -> Nullable<Text>,
        libelle_cedex -> Nullable<Text>,
        code_pays_etranger -> Nullable<Text>,
        libelle_pays_etranger -> Nullable<Text>,
        identifiant_adresse -> Nullable<Text>,
        coordonnee_lambert_x -> Nullable<Text>,
        coordonnee_lambert_y -> Nullable<Text>,
        complement_adresse2 -> Nullable<Text>,
        numero_voie_2 -> Nullable<Text>,
        indice_repetition_2 -> Nullable<Text>,
        type_voie_2 -> Nullable<Text>,
        libelle_voie_2 -> Nullable<Text>,
        code_postal_2 -> Nullable<Text>,
        libelle_commune_2 -> Nullable<Text>,
        libelle_commune_etranger_2 -> Nullable<Text>,
        distribution_speciale_2 -> Nullable<Text>,
        code_commune_2 -> Nullable<Text>,
        code_cedex_2 -> Nullable<Text>,
        libelle_cedex_2 -> Nullable<Text>,
        code_pays_etranger_2 -> Nullable<Text>,
        libelle_pays_etranger_2 -> Nullable<Text>,
        date_debut -> Nullable<Date>,
        #[max_length = 1]
        etat_administratif -> Varchar,
        enseigne_1 -> Nullable<Text>,
        enseigne_2 -> Nullable<Text>,
        enseigne_3 -> Nullable<Text>,
        denomination_usuelle -> Nullable<Text>,
        activite_principale -> Nullable<Text>,
        nomenclature_activite_principale -> Nullable<Text>,
        caractere_employeur -> Nullable<Text>,
    }
}

diesel::table! {
    group_metadata (id) {
        id -> Int4,
        group_type -> Text,
        insee_name -> Text,
        file_name -> Text,
        staging_file_timestamp -> Nullable<Timestamptz>,
        staging_csv_file_timestamp -> Nullable<Timestamptz>,
        staging_imported_timestamp -> Nullable<Timestamptz>,
        last_imported_timestamp -> Nullable<Timestamptz>,
        url -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_insee_synced_timestamp -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    unite_legale (siren) {
        #[max_length = 9]
        siren -> Varchar,
        #[max_length = 1]
        statut_diffusion -> Varchar,
        unite_purgee -> Nullable<Text>,
        date_creation -> Nullable<Date>,
        sigle -> Nullable<Text>,
        #[max_length = 4]
        sexe -> Nullable<Varchar>,
        prenom_1 -> Nullable<Text>,
        prenom_2 -> Nullable<Text>,
        prenom_3 -> Nullable<Text>,
        prenom_4 -> Nullable<Text>,
        prenom_usuel -> Nullable<Text>,
        pseudonyme -> Nullable<Text>,
        identifiant_association -> Nullable<Text>,
        #[max_length = 3]
        tranche_effectifs -> Nullable<Varchar>,
        annee_effectifs -> Nullable<Int4>,
        date_dernier_traitement -> Nullable<Timestamp>,
        nombre_periodes -> Nullable<Int4>,
        categorie_entreprise -> Nullable<Text>,
        annee_categorie_entreprise -> Nullable<Int4>,
        date_debut -> Nullable<Date>,
        #[max_length = 1]
        etat_administratif -> Varchar,
        nom -> Nullable<Text>,
        nom_usage -> Nullable<Text>,
        denomination -> Nullable<Text>,
        denomination_usuelle_1 -> Nullable<Text>,
        denomination_usuelle_2 -> Nullable<Text>,
        denomination_usuelle_3 -> Nullable<Text>,
        categorie_juridique -> Nullable<Text>,
        activite_principale -> Nullable<Text>,
        nomenclature_activite_principale -> Nullable<Text>,
        nic_siege -> Nullable<Text>,
        economie_sociale_solidaire -> Nullable<Text>,
        #[max_length = 1]
        societe_mission -> Nullable<Varchar>,
        #[max_length = 1]
        caractere_employeur -> Nullable<Varchar>,
    }
}

diesel::table! {
    unite_legale_staging (siren) {
        #[max_length = 9]
        siren -> Varchar,
        #[max_length = 1]
        statut_diffusion -> Varchar,
        unite_purgee -> Nullable<Text>,
        date_creation -> Nullable<Date>,
        sigle -> Nullable<Text>,
        #[max_length = 4]
        sexe -> Nullable<Varchar>,
        prenom_1 -> Nullable<Text>,
        prenom_2 -> Nullable<Text>,
        prenom_3 -> Nullable<Text>,
        prenom_4 -> Nullable<Text>,
        prenom_usuel -> Nullable<Text>,
        pseudonyme -> Nullable<Text>,
        identifiant_association -> Nullable<Text>,
        #[max_length = 3]
        tranche_effectifs -> Nullable<Varchar>,
        annee_effectifs -> Nullable<Int4>,
        date_dernier_traitement -> Nullable<Timestamp>,
        nombre_periodes -> Nullable<Int4>,
        categorie_entreprise -> Nullable<Text>,
        annee_categorie_entreprise -> Nullable<Int4>,
        date_debut -> Nullable<Date>,
        #[max_length = 1]
        etat_administratif -> Varchar,
        nom -> Nullable<Text>,
        nom_usage -> Nullable<Text>,
        denomination -> Nullable<Text>,
        denomination_usuelle_1 -> Nullable<Text>,
        denomination_usuelle_2 -> Nullable<Text>,
        denomination_usuelle_3 -> Nullable<Text>,
        categorie_juridique -> Nullable<Text>,
        activite_principale -> Nullable<Text>,
        nomenclature_activite_principale -> Nullable<Text>,
        nic_siege -> Nullable<Text>,
        economie_sociale_solidaire -> Nullable<Text>,
        #[max_length = 1]
        societe_mission -> Nullable<Varchar>,
        #[max_length = 1]
        caractere_employeur -> Nullable<Varchar>,
    }
}

diesel::table! {
    update_metadata (id) {
        id -> Int4,
        synthetic_group_type -> Text,
        force -> Bool,
        status -> Text,
        summary -> Nullable<Jsonb>,
        error -> Nullable<Text>,
        launched_timestamp -> Timestamptz,
        finished_timestamp -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    etablissement,
    etablissement_staging,
    group_metadata,
    unite_legale,
    unite_legale_staging,
    update_metadata,
);
