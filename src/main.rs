use std::collections::{HashMap, LinkedList};
use serde::{Serialize, Deserialize};
use actix_web::{get, post, put, web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use log::debug;
use rust_webhook_transformer::transformer::{Transformer, GrafanaToHookshotTranformer, TransformerConfigTypes};

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

        let mut app = App::new();

        for (id, transformers) in config.transformers.iter() {
            debug!("Configuring endpoint {}", id);
            for transformer in transformers.iter() {
                debug!("Configuring transformer {:?}", transformer);
                match transformer {
                    TransformerConfigTypes::GrafanaToHookshot(transformer) => {
                        debug!("Configuring GrafanaToHookshotTransformer");
                        match transformer.route_get() {
                            Some(route) => {
                                let path = "/".to_owned() + id;
                                app = app.route(&path, route)
                            },
                            None => {
                                debug!("Transformer has no GET route");
                            }
                        }
                    }
                }
            }
        }

        app.wrap(logger).app_data(config.clone())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}