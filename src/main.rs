use std::collections::{HashMap, LinkedList};
use serde::{Serialize, Deserialize};
use actix_web::{get, post, put, web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use log::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GrafanaToHookshotConfig {
    uri: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum TransformerConfigTypes {
    GrafanaToHookshot(GrafanaToHookshotConfig)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyConfig {
    transformers: HashMap<String, LinkedList<TransformerConfigTypes>>
}

#[get("/{id}")]
async fn hook_get(config: web::Data<MyConfig>, path: web::Path<String>) -> impl Responder {
    let id: String = path.into_inner();
    if config.get_ref().transformers.contains_key(&id) {
        debug!("Processing id {}", id);
        // TODO
        HttpResponse::Ok().body("OK")
    } else {
        return HttpResponse::NotFound().body("Unknown endpoint id");
    }
}

#[post("/{id}")]
async fn hook_post(config: web::Data<MyConfig>, path: web::Path<String>) -> impl Responder {
    let id: String = path.into_inner();
    if config.get_ref().transformers.contains_key(&id) {
        debug!("Processing id {}", id);
        // TODO
        HttpResponse::Ok().body("OK")
    } else {
        return HttpResponse::NotFound().body("Unknown endpoint id");
    }
}

#[put("/{id}")]
async fn hook_put(config: web::Data<MyConfig>, path: web::Path<String>) -> impl Responder {
    let id: String = path.into_inner();
    if config.get_ref().transformers.contains_key(&id) {
        debug!("Processing id {}", id);
        // TODO
        HttpResponse::Ok().body("OK")
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
    let config: MyConfig = serde_yaml::from_str(
            std::fs::read_to_string("config.yaml")
                .expect("Failed to open and read the config file").as_str()
        ).expect("Failed to parse the config file");

    HttpServer::new(move || {
        let logger = Logger::default();

        // TODO for every configured endpoint, create a route
        // -> https://actix.rs/docs/url-dispatch#resource-configuration

        App::new()
            .wrap(logger)
            .app_data(config.clone())
            .service(hook_get)
            .service(hook_post)
            .service(hook_put)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}