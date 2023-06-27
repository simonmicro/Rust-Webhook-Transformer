use actix_web::{self, web::{Bytes, self}, HttpResponse};
use serde::{Serialize, Deserialize};

pub trait Transformer {
    fn route_get(&self) -> Option<actix_web::Route>;
    fn route_post(&self) -> Option<actix_web::Route>;
    fn route_put(&self) -> Option<actix_web::Route>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformerConfigTypes {
    GrafanaToHookshot(GrafanaToHookshotTranformer)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaToHookshotTranformer {
    uri: String
}
impl GrafanaToHookshotTranformer {
    // fn handle(&self) -> impl actix_web::Responder {
    //     "OK"
    // }
}

// async fn index(
    // bytes: actix_web::web::Bytes,
// ) -> String {
    // "asd".to_string()
// }

impl Transformer for GrafanaToHookshotTranformer {
    fn route_get(&self) -> Option<actix_web::Route> {
        None
    }
    fn route_post(&self) -> Option<actix_web::Route> {
        let cfg = self.clone();
        let f = move || {
            cfg.uri.len(); // do smth
            "OK"
        };
        let f = move || async {
            f()
        };
        Some(actix_web::web::post().to(move || async {
            f().await
        }))
    }
    fn route_put(&self) -> Option<actix_web::Route> {
        let cfg = self.clone();
        Some(actix_web::web::put().to(move || async {
            cfg.uri.len(); // do smth
            "OK"
        }))
    }
}