use poem::{
    Body, Endpoint, IntoResponse, Request, Response, Result,
    http::{Method, Uri},
};
use serde_json::{Map, Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub async fn log_request_to_stdout<E>(next: Arc<E>, mut request: Request) -> Result<Response>
where
    E: Endpoint,
{
    if request.method() == Method::OPTIONS {
        return next.call(request).await.map(IntoResponse::into_response);
    }

    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let method = request.method().clone();
    let uri = request.original_uri().clone();
    let function_name = function_name_for_request(&method, uri.path());
    let content_type = request.content_type().map(ToOwned::to_owned);
    let body = request.take_body().into_vec().await?;
    let inputs = format_inputs(&body, content_type.as_deref(), &uri);
    request.set_body(Body::from(body));

    println!("[primerlab-api][request:{request_id}] {function_name} inputs=\n{inputs}");

    next.call(request).await.map(IntoResponse::into_response)
}

fn function_name_for_request(method: &Method, path: &str) -> String {
    match (method.as_str(), path) {
        ("GET", "/health") => "health".to_string(),
        ("POST", "/api/auth/register") => "register".to_string(),
        ("POST", "/api/auth/login") => "login".to_string(),
        ("GET", "/api/students") => "list_students".to_string(),
        ("POST", "/api/lesson/start") => "start_lesson".to_string(),
        ("POST", "/api/tutor/respond") => "tutor_respond".to_string(),
        ("POST", "/api/artifact/infographic") => "infographic".to_string(),
        ("POST", "/api/narration/speech") => "narration_speech".to_string(),
        ("POST", "/api/tutor/stagegate") => "stagegate".to_string(),
        ("POST", "/api/memory/profile") => "memory_profile".to_string(),
        ("POST", "/api/memory/graph") => "memory_graph".to_string(),
        ("GET", path) if path.strip_prefix("/api/students/").is_some() => "get_student".to_string(),
        _ => format!("unmatched_route {} {}", method.as_str(), path),
    }
}

fn format_inputs(body: &[u8], content_type: Option<&str>, uri: &Uri) -> String {
    if !body.is_empty() {
        return format_body(body, content_type);
    }

    let mut inputs = Map::new();
    if let Some(student_id) = uri
        .path()
        .strip_prefix("/api/students/")
        .filter(|value| !value.is_empty() && !value.contains('/'))
    {
        inputs.insert("studentId".to_string(), json!(student_id));
    }
    if let Some(query) = uri.query().filter(|value| !value.is_empty()) {
        inputs.insert("query".to_string(), json!(query));
    }

    if inputs.is_empty() {
        "<empty>".to_string()
    } else {
        serde_json::to_string_pretty(&Value::Object(inputs)).unwrap_or_else(|_| "{}".to_string())
    }
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
