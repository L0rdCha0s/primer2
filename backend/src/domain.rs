use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_STUDENT_PUBLIC_ID: &str = "mina-demo";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentRecord {
    pub student_id: String,
    pub display_name: String,
    pub age: Option<u8>,
    pub age_band: String,
    pub biography: Option<String>,
    pub interests: Vec<String>,
    pub preferred_explanation_style: String,
    pub level_context: String,
    pub memories: Vec<StudentMemory>,
    pub progress: Vec<ConceptProgress>,
    pub suggested_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentMemory {
    pub assertion_id: Option<String>,
    pub memory_type: String,
    pub content: String,
    pub confidence: f32,
    pub tags: Vec<String>,
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub known_from: Option<String>,
    pub known_to: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptProgress {
    pub topic: String,
    pub level: String,
    pub mastery_score: f32,
    pub status: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub age: Option<u8>,
    pub age_band: Option<String>,
    pub biography: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonStartRequest {
    pub student_id: Option<String>,
    pub topic: String,
    pub question: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfographicRequest {
    pub student_id: Option<String>,
    pub topic: String,
    pub lesson_summary: Option<String>,
    pub infographic_prompt: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagegateRequest {
    pub student_id: Option<String>,
    pub topic: String,
    pub answer: String,
    pub stage_level: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryProfileRequest {
    pub student_id: Option<String>,
    pub valid_as_of: Option<DateTime<Utc>>,
    pub known_as_of: Option<DateTime<Utc>>,
    pub max_facts: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntityRecord {
    pub entity_id: String,
    pub kind: String,
    pub canonical_name: String,
    pub identity_key: String,
    pub normalized_key: String,
    pub properties: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAssertionRecord {
    pub assertion_id: String,
    pub subject: String,
    pub subject_identity_key: String,
    pub predicate: String,
    pub object: Option<String>,
    pub object_identity_key: Option<String>,
    pub object_text: Option<String>,
    pub object_value: Option<Value>,
    pub content: String,
    pub memory_type: String,
    pub confidence: f64,
    pub salience: f64,
    pub tags: Vec<String>,
    pub qualifiers: Value,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub known_from: Option<String>,
    pub known_to: Option<String>,
    pub observed_at: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentMemoryProfile {
    pub student_id: String,
    pub entity: MemoryEntityRecord,
    pub subject_facts: Vec<MemoryAssertionRecord>,
    pub inbound_facts: Vec<MemoryAssertionRecord>,
    pub timeline: Vec<MemoryAssertionRecord>,
    pub valid_as_of: String,
    pub known_as_of: String,
}
