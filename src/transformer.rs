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
    uri: String,
    just_show_message: bool
}
impl GrafanaToHookshotTransformer {
    async fn submit(&self, msg: &str) -> Result<(), String> {
        debug!("TODO: Submit to {} -> {}", self.uri, msg);
        Ok(())
    }

    async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        if request.method() != "POST" && request.method() != "PUT" {
            return Err("Only POST and PUT requests are supported".to_string());
        }

        if let Ok(body) = String::from_utf8(body.to_vec()) {
            if let Ok(body_as_json) = serde_json::from_str::<serde_json::Value>(body.as_str()) {
                if let Some(body_as_json) = body_as_json.as_object() {
                    if self.just_show_message {
                        if let Some(msg) = body_as_json.get("message") {
                            if let Some(msg) = msg.as_str() {
                                self.submit(msg).await
                            } else {
                                Err("The message is not a string".to_string())
                            }
                        } else {
                            Err("The body does not contain a message".to_string())
                        }
                    } else {
                        // TODO
                        Err("TODO: Implement".to_string())
                    }
                } else {
                    Err("The body is not a JSON object".to_string())
                }
            } else {
                Err("Failed to parse the body as JSON: ".to_string())
            }
        } else {
            Err("Failed to parse the body as UTF-8: ".to_string())
        }
    }
}