use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub xp_total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeCharacter {
    pub character_id: String,
    pub name: String,
    pub role: Option<String>,
    pub current_biography: String,
    pub topic_affinities: Vec<String>,
    pub consistency_notes: Vec<String>,
    pub introduced_at: String,
    pub last_seen_at: String,
    pub last_seen_topic: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentBookPageRecord {
    pub page_id: String,
    pub lesson_id: String,
    pub kind: String,
    pub topic: Option<String>,
    pub stage_level: Option<String>,
    pub position: i32,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentBookLessonRecord {
    pub lesson_id: String,
    pub topic: String,
    pub stage_level: Option<String>,
    pub position: i32,
    pub lesson: Value,
    pub latest_infographic: Option<Value>,
    pub latest_stagegate: Option<Value>,
    pub latest_answer: Option<String>,
    pub pages: Vec<StudentBookPageRecord>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentBookState {
    pub student_id: String,
    pub book_id: String,
    pub current_lesson_id: Option<String>,
    pub current_lesson: Option<Value>,
    pub lessons: Vec<StudentBookLessonRecord>,
    pub latest_infographic: Option<Value>,
    pub latest_stagegate: Option<Value>,
    pub latest_answer: Option<String>,
    pub has_passed_stagegate: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub activation_code: Option<String>,
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
    pub topic: Option<String>,
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
pub struct InfographicExplanationRequest {
    pub student_id: Option<String>,
    pub topic: String,
    pub image_src: String,
    pub title: Option<String>,
    pub alt: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationRequest {
    pub student_id: Option<String>,
    pub topic: Option<String>,
    pub text: String,
    pub instructions: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGraphRequest {
    pub student_id: Option<String>,
    pub node_id: Option<String>,
    pub valid_as_of: Option<DateTime<Utc>>,
    pub known_as_of: Option<DateTime<Utc>>,
    pub max_edges: Option<u32>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGraphNode {
    pub id: String,
    pub node_type: String,
    pub kind: String,
    pub label: String,
    pub summary: Option<String>,
    pub expanded: bool,
    pub fact_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub assertion_id: String,
    pub predicate: String,
    pub content: String,
    pub memory_type: String,
    pub confidence: f64,
    pub observed_at: String,
    pub valid_from: Option<String>,
    pub known_from: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentMemoryGraph {
    pub student_id: String,
    pub root_node_id: String,
    pub selected_node_id: String,
    pub nodes: Vec<MemoryGraphNode>,
    pub edges: Vec<MemoryGraphEdge>,
    pub valid_as_of: String,
    pub known_as_of: String,
}
