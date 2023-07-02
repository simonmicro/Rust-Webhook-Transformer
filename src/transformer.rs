use actix_web::{HttpRequest, web};
use serde::{Serialize, Deserialize};
use serde_json;
use log::{debug, warn};

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
        debug!("Received body: {}", body);

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
            // Count how many alerts are raised (and how many are resolved)
            let mut alerts_firing = 0;
            let mut alerts_alerting = 0;
            let mut alerts_resolved = 0;
            let mut alert_list = std::collections::LinkedList::new();
            let alerts = body.get("alerts").ok_or("The body does not contain alerts".to_string())?;
            let alerts = alerts.as_array().ok_or("The alerts are not an array".to_string())?;
            for alert in alerts {
                // Parse the alert
                let alert = alert.as_object().ok_or("An alert is not an object".to_string())?;
                let status = alert.get("status").ok_or("An alert does not have a status".to_string())?;
                let status = status.as_str().ok_or("An alert's status is not a string".to_string())?;
                // Count the alert
                match status {
                    "firing" => alerts_firing += 1,
                    "alerting" => alerts_alerting += 1,
                    "resolved" => alerts_resolved += 1,
                    _ => debug!("Unknown alert status: {}", status)
                }
                // Parse the alert further
                let labels = alert.get("labels").ok_or("An alert does not have labels".to_string())?;
                let labels = labels.as_object().ok_or("An alert's labels are not an object".to_string())?;
                
                let alertname = labels.get("alertname").ok_or("An alert does not have a alertname in its labels".to_string())?;
                let alertname = alertname.as_str().ok_or("An alert's alertname in its labels is not a string".to_string())?;
                
                let instance = labels.get("instance").ok_or("An alert does not have a instance in its labels".to_string())?;
                let instance = instance.as_str().ok_or("An alert's instance in its labels is not a string".to_string())?;

                let annotations = alert.get("annotations").ok_or("An alert does not have annotations".to_string())?;
                let annotations = annotations.as_object().ok_or("An alert's annotations are not an object".to_string())?;
                
                let summary = annotations.get("summary").map(|v| v.as_str().map(|v| if v.len() > 0 {Some(v)} else {None} ).flatten()).flatten();
                
                // TODO description optional

                // TODO values?

                let silence_url = alert.get("silenceURL").map(|v| v.as_str().map(|v| if v.len() > 0 {Some(v)} else {None} ).flatten()).flatten();
                let panel_url = alert.get("panelURL").map(|v| v.as_str().map(|v| if v.len() > 0 {Some(v)} else {None} ).flatten()).flatten();
                let dashboard_url = alert.get("dashboardURL").map(|v| v.as_str().map(|v| if v.len() > 0 {Some(v)} else {None} ).flatten()).flatten();
                let actions = if dashboard_url.is_some() || panel_url.is_some() || silence_url.is_some() {
                    let mut actions = std::collections::LinkedList::new();
                    if dashboard_url.is_some() {
                        actions.push_back(format!("[dashboard]({})", dashboard_url.unwrap()));
                    }
                    if panel_url.is_some() {
                        actions.push_back(format!("[panel]({})", panel_url.unwrap()));
                    }
                    if silence_url.is_some() {
                        actions.push_back(format!("[silence]({})", silence_url.unwrap()));
                    }
                    let mut actions_str = " →".to_string();
                    for action in actions {
                        if actions_str.len() > 0 {
                            actions_str += " ";
                        }
                        actions_str += &action;
                    }
                    Some(actions_str)
                } else {
                    None
                };

                // Create the alert string
                let mut as_multiline_str = std::collections::LinkedList::new();
                as_multiline_str.push_back(format!("{} **{}**@{}{}",
                    match status {
                        "firing" => {"🔴"},
                        "alerting" => {"🟡"},
                        "resolved" => {"🟢"},
                        _ => {"⚪"}
                    },
                    alertname,
                    instance,
                    if summary.is_some() {format!(": {}", summary.unwrap())} else {"".to_string()}
                ));
                // Add actions
                if actions.is_some() {
                    as_multiline_str.push_back(actions.unwrap());
                }
                alert_list.push_back(as_multiline_str);
            }
            // Create the message (title)
            let mut message;
            if alerts_firing == 0 {
                message = "**All alerts are resolved**\n".to_string();
            } else {
                message = format!("**{} alert{} firing ({} pending, {} resolved)**\n", alerts_firing, if alerts_firing == 1 { "" } else { "s" }, alerts_alerting, alerts_resolved);
            }
            // Append the alerts
            for alert in alert_list {
                if alert.len() == 0 {
                    warn!("An alert is empty?! This is a bug!");
                    continue;
                }
                message += "\n - ";
                message += alert.front().unwrap();
                for line in alert.iter().skip(1) {
                    message += "\n";
                    message += line;
                }
            }
            message += "\n\n";
            // Final message
            let message = HookshotMessage {
                text: message,
                html: None,
                username: None
            };
            self.submit(&message).await
        }
    }
}