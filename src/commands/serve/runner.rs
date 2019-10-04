use rocket::config::Config;
use rocket_contrib::json::Json;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Point {
    x: i32,
    y: i32,
    siren: String,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/unites_legales/<siren>")]
fn unites_legales(siren: String) -> Json<Point> {
    Json(Point {
        x: 2,
        y: 4,
        siren: siren,
    })
}

pub fn run(config: Config) {
    rocket::custom(config)
        .mount("/", routes![index, unites_legales])
        .launch();
}
