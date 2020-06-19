mod columns;
pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel};
use super::schema::unite_legale::dsl;
use crate::connectors::{insee::INITIAL_CURSOR, Connectors};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use common::UniteLegale;
use diesel::pg::upsert::excluded;
use diesel::prelude::*;
use diesel::sql_query;
use error::Error;

pub fn get(connectors: &Connectors, siren: &String) -> Result<UniteLegale, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::unite_legale
        .find(siren)
        .first::<UniteLegale>(&connection)
        .map_err(|error| error.into())
}

pub struct UniteLegaleModel {}

#[async_trait]
impl UpdatableModel for UniteLegaleModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        dsl::unite_legale
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::unite_legale_staging::dsl;

        let connection = connectors.local.pool.get()?;
        dsl::unite_legale_staging
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn insert_in_staging(
        &self,
        connectors: &Connectors,
        file_path: String,
    ) -> Result<bool, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        sql_query("TRUNCATE unite_legale_staging").execute(&connection)?;
        let query = format!(
            "COPY unite_legale_staging({}) FROM '{}' DELIMITER ',' CSV HEADER",
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
            sql_query("ALTER TABLE unite_legale RENAME TO unite_legale_temp")
                .execute(&connection)?;
            sql_query("ALTER TABLE unite_legale_staging RENAME TO unite_legale")
                .execute(&connection)?;
            sql_query("ALTER TABLE unite_legale_temp RENAME TO unite_legale_staging")
                .execute(&connection)?;
            sql_query("TRUNCATE unite_legale_staging").execute(&connection)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET last_imported_timestamp = staging_imported_timestamp
                WHERE group_type = 'unites_legales'
                "#,
            )
            .execute(&connection)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET staging_imported_timestamp = NULL
                WHERE group_type = 'unites_legales'
                "#,
            )
            .execute(&connection)?;

            Ok(())
        })
    }

    // SELECT date_dernier_traitement FROM unite_legale WHERE date_dernier_traitement IS NOT NULL ORDER BY date_dernier_traitement DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        dsl::unite_legale
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
            let (next_cursor, unites_legales) = insee
                .get_daily_unites_legales(start_timestamp, cursor)
                .await?;

            current_cursor = next_cursor;

            updated_count += diesel::insert_into(dsl::unite_legale)
                .values(&unites_legales)
                .on_conflict(dsl::siren)
                .do_update()
                .set((
                    dsl::statut_diffusion.eq(excluded(dsl::statut_diffusion)),
                    dsl::unite_purgee.eq(excluded(dsl::unite_purgee)),
                    dsl::date_creation.eq(excluded(dsl::date_creation)),
                    dsl::sigle.eq(excluded(dsl::sigle)),
                    dsl::sexe.eq(excluded(dsl::sexe)),
                    dsl::prenom_1.eq(excluded(dsl::prenom_1)),
                    dsl::prenom_2.eq(excluded(dsl::prenom_2)),
                    dsl::prenom_3.eq(excluded(dsl::prenom_3)),
                    dsl::prenom_4.eq(excluded(dsl::prenom_4)),
                    dsl::prenom_usuel.eq(excluded(dsl::prenom_usuel)),
                    dsl::pseudonyme.eq(excluded(dsl::pseudonyme)),
                    dsl::identifiant_association.eq(excluded(dsl::identifiant_association)),
                    dsl::tranche_effectifs.eq(excluded(dsl::tranche_effectifs)),
                    dsl::annee_effectifs.eq(excluded(dsl::annee_effectifs)),
                    dsl::date_dernier_traitement.eq(excluded(dsl::date_dernier_traitement)),
                    dsl::nombre_periodes.eq(excluded(dsl::nombre_periodes)),
                    dsl::categorie_entreprise.eq(excluded(dsl::categorie_entreprise)),
                    dsl::annee_categorie_entreprise.eq(excluded(dsl::annee_categorie_entreprise)),
                    dsl::date_debut.eq(excluded(dsl::date_debut)),
                    dsl::etat_administratif.eq(excluded(dsl::etat_administratif)),
                    dsl::nom.eq(excluded(dsl::nom)),
                    dsl::nom_usage.eq(excluded(dsl::nom_usage)),
                    dsl::denomination.eq(excluded(dsl::denomination)),
                    dsl::denomination_usuelle_1.eq(excluded(dsl::denomination_usuelle_1)),
                    dsl::denomination_usuelle_2.eq(excluded(dsl::denomination_usuelle_2)),
                    dsl::denomination_usuelle_3.eq(excluded(dsl::denomination_usuelle_3)),
                    dsl::categorie_juridique.eq(excluded(dsl::categorie_juridique)),
                    dsl::activite_principale.eq(excluded(dsl::activite_principale)),
                    dsl::nomenclature_activite_principale
                        .eq(excluded(dsl::nomenclature_activite_principale)),
                    dsl::nic_siege.eq(excluded(dsl::nic_siege)),
                    dsl::economie_sociale_solidaire.eq(excluded(dsl::economie_sociale_solidaire)),
                    dsl::caractere_employeur.eq(excluded(dsl::caractere_employeur)),
                ))
                .execute(&connection)?;
        }

        Ok(updated_count)
    }
}
