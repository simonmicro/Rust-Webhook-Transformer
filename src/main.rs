use std::{collections::{HashMap, LinkedList}};
use serde::{Serialize, Deserialize};
use futures::future;
use actix_web::{route, web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use rust_webhook_transformer::transformer::TransformerConfigTypes;
use log::error;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    continue_on_error: bool,
    transformers: HashMap<String, LinkedList<TransformerConfigTypes>>
}

/// Forward the request to the transformers
#[route("/{id}", method = "GET", method = "POST", method = "PUT")]
async fn forward_to_transformers(config: web::Data<Config>, path: web::Path<String>, request: actix_web::HttpRequest, body: web::Bytes) -> impl Responder {
    let id: String = path.into_inner();
    match config.get_ref().transformers.get(&id) {
        Some(transformers) => {
            let list_of_future_responses: Vec<_> = transformers.iter().map(|transformer| {
                transformer.handle(&request, &body)
            }).map(Box::pin).collect(); // without collect it's a lazy iterator
            let mut err = "".to_string(); // most recent error
            if config.continue_on_error {
                let mut futs = list_of_future_responses;
                while !futs.is_empty() {
                    match future::select_all(futs).await {
                        (Ok(()), _index, remaining) => {
                            futs = remaining;
                        }
                        (Err(_e), _index, remaining) => {
                            error!("Error while handling tranformer for request: {:?}", _e);
                            err = _e;
                            futs = remaining;
                        }
                    }
                }
            } else {
                let results = future::join_all(list_of_future_responses).await;
                for result in results {
                    match result {
                        Ok(()) => {},
                        Err(e) => {
                            // If we reach this, the find_al() call aborted further processing and we can just return the error
                            error!("Error while handling tranformer for request: {:?}", e);
                            err = e;
                            break;
                        }
                    }
                }
            }
            if err.len() == 0 {
                return HttpResponse::Ok().body("OK");
            } else {
                return HttpResponse::InternalServerError().body("Internal server error: ".to_string() + &err.to_string()); // at least one transformer failed
            }
        },
        None => {
            return HttpResponse::NotFound().body("Unknown endpoint id");
        }
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