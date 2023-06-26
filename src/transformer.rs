use actix_web::HttpRequest;
use serde::{Serialize, Deserialize};
use log::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformerConfigTypes {
    GrafanaToHookshot(GrafanaToHookshotTransformer)
}

impl TransformerConfigTypes {
    /// Handle the request with the transformer (resolves the enum)
    pub fn handle(&self, request: &HttpRequest) {
        match self {
            TransformerConfigTypes::GrafanaToHookshot(inner_transformer) => {
                inner_transformer.handle(&request);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaToHookshotTransformer {
    pub uri: String
}
impl GrafanaToHookshotTransformer {
    pub fn handle(&self, req: &HttpRequest) {
        debug!("TODO: {} -> {:?}", self.uri, req);
    }
}