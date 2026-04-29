use crate::domain::{
    InfographicRequest, LessonStartRequest, NarrationRequest, StagegateRequest, StudentRecord,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct OpenAiClient {
    api_key: Option<String>,
    http: reqwest::Client,
    image_model: String,
    speech_model: String,
    speech_voice: String,
    text_model: String,
}

const OPENAI_SPEECH_INPUT_LIMIT: usize = 4096;
const DEFAULT_NARRATION_INSTRUCTIONS: &str = "Narrate like a warm, curious story tutor reading a real book aloud. Use clear pacing, gentle emphasis, and an encouraging tone for a young learner.";

impl OpenAiClient {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|key| !key.is_empty()),
            http: reqwest::Client::new(),
            image_model: std::env::var("OPENAI_IMAGE_MODEL")
                .unwrap_or_else(|_| "gpt-image-2".to_string()),
            speech_model: std::env::var("OPENAI_TTS_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini-tts".to_string()),
            speech_voice: "fable".to_string(),
            text_model: std::env::var("OPENAI_TEXT_MODEL")
                .unwrap_or_else(|_| "gpt-5.5".to_string()),
        }
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn text_model(&self) -> &str {
        &self.text_model
    }

    pub fn image_model(&self) -> &str {
        &self.image_model
    }

    pub fn speech_model(&self) -> &str {
        &self.speech_model
    }

    pub fn speech_voice(&self) -> &str {
        &self.speech_voice
    }

    pub async fn guide_lesson(
        &self,
        student: &StudentRecord,
        request: &LessonStartRequest,
    ) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Err(
                "OpenAI API key is required to generate adaptive lessons. Set OPENAI_API_KEY in backend/.env.".to_string(),
            );
        };

        let system_prompt = r#"You are PrimerLab, an adaptive educational story tutor.

You decide how to guide the learner, but only use memory and progress for educational personalization.
Keep the lesson age-appropriate, accurate, concise, and stage-gated.
Let the learner choose topics, then shape the path using their memories, current mastery, and level.
If this is the learner's first lesson, use the signup biography as the primary personalization source for the opening topic, examples, story motif, communication style, and stagegate prompt.
Prefer interaction context from the student's question over canned curriculum. Ask the next useful check-for-understanding question inside the lesson.
Do not invent memories. Do not store sensitive personal data.
Return only JSON that matches the provided schema."#;

        let user_payload = json!({
            "student": student,
            "signupBiography": student.biography.clone(),
            "isFirstLesson": student.progress.is_empty(),
            "requestedTopic": request.topic,
            "studentQuestion": request.question,
            "task": "Respond to the student's interaction, choose the right learning level, teach the requested topic through the persistent story world, recommend next topics, create a stagegate prompt, and create an image-generation prompt for an infographic. For the first lesson, ground the story and examples in the signup biography without adding facts that are not present."
        });

        self.responses_json(
            api_key,
            system_prompt,
            user_payload,
            "primer_lesson",
            lesson_schema(),
        )
        .await
        .map(|mut lesson| {
            lesson["aiMode"] = json!("openai_responses");
            lesson["model"] = json!(self.text_model);
            lesson
        })
    }

    pub async fn grade_stagegate(
        &self,
        student: &StudentRecord,
        request: &StagegateRequest,
    ) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Err(
                "OpenAI API key is required to grade stagegates. Set OPENAI_API_KEY in backend/.env."
                    .to_string(),
            );
        };

        let system_prompt = r#"You are PrimerLab's stagegate assessor.

Grade fairly against the rubric. Do not pass vague answers.
Use the learner's memory and progress only to choose helpful feedback.
Return only JSON that matches the provided schema."#;

        let user_payload = json!({
            "student": student,
            "topic": request.topic,
            "stageLevel": request.stage_level.as_deref().unwrap_or("intuition"),
            "answer": request.answer,
            "rubric": {
                "accuracy": "Does the answer state correct facts?",
                "causalReasoning": "Does the answer explain why the process happens?",
                "vocabulary": "Does the answer use important terms correctly?",
                "transfer": "Can the learner apply the idea beyond the exact example?"
            },
            "passingRule": "average_score >= 0.75"
        });

        self.responses_json(
            api_key,
            system_prompt,
            user_payload,
            "stagegate_result",
            stagegate_schema(),
        )
        .await
        .map(|mut result| {
            result["aiMode"] = json!("openai_responses");
            result["model"] = json!(self.text_model);
            result
        })
    }

    pub async fn generate_infographic(
        &self,
        student: &StudentRecord,
        request: &InfographicRequest,
    ) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(json!({
                "aiMode": "missing_openai_api_key",
                "model": self.image_model,
                "generated": false,
                "prompt": build_infographic_prompt(student, request),
                "message": "Set OPENAI_API_KEY in backend/.env to generate a gpt-image-2 infographic."
            }));
        };

        let prompt = build_infographic_prompt(student, request);
        let body = json!({
            "model": self.image_model,
            "prompt": prompt,
            "size": request.size.as_deref().unwrap_or("1024x1024"),
            "n": 1
        });

        let response = self
            .http
            .post("https://api.openai.com/v1/images/generations")
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| format!("OpenAI image request failed: {error}"))?;

        let status = response.status();
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| format!("OpenAI image response was not JSON: {error}"))?;

        if !status.is_success() {
            return Err(format!("OpenAI image API returned {status}: {payload}"));
        }

        let first = payload
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .cloned()
            .unwrap_or(Value::Null);

        let image_data_url = first
            .get("b64_json")
            .and_then(Value::as_str)
            .map(|b64| format!("data:image/png;base64,{b64}"));

        Ok(json!({
            "aiMode": "openai_image_generation",
            "model": self.image_model,
            "generated": true,
            "prompt": prompt,
            "imageDataUrl": image_data_url,
            "imageUrl": first.get("url").and_then(Value::as_str),
            "raw": payload
        }))
    }

    pub async fn generate_narration(&self, request: &NarrationRequest) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(json!({
                "aiMode": "missing_openai_api_key",
                "model": self.speech_model,
                "voice": self.speech_voice,
                "generated": false,
                "message": "Set OPENAI_API_KEY in backend/.env to generate OpenAI TTS narration."
            }));
        };

        let input = trim_speech_input(&request.text);
        if input.is_empty() {
            return Err("Narration text is required.".to_string());
        }

        let instructions = request
            .instructions
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_NARRATION_INSTRUCTIONS);

        let mut body = json!({
            "model": self.speech_model,
            "voice": self.speech_voice,
            "input": input,
            "response_format": "mp3",
            "speed": 0.95
        });

        if self.speech_model.starts_with("gpt-4o-mini-tts") {
            body["instructions"] = json!(instructions);
        }

        let response = self
            .http
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| format!("OpenAI TTS request failed: {error}"))?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();
        let payload = response
            .bytes()
            .await
            .map_err(|error| format!("OpenAI TTS response was not readable: {error}"))?;

        if !status.is_success() {
            return Err(format!(
                "OpenAI TTS API returned {status}: {}",
                String::from_utf8_lossy(&payload)
            ));
        }

        Ok(json!({
            "aiMode": "openai_audio_speech",
            "model": self.speech_model,
            "voice": self.speech_voice,
            "generated": true,
            "topic": request.topic,
            "contentType": content_type,
            "audioDataUrl": format!(
                "data:{content_type};base64,{}",
                BASE64_STANDARD.encode(payload)
            ),
            "inputCharacters": input.chars().count()
        }))
    }

    async fn responses_json(
        &self,
        api_key: &str,
        system_prompt: &str,
        user_payload: Value,
        schema_name: &str,
        schema: Value,
    ) -> Result<Value, String> {
        let body = json!({
            "model": self.text_model,
            "input": [
                {
                    "role": "system",
                    "content": [{"type": "input_text", "text": system_prompt}]
                },
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": user_payload.to_string()}]
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": schema_name,
                    "strict": true,
                    "schema": schema
                }
            }
        });

        let response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| format!("OpenAI Responses request failed: {error}"))?;

        let status = response.status();
        let payload = response
            .json::<Value>()
            .await
            .map_err(|error| format!("OpenAI Responses payload was not JSON: {error}"))?;

        if !status.is_success() {
            return Err(format!("OpenAI Responses API returned {status}: {payload}"));
        }

        let output_text = extract_response_text(&payload)
            .ok_or_else(|| format!("OpenAI Responses payload had no output text: {payload}"))?;

        serde_json::from_str(&output_text)
            .map_err(|error| format!("OpenAI Responses output was not valid JSON: {error}"))
    }
}

fn trim_speech_input(text: &str) -> String {
    text.trim()
        .chars()
        .take(OPENAI_SPEECH_INPUT_LIMIT)
        .collect()
}

pub fn build_infographic_prompt(student: &StudentRecord, request: &InfographicRequest) -> String {
    format!(
        "Create a clear educational infographic for a learner aged {age_band}. Topic: {topic}. Context: {summary}. Style: {style}. Signup biography: {biography}. Use concise labels, accurate diagrams, and a story-world visual motif. Avoid tiny unreadable text. Learner memories: {memories}",
        age_band = student.age_band,
        topic = request.topic,
        summary = request
            .lesson_summary
            .as_deref()
            .or(request.infographic_prompt.as_deref())
            .unwrap_or("explain the concept visually"),
        style = student.preferred_explanation_style,
        biography = student.biography.as_deref().unwrap_or("not provided"),
        memories = student
            .memories
            .iter()
            .map(|memory| memory.content.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    )
}

fn extract_response_text(payload: &Value) -> Option<String> {
    if let Some(text) = payload.get("output_text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    payload
        .get("output")
        .and_then(Value::as_array)?
        .iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find_map(|content| {
            content
                .get("text")
                .and_then(Value::as_str)
                .or_else(|| content.get("output_text").and_then(Value::as_str))
                .map(ToString::to_string)
        })
}

fn lesson_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "topic",
            "stageLevel",
            "communicationStyle",
            "storyScene",
            "plainExplanation",
            "analogy",
            "checkForUnderstanding",
            "suggestedTopics",
            "stagegatePrompt",
            "infographicPrompt",
            "keyTerms"
        ],
        "properties": {
            "topic": { "type": "string" },
            "stageLevel": { "type": "string", "enum": ["intuition", "mechanism", "transfer"] },
            "communicationStyle": { "type": "string" },
            "storyScene": { "type": "string" },
            "plainExplanation": { "type": "string" },
            "analogy": { "type": "string" },
            "checkForUnderstanding": { "type": "string" },
            "suggestedTopics": {
                "type": "array",
                "minItems": 3,
                "maxItems": 5,
                "items": { "type": "string" }
            },
            "stagegatePrompt": { "type": "string" },
            "infographicPrompt": { "type": "string" },
            "keyTerms": {
                "type": "array",
                "minItems": 2,
                "maxItems": 5,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["term", "definition"],
                    "properties": {
                        "term": { "type": "string" },
                        "definition": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn stagegate_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "passed",
            "score",
            "rubric",
            "masteryEvidence",
            "gaps",
            "feedbackToStudent",
            "nextLevelUnlocked",
            "newMemories"
        ],
        "properties": {
            "passed": { "type": "boolean" },
            "score": { "type": "number", "minimum": 0, "maximum": 1 },
            "rubric": {
                "type": "object",
                "additionalProperties": false,
                "required": ["accuracy", "causalReasoning", "vocabulary", "transfer"],
                "properties": {
                    "accuracy": { "type": "number", "minimum": 0, "maximum": 1 },
                    "causalReasoning": { "type": "number", "minimum": 0, "maximum": 1 },
                    "vocabulary": { "type": "number", "minimum": 0, "maximum": 1 },
                    "transfer": { "type": "number", "minimum": 0, "maximum": 1 }
                }
            },
            "masteryEvidence": { "type": "array", "items": { "type": "string" } },
            "gaps": { "type": "array", "items": { "type": "string" } },
            "feedbackToStudent": { "type": "string" },
            "nextLevelUnlocked": {
                "anyOf": [
                    { "type": "string", "enum": ["mechanism", "transfer", "complete"] },
                    { "type": "null" }
                ]
            },
            "newMemories": {
                "type": "array",
                "maxItems": 4,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["memoryType", "content", "confidence", "tags"],
                    "properties": {
                        "memoryType": { "type": "string", "enum": ["preference", "knowledge", "misconception", "interest"] },
                        "content": { "type": "string" },
                        "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
                        "tags": { "type": "array", "items": { "type": "string" } }
                    }
                }
            }
        }
    })
}
