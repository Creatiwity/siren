//! `position` reprojection: INSEE's "Lambert" coordinates are Lambert 93 in
//! metropolitan France only, overseas territories use their own UTM zone.
//! Samples are real establishments; expected points are the town they sit in.

use diesel::QueryableByName;
use diesel::sql_query;
use diesel::sql_types::{Double, Text};
use diesel_async::RunQueryDsl;

use crate::tests::require_database;

#[derive(QueryableByName)]
struct Point {
    #[diesel(sql_type = Text)]
    code_commune: String,
    #[diesel(sql_type = Double)]
    lon: f64,
    #[diesel(sql_type = Double)]
    lat: f64,
}

#[tokio::test]
async fn position_projetee_selon_le_territoire() {
    let mut connection = require_database!();

    // (commune, x, y, lon, lat)
    let samples = [
        ("75101", "651788.83", "6862459.32", 2.3429, 48.8604), // Paris
        ("97105", "636380.86", "1769229.64", -61.7254, 15.9989), // Basse-Terre
        ("97209", "708269.31", "1615451.04", -61.0666, 14.6043), // Fort-de-France
        ("97302", "352993.22", "546336.38", -52.3259, 4.9414), // Cayenne
        ("97411", "339422.88", "7690009.93", 55.4563, -20.8832), // Saint-Denis
        ("97502", "562244.22", "5179698.06", -56.1848, 46.7679), // Saint-Pierre
        ("97611", "524151.52", "8587955.11", 45.2225, -12.7730), // Mamoudzou
        ("97701", "517459.82", "1979225.74", -62.8352, 17.9009), // Saint-Barthélemy
        ("97801", "490897.81", "1997450.5", -63.0860, 18.0656), // Saint-Martin
    ];

    sql_query("CREATE TEMP TABLE position_test (LIKE etablissement INCLUDING GENERATED)")
        .execute(&mut connection)
        .await
        .expect("creation de position_test");

    for (commune, x, y, _, _) in samples {
        sql_query(
            "INSERT INTO position_test (siren, nic, siret, statut_diffusion, etablissement_siege, \
             etat_administratif, code_commune, coordonnee_lambert_x, coordonnee_lambert_y) \
             VALUES ('000000000', '00000', '00000000000000', 'O', true, 'A', $1, $2, $3)",
        )
        .bind::<Text, _>(commune)
        .bind::<Text, _>(x)
        .bind::<Text, _>(y)
        .execute(&mut connection)
        .await
        .expect("insertion dans position_test");
    }

    let points = sql_query(
        "SELECT code_commune, ST_X(position::geometry) AS lon, ST_Y(position::geometry) AS lat \
         FROM position_test",
    )
    .load::<Point>(&mut connection)
    .await
    .expect("lecture de position_test");

    assert_eq!(points.len(), samples.len());
    for (commune, _, _, lon, lat) in samples {
        let point = points.iter().find(|p| p.code_commune == commune).unwrap();
        assert!(
            (point.lon - lon).abs() < 1e-3 && (point.lat - lat).abs() < 1e-3,
            "{commune} projete en ({}, {}), attendu ({lon}, {lat})",
            point.lon,
            point.lat
        );
    }
}
