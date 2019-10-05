CREATE TABLE "public"."etablissement"
(
    "siret" varchar(14) NOT NULL,
    PRIMARY KEY ("siret"),
    "siren" varchar(9) NOT NULL,
    "nic" text NOT NULL,
    "statut_diffusion" varchar(1) NOT NULL,
    "date_creation" date,
    "tranche_effectifs" varchar(3),
    "annee_effectifs" int4,
    "activite_principale_registre_metiers" text,
    "date_dernier_traitement" timestamp,
    "etablissement_siege" bool NOT NULL,
    "nombre_periodes" int4,
    "complement_adresse" text,
    "numero_voie" text,
    "indice_repetition" text,
    "type_voie" text,
    "libelle_voie" text,
    "code_postal" text,
    "libelle_commune" text,
    "libelle_commune_etranger" text,
    "distribution_speciale" text,
    "code_commune" text,
    "code_cedex" text,
    "libelle_cedex" text,
    "code_pays_etranger" text,
    "libelle_pays_etranger" text,
    "complement_adresse2" text,
    "numero_voie_2" text,
    "indice_repetition_2" text,
    "type_voie_2" text,
    "libelle_voie_2" text,
    "code_postal_2" text,
    "libelle_commune_2" text,
    "libelle_commune_etranger_2" text,
    "distribution_speciale_2" text,
    "code_commune_2" text,
    "code_cedex_2" text,
    "libelle_cedex_2" text,
    "code_pays_etranger_2" text,
    "libelle_pays_etranger_2" text,
    "date_debut" date,
    "etat_administratif" varchar(1) NOT NULL,
    "enseigne_1" text,
    "enseigne_2" text,
    "enseigne_3" text,
    "denomination_usuelle" text,
    "activite_principale" text,
    "nomenclature_activite_principale" text,
    "caractere_employeur" text
);

CREATE INDEX "etablissement_siren_index" ON "public"."etablissement" USING BTREE
("siren");

CREATE TABLE "public"."etablissement_staging"
(
    LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING INDEXES
);

CREATE TABLE "public"."unite_legale"
(
    "siren" varchar(9) NOT NULL,
    PRIMARY KEY("siren"),
    "statut_diffusion" varchar(1) NOT NULL,
    "unite_purgee" text,
    "date_creation" date,
    "sigle" text,
    "sexe" varchar(1),
    "prenom_1" text,
    "prenom_2" text,
    "prenom_3" text,
    "prenom_4" text,
    "prenom_usuel" text,
    "pseudonyme" text,
    "identifiant_association" text,
    "tranche_effectifs" varchar(3),
    "annee_effectifs" int4,
    "date_dernier_traitement" timestamp,
    "nombre_periodes" int4,
    "categorie_entreprise" text,
    "annee_categorie_entreprise" int4,
    "date_debut" date,
    "etat_administratif" varchar(1) NOT NULL,
    "nom" text,
    "nom_usage" text,
    "denomination" text,
    "denomination_usuelle_1" text,
    "denomination_usuelle_2" text,
    "denomination_usuelle_3" text,
    "categorie_juridique" text,
    "activite_principale" text,
    "nomenclature_activite_principale" text,
    "nic_siege" text,
    "economie_sociale_solidaire" text,
    "caractere_employeur" varchar(1)
);

CREATE TABLE "public"."unite_legale_staging"
(
    LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING INDEXES
);
