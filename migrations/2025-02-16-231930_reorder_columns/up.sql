ALTER TABLE "public"."unite_legale" RENAME COLUMN "caractere_employeur" TO "caractere_employeur_old";

ALTER TABLE "public"."unite_legale"
ADD COLUMN "caractere_employeur" varchar(1);

UPDATE "public"."unite_legale" SET "caractere_employeur" = "caractere_employeur_old";

ALTER TABLE "public"."unite_legale" DROP COLUMN "caractere_employeur_old";

DROP TABLE "public"."unite_legale_staging";
CREATE TABLE "public"."unite_legale_staging" (LIKE "public"."unite_legale" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES);

ALTER TABLE "public"."etablissement" RENAME COLUMN "siren" TO "siren_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "siren" varchar(9) NOT NULL;

ALTER TABLE "public"."etablissement" RENAME COLUMN "nic" TO "nic_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "nic" text NOT NULL;

ALTER TABLE "public"."etablissement" RENAME COLUMN "siret" TO "siret_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "siret" varchar(14);

ALTER TABLE "public"."etablissement" RENAME COLUMN "statut_diffusion" TO "statut_diffusion_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "statut_diffusion" varchar(1) NOT NULL;

ALTER TABLE "public"."etablissement" RENAME COLUMN "date_creation" TO "date_creation_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "date_creation" date;

ALTER TABLE "public"."etablissement" RENAME COLUMN "tranche_effectifs" TO "tranche_effectifs_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "tranche_effectifs" varchar(3);

ALTER TABLE "public"."etablissement" RENAME COLUMN "annee_effectifs" TO "annee_effectifs_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "annee_effectifs" int4;

ALTER TABLE "public"."etablissement" RENAME COLUMN "activite_principale_registre_metiers" TO "activite_principale_registre_metiers_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "activite_principale_registre_metiers" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "date_dernier_traitement" TO "date_dernier_traitement_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "date_dernier_traitement" timestamp;

ALTER TABLE "public"."etablissement" RENAME COLUMN "etablissement_siege" TO "etablissement_siege_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "etablissement_siege" bool NOT NULL;

ALTER TABLE "public"."etablissement" RENAME COLUMN "nombre_periodes" TO "nombre_periodes_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "nombre_periodes" int4;

ALTER TABLE "public"."etablissement" RENAME COLUMN "complement_adresse" TO "complement_adresse_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "complement_adresse" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "numero_voie" TO "numero_voie_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "numero_voie" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "indice_repetition" TO "indice_repetition_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "indice_repetition" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "dernier_numero_voie" TO "dernier_numero_voie_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "dernier_numero_voie" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "indice_repetition_dernier_numero_voie" TO "indice_repetition_dernier_numero_voie_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "indice_repetition_dernier_numero_voie" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "type_voie" TO "type_voie_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "type_voie" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_voie" TO "libelle_voie_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_voie" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_postal" TO "code_postal_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_postal" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_commune" TO "libelle_commune_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_commune" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_commune_etranger" TO "libelle_commune_etranger_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_commune_etranger" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "distribution_speciale" TO "distribution_speciale_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "distribution_speciale" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_commune" TO "code_commune_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_commune" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_cedex" TO "code_cedex_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_cedex" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_cedex" TO "libelle_cedex_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_cedex" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_pays_etranger" TO "code_pays_etranger_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_pays_etranger" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_pays_etranger" TO "libelle_pays_etranger_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_pays_etranger" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "identifiant_adresse" TO "identifiant_adresse_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "identifiant_adresse" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "coordonnee_lambert_x" TO "coordonnee_lambert_x_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "coordonnee_lambert_x" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "coordonnee_lambert_y" TO "coordonnee_lambert_y_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "coordonnee_lambert_y" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "complement_adresse2" TO "complement_adresse2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "complement_adresse2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "numero_voie_2" TO "numero_voie_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "numero_voie_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "indice_repetition_2" TO "indice_repetition_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "indice_repetition_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "type_voie_2" TO "type_voie_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "type_voie_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_voie_2" TO "libelle_voie_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_voie_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_postal_2" TO "code_postal_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_postal_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_commune_2" TO "libelle_commune_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_commune_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_commune_etranger_2" TO "libelle_commune_etranger_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_commune_etranger_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "distribution_speciale_2" TO "distribution_speciale_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "distribution_speciale_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_commune_2" TO "code_commune_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_commune_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_cedex_2" TO "code_cedex_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_cedex_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_cedex_2" TO "libelle_cedex_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_cedex_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "code_pays_etranger_2" TO "code_pays_etranger_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "code_pays_etranger_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "libelle_pays_etranger_2" TO "libelle_pays_etranger_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "libelle_pays_etranger_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "date_debut" TO "date_debut_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "date_debut" date;

ALTER TABLE "public"."etablissement" RENAME COLUMN "etat_administratif" TO "etat_administratif_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "etat_administratif" varchar(1) NOT NULL;

ALTER TABLE "public"."etablissement" RENAME COLUMN "enseigne_1" TO "enseigne_1_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "enseigne_1" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "enseigne_2" TO "enseigne_2_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "enseigne_2" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "enseigne_3" TO "enseigne_3_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "enseigne_3" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "denomination_usuelle" TO "denomination_usuelle_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "denomination_usuelle" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "activite_principale" TO "activite_principale_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "activite_principale" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "nomenclature_activite_principale" TO "nomenclature_activite_principale_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "nomenclature_activite_principale" text;

ALTER TABLE "public"."etablissement" RENAME COLUMN "caractere_employeur" TO "caractere_employeur_old";
ALTER TABLE "public"."etablissement" ADD COLUMN "caractere_employeur" text;

UPDATE "public"."etablissement" SET
"siren" = "siren_old",
"nic" = "nic_old",
"siret" = "siret_old",
"statut_diffusion" = "statut_diffusion_old",
"date_creation" = "date_creation_old",
"tranche_effectifs" = "tranche_effectifs_old",
"annee_effectifs" = "annee_effectifs_old",
"activite_principale_registre_metiers" = "activite_principale_registre_metiers_old",
"date_dernier_traitement" = "date_dernier_traitement_old",
"etablissement_siege" = "etablissement_siege_old",
"nombre_periodes" = "nombre_periodes_old",
"complement_adresse" = "complement_adresse_old",
"numero_voie" = "numero_voie_old",
"indice_repetition" = "indice_repetition_old",
"dernier_numero_voie" = "dernier_numero_voie_old",
"indice_repetition_dernier_numero_voie" = "indice_repetition_dernier_numero_voie_old",
"type_voie" = "type_voie_old",
"libelle_voie" = "libelle_voie_old",
"code_postal" = "code_postal_old",
"libelle_commune" = "libelle_commune_old",
"libelle_commune_etranger" = "libelle_commune_etranger_old",
"distribution_speciale" = "distribution_speciale_old",
"code_commune" = "code_commune_old",
"code_cedex" = "code_cedex_old",
"libelle_cedex" = "libelle_cedex_old",
"code_pays_etranger" = "code_pays_etranger_old",
"libelle_pays_etranger" = "libelle_pays_etranger_old",
"identifiant_adresse" = "identifiant_adresse_old",
"coordonnee_lambert_x" = "coordonnee_lambert_x_old",
"coordonnee_lambert_y" = "coordonnee_lambert_y_old",
"complement_adresse2" = "complement_adresse2_old",
"numero_voie_2" = "numero_voie_2_old",
"indice_repetition_2" = "indice_repetition_2_old",
"type_voie_2" = "type_voie_2_old",
"libelle_voie_2" = "libelle_voie_2_old",
"code_postal_2" = "code_postal_2_old",
"libelle_commune_2" = "libelle_commune_2_old",
"libelle_commune_etranger_2" = "libelle_commune_etranger_2_old",
"distribution_speciale_2" = "distribution_speciale_2_old",
"code_commune_2" = "code_commune_2_old",
"code_cedex_2" = "code_cedex_2_old",
"libelle_cedex_2" = "libelle_cedex_2_old",
"code_pays_etranger_2" = "code_pays_etranger_2_old",
"libelle_pays_etranger_2" = "libelle_pays_etranger_2_old",
"date_debut" = "date_debut_old",
"etat_administratif" = "etat_administratif_old",
"enseigne_1" = "enseigne_1_old",
"enseigne_2" = "enseigne_2_old",
"enseigne_3" = "enseigne_3_old",
"denomination_usuelle" = "denomination_usuelle_old",
"activite_principale" = "activite_principale_old",
"nomenclature_activite_principale" = "nomenclature_activite_principale_old",
"caractere_employeur" = "caractere_employeur_old";

ALTER TABLE "public"."etablissement" DROP COLUMN "siren_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "nic_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "siret_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "statut_diffusion_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "date_creation_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "tranche_effectifs_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "annee_effectifs_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "activite_principale_registre_metiers_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "date_dernier_traitement_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "etablissement_siege_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "nombre_periodes_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "complement_adresse_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "numero_voie_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "indice_repetition_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "dernier_numero_voie_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "indice_repetition_dernier_numero_voie_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "type_voie_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_voie_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_postal_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_commune_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_commune_etranger_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "distribution_speciale_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_commune_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_cedex_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_cedex_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_pays_etranger_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_pays_etranger_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "identifiant_adresse_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "coordonnee_lambert_x_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "coordonnee_lambert_y_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "complement_adresse2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "numero_voie_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "indice_repetition_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "type_voie_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_voie_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_postal_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_commune_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_commune_etranger_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "distribution_speciale_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_commune_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_cedex_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_cedex_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "code_pays_etranger_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "libelle_pays_etranger_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "date_debut_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "etat_administratif_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "enseigne_1_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "enseigne_2_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "enseigne_3_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "denomination_usuelle_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "activite_principale_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "nomenclature_activite_principale_old";
ALTER TABLE "public"."etablissement" DROP COLUMN "caractere_employeur_old";

ALTER TABLE "public"."etablissement" ADD PRIMARY KEY ("siret");

CREATE INDEX "etablissement_siren_index" ON "public"."etablissement" USING BTREE
("siren");
CREATE INDEX "etablissement_date_dernier_traitement_idx" ON "public"."etablissement" USING BTREE ("date_dernier_traitement");

DROP TABLE "public"."etablissement_staging";
CREATE TABLE "public"."etablissement_staging" (LIKE "public"."etablissement" INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING IDENTITY INCLUDING INDEXES);

ALTER TABLE "public"."update_metadata"
DROP COLUMN "data_only";
