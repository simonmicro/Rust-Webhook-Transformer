use std::{collections::{HashMap, LinkedList}};
use serde::{Serialize, Deserialize};
use actix_web::{route, web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use rust_webhook_transformer::transformer::TransformerConfigTypes;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    transformers: HashMap<String, LinkedList<TransformerConfigTypes>>
}

/// Forward the request to the transformers
#[route("/{id}", method = "GET", method = "POST", method = "PUT")]
async fn forward_to_transformers(config: web::Data<Config>, path: web::Path<String>, request: actix_web::HttpRequest) -> impl Responder {
    let id: String = path.into_inner();
    if config.get_ref().transformers.contains_key(&id) {
        for transformer in config.get_ref().transformers.get(&id).unwrap().iter() {
            transformer.handle(&request).await;
        }
        HttpResponse::Ok().body("OK") // just return 200 OK
    } else {
        return HttpResponse::NotFound().body("Unknown endpoint id");
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        // Set log level to info if not otherwise specified
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Load config (and just panic if it fails)
    let config: Config = serde_yaml::from_str(
            std::fs::read_to_string("config.yaml")
                .expect("Failed to open and read the config file").as_str()
        ).expect("Failed to parse the config file");

    HttpServer::new(move || {
        let logger = Logger::default();

        let mut app = App::new();
        app = app.service(forward_to_transformers);

        app.wrap(logger).app_data(web::Data::new(config.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}