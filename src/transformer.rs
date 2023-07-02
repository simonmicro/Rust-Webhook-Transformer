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

#[derive(Debug, Serialize, Deserialize)]
struct HookshotMessage {
    text: String, // automatically converted from Markdown to HTML
    html: Option<String>, // if not provided, the text will be (converted and) used
    username: Option<String>, // will be prepended to the message
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaToHookshotTransformer {
    uri: String,
    just_show_message: Option<bool>
}
impl GrafanaToHookshotTransformer {
    async fn submit(&self, msg: &HookshotMessage) -> Result<(), String> {
        debug!("Submitting message to Hookshot (via {}): {:#?}", self.uri, msg);
        let client = reqwest::Client::new();
        client.post(&self.uri)
            .body(serde_json::to_string(msg).map_err(|e| e.to_string())?)
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        if request.method() != "POST" && request.method() != "PUT" {
            return Err("Only POST and PUT requests are supported".to_string());
        }

        let body = String::from_utf8(body.to_vec()).map_err(|e| "Failed to parse the body as UTF-8: ".to_string() + &e.to_string())?;
        let body = serde_json::from_str::<serde_json::Value>(body.as_str()).map_err(|e| "Failed to parse the body as JSON: ".to_string() + &e.to_string())?;
        let body = body.as_object().ok_or("The body is not a JSON object".to_string())?;

        if self.just_show_message.unwrap_or(false) {
            let message = body.get("message").ok_or("The body does not contain a message".to_string())?;
            let message = message.as_str().ok_or("The message is not a string".to_string())?;
            let message = HookshotMessage {
                text: message.to_string(), // Grafana already sends Markdown
                html: None,
                username: None
            };
            self.submit(&message).await
        } else {
            // TODO
            Err("TODO: Implement".to_string())
        }
    }
}