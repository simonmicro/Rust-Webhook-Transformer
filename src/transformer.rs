use actix_web::{HttpRequest, web};
use serde::{Serialize, Deserialize};
use serde_json;
use log::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformerConfigTypes {
    // Note that, the enum names will be used as YAML tag names
    GrafanaToHookshot(GrafanaToHookshotTransformer)
}

impl TransformerConfigTypes {
    /// Handle the request with the transformer (resolves the enum)
    pub async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        match self {
            TransformerConfigTypes::GrafanaToHookshot(inner_transformer) => {
                inner_transformer.handle(&request, &body).await
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaToHookshotTransformer {
    uri: String
}
impl GrafanaToHookshotTransformer {
    async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        if request.method() != "POST" && request.method() != "PUT" {
            return Err("Only POST and PUT requests are supported".to_string());
        }
        // Parse the body as JSON
        match String::from_utf8(body.to_vec()) {
            Ok(body_as_string) => {
                match serde_json::from_str::<serde_json::Value>(body_as_string.as_str()) {
                    Ok(body_as_json) => {
                        debug!("TODO: {} -> {:?}", self.uri, body_as_json);
                        Ok(())
                    },
                    Err(e) => {
                        Err("Failed to parse the body as JSON: ".to_string() + &e.to_string())
                    }
                }
            },
            Err(e) => {
                Err("Failed to parse the body as UTF-8: ".to_string() + &e.to_string())
            }
        }
    }
}