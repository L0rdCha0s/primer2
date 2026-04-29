use poem::{
    get, handler,
    listener::TcpListener,
    middleware::Cors,
    post,
    web::{Data, Json, Path},
    EndpointExt, Route, Server,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_STUDENT_ID: &str = "mina-demo";
const DEFAULT_DATA_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/students.json");

type SharedStore = Arc<Mutex<LearningStore>>;

#[derive(Clone)]
struct AppState {
    openai: OpenAiClient,
    store: SharedStore,
    data_path: PathBuf,
}

#[derive(Clone)]
struct OpenAiClient {
    api_key: Option<String>,
    http: reqwest::Client,
    image_model: String,
    text_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningStore {
    students: HashMap<String, StudentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StudentRecord {
    student_id: String,
    display_name: String,
    age_band: String,
    interests: Vec<String>,
    preferred_explanation_style: String,
    level_context: String,
    memories: Vec<StudentMemory>,
    progress: Vec<ConceptProgress>,
    suggested_topics: Vec<String>,
    updated_at_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StudentMemory {
    memory_type: String,
    content: String,
    confidence: f32,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConceptProgress {
    topic: String,
    level: String,
    mastery_score: f32,
    status: String,
    evidence: Vec<String>,
    updated_at_epoch: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LessonStartRequest {
    student_id: Option<String>,
    topic: String,
    question: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InfographicRequest {
    student_id: Option<String>,
    topic: String,
    lesson_summary: Option<String>,
    infographic_prompt: Option<String>,
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StagegateRequest {
    student_id: Option<String>,
    topic: String,
    answer: String,
    stage_level: Option<String>,
}

impl OpenAiClient {
    fn from_env() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok().filter(|key| !key.is_empty()),
            http: reqwest::Client::new(),
            image_model: std::env::var("OPENAI_IMAGE_MODEL")
                .unwrap_or_else(|_| "gpt-image-2".to_string()),
            text_model: std::env::var("OPENAI_TEXT_MODEL")
                .unwrap_or_else(|_| "gpt-5.5".to_string()),
        }
    }

    async fn guide_lesson(
        &self,
        student: &StudentRecord,
        request: &LessonStartRequest,
    ) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(fallback_lesson(student, request));
        };

        let system_prompt = r#"You are PrimerLab, an adaptive educational story tutor.

You decide how to guide the learner, but only use memory and progress for educational personalization.
Keep the lesson age-appropriate, accurate, concise, and stage-gated.
Let the learner choose topics, then shape the path using their memories, current mastery, and level.
Do not invent memories. Do not store sensitive personal data.
Return only JSON that matches the provided schema."#;

        let user_payload = json!({
            "student": student,
            "requestedTopic": request.topic,
            "studentQuestion": request.question,
            "task": "Choose the right learning level, teach the requested topic through the persistent story world, recommend next topics, create a stagegate prompt, and create an image-generation prompt for an infographic."
        });

        self.responses_json(api_key, system_prompt, user_payload, "primer_lesson", lesson_schema())
            .await
            .map(|mut lesson| {
                lesson["aiMode"] = json!("openai_responses");
                lesson["model"] = json!(self.text_model);
                lesson
            })
    }

    async fn grade_stagegate(
        &self,
        student: &StudentRecord,
        request: &StagegateRequest,
    ) -> Result<Value, String> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(fallback_stagegate(request));
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

    async fn generate_infographic(
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
                    "content": [
                        {
                            "type": "input_text",
                            "text": system_prompt
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": user_payload.to_string()
                        }
                    ]
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

#[handler]
fn health(Data(state): Data<&AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "primerlab-api",
        "textModel": state.openai.text_model,
        "imageModel": state.openai.image_model,
        "hasOpenAiKey": state.openai.api_key.is_some()
    }))
}

#[handler]
fn get_student(Path(student_id): Path<String>, Data(state): Data<&AppState>) -> Json<Value> {
    let store = state.store.lock().expect("learning store lock poisoned");
    let student = store
        .students
        .get(&student_id)
        .or_else(|| store.students.get(DEFAULT_STUDENT_ID));

    Json(json!({
        "student": student,
    }))
}

#[handler]
fn list_students(Data(state): Data<&AppState>) -> Json<Value> {
    let store = state.store.lock().expect("learning store lock poisoned");
    Json(json!({
        "students": store.students.values().collect::<Vec<_>>(),
    }))
}

#[handler]
async fn start_lesson(
    Data(state): Data<&AppState>,
    Json(request): Json<LessonStartRequest>,
) -> Json<Value> {
    Json(start_lesson_impl(state, request).await)
}

#[handler]
async fn tutor_respond(
    Data(state): Data<&AppState>,
    Json(request): Json<LessonStartRequest>,
) -> Json<Value> {
    Json(start_lesson_impl(state, request).await)
}

async fn start_lesson_impl(state: &AppState, request: LessonStartRequest) -> Value {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_ID.to_string());
    let student = get_or_seed_student(&state.store, &student_id);

    let lesson = match state.openai.guide_lesson(&student, &request).await {
        Ok(lesson) => lesson,
        Err(error) => json!({
            "aiMode": "openai_error",
            "error": error,
            "fallback": fallback_lesson(&student, &request)
        }),
    };

    update_progress_after_lesson(&state, &student_id, &request.topic, &lesson);

    json!({
        "studentId": student_id,
        "lesson": lesson,
        "student": get_or_seed_student(&state.store, &student_id)
    })
}

#[handler]
async fn infographic(
    Data(state): Data<&AppState>,
    Json(request): Json<InfographicRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_ID.to_string());
    let student = get_or_seed_student(&state.store, &student_id);

    let result = match state.openai.generate_infographic(&student, &request).await {
        Ok(result) => result,
        Err(error) => json!({
            "aiMode": "openai_error",
            "generated": false,
            "error": error,
            "prompt": build_infographic_prompt(&student, &request)
        }),
    };

    Json(json!({
        "studentId": student_id,
        "artifact": result
    }))
}

#[handler]
async fn stagegate(
    Data(state): Data<&AppState>,
    Json(request): Json<StagegateRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_ID.to_string());
    let student = get_or_seed_student(&state.store, &student_id);

    let result = match state.openai.grade_stagegate(&student, &request).await {
        Ok(result) => result,
        Err(error) => json!({
            "aiMode": "openai_error",
            "error": error,
            "fallback": fallback_stagegate(&request)
        }),
    };

    update_progress_after_stagegate(&state, &student_id, &request, &result);

    Json(json!({
        "studentId": student_id,
        "result": result,
        "student": get_or_seed_student(&state.store, &student_id)
    }))
}

fn get_or_seed_student(store: &SharedStore, student_id: &str) -> StudentRecord {
    let mut store = store.lock().expect("learning store lock poisoned");
    if !store.students.contains_key(student_id) {
        store
            .students
            .insert(student_id.to_string(), seed_student(student_id));
    }

    store
        .students
        .get(student_id)
        .cloned()
        .expect("student must exist after seed")
}

fn update_progress_after_lesson(state: &AppState, student_id: &str, topic: &str, lesson: &Value) {
    let mut store = state.store.lock().expect("learning store lock poisoned");
    let student = store
        .students
        .entry(student_id.to_string())
        .or_insert_with(|| seed_student(student_id));

    student.updated_at_epoch = now_epoch();
    student.suggested_topics = lesson
        .get("suggestedTopics")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .filter(|items: &Vec<String>| !items.is_empty())
        .unwrap_or_else(|| student.suggested_topics.clone());

    if !student.progress.iter().any(|progress| progress.topic == topic) {
        student.progress.push(ConceptProgress {
            topic: topic.to_string(),
            level: lesson
                .get("stageLevel")
                .and_then(Value::as_str)
                .unwrap_or("intuition")
                .to_string(),
            mastery_score: 0.0,
            status: "exploring".to_string(),
            evidence: vec!["Learner started a guided exploration.".to_string()],
            updated_at_epoch: now_epoch(),
        });
    }

    save_store(&state.data_path, &store);
}

fn update_progress_after_stagegate(
    state: &AppState,
    student_id: &str,
    request: &StagegateRequest,
    result: &Value,
) {
    let mut store = state.store.lock().expect("learning store lock poisoned");
    let student = store
        .students
        .entry(student_id.to_string())
        .or_insert_with(|| seed_student(student_id));
    let score = result
        .get("score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0) as f32;
    let passed = result
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(score >= 0.75);

    if let Some(progress) = student
        .progress
        .iter_mut()
        .find(|progress| progress.topic == request.topic)
    {
        progress.mastery_score = score;
        progress.status = if passed { "passed" } else { "practicing" }.to_string();
        progress.updated_at_epoch = now_epoch();
        progress.evidence = result
            .get("masteryEvidence")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_else(|| progress.evidence.clone());
    } else {
        student.progress.push(ConceptProgress {
            topic: request.topic.clone(),
            level: request
                .stage_level
                .clone()
                .unwrap_or_else(|| "intuition".to_string()),
            mastery_score: score,
            status: if passed { "passed" } else { "practicing" }.to_string(),
            evidence: vec!["Stagegate submitted.".to_string()],
            updated_at_epoch: now_epoch(),
        });
    }

    if let Some(memories) = result.get("newMemories").and_then(Value::as_array) {
        for memory in memories {
            let content = memory
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            if content.is_empty()
                || student
                    .memories
                    .iter()
                    .any(|existing| existing.content == content)
            {
                continue;
            }

            student.memories.push(StudentMemory {
                memory_type: memory
                    .get("memoryType")
                    .and_then(Value::as_str)
                    .unwrap_or("knowledge")
                    .to_string(),
                content: content.to_string(),
                confidence: memory
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.7)
                    .clamp(0.0, 1.0) as f32,
                tags: memory
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|tags| {
                        tags.iter()
                            .filter_map(Value::as_str)
                            .map(ToString::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
    }

    student.updated_at_epoch = now_epoch();
    save_store(&state.data_path, &store);
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

fn build_infographic_prompt(student: &StudentRecord, request: &InfographicRequest) -> String {
    format!(
        "Create a clear educational infographic for a learner aged {age_band}. Topic: {topic}. Context: {summary}. Style: {style}. Use concise labels, accurate diagrams, and a story-world visual motif. Avoid tiny unreadable text. Learner memories: {memories}",
        age_band = student.age_band,
        topic = request.topic,
        summary = request
            .lesson_summary
            .as_deref()
            .or(request.infographic_prompt.as_deref())
            .unwrap_or("explain the concept visually"),
        style = student.preferred_explanation_style,
        memories = student
            .memories
            .iter()
            .map(|memory| memory.content.as_str())
            .collect::<Vec<_>>()
            .join(" | ")
    )
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

fn fallback_lesson(student: &StudentRecord, request: &LessonStartRequest) -> Value {
    let topic = request.topic.trim();
    json!({
        "aiMode": "missing_openai_api_key",
        "topic": topic,
        "stageLevel": "intuition",
        "communicationStyle": format!("Use {} explanations for a learner at {} level.", student.preferred_explanation_style, student.level_context),
        "storyScene": format!("In the Clockwork Reef, Tala opens a blank brass folio for {topic}. Add OPENAI_API_KEY in backend/.env so the Primer can write this lesson with the Responses API."),
        "plainExplanation": format!("The platform is ready to ask OpenAI to explain {topic}, adapt to this student's memories, and record progress."),
        "analogy": "The Primer will choose an analogy after reading the student's memories and mastery record.",
        "checkForUnderstanding": format!("What part of {topic} would you like to explore first?"),
        "suggestedTopics": ["lightning", "coral reef ecosystems", "fractions through music", "photosynthesis"],
        "stagegatePrompt": format!("Explain one important idea about {topic} in your own words."),
        "infographicPrompt": format!("Create an age-appropriate infographic that teaches {topic} through the Clockwork Reef."),
        "keyTerms": [
            { "term": "Topic", "definition": topic },
            { "term": "Stagegate", "definition": "A mastery check that unlocks the next level." }
        ]
    })
}

fn fallback_stagegate(request: &StagegateRequest) -> Value {
    let score = if request.answer.trim().len() > 48 { 0.78 } else { 0.45 };
    let passed = score >= 0.75;

    json!({
        "aiMode": "missing_openai_api_key",
        "passed": passed,
        "score": score,
        "rubric": {
            "accuracy": score,
            "causalReasoning": score,
            "vocabulary": score,
            "transfer": score
        },
        "masteryEvidence": if passed {
            json!(["Submitted a complete enough demo answer for local fallback grading."])
        } else {
            json!([])
        },
        "gaps": if passed {
            json!(["Connect this answer to a new example next."])
        } else {
            json!(["Add more detail before the Primer unlocks the next level."])
        },
        "feedbackToStudent": if passed {
            "Local fallback passed this answer. Add OPENAI_API_KEY for real rubric grading."
        } else {
            "This answer needs more detail. Add OPENAI_API_KEY for real rubric grading."
        },
        "nextLevelUnlocked": if passed { json!("mechanism") } else { Value::Null },
        "newMemories": if passed {
            json!([{
                "memoryType": "knowledge",
                "content": format!("Learner made progress on {} at the intuition level.", request.topic),
                "confidence": 0.6,
                "tags": ["local-fallback", request.topic.as_str()]
            }])
        } else {
            json!([])
        }
    })
}

fn load_store(data_path: &PathBuf) -> LearningStore {
    fs::read_to_string(data_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(default_store)
}

fn save_store(data_path: &PathBuf, store: &LearningStore) {
    if let Some(parent) = data_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(content) = serde_json::to_string_pretty(store) {
        let _ = fs::write(data_path, content);
    }
}

fn default_store() -> LearningStore {
    LearningStore {
        students: HashMap::from([(DEFAULT_STUDENT_ID.to_string(), seed_student(DEFAULT_STUDENT_ID))]),
    }
}

fn seed_student(student_id: &str) -> StudentRecord {
    StudentRecord {
        student_id: student_id.to_string(),
        display_name: "Mina".to_string(),
        age_band: "11-13".to_string(),
        interests: vec![
            "marine biology".to_string(),
            "drawing".to_string(),
            "puzzles".to_string(),
        ],
        preferred_explanation_style: "visual, story-first, ocean-current analogies".to_string(),
        level_context: "early middle-school science".to_string(),
        memories: vec![
            StudentMemory {
                memory_type: "preference".to_string(),
                content: "Learner likes visual puzzles and diagram-first explanations.".to_string(),
                confidence: 0.9,
                tags: vec!["style".to_string(), "visual".to_string()],
            },
            StudentMemory {
                memory_type: "preference".to_string(),
                content: "Ocean-current analogies help the learner compare invisible forces."
                    .to_string(),
                confidence: 0.84,
                tags: vec!["analogy".to_string(), "electricity".to_string()],
            },
            StudentMemory {
                memory_type: "misconception".to_string(),
                content: "Learner may confuse voltage with current.".to_string(),
                confidence: 0.74,
                tags: vec!["electricity".to_string(), "misconception".to_string()],
            },
        ],
        progress: vec![ConceptProgress {
            topic: "energy".to_string(),
            level: "intuition".to_string(),
            mastery_score: 0.81,
            status: "passed".to_string(),
            evidence: vec!["Passed Energy: Intuition in the seeded demo state.".to_string()],
            updated_at_epoch: now_epoch(),
        }],
        suggested_topics: vec![
            "lightning".to_string(),
            "coral reef ecosystems".to_string(),
            "fractions through music".to_string(),
            "photosynthesis".to_string(),
        ],
        updated_at_epoch: now_epoch(),
    }
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env"));
    let _ = dotenvy::dotenv();

    let data_path = std::env::var("PRIMERLAB_DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATA_PATH));
    let store = Arc::new(Mutex::new(load_store(&data_path)));
    let state = AppState {
        openai: OpenAiClient::from_env(),
        store,
        data_path,
    };
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:4000".to_string());

    let app = Route::new()
        .at("/health", get(health))
        .at("/api/students", get(list_students))
        .at("/api/students/:student_id", get(get_student))
        .at("/api/lesson/start", post(start_lesson))
        .at("/api/tutor/respond", post(tutor_respond))
        .at("/api/artifact/infographic", post(infographic))
        .at("/api/tutor/stagegate", post(stagegate))
        .with(Cors::new())
        .data(state);

    println!("primerlab-api listening on http://{bind_addr}");
    Server::new(TcpListener::bind(bind_addr)).run(app).await
}
