use crate::domain::InfographicExplanationRequest;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde_json::{Value, json};
use std::{
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

#[derive(Clone, Debug)]
pub struct VoiceoverIdentity {
    pub cache_key: String,
    pub image_hash: String,
    pub image_length: i64,
}

#[derive(Clone, Debug)]
pub struct SavedVoiceoverFile {
    pub content_type: String,
    pub relative_path: String,
}

const CACHE_KEY_PREFIX: &str = "primerlab-infographic-voiceover:v1";

pub fn voiceover_identity(
    student_id: &str,
    request: &InfographicExplanationRequest,
) -> VoiceoverIdentity {
    let image_hash = stable_hash(&request.image_src);
    let stable_identity = json!({
        "studentId": student_id.trim(),
        "topic": request.topic.trim(),
        "title": request.title.as_deref().map(str::trim).unwrap_or(""),
        "alt": request.alt.as_deref().map(str::trim).unwrap_or(""),
        "imageHash": image_hash,
        "imageLength": request.image_src.len()
    })
    .to_string();
    VoiceoverIdentity {
        cache_key: format!("{CACHE_KEY_PREFIX}:{}", stable_hash(&stable_identity)),
        image_hash,
        image_length: request.image_src.len() as i64,
    }
}

pub async fn write_voiceover_audio(
    student_id: &str,
    cache_key: &str,
    audio_data_url: &str,
) -> Result<SavedVoiceoverFile, String> {
    let (content_type, encoded_audio) = split_audio_data_url(audio_data_url)?;
    let audio = BASE64_STANDARD
        .decode(encoded_audio)
        .map_err(|error| format!("Generated voiceover audio was not valid base64: {error}"))?;
    let relative_path = voiceover_relative_path(student_id, cache_key, &content_type);
    let path = artifact_path(&relative_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("Could not create voiceover directory: {error}"))?;
    }
    fs::write(&path, audio)
        .await
        .map_err(|error| format!("Could not save voiceover audio: {error}"))?;

    Ok(SavedVoiceoverFile {
        content_type,
        relative_path,
    })
}

pub async fn read_voiceover_audio(
    relative_path: &str,
    content_type: &str,
) -> Result<String, String> {
    let path = artifact_path(relative_path)?;
    let audio = fs::read(&path)
        .await
        .map_err(|error| format!("Could not read saved voiceover audio: {error}"))?;
    Ok(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(audio)
    ))
}

pub fn persisted_explanation_payload(
    explanation: &Value,
    saved_file: &SavedVoiceoverFile,
) -> Value {
    let mut payload = explanation.clone();
    if let Some(speech) = payload.get_mut("speech").and_then(Value::as_object_mut) {
        speech.remove("audioDataUrl");
        speech.insert("contentType".to_string(), json!(saved_file.content_type));
    }
    payload["cached"] = json!(false);
    payload["persistedVoiceover"] = json!({
        "saved": true,
        "filePath": saved_file.relative_path,
        "contentType": saved_file.content_type
    });
    payload
}

pub fn response_from_saved_explanation(
    explanation: &Value,
    audio_data_url: String,
    file_path: &str,
    content_type: &str,
) -> Value {
    let mut payload = explanation.clone();
    payload["cached"] = json!(true);
    payload["generated"] = json!(true);
    payload["speechGenerated"] = json!(true);
    payload["persistedVoiceover"] = json!({
        "reused": true,
        "filePath": file_path,
        "contentType": content_type
    });

    if !payload.get("speech").is_some_and(Value::is_object) {
        payload["speech"] = json!({});
    }
    if let Some(speech) = payload.get_mut("speech").and_then(Value::as_object_mut) {
        speech.insert("generated".to_string(), json!(true));
        speech.insert("audioDataUrl".to_string(), json!(audio_data_url));
        speech.insert("contentType".to_string(), json!(content_type));
    }

    payload
}

fn split_audio_data_url(audio_data_url: &str) -> Result<(String, &str), String> {
    let Some(rest) = audio_data_url.strip_prefix("data:") else {
        return Err("Generated voiceover audio was not a data URL.".to_string());
    };
    let Some((metadata, encoded_audio)) = rest.split_once(',') else {
        return Err("Generated voiceover audio data URL was malformed.".to_string());
    };
    let Some(content_type) = metadata.strip_suffix(";base64") else {
        return Err("Generated voiceover audio data URL was not base64 encoded.".to_string());
    };
    if !content_type.starts_with("audio/") {
        return Err("Generated voiceover audio data URL was not audio content.".to_string());
    }

    Ok((content_type.to_string(), encoded_audio))
}

fn voiceover_relative_path(student_id: &str, cache_key: &str, content_type: &str) -> String {
    let extension = match content_type {
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        _ => "bin",
    };
    let safe_cache_key = safe_path_segment(cache_key);
    format!(
        "infographic-voiceovers/{}/{}-{safe_cache_key}.{extension}",
        safe_path_segment(student_id),
        timestamp_millis()
    )
}

fn artifact_path(relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Saved voiceover path was not a safe relative path.".to_string());
    }

    Ok(artifact_root().join(relative))
}

fn artifact_root() -> PathBuf {
    std::env::var("PRIMERLAB_ARTIFACT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("local-artifacts"))
}

fn safe_path_segment(value: &str) -> String {
    let segment = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if segment.is_empty() {
        "student".to_string()
    } else {
        segment
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn stable_hash(value: &str) -> String {
    let mut hash: u32 = 2_166_136_261;
    for byte in value.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    base36(hash)
}

fn base36(mut value: u32) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }

    let mut encoded = Vec::new();
    while value > 0 {
        encoded.push(DIGITS[(value % 36) as usize] as char);
        value /= 36;
    }
    encoded.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explanation_request(image_src: &str) -> InfographicExplanationRequest {
        InfographicExplanationRequest {
            student_id: Some("student-123".to_string()),
            topic: "reef currents".to_string(),
            image_src: image_src.to_string(),
            title: Some("Reef currents".to_string()),
            alt: Some("Generated infographic".to_string()),
            prompt: None,
        }
    }

    #[test]
    fn voiceover_identity_is_stable_and_image_sensitive() {
        let first = voiceover_identity("student-123", &explanation_request("data:image/png,aaa"));
        let second = voiceover_identity("student-123", &explanation_request("data:image/png,aaa"));
        let different =
            voiceover_identity("student-123", &explanation_request("data:image/png,bbb"));

        assert_eq!(first.cache_key, second.cache_key);
        assert_ne!(first.cache_key, different.cache_key);
        assert_eq!(first.image_length, "data:image/png,aaa".len() as i64);
    }

    #[test]
    fn persisted_payload_removes_embedded_audio_but_keeps_file_reference() {
        let saved_file = SavedVoiceoverFile {
            content_type: "audio/mpeg".to_string(),
            relative_path: "infographic-voiceovers/student/audio.mp3".to_string(),
        };
        let payload = persisted_explanation_payload(
            &json!({
                "generated": true,
                "speechGenerated": true,
                "speech": {
                    "generated": true,
                    "audioDataUrl": "data:audio/mpeg;base64,abc"
                }
            }),
            &saved_file,
        );

        assert!(payload["speech"].get("audioDataUrl").is_none());
        assert_eq!(
            payload["persistedVoiceover"]["filePath"],
            "infographic-voiceovers/student/audio.mp3"
        );
    }

    #[test]
    fn saved_response_restores_audio_data_url() {
        let response = response_from_saved_explanation(
            &json!({
                "generated": true,
                "speechGenerated": true,
                "speech": { "generated": true }
            }),
            "data:audio/mpeg;base64,abc".to_string(),
            "infographic-voiceovers/student/audio.mp3",
            "audio/mpeg",
        );

        assert_eq!(response["cached"], true);
        assert_eq!(
            response["speech"]["audioDataUrl"],
            "data:audio/mpeg;base64,abc"
        );
    }
}
