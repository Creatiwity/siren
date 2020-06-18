mod columns;
pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel};
use super::schema::etablissement::dsl;
use crate::connectors::{insee::INITIAL_CURSOR, Connectors};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use common::Etablissement;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel::sql_query;
use error::Error;

pub fn get(connectors: &Connectors, siret: &String) -> Result<Etablissement, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::etablissement
        .find(siret)
        .first::<Etablissement>(&connection)
        .map_err(|error| error.into())
}

pub fn get_with_siren(
    connectors: &Connectors,
    siren: &String,
) -> Result<Vec<Etablissement>, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::etablissement
        .filter(dsl::siren.eq(siren))
        .load::<Etablissement>(&connection)
        .map_err(|error| error.into())
}

pub fn get_siege_with_siren(
    connectors: &Connectors,
    siren: &String,
) -> Result<Etablissement, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::etablissement
        .filter(dsl::siren.eq(siren).and(dsl::etablissement_siege.eq(true)))
        .first::<Etablissement>(&connection)
        .map_err(|error| error.into())
}

pub struct EtablissementModel {}

#[async_trait]
impl UpdatableModel for EtablissementModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        dsl::etablissement
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::etablissement_staging::dsl;

        let connection = connectors.local.pool.get()?;
        dsl::etablissement_staging
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn insert_in_staging(
        &self,
        connectors: &Connectors,
        file_path: String,
    ) -> Result<bool, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        sql_query("TRUNCATE etablissement_staging").execute(&connection)?;
        let query = format!(
            "COPY etablissement_staging({}) FROM '{}' DELIMITER ',' CSV HEADER",
            columns::COLUMNS,
            file_path
        );
        sql_query(query)
            .execute(&connection)
            .map(|count| count > 0)
            .map_err(|error| error.into())
    }

    fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let connection = connectors.local.pool.get()?;
        connection.build_transaction().read_write().run(|| {
            sql_query("ALTER TABLE etablissement RENAME TO etablissement_temp")
                .execute(&connection)?;
            sql_query("ALTER TABLE etablissement_staging RENAME TO etablissement")
                .execute(&connection)?;
            sql_query("ALTER TABLE etablissement_temp RENAME TO etablissement_staging")
                .execute(&connection)?;
            sql_query("TRUNCATE etablissement_staging").execute(&connection)?;
            sql_query(
                r#"
            UPDATE group_metadata
            SET last_imported_timestamp = staging_imported_timestamp
            WHERE group_type = 'etablissements'
            "#,
            )
            .execute(&connection)?;
            sql_query(
                r#"
            UPDATE group_metadata
            SET staging_imported_timestamp = NULL
            WHERE group_type = 'etablissements'
            "#,
            )
            .execute(&connection)?;

            Ok(())
        })
    }

    // SELECT date_dernier_traitement FROM etablissement WHERE date_dernier_traitement IS NOT NULL ORDER BY date_dernier_traitement DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        dsl::etablissement
            .select(dsl::date_dernier_traitement)
            .order(dsl::date_dernier_traitement.desc())
            .filter(dsl::date_dernier_traitement.is_not_null())
            .first::<Option<NaiveDateTime>>(&connection)
            .map_err(|error| error.into())
    }

    async fn update_daily_data(
        &self,
        connectors: &Connectors,
        start_timestamp: NaiveDateTime,
    ) -> Result<usize, UpdatableError> {
        let insee = connectors
            .insee
            .as_ref()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        let connection = connectors.local.pool.get()?;

        let mut current_cursor: Option<String> = Some(INITIAL_CURSOR.to_string());
        let mut updated_count: usize = 0;

        while let Some(cursor) = current_cursor {
            let (next_cursor, etablissements) = insee
                .get_daily_etablissements(start_timestamp, cursor)
                .await?;

            current_cursor = next_cursor;

            updated_count += diesel::insert_into(dsl::etablissement)
                .values(&etablissements)
                .on_conflict(dsl::siret)
                .do_update()
                .set((
                    dsl::nic.eq(excluded(dsl::nic)),
                    dsl::siren.eq(excluded(dsl::siren)),
                    dsl::statut_diffusion.eq(excluded(dsl::statut_diffusion)),
                    dsl::date_creation.eq(excluded(dsl::date_creation)),
                    dsl::tranche_effectifs.eq(excluded(dsl::tranche_effectifs)),
                    dsl::annee_effectifs.eq(excluded(dsl::annee_effectifs)),
                    dsl::activite_principale_registre_metiers
                        .eq(excluded(dsl::activite_principale_registre_metiers)),
                    dsl::date_dernier_traitement.eq(excluded(dsl::date_dernier_traitement)),
                    dsl::etablissement_siege.eq(excluded(dsl::etablissement_siege)),
                    dsl::nombre_periodes.eq(excluded(dsl::nombre_periodes)),
                    dsl::complement_adresse.eq(excluded(dsl::complement_adresse)),
                    dsl::numero_voie.eq(excluded(dsl::numero_voie)),
                    dsl::indice_repetition.eq(excluded(dsl::indice_repetition)),
                    dsl::type_voie.eq(excluded(dsl::type_voie)),
                    dsl::libelle_voie.eq(excluded(dsl::libelle_voie)),
                    dsl::code_postal.eq(excluded(dsl::code_postal)),
                    dsl::libelle_commune.eq(excluded(dsl::libelle_commune)),
                    dsl::libelle_commune_etranger.eq(excluded(dsl::libelle_commune_etranger)),
                    dsl::distribution_speciale.eq(excluded(dsl::distribution_speciale)),
                    dsl::code_commune.eq(excluded(dsl::code_commune)),
                    dsl::code_cedex.eq(excluded(dsl::code_cedex)),
                    dsl::libelle_cedex.eq(excluded(dsl::libelle_cedex)),
                    dsl::code_pays_etranger.eq(excluded(dsl::code_pays_etranger)),
                    dsl::libelle_pays_etranger.eq(excluded(dsl::libelle_pays_etranger)),
                    dsl::complement_adresse2.eq(excluded(dsl::complement_adresse2)),
                    dsl::numero_voie_2.eq(excluded(dsl::numero_voie_2)),
                    dsl::indice_repetition_2.eq(excluded(dsl::indice_repetition_2)),
                    dsl::type_voie_2.eq(excluded(dsl::type_voie_2)),
                    dsl::libelle_voie_2.eq(excluded(dsl::libelle_voie_2)),
                    dsl::code_postal_2.eq(excluded(dsl::code_postal_2)),
                    dsl::libelle_commune_2.eq(excluded(dsl::libelle_commune_2)),
                    dsl::libelle_commune_etranger_2.eq(excluded(dsl::libelle_commune_etranger_2)),
                    dsl::distribution_speciale_2.eq(excluded(dsl::distribution_speciale_2)),
                    dsl::code_commune_2.eq(excluded(dsl::code_commune_2)),
                    dsl::code_cedex_2.eq(excluded(dsl::code_cedex_2)),
                    dsl::libelle_cedex_2.eq(excluded(dsl::libelle_cedex_2)),
                    dsl::code_pays_etranger_2.eq(excluded(dsl::code_pays_etranger_2)),
                    dsl::libelle_pays_etranger_2.eq(excluded(dsl::libelle_pays_etranger_2)),
                    dsl::date_debut.eq(excluded(dsl::date_debut)),
                    dsl::etat_administratif.eq(excluded(dsl::etat_administratif)),
                    dsl::enseigne_1.eq(excluded(dsl::enseigne_1)),
                    dsl::enseigne_2.eq(excluded(dsl::enseigne_2)),
                    dsl::enseigne_3.eq(excluded(dsl::enseigne_3)),
                    dsl::denomination_usuelle.eq(excluded(dsl::denomination_usuelle)),
                    dsl::activite_principale.eq(excluded(dsl::activite_principale)),
                    dsl::nomenclature_activite_principale
                        .eq(excluded(dsl::nomenclature_activite_principale)),
                    dsl::caractere_employeur.eq(excluded(dsl::caractere_employeur)),
                ))
                .execute(&connection)?;
        }

        Ok(updated_count)
    }
}
