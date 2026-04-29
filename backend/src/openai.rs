use crate::domain::{
    InfographicRequest, LessonStartRequest, NarrationRequest, NarrativeCharacter, StagegateRequest,
    StudentRecord,
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

    #[cfg(test)]
    pub(crate) fn for_tests(api_key: Option<&str>) -> Self {
        Self {
            api_key: api_key.map(ToString::to_string),
            http: reqwest::Client::new(),
            image_model: "gpt-image-2".to_string(),
            speech_model: "gpt-4o-mini-tts".to_string(),
            speech_voice: "fable".to_string(),
            text_model: "gpt-5.5".to_string(),
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
        narrative_characters: &[NarrativeCharacter],
        request: &LessonStartRequest,
    ) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(profile_bootstrap_lesson(
                student,
                narrative_characters,
                request,
                "missing_openai_api_key",
                &self.text_model,
            ));
        };

        let system_prompt = r#"You are Primer, an adaptive educational story tutor.

You decide how to guide the learner, but only use memory and progress for educational personalization.
Keep the lesson age-appropriate, accurate, concise, and stage-gated.
If the user did not request a concrete topic, choose the most engaging starting point from the signup biography, interests, age band, memories, and progress.
If this is the learner's first lesson, use the signup biography as the primary personalization source for the opening topic, examples, story motif, communication style, and stagegate prompt.
When you choose the opening topic, set `topic` to a concrete concept or question that can be taught now; do not return a vague label like "personalized lesson".
Write student-facing text in a natural tutor voice, not as app narration. Do not say that Primer, the book, the page, the lesson, or the system performs an action.
For storyScene, plainExplanation, analogy, checkForUnderstanding, communicationStyle, stagegatePrompt, and infographicPrompt, make the concept, learner, evidence, diagram, or story character the grammatical actor.
Avoid meta headings or labels such as "the Primer adjusts", "the book reads", "the page chooses", "AI follow-up", or "generated path". If a phrase sounds like product UI copy, rewrite it as topic guidance for the student.
Use supplied narrativeCharacters when they fit the topic. Preserve their names, roles, biography facts, voice, relationships, and visual motifs exactly unless the lesson itself adds a new compatible detail.
Prefer reusing a relevant existing character over introducing a new one. Only introduce or update characters that appear in storyScene.
Return narrativeCharacters as the complete list of characters used or materially updated by this lesson. Do not include the learner as a narrative character.
Prefer interaction context from the student's question over canned curriculum. Ask the next useful check-for-understanding question inside the lesson.
Do not invent memories. Do not store sensitive personal data.
Return only JSON that matches the provided schema."#;

        let requested_topic = clean_optional(request.topic.as_deref());
        let is_first_lesson = student.progress.is_empty();
        let should_choose_starting_point = requested_topic.is_none() || is_first_lesson;
        let user_payload = json!({
            "student": student,
            "signupBiography": student.biography.clone(),
            "narrativeCharacters": narrative_characters,
            "characterPolicy": {
                "reuseRule": "Use relevant stored character biographies to keep recurring story characters consistent across lessons.",
                "updateRule": "When a character appears, return an updated biography that preserves stable facts and adds only lesson-derived changes.",
                "retrievalHint": "Choose characters whose topicAffinities, role, currentBiography, or lastSeenTopic fit the requested topic."
            },
            "isFirstLesson": is_first_lesson,
            "shouldChooseStartingPointFromProfile": should_choose_starting_point,
            "requestedTopic": requested_topic,
            "studentQuestion": request.question,
            "task": "Create the student's opening engagement path. If requestedTopic is absent or this is the first lesson, select the best starting concept from the signup biography and interests. Teach through a coherent story frame, recommend next topics, create a stagegate prompt, and create an image-generation prompt for an infographic. Ground personalization in supplied profile facts only."
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

        let system_prompt = r#"You are Primer's stagegate assessor.

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

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn profile_bootstrap_lesson(
    student: &StudentRecord,
    narrative_characters: &[NarrativeCharacter],
    request: &LessonStartRequest,
    ai_mode: &str,
    model: &str,
) -> Value {
    let topic = starter_topic(student, request);
    let anchor = starter_anchor(student);
    let biography = student
        .biography
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("the learner's signup profile");
    let suggestions = suggested_starter_topics(student, &topic);
    let character_scene = narrative_characters
        .first()
        .map(|character| {
            let role = character
                .role
                .as_deref()
                .unwrap_or("recurring story guide");
            format!(
                "{character_name}, the established {role}, brings a familiar cause-map approach: {biography}",
                character_name = character.name.as_str(),
                role = role,
                biography = character.current_biography.as_str()
            )
        })
        .unwrap_or_else(|| {
            format!(
                "{name}'s signup profile points toward a starting idea they already care about.",
                name = student.display_name
            )
        });
    let used_characters = narrative_characters
        .first()
        .map(|character| {
            json!({
                "name": character.name.clone(),
                "role": character.role.as_deref().unwrap_or("recurring story guide"),
                "biography": character.current_biography.clone(),
                "topicAffinities": character.topic_affinities.clone(),
                "consistencyNotes": character.consistency_notes.clone(),
                "usedInScene": true,
                "revisionNote": "Reused by the local fallback lesson generator without changing established biography."
            })
        })
        .into_iter()
        .collect::<Vec<_>>();

    json!({
        "topic": topic,
        "stageLevel": "intuition",
        "communicationStyle": format!(
            "Visual, story-first explanations anchored in {anchor}, with short checks before adding detail."
        ),
        "storyScene": format!(
            "{character_scene} Start with {anchor}, then turn one visible pattern from that profile detail into a question that can be tested, sketched, and explained."
        ),
        "plainExplanation": format!(
            "A useful first lesson starts with something the learner already cares about. From the biography: {biography}. The core move is to notice what changes, what stays the same, and what cause might explain the pattern."
        ),
        "analogy": format!(
            "Treat {anchor} like a map: first notice landmarks, then draw arrows between causes and effects, then test whether the arrows explain what happens next."
        ),
        "checkForUnderstanding": format!(
            "What is one pattern you notice in {anchor}, and what cause might explain it?"
        ),
        "suggestedTopics": suggestions,
        "stagegatePrompt": format!(
            "Explain the core idea behind {anchor} in your own words, then connect it to one detail from your biography or interests."
        ),
        "infographicPrompt": format!(
            "Create an age-appropriate infographic for {name} about {anchor}. Use only profile-grounded motifs, clear labels, a simple cause-and-effect flow, and no tiny text.",
            name = student.display_name
        ),
        "keyTerms": [
            {
                "term": "Observation",
                "definition": "Something you can notice, describe, or measure."
            },
            {
                "term": "Pattern",
                "definition": "A repeated relationship that helps predict what may happen next."
            },
            {
                "term": "Model",
                "definition": "A simplified explanation of how causes connect to effects."
            }
        ],
        "narrativeCharacters": used_characters,
        "aiMode": ai_mode,
        "model": model
    })
}

fn starter_topic(student: &StudentRecord, request: &LessonStartRequest) -> String {
    if let Some(topic) = clean_optional(request.topic.as_deref()) {
        return topic;
    }

    match student.interests.first() {
        Some(interest) => format!("patterns and causes in {interest}"),
        None => "patterns and causes from your biography".to_string(),
    }
}

fn starter_anchor(student: &StudentRecord) -> String {
    student
        .interests
        .first()
        .cloned()
        .or_else(|| {
            student
                .biography
                .as_deref()
                .and_then(|bio| bio.split(|ch| matches!(ch, '.' | ',' | ';')).next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "the learner's interests".to_string())
}

fn suggested_starter_topics(student: &StudentRecord, current_topic: &str) -> Vec<String> {
    let mut topics = Vec::new();
    push_unique_topic(&mut topics, current_topic.to_string());
    for topic in &student.suggested_topics {
        push_unique_topic(&mut topics, topic.clone());
    }
    for interest in &student.interests {
        push_unique_topic(&mut topics, format!("systems inside {interest}"));
        push_unique_topic(&mut topics, format!("evidence and patterns in {interest}"));
    }
    for fallback in [
        "cause and effect in everyday systems",
        "building models from observations",
        "how evidence changes an explanation",
    ] {
        push_unique_topic(&mut topics, fallback.to_string());
    }
    topics.truncate(5);
    topics
}

fn push_unique_topic(topics: &mut Vec<String>, topic: String) {
    let clean = topic.trim();
    if clean.is_empty() || topics.iter().any(|item| item.eq_ignore_ascii_case(clean)) {
        return;
    }
    topics.push(clean.to_string());
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
            "keyTerms",
            "narrativeCharacters"
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
            },
            "narrativeCharacters": {
                "type": "array",
                "maxItems": 4,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": [
                        "name",
                        "role",
                        "biography",
                        "topicAffinities",
                        "consistencyNotes",
                        "usedInScene",
                        "revisionNote"
                    ],
                    "properties": {
                        "name": { "type": "string" },
                        "role": { "type": "string" },
                        "biography": { "type": "string" },
                        "topicAffinities": {
                            "type": "array",
                            "maxItems": 8,
                            "items": { "type": "string" }
                        },
                        "consistencyNotes": {
                            "type": "array",
                            "maxItems": 8,
                            "items": { "type": "string" }
                        },
                        "usedInScene": { "type": "boolean" },
                        "revisionNote": { "type": "string" }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ConceptProgress, StudentMemory};

    fn student_record() -> StudentRecord {
        StudentRecord {
            student_id: "student-123".to_string(),
            display_name: "Mina".to_string(),
            age: Some(12),
            age_band: "11-13".to_string(),
            biography: Some(
                "Mina loves marine biology, drawing diagrams, and puzzles.".to_string(),
            ),
            interests: vec!["marine biology".to_string(), "drawing".to_string()],
            preferred_explanation_style: "visual".to_string(),
            level_context: "early middle-school science".to_string(),
            memories: vec![StudentMemory {
                assertion_id: Some("memory-1".to_string()),
                memory_type: "preference".to_string(),
                content: "Learner likes diagram-first explanations.".to_string(),
                confidence: 0.9,
                tags: vec!["style".to_string(), "visual".to_string()],
                subject: Some("Mina".to_string()),
                predicate: Some("prefers".to_string()),
                valid_from: None,
                valid_to: None,
                known_from: None,
                known_to: None,
                source: Some("test".to_string()),
            }],
            progress: vec![ConceptProgress {
                topic: "energy".to_string(),
                level: "intuition".to_string(),
                mastery_score: 0.81,
                status: "passed".to_string(),
                evidence: vec!["Passed Energy: Intuition.".to_string()],
            }],
            suggested_topics: vec!["coral reef systems".to_string()],
        }
    }

    fn narrative_characters() -> Vec<NarrativeCharacter> {
        vec![NarrativeCharacter {
            character_id: "character-1".to_string(),
            name: "Tala".to_string(),
            role: Some("reef guide".to_string()),
            current_biography: "Tala is a calm reef guide who helps Mina draw careful cause maps."
                .to_string(),
            topic_affinities: vec!["marine biology".to_string(), "systems".to_string()],
            consistency_notes: vec!["Keep Tala calm and precise.".to_string()],
            introduced_at: "2026-04-29T00:00:00Z".to_string(),
            last_seen_at: "2026-04-29T00:00:00Z".to_string(),
            last_seen_topic: Some("reef systems".to_string()),
        }]
    }

    #[test]
    fn infographic_prompt_uses_profile_and_memory_context() {
        let student = student_record();
        let prompt = build_infographic_prompt(
            &student,
            &InfographicRequest {
                student_id: Some(student.student_id.clone()),
                topic: "lightning".to_string(),
                lesson_summary: Some("charges separate and move suddenly".to_string()),
                infographic_prompt: None,
                size: None,
            },
        );

        assert!(prompt.contains("Topic: lightning"));
        assert!(prompt.contains("charges separate and move suddenly"));
        assert!(prompt.contains("Mina loves marine biology"));
        assert!(prompt.contains("Learner likes diagram-first explanations"));
        assert!(!prompt.contains("gpt-image-2"));
    }

    #[tokio::test]
    async fn guide_lesson_without_key_returns_profile_bootstrap_lesson() {
        let client = OpenAiClient::for_tests(None);
        let student = student_record();
        let characters = narrative_characters();
        let lesson = client
            .guide_lesson(
                &student,
                &characters,
                &LessonStartRequest {
                    student_id: Some(student.student_id.clone()),
                    topic: None,
                    question: Some("Choose a first lesson.".to_string()),
                },
            )
            .await
            .expect("fallback lesson should be available without credentials");

        assert_eq!(lesson["aiMode"], "missing_openai_api_key");
        assert_eq!(lesson["stageLevel"], "intuition");
        assert_eq!(lesson["topic"], "patterns and causes in marine biology");
        assert_eq!(lesson["keyTerms"].as_array().unwrap().len(), 3);
        assert_eq!(lesson["narrativeCharacters"][0]["name"], "Tala");
        for field in [
            "communicationStyle",
            "storyScene",
            "plainExplanation",
            "analogy",
            "checkForUnderstanding",
        ] {
            let text = lesson[field]
                .as_str()
                .unwrap_or_else(|| panic!("{field} should be a string"));
            assert!(!text.contains("The Primer"), "{field}: {text}");
            assert!(!text.contains("the Primer"), "{field}: {text}");
            assert!(!text.contains("The book"), "{field}: {text}");
            assert!(!text.contains("the book"), "{field}: {text}");
            assert!(!text.contains("The page"), "{field}: {text}");
            assert!(!text.contains("the page"), "{field}: {text}");
        }
        assert!(
            lesson["stagegatePrompt"]
                .as_str()
                .unwrap()
                .contains("marine biology")
        );
    }

    #[tokio::test]
    async fn explicit_topic_survives_profile_bootstrap_lesson() {
        let client = OpenAiClient::for_tests(None);
        let student = student_record();
        let lesson = client
            .guide_lesson(
                &student,
                &[],
                &LessonStartRequest {
                    student_id: Some(student.student_id.clone()),
                    topic: Some(" lightning ".to_string()),
                    question: None,
                },
            )
            .await
            .expect("fallback lesson should be available without credentials");

        assert_eq!(lesson["topic"], "lightning");
        assert!(
            lesson["suggestedTopics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|topic| topic == "lightning")
        );
    }

    #[tokio::test]
    async fn media_tools_return_schema_stable_fallbacks_without_key() {
        let client = OpenAiClient::for_tests(None);
        let student = student_record();
        let infographic = client
            .generate_infographic(
                &student,
                &InfographicRequest {
                    student_id: Some(student.student_id.clone()),
                    topic: "reef currents".to_string(),
                    lesson_summary: None,
                    infographic_prompt: Some("Show arrows and labels.".to_string()),
                    size: Some("1024x1024".to_string()),
                },
            )
            .await
            .expect("infographic fallback should be successful");
        let narration = client
            .generate_narration(&NarrationRequest {
                student_id: Some(student.student_id.clone()),
                topic: Some("reef currents".to_string()),
                text: "Tell the story.".to_string(),
                instructions: None,
            })
            .await
            .expect("narration fallback should be successful");

        assert_eq!(infographic["aiMode"], "missing_openai_api_key");
        assert_eq!(infographic["generated"], false);
        assert!(
            infographic["prompt"]
                .as_str()
                .unwrap()
                .contains("Show arrows and labels")
        );
        assert_eq!(narration["aiMode"], "missing_openai_api_key");
        assert_eq!(narration["generated"], false);
        assert_eq!(narration["voice"], "fable");
    }

    #[tokio::test]
    async fn stagegate_grading_requires_openai_key() {
        let client = OpenAiClient::for_tests(None);
        let student = student_record();
        let error = client
            .grade_stagegate(
                &student,
                &StagegateRequest {
                    student_id: Some(student.student_id.clone()),
                    topic: "lightning".to_string(),
                    answer: "Charges separate and then move.".to_string(),
                    stage_level: Some("intuition".to_string()),
                },
            )
            .await
            .expect_err("stagegate grading should not invent a pass without credentials");

        assert!(error.contains("OpenAI API key is required to grade stagegates"));
    }

    #[test]
    fn extracts_response_text_from_supported_payload_shapes() {
        assert_eq!(
            extract_response_text(&json!({ "output_text": "{\"ok\":true}" })),
            Some("{\"ok\":true}".to_string())
        );
        assert_eq!(
            extract_response_text(&json!({
                "output": [
                    {
                        "content": [
                            { "type": "output_text", "text": "{\"lesson\":\"ready\"}" }
                        ]
                    }
                ]
            })),
            Some("{\"lesson\":\"ready\"}".to_string())
        );
        assert_eq!(extract_response_text(&json!({ "output": [] })), None);
    }

    #[test]
    fn speech_input_is_trimmed_and_limited_by_characters() {
        let input = format!("  {}  ", "a".repeat(OPENAI_SPEECH_INPUT_LIMIT + 20));
        let trimmed = trim_speech_input(&input);

        assert_eq!(trimmed.chars().count(), OPENAI_SPEECH_INPUT_LIMIT);
        assert!(trimmed.chars().all(|ch| ch == 'a'));
    }
}
