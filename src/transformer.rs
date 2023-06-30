use actix_web::HttpRequest;
use serde::{Serialize, Deserialize};
use log::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformerConfigTypes {
    // Note that, the enum names will be used as YAML tag names
    GrafanaToHookshot(GrafanaToHookshotTransformer)
}

impl TransformerConfigTypes {
    /// Handle the request with the transformer (resolves the enum)
    pub async fn handle(&self, request: &HttpRequest) -> Result<(), &str> {
        match self {
            TransformerConfigTypes::GrafanaToHookshot(inner_transformer) => {
                inner_transformer.handle(&request).await
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaToHookshotTransformer {
    uri: String
}
impl GrafanaToHookshotTransformer {
    async fn handle(&self, req: &HttpRequest) -> Result<(), &str> {
        debug!("TODO: {} -> {:?}", self.uri, req);
        Ok(())
    }
}