use actix_web::{web, HttpRequest};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformerConfigTypes {
    // Note that, the enum names will be used as YAML tag names
    GrafanaToHookshot(GrafanaToHookshotTransformer),
    UptimeKumaToHookshot(UptimeKumaToHookshotTransformer),
    GitlabToHookshot(GitlabToHookshotTransformer),
}

impl TransformerConfigTypes {
    /// Handle the request with the transformer (resolves the enum)
    pub async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        match self {
            TransformerConfigTypes::GrafanaToHookshot(inner_transformer) => {
                inner_transformer.handle(&request, &body).await
            }
            TransformerConfigTypes::UptimeKumaToHookshot(inner_transformer) => {
                inner_transformer.handle(&request, &body).await
            }
            TransformerConfigTypes::GitlabToHookshot(inner_transformer) => {
                inner_transformer.handle(&request, &body).await
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct HookshotMessage {
    text: String,             // automatically converted from Markdown to HTML
    html: Option<String>,     // if not provided, the text will be (converted and) used
    username: Option<String>, // will be prepended to the message
}
impl HookshotMessage {
    async fn submit(&self, uri: &str) -> Result<(), String> {
        debug!("Submitting message to Hookshot (via {}): {:#?}", uri, self);
        let client = reqwest::Client::new();
        client
            .post(uri)
            .body(serde_json::to_string(self).map_err(|e| e.to_string())?)
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaToHookshotTransformer {
    uri: String,
    just_show_message: Option<bool>,
}
impl GrafanaToHookshotTransformer {
    async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        if request.method() != "POST" && request.method() != "PUT" {
            return Err("Only POST and PUT requests are supported".to_string());
        }

        let body = String::from_utf8(body.to_vec())
            .map_err(|e| "Failed to parse the body as UTF-8: ".to_string() + &e.to_string())?;
        debug!("Received body: {}", body);

        let body = serde_json::from_str::<serde_json::Value>(body.as_str())
            .map_err(|e| "Failed to parse the body as JSON: ".to_string() + &e.to_string())?;
        let body = body
            .as_object()
            .ok_or("The body is not a JSON object".to_string())?;

        if self.just_show_message.unwrap_or(false) {
            let message = body
                .get("message")
                .ok_or("The body does not contain a message".to_string())?;
            let message = message
                .as_str()
                .ok_or("The message is not a string".to_string())?;
            let message = HookshotMessage {
                text: message.to_string(), // Grafana already sends Markdown
                html: None,
                username: None,
            };
            message.submit(&self.uri).await
        } else {
            // Count how many alerts are raised (and how many are resolved)
            let mut alerts_firing = 0;
            let mut alerts_alerting = 0;
            let mut alerts_resolved = 0;
            let mut alert_list = std::collections::LinkedList::new();
            let alerts = body
                .get("alerts")
                .ok_or("The body does not contain alerts".to_string())?;
            let alerts = alerts
                .as_array()
                .ok_or("The alerts are not an array".to_string())?;
            for alert in alerts {
                // Parse the alert
                let alert = alert
                    .as_object()
                    .ok_or("An alert is not an object".to_string())?;
                let status = alert
                    .get("status")
                    .ok_or("An alert does not have a status".to_string())?;
                let status = status
                    .as_str()
                    .ok_or("An alert's status is not a string".to_string())?;
                // Count the alert
                match status {
                    "firing" => alerts_firing += 1,
                    "alerting" => alerts_alerting += 1,
                    "resolved" => alerts_resolved += 1,
                    _ => debug!("Unknown alert status: {}", status),
                }
                // Parse the alert further
                let labels = alert
                    .get("labels")
                    .ok_or("An alert does not have labels".to_string())?;
                let labels = labels
                    .as_object()
                    .ok_or("An alert's labels are not an object".to_string())?;

                let alertname = labels
                    .get("alertname")
                    .ok_or("An alert does not have a alertname in its labels".to_string())?;
                let alertname = alertname
                    .as_str()
                    .ok_or("An alert's alertname in its labels is not a string".to_string())?;

                let instance = labels.get("instance").map(|v| v.as_str()).flatten();

                let annotations = alert
                    .get("annotations")
                    .ok_or("An alert does not have annotations".to_string())?;
                let annotations = annotations
                    .as_object()
                    .ok_or("An alert's annotations are not an object".to_string())?;

                let summary = annotations.get("summary").map(|v| v.as_str()).flatten();
                let description = annotations.get("description").map(|v| v.as_str()).flatten();

                // TODO values?

                let silence_url = alert
                    .get("silenceURL")
                    .map(|v| {
                        v.as_str()
                            .map(|v| if v.len() > 0 { Some(v) } else { None })
                            .flatten()
                    })
                    .flatten();
                let panel_url = alert
                    .get("panelURL")
                    .map(|v| {
                        v.as_str()
                            .map(|v| if v.len() > 0 { Some(v) } else { None })
                            .flatten()
                    })
                    .flatten();
                let dashboard_url = alert
                    .get("dashboardURL")
                    .map(|v| {
                        v.as_str()
                            .map(|v| if v.len() > 0 { Some(v) } else { None })
                            .flatten()
                    })
                    .flatten();
                let actions = {
                    let mut actions = std::collections::LinkedList::new();
                    if dashboard_url.is_some() {
                        actions.push_back(format!(
                            "<a href=\"{}\">dashboard</a>",
                            dashboard_url.unwrap()
                        ));
                    }
                    if panel_url.is_some() {
                        actions.push_back(format!("<a href=\"{}\">panel</a>", panel_url.unwrap()));
                    }
                    if silence_url.is_some() {
                        actions
                            .push_back(format!("<a href=\"{}\">silence</a>", silence_url.unwrap()));
                    }
                    if actions.len() > 0 {
                        let mut actions_str = "→ ".to_string() + &actions.front().unwrap();
                        for line in actions.iter().skip(1) {
                            actions_str += ", ";
                            actions_str += line;
                        }
                        Some(actions_str)
                    } else {
                        None
                    }
                };

                // Create the alert string
                let mut as_multiline_str = std::collections::LinkedList::new();
                as_multiline_str.push_back(format!(
                    "{} <b>{}</b>{}{}",
                    match status {
                        "firing" => {
                            "🔴"
                        }
                        "alerting" => {
                            "🟡"
                        }
                        "resolved" => {
                            "🟢"
                        }
                        _ => {
                            "⚪"
                        }
                    },
                    alertname,
                    if instance.is_some() {
                        format!(" at <code>{}</code>", instance.unwrap())
                    } else {
                        "".to_string()
                    },
                    if summary.is_some() {
                        format!(": {}", summary.unwrap())
                    } else {
                        "".to_string()
                    }
                ));
                // Add description
                if description.is_some() {
                    as_multiline_str.push_back(description.unwrap().to_string());
                }
                // Add actions
                if actions.is_some() {
                    as_multiline_str.push_back(actions.unwrap());
                }
                alert_list.push_back(as_multiline_str);
            }
            // Create the message (title)
            let title = if alerts_firing > 0 {
                format!(
                    "🚨 {} alert{} firing{}",
                    alerts_firing,
                    if alerts_firing == 1 { " is" } else { "s are" },
                    if alerts_alerting > 0 || alerts_resolved > 0 {
                        format!(
                            " ({}{}{})",
                            if alerts_alerting > 0 {
                                format!("{} pending", alerts_alerting)
                            } else {
                                "".to_string()
                            },
                            if alerts_alerting > 0 && alerts_resolved > 0 {
                                " and ".to_string()
                            } else {
                                "".to_string()
                            },
                            if alerts_resolved > 0 {
                                format!("{} resolved", alerts_resolved)
                            } else {
                                "".to_string()
                            }
                        )
                    } else {
                        "".to_string()
                    }
                )
            } else if alerts_alerting > 0 {
                format!(
                    "⚠️ {} alert{} pending{}...",
                    alerts_alerting,
                    if alerts_alerting == 1 { " is" } else { "s are" },
                    if alerts_resolved > 0 {
                        format!(" ({} resolved)", alerts_resolved)
                    } else {
                        "".to_string()
                    }
                )
            } else {
                "✅ All alerts are resolved!".to_string()
            };
            let mut message_html = format!("<h3>{}</h3>", title);
            // Append the alerts
            for alert in alert_list {
                if alert.len() == 0 {
                    warn!("An alert is empty?! This is a bug!");
                    continue;
                }
                message_html += "<p>";
                message_html += alert.front().unwrap();
                for line in alert.iter().skip(1) {
                    message_html += "<br>";
                    message_html += line;
                }
                message_html += "</p>";
            }
            // Final message
            let message = HookshotMessage {
                text: title,
                html: Some(message_html),
                username: None,
            };
            message.submit(&self.uri).await
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeKumaToHookshotTransformer {
    uri: String,
    just_show_message: Option<bool>,
}
impl UptimeKumaToHookshotTransformer {
    async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        if request.method() != "POST" {
            return Err("Only POST requests are supported".to_string());
        }

        let body = String::from_utf8(body.to_vec())
            .map_err(|e| "Failed to parse the body as UTF-8: ".to_string() + &e.to_string())?;
        debug!("Received body: {}", body);

        let body = serde_json::from_str::<serde_json::Value>(body.as_str())
            .map_err(|e| "Failed to parse the body as JSON: ".to_string() + &e.to_string())?;
        let body = body
            .as_object()
            .ok_or("The body is not a JSON object".to_string())?;

        if self.just_show_message.unwrap_or(false) {
            let message = body
                .get("msg")
                .ok_or("The body does not contain a msg".to_string())?;
            let message = message
                .as_str()
                .ok_or("The msg is not a string".to_string())?;
            let message = HookshotMessage {
                text: message.to_string(), // UptimeKuma not uses Markdown, but fany emojis
                html: None,
                username: None,
            };
            message.submit(&self.uri).await
        } else {
            let heartbeat = body
                .get("heartbeat")
                .ok_or("The body does not contain a heartbeat".to_string())?;

            let monitor = body
                .get("monitor")
                .ok_or("The body does not contain a monitor".to_string())?;
            let name = monitor
                .get("name")
                .ok_or("The monitor does not contain a name".to_string())?;
            let name = name
                .as_str()
                .ok_or("The name is not a string".to_string())?;
            let message = heartbeat
                .get("msg")
                .ok_or("The heartbeat does not contain a msg".to_string())?;
            let message = message
                .as_str()
                .ok_or("The msg is not a string".to_string())?;
            let monitor_msg = body.get("msg");
            let mut is_up = None;
            if let Some(monitor_msg) = monitor_msg {
                let monitor_msg = monitor_msg.as_str().unwrap_or(""); // if this is not a string, treat it as empty
                if monitor_msg.contains("[✅ ") {
                    is_up = Some(true);
                } else if monitor_msg.contains("[🔴 ") {
                    is_up = Some(false);
                } else if monitor_msg.contains("Up]") {
                    // well, try that again with a little bit more fuzzy matching
                    is_up = Some(true);
                } else if monitor_msg.contains("Down]") {
                    // well, try that again with a little bit more fuzzy matching
                    is_up = Some(false);
                }
            }

            let message_html = format!(
                "<p>{} <b>{}</b>: {}</p>",
                match is_up {
                    Some(true) => "🟢",
                    Some(false) => "🔴",
                    None => "⚪", // ouch, we don't know if it's up or down
                },
                name,
                message
            );
            let message = HookshotMessage {
                text: message.to_string(), // UptimeKuma not uses Markdown, but fany emojis
                html: Some(message_html),
                username: None,
            };
            message.submit(&self.uri).await
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabToHookshotTransformer {
    uri: String,
}
impl GitlabToHookshotTransformer {
    async fn handle(&self, request: &HttpRequest, body: &web::Bytes) -> Result<(), String> {
        if request.method() != "POST" {
            return Err("Only POST requests are supported".to_string());
        }

        let body = String::from_utf8(body.to_vec())
            .map_err(|e| "Failed to parse the body as UTF-8: ".to_string() + &e.to_string())?;
        debug!("Received body: {}", body);

        let body = serde_json::from_str::<serde_json::Value>(body.as_str())
            .map_err(|e| "Failed to parse the body as JSON: ".to_string() + &e.to_string())?;
        let body = body
            .as_object()
            .ok_or("The body is not a JSON object".to_string())?;

        let object_kind = body
            .get("object_kind")
            .ok_or("The body does not contain an object_kind".to_string())?;
        let object_kind = object_kind
            .as_str()
            .ok_or("The object_kind is not a string".to_string())?;

        match object_kind {
            "push" => {
                let project = body
                    .get("project")
                    .ok_or("The body does not contain a project".to_string())?;
                let project = project
                    .as_object()
                    .ok_or("The project is not a JSON object".to_string())?;
                let project_name = project
                    .get("name")
                    .ok_or("The project does not contain a name".to_string())?;
                let project_name = project_name
                    .as_str()
                    .ok_or("The name is not a string".to_string())?;
                let project_url = project
                    .get("web_url")
                    .ok_or("The project does not contain a web_url".to_string())?;
                let project_url = project_url
                    .as_str()
                    .ok_or("The web_url is not a string".to_string())?;
                let user = body
                    .get("user_name")
                    .ok_or("The body does not contain a user_name".to_string())?;
                let user = user
                    .as_str()
                    .ok_or("The user_name is not a string".to_string())?;
                let commits = body
                    .get("commits")
                    .ok_or("The body does not contain a commits".to_string())?;
                let commits = commits
                    .as_array()
                    .ok_or("The commits is not an array".to_string())?;
                let message = format!(
                    "{} pushed {} commit{} to {}",
                    user,
                    commits.len(),
                    if commits.len() == 1 { "" } else { "s" },
                    project_name
                );
                let mut message_html = format!(
                    "<h3>{} pushed {} commit{} to <a href=\"{}\">{}</a></h3>",
                    user,
                    commits.len(),
                    if commits.len() == 1 { "" } else { "s" },
                    project_url,
                    project_name
                );
                for commit in commits {
                    let commit = commit
                        .as_object()
                        .ok_or("A commit is not a JSON object".to_string())?;
                    let commit_id = commit
                        .get("id")
                        .ok_or("A commit does not contain an id".to_string())?;
                    let commit_id = commit_id
                        .as_str()
                        .ok_or("The id is not a string".to_string())?;
                    let commit_url = commit
                        .get("url")
                        .ok_or("A commit does not contain an url".to_string())?;
                    let commit_url = commit_url
                        .as_str()
                        .ok_or("The url is not a string".to_string())?;
                    let commit_message = commit
                        .get("message")
                        .ok_or("A commit does not contain a message".to_string())?;
                    let commit_message = commit_message
                        .as_str()
                        .ok_or("The message is not a string".to_string())?;
                    message_html += &format!(
                        "<a href=\"{}\"><code>{}</code></a> {}<br>",
                        commit_url,
                        commit_id[0..8].to_string(),
                        commit_message
                    );
                }
                let message = HookshotMessage {
                    text: message,
                    html: Some(message_html),
                    username: None,
                };
                message.submit(&self.uri).await
            }
            "tag_push" => {
                let project = body
                    .get("project")
                    .ok_or("The body does not contain a project".to_string())?;
                let project = project
                    .as_object()
                    .ok_or("The project is not a JSON object".to_string())?;
                let project_name = project
                    .get("name")
                    .ok_or("The project does not contain a name".to_string())?;
                let project_name = project_name
                    .as_str()
                    .ok_or("The name is not a string".to_string())?;
                let project_url = project
                    .get("web_url")
                    .ok_or("The project does not contain a web_url".to_string())?;
                let project_url = project_url
                    .as_str()
                    .ok_or("The web_url is not a string".to_string())?;
                let user = body
                    .get("user_name")
                    .ok_or("The body does not contain a user_name".to_string())?;
                let user = user
                    .as_str()
                    .ok_or("The user_name is not a string".to_string())?;
                let message = format!("{} pushed a tag to {}", user, project_name);
                let message_html = format!(
                    "<h3>{} pushed a tag to <a href=\"{}\">{}</a></h3>",
                    user, project_url, project_name
                );
                let message = HookshotMessage {
                    text: message,
                    html: Some(message_html),
                    username: None,
                };
                message.submit(&self.uri).await
            }
            "pipeline" => {
                let project = body
                    .get("project")
                    .ok_or("The body does not contain a project".to_string())?;
                let project = project
                    .as_object()
                    .ok_or("The project is not a JSON object".to_string())?;
                let project_name = project
                    .get("name")
                    .ok_or("The project does not contain a name".to_string())?;
                let project_name = project_name
                    .as_str()
                    .ok_or("The name is not a string".to_string())?;
                let project_url = project
                    .get("web_url")
                    .ok_or("The project does not contain a web_url".to_string())?;
                let project_url = project_url
                    .as_str()
                    .ok_or("The web_url is not a string".to_string())?;
                let pipeline = body
                    .get("object_attributes")
                    .ok_or("The body does not contain object_attributes".to_string())?;
                let pipeline = pipeline
                    .as_object()
                    .ok_or("The object_attributes is not a JSON object".to_string())?;
                let pipeline_id = pipeline
                    .get("id")
                    .ok_or("The pipeline does not contain an id".to_string())?;
                let pipeline_id = pipeline_id
                    .as_u64()
                    .ok_or("The id is not an integer".to_string())?;
                let pipeline_status = pipeline
                    .get("status")
                    .ok_or("The pipeline does not contain a status".to_string())?;
                let pipeline_status = pipeline_status
                    .as_str()
                    .ok_or("The status is not a string".to_string())?;
                let pipeline_url = pipeline
                    .get("url")
                    .ok_or("The pipeline does not contain an url".to_string())?;
                let pipeline_url = pipeline_url
                    .as_str()
                    .ok_or("The url is not a string".to_string())?;
                let message = format!(
                    "Pipeline #{} {} for {}",
                    pipeline_id, pipeline_status, project_name
                );
                let message_html = format!(
                    "<h3>Pipeline <a href=\"{}\">#{}</a> {} for <a href=\"{}\">{}</a></h3>",
                    pipeline_url, pipeline_id, pipeline_status, project_url, project_name
                );
                let message = HookshotMessage {
                    text: message,
                    html: Some(message_html),
                    username: None,
                };
                message.submit(&self.uri).await
            }
            other => Err(format!("Unsupported object_kind: {}", other)),
        }
    }
}
