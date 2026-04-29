use poem::{Body, Endpoint, IntoResponse, Request, Response, Result, http::HeaderMap};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub async fn log_request_to_stdout<E>(next: Arc<E>, mut request: Request) -> Result<Response>
where
    E: Endpoint,
{
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let method = request.method().clone();
    let uri = request.original_uri().clone();
    let remote_addr = request.remote_addr().to_string();
    let content_type = request.content_type().map(ToOwned::to_owned);
    let headers = format_headers(request.headers());
    let body = request.take_body().into_vec().await?;
    let body_byte_count = body.len();
    let body_log = format_body(&body, content_type.as_deref());
    request.set_body(Body::from(body));

    println!(
        "[primerlab-api][request:{request_id}] --> {method} {uri} remote={remote_addr} bodyBytes={}",
        body_byte_count
    );
    println!("[primerlab-api][request:{request_id}] headers={headers}");
    println!("[primerlab-api][request:{request_id}] body=\n{body_log}");

    let started_at = Instant::now();
    match next.call(request).await {
        Ok(output) => {
            let response = output.into_response();
            println!(
                "[primerlab-api][request:{request_id}] <-- {method} {uri} status={} durationMs={}",
                response.status().as_u16(),
                started_at.elapsed().as_millis()
            );
            Ok(response)
        }
        Err(error) => {
            println!(
                "[primerlab-api][request:{request_id}] <-- {method} {uri} error={error} durationMs={}",
                started_at.elapsed().as_millis()
            );
            Err(error)
        }
    }
}

fn format_headers(headers: &HeaderMap) -> String {
    let mut grouped_headers: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (name, value) in headers {
        let name = name.as_str().to_string();
        let value = if is_sensitive_header(&name) {
            "[redacted]".to_string()
        } else {
            value
                .to_str()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|_| "<non-utf8 header value>".to_string())
        };

        grouped_headers.entry(name).or_default().push(value);
    }

    let mut json_headers = Map::new();
    for (name, values) in grouped_headers {
        let value = if values.len() == 1 {
            json!(values[0])
        } else {
            json!(values)
        };
        json_headers.insert(name, value);
    }

    serde_json::to_string_pretty(&Value::Object(json_headers)).unwrap_or_else(|_| "{}".to_string())
}

fn format_body(body: &[u8], content_type: Option<&str>) -> String {
    if body.is_empty() {
        return "<empty>".to_string();
    }

    let Ok(text) = std::str::from_utf8(body) else {
        return format!("<{} bytes non-utf8 body>", body.len());
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }

    if is_json_body(content_type, trimmed) {
        if let Ok(mut json_body) = serde_json::from_str::<Value>(trimmed) {
            redact_json_value(&mut json_body);
            return serde_json::to_string_pretty(&json_body)
                .unwrap_or_else(|_| trimmed.to_string());
        }
    }

    text.to_string()
}

fn is_json_body(content_type: Option<&str>, trimmed_body: &str) -> bool {
    content_type
        .map(|value| value.to_ascii_lowercase().contains("json"))
        .unwrap_or(false)
        || trimmed_body.starts_with('{')
        || trimmed_body.starts_with('[')
}

fn redact_json_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if is_sensitive_json_key(key) {
                    *value = json!("[redacted]");
                } else {
                    redact_json_value(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_json_value(value);
            }
        }
        _ => {}
    }
}

fn is_sensitive_header(name: &str) -> bool {
    matches!(
        normalize_name(name).as_str(),
        "authorization"
            | "cookie"
            | "proxyauthorization"
            | "setcookie"
            | "xapikey"
            | "xopenaiapikey"
    )
}

fn is_sensitive_json_key(key: &str) -> bool {
    let key = normalize_name(key);
    key.contains("password")
        || key.contains("token")
        || key.contains("apikey")
        || key.contains("secret")
        || key == "authorization"
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|value| *value != '-' && *value != '_')
        .flat_map(char::to_lowercase)
        .collect()
}
