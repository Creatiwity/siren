//! Head-office flag after a legal unit sync.
//!
//! Insee moves a head office by changing `nicSiegeUniteLegale`, and leaves the
//! `dateDernierTraitementEtablissement` of the new head office untouched. The
//! daily establishment sync, keyed on that date, never brings the new flag: the
//! legal unit ends up with no head office at all (seen on SIREN 353948847).

use diesel::sql_query;
use diesel_async::{AsyncConnection, RunQueryDsl};

use crate::models::etablissement;
use crate::tests::require_database;

#[tokio::test]
async fn siege_realigne_sur_le_nic_siege_de_l_unite_legale() {
    let mut connection = require_database!();
    connection.begin_test_transaction().await.unwrap();

    // 000000001: head office moved from 00036 to 00044, only the legal unit
    // was synced. 000000002: no `nic_siege`, nothing to derive the flag from.
    sql_query(
        "INSERT INTO unite_legale (siren, statut_diffusion, etat_administratif, nic_siege) VALUES \
         ('000000001', 'O', 'A', '00044'), ('000000002', 'O', 'A', NULL)",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    sql_query(
        "INSERT INTO etablissement \
         (siren, nic, siret, statut_diffusion, etablissement_siege, etat_administratif) VALUES \
         ('000000001', '00036', '00000000100036', 'O', true, 'F'), \
         ('000000001', '00044', '00000000100044', 'O', false, 'A'), \
         ('000000002', '00010', '00000000200010', 'O', true, 'A')",
    )
    .execute(&mut connection)
    .await
    .unwrap();

    etablissement::realign_sieges(&mut connection, &["000000001", "000000002"])
        .await
        .unwrap();

    let mut flags = Vec::new();
    for siren in ["000000001", "000000002"] {
        for e in etablissement::get_with_siren(&mut connection, siren)
            .await
            .unwrap()
        {
            flags.push((e.siret, e.etablissement_siege));
        }
    }
    flags.sort();

    assert_eq!(
        flags,
        [
            ("00000000100036".to_string(), false),
            ("00000000100044".to_string(), true),
            ("00000000200010".to_string(), true),
        ]
    );
}
