use std::collections::{BTreeMap, BTreeSet};

use crate::{
    db,
    domain::{StudentBookEntryRecord, StudentMemory},
    entities::{concept_progress, student},
    memory,
    openai::OpenAiClient,
};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SCIENCE_SOURCE_URL: &str = "https://www.australiancurriculum.edu.au/curriculum-information/understand-this-learning-area/science";
const ENGLISH_SOURCE_URL: &str =
    "https://www.australiancurriculum.edu.au/f-10-curriculum/learning-areas/english";
const MATHEMATICS_SOURCE_URL: &str = "https://www.australiancurriculum.edu.au/curriculum-information/understand-this-learning-area/mathematics";
const REPORTING_SOURCE_URL: &str = "https://www.australiancurriculum.edu.au/help/f-10-curriculum-overview/planning--teaching--assessing-and-reporting";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentReportCard {
    pub student_id: String,
    pub display_name: String,
    pub generated_at: String,
    pub ai_mode: String,
    pub model: Option<String>,
    pub narrative_error: Option<String>,
    pub year_level: CurriculumYearLevel,
    pub student_summary: String,
    pub parent_summary: String,
    pub learned_topics: Vec<ReportLearnedTopic>,
    pub stagegate_summary: StagegateSummary,
    pub curriculum_coverage: Vec<CurriculumCoverage>,
    pub strengths: Vec<String>,
    pub growth_areas: Vec<String>,
    pub next_steps: Vec<String>,
    pub memory_highlights: Vec<String>,
    pub sources: Vec<CurriculumSource>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurriculumYearLevel {
    pub code: String,
    pub label: String,
    pub age: Option<u8>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportLearnedTopic {
    pub topic: String,
    pub levels: Vec<String>,
    pub best_score: f64,
    pub status: String,
    pub evidence: Vec<String>,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagegateSummary {
    pub total_attempts: usize,
    pub passed_attempts: usize,
    pub average_score: f64,
    pub latest_attempt: Option<StagegateAttempt>,
    pub attempts: Vec<StagegateAttempt>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagegateAttempt {
    pub topic: String,
    pub stage_level: String,
    pub score: f64,
    pub passed: bool,
    pub feedback: String,
    pub mastery_evidence: Vec<String>,
    pub gaps: Vec<String>,
    pub submitted_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurriculumCoverage {
    pub learning_area: String,
    pub strand: String,
    pub year_level: String,
    pub reference_id: String,
    pub reference_label: String,
    pub source_url: String,
    pub status: String,
    pub evidence_topics: Vec<String>,
    pub evidence_count: usize,
    pub average_score: f64,
    pub parent_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurriculumSource {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportCardNarrative {
    pub student_summary: String,
    pub parent_summary: String,
    pub strengths: Vec<String>,
    pub growth_areas: Vec<String>,
    pub next_steps: Vec<String>,
}

pub async fn report_card_for_student(
    db: &DatabaseConnection,
    public_id: &str,
    openai: &OpenAiClient,
) -> Result<Option<StudentReportCard>, DbErr> {
    let Some(mut report) = build_deterministic_report_card(db, public_id).await? else {
        return Ok(None);
    };

    report.model = Some(openai.text_model().to_string());
    if !openai.has_api_key() {
        report.ai_mode = "missing_openai_api_key".to_string();
        return Ok(Some(report));
    }

    match openai.rewrite_report_card_narrative(&report).await {
        Ok(narrative) => {
            apply_narrative(&mut report, narrative);
            report.ai_mode = "openai_responses".to_string();
        }
        Err(error) => {
            report.ai_mode = "openai_unavailable".to_string();
            report.narrative_error = Some(error);
        }
    }

    Ok(Some(report))
}

async fn build_deterministic_report_card(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<Option<StudentReportCard>, DbErr> {
    let Some(student_row) = student::Entity::find()
        .filter(student::Column::PublicId.eq(public_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let progress_rows = concept_progress::Entity::find()
        .filter(concept_progress::Column::StudentId.eq(student_row.id))
        .order_by_asc(concept_progress::Column::Topic)
        .order_by_asc(concept_progress::Column::Level)
        .all(db)
        .await?;
    let book_entries = db::book_state_for_student(db, public_id)
        .await?
        .map(|book| book.entries)
        .unwrap_or_default();
    let memories = memory::student_memories(db, student_row.id)
        .await
        .unwrap_or_default();

    Ok(Some(deterministic_report_from_parts(
        &student_row,
        progress_rows,
        book_entries,
        memories,
    )))
}

fn deterministic_report_from_parts(
    student: &student::Model,
    progress_rows: Vec<concept_progress::Model>,
    book_entries: Vec<StudentBookEntryRecord>,
    memories: Vec<StudentMemory>,
) -> StudentReportCard {
    let learned_topics = build_learned_topics(progress_rows);
    let stagegate_summary = build_stagegate_summary(&book_entries);
    let year_level = curriculum_year_for_age(
        student.age_years.and_then(|age| u8::try_from(age).ok()),
        &student.age_band,
    );
    let curriculum_coverage =
        build_curriculum_coverage(&year_level, &learned_topics, &stagegate_summary);
    let memory_highlights = memories
        .iter()
        .take(4)
        .map(|memory| memory.content.clone())
        .collect::<Vec<_>>();
    let narrative = deterministic_narrative(
        student,
        &year_level,
        &learned_topics,
        &stagegate_summary,
        &curriculum_coverage,
        &memory_highlights,
    );

    StudentReportCard {
        student_id: student.public_id.clone(),
        display_name: student.display_name.clone(),
        generated_at: Utc::now().to_rfc3339(),
        ai_mode: "deterministic_fallback".to_string(),
        model: None,
        narrative_error: None,
        year_level,
        student_summary: narrative.student_summary,
        parent_summary: narrative.parent_summary,
        learned_topics,
        stagegate_summary,
        curriculum_coverage,
        strengths: narrative.strengths,
        growth_areas: narrative.growth_areas,
        next_steps: narrative.next_steps,
        memory_highlights,
        sources: curriculum_sources(),
    }
}

fn apply_narrative(report: &mut StudentReportCard, narrative: ReportCardNarrative) {
    report.student_summary = trim_or_keep(narrative.student_summary, &report.student_summary);
    report.parent_summary = trim_or_keep(narrative.parent_summary, &report.parent_summary);
    report.strengths = bounded_non_empty(narrative.strengths, &report.strengths, 4);
    report.growth_areas = bounded_non_empty(narrative.growth_areas, &report.growth_areas, 4);
    report.next_steps = bounded_non_empty(narrative.next_steps, &report.next_steps, 4);
}

fn trim_or_keep(next: String, fallback: &str) -> String {
    let trimmed = next.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(900).collect()
    }
}

fn bounded_non_empty(next: Vec<String>, fallback: &[String], max_items: usize) -> Vec<String> {
    let values = next
        .into_iter()
        .filter_map(|value| clean_text(&value, 260))
        .take(max_items)
        .collect::<Vec<_>>();
    if values.is_empty() {
        fallback.iter().take(max_items).cloned().collect()
    } else {
        values
    }
}

fn curriculum_year_for_age(age: Option<u8>, age_band: &str) -> CurriculumYearLevel {
    match age {
        Some(5) => CurriculumYearLevel {
            code: "F".to_string(),
            label: "Foundation".to_string(),
            age,
            note: None,
        },
        Some(6..=15) => {
            let year = age.unwrap() - 5;
            CurriculumYearLevel {
                code: year.to_string(),
                label: format!("Year {year}"),
                age,
                note: None,
            }
        }
        Some(16..=18) => CurriculumYearLevel {
            code: "10".to_string(),
            label: "Year 10".to_string(),
            age,
            note: Some(
                "Age is above the F-10 default; this MVP clamps curriculum links to Year 10 and flags senior pathway review needed."
                    .to_string(),
            ),
        },
        _ => {
            let (code, label) = match age_band {
                "5-7" => ("1", "Year 1"),
                "8-10" => ("4", "Year 4"),
                "14-18" => ("10", "Year 10"),
                _ => ("7", "Year 7"),
            };
            CurriculumYearLevel {
                code: code.to_string(),
                label: label.to_string(),
                age,
                note: Some(format!(
                    "Estimated from the learner age band ({age_band}) because exact age was unavailable."
                )),
            }
        }
    }
}

fn build_learned_topics(progress_rows: Vec<concept_progress::Model>) -> Vec<ReportLearnedTopic> {
    #[derive(Default)]
    struct TopicAggregate {
        levels: BTreeSet<String>,
        best_score: f64,
        statuses: Vec<String>,
        evidence: BTreeSet<String>,
        last_updated: Option<DateTime<FixedOffset>>,
    }

    let mut by_topic: BTreeMap<String, TopicAggregate> = BTreeMap::new();
    for row in progress_rows {
        let aggregate = by_topic.entry(row.topic).or_default();
        aggregate.levels.insert(row.level);
        aggregate.best_score = aggregate.best_score.max(row.mastery_score.clamp(0.0, 1.0));
        aggregate.statuses.push(row.status);
        for evidence in string_array(&row.evidence) {
            aggregate.evidence.insert(evidence);
        }
        aggregate.last_updated = Some(match aggregate.last_updated {
            Some(current) => current.max(row.updated_at),
            None => row.updated_at,
        });
    }

    by_topic
        .into_iter()
        .map(|(topic, aggregate)| ReportLearnedTopic {
            topic,
            levels: aggregate.levels.into_iter().collect(),
            best_score: round_score(aggregate.best_score),
            status: topic_status(&aggregate.statuses),
            evidence: aggregate.evidence.into_iter().take(6).collect(),
            last_updated: aggregate
                .last_updated
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
        })
        .collect()
}

fn topic_status(statuses: &[String]) -> String {
    if statuses.iter().any(|status| status == "passed") {
        "passed".to_string()
    } else if statuses.iter().any(|status| status == "practicing") {
        "practicing".to_string()
    } else {
        "exploring".to_string()
    }
}

fn build_stagegate_summary(entries: &[StudentBookEntryRecord]) -> StagegateSummary {
    let attempts = entries
        .iter()
        .filter(|entry| entry.kind == "stagegate")
        .filter_map(stagegate_attempt_from_entry)
        .collect::<Vec<_>>();
    let total_attempts = attempts.len();
    let passed_attempts = attempts.iter().filter(|attempt| attempt.passed).count();
    let average_score = if attempts.is_empty() {
        0.0
    } else {
        attempts.iter().map(|attempt| attempt.score).sum::<f64>() / attempts.len() as f64
    };

    StagegateSummary {
        total_attempts,
        passed_attempts,
        average_score: round_score(average_score),
        latest_attempt: attempts.last().cloned(),
        attempts,
    }
}

fn stagegate_attempt_from_entry(entry: &StudentBookEntryRecord) -> Option<StagegateAttempt> {
    let result = entry.payload.get("result")?;
    let request = entry.payload.get("request");
    let score = result
        .get("score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let passed = result
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(score >= 0.75);

    Some(StagegateAttempt {
        topic: entry
            .topic
            .clone()
            .or_else(|| {
                request
                    .and_then(|value| value.get("topic"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "untitled topic".to_string()),
        stage_level: entry
            .stage_level
            .clone()
            .or_else(|| {
                request
                    .and_then(|value| value.get("stageLevel"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "intuition".to_string()),
        score: round_score(score),
        passed,
        feedback: result
            .get("feedbackToStudent")
            .and_then(Value::as_str)
            .unwrap_or("No feedback was saved for this attempt.")
            .to_string(),
        mastery_evidence: string_array(result.get("masteryEvidence").unwrap_or(&Value::Null)),
        gaps: string_array(result.get("gaps").unwrap_or(&Value::Null)),
        submitted_at: entry.created_at.clone(),
    })
}

fn build_curriculum_coverage(
    year_level: &CurriculumYearLevel,
    learned_topics: &[ReportLearnedTopic],
    stagegate_summary: &StagegateSummary,
) -> Vec<CurriculumCoverage> {
    let topics = learned_topics
        .iter()
        .map(|topic| topic.topic.clone())
        .collect::<Vec<_>>();
    let stagegate_topics = stagegate_summary
        .attempts
        .iter()
        .map(|attempt| attempt.topic.clone())
        .collect::<Vec<_>>();
    let math_topics = learned_topics
        .iter()
        .filter(|topic| has_math_signal(topic))
        .map(|topic| topic.topic.clone())
        .collect::<Vec<_>>();
    let has_pass = stagegate_summary.passed_attempts > 0;
    let has_learning = !learned_topics.is_empty();
    let average_score = stagegate_summary.average_score;

    vec![
        CurriculumCoverage {
            learning_area: "Science".to_string(),
            strand: "Science Inquiry and Science Understanding".to_string(),
            year_level: year_level.label.clone(),
            reference_id: "primer-ac9-science-inquiry-understanding".to_string(),
            reference_label:
                "Australian Curriculum v9.0 Science content descriptions and achievement standards"
                    .to_string(),
            source_url: SCIENCE_SOURCE_URL.to_string(),
            status: coverage_status(has_learning, has_pass, average_score),
            evidence_topics: topics.clone(),
            evidence_count: topics.len(),
            average_score,
            parent_note:
                "Evidence is linked when the learner explains systems, causes, patterns, or investigations in a stagegate."
                    .to_string(),
        },
        CurriculumCoverage {
            learning_area: "English".to_string(),
            strand: "Language, Literacy and Literature".to_string(),
            year_level: year_level.label.clone(),
            reference_id: "primer-ac9-english-communication".to_string(),
            reference_label:
                "Australian Curriculum v9.0 English communication and vocabulary evidence"
                    .to_string(),
            source_url: ENGLISH_SOURCE_URL.to_string(),
            status: coverage_status(!stagegate_topics.is_empty(), has_pass, average_score),
            evidence_topics: stagegate_topics.clone(),
            evidence_count: stagegate_topics.len(),
            average_score,
            parent_note:
                "Evidence is linked when the learner writes explanations, uses vocabulary, and communicates reasoning for an audience."
                    .to_string(),
        },
        CurriculumCoverage {
            learning_area: "Mathematics".to_string(),
            strand: "Mathematical reasoning, data, measurement and patterns".to_string(),
            year_level: year_level.label.clone(),
            reference_id: "primer-ac9-mathematics-reasoning-data".to_string(),
            reference_label:
                "Australian Curriculum v9.0 Mathematics proficiency and strand connections"
                    .to_string(),
            source_url: MATHEMATICS_SOURCE_URL.to_string(),
            status: coverage_status(!math_topics.is_empty(), has_pass && !math_topics.is_empty(), average_score),
            evidence_topics: math_topics.clone(),
            evidence_count: math_topics.len(),
            average_score: if math_topics.is_empty() { 0.0 } else { average_score },
            parent_note:
                "Evidence is linked when the lesson or answer uses patterns, data, measurement, graphs, quantity, or structured reasoning."
                    .to_string(),
        },
    ]
}

fn coverage_status(has_evidence: bool, has_pass: bool, average_score: f64) -> String {
    if has_pass && average_score >= 0.75 {
        "covered".to_string()
    } else if has_evidence {
        "developing".to_string()
    } else {
        "not_evidenced".to_string()
    }
}

fn has_math_signal(topic: &ReportLearnedTopic) -> bool {
    let text = format!("{} {}", topic.topic, topic.evidence.join(" ")).to_ascii_lowercase();
    [
        "pattern",
        "data",
        "measure",
        "number",
        "graph",
        "ratio",
        "rate",
        "probability",
        "model",
        "force",
        "current",
        "scale",
        "quantity",
    ]
    .iter()
    .any(|keyword| text.contains(keyword))
}

fn deterministic_narrative(
    student: &student::Model,
    year_level: &CurriculumYearLevel,
    learned_topics: &[ReportLearnedTopic],
    stagegate_summary: &StagegateSummary,
    curriculum_coverage: &[CurriculumCoverage],
    memory_highlights: &[String],
) -> ReportCardNarrative {
    let topic_count = learned_topics.len();
    let best_topic = learned_topics
        .iter()
        .max_by(|left, right| left.best_score.total_cmp(&right.best_score));
    let latest_attempt = stagegate_summary.latest_attempt.as_ref();
    let student_summary = match (topic_count, best_topic) {
        (0, _) => format!(
            "{name}, your report card is ready to start. Complete a lesson and stagegate to add the first learning evidence.",
            name = student.display_name
        ),
        (_, Some(topic)) => format!(
            "{name}, you have explored {topic_count} topic{plural}. Your strongest current evidence is {topic_name} at {score}%.",
            name = student.display_name,
            plural = if topic_count == 1 { "" } else { "s" },
            topic_name = topic.topic,
            score = percent(topic.best_score)
        ),
        _ => "Your report card is ready.".to_string(),
    };
    let parent_summary = format!(
        "{name}'s report card links Primer learning evidence to Australian Curriculum v9.0 references for {year}. This is an evidence summary for parent review, not an official school grade.",
        name = student.display_name,
        year = year_level.label
    );
    let mut strengths = Vec::new();
    if let Some(topic) = best_topic {
        strengths.push(format!(
            "Strongest topic evidence: {topic} ({score}%).",
            topic = topic.topic,
            score = percent(topic.best_score)
        ));
    }
    if stagegate_summary.passed_attempts > 0 {
        strengths.push(format!(
            "Passed {passed} of {total} recorded stagegate attempt{plural}.",
            passed = stagegate_summary.passed_attempts,
            total = stagegate_summary.total_attempts,
            plural = if stagegate_summary.total_attempts == 1 {
                ""
            } else {
                "s"
            }
        ));
    }
    if let Some(memory) = memory_highlights.first() {
        strengths.push(format!("Personalisation evidence is active: {memory}"));
    }
    if strengths.is_empty() {
        strengths
            .push("A learning record has started and is ready for first evidence.".to_string());
    }

    let mut growth_areas = latest_attempt
        .map(|attempt| attempt.gaps.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|gap| clean_text(&gap, 220))
        .take(3)
        .collect::<Vec<_>>();
    for coverage in curriculum_coverage
        .iter()
        .filter(|coverage| coverage.status == "not_evidenced")
    {
        growth_areas.push(format!(
            "Add clearer evidence for {} through a future lesson or stagegate.",
            coverage.learning_area
        ));
    }
    if growth_areas.is_empty() {
        growth_areas.push(
            "Use the next answer to give a clearer cause, example, and transfer case.".to_string(),
        );
    }
    growth_areas.truncate(4);

    let mut next_steps = Vec::new();
    match latest_attempt {
        Some(attempt) if attempt.passed => next_steps.push(format!(
            "Start the next stage for {} and explain the mechanism behind the idea.",
            attempt.topic
        )),
        Some(attempt) => next_steps.push(format!(
            "Revise {} and resubmit the stagegate with one cause and one example.",
            attempt.topic
        )),
        None => next_steps
            .push("Complete the first stagegate to create reportable evidence.".to_string()),
    }
    for topic in suggested_topics(student).into_iter().take(2) {
        next_steps.push(format!("Explore {topic} as a follow-up pathway."));
    }
    next_steps.truncate(4);

    ReportCardNarrative {
        student_summary,
        parent_summary,
        strengths,
        growth_areas,
        next_steps,
    }
}

fn suggested_topics(student: &student::Model) -> Vec<String> {
    string_array(&student.suggested_topics)
        .into_iter()
        .filter_map(|topic| clean_text(&topic, 90))
        .collect()
}

fn curriculum_sources() -> Vec<CurriculumSource> {
    vec![
        CurriculumSource {
            label: "Australian Curriculum v9.0 reporting guidance".to_string(),
            url: REPORTING_SOURCE_URL.to_string(),
        },
        CurriculumSource {
            label: "Australian Curriculum v9.0 Science".to_string(),
            url: SCIENCE_SOURCE_URL.to_string(),
        },
        CurriculumSource {
            label: "Australian Curriculum v9.0 English".to_string(),
            url: ENGLISH_SOURCE_URL.to_string(),
        },
        CurriculumSource {
            label: "Australian Curriculum v9.0 Mathematics".to_string(),
            url: MATHEMATICS_SOURCE_URL.to_string(),
        },
    ]
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn clean_text(value: &str, max_chars: usize) -> Option<String> {
    let text = value
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return None;
    }

    Some(text.chars().take(max_chars).collect())
}

fn round_score(score: f64) -> f64 {
    (score.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

fn percent(score: f64) -> i32 {
    (score.clamp(0.0, 1.0) * 100.0).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;
    use uuid::Uuid;

    fn observed_at() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 4, 29, 0, 0, 0)
            .unwrap()
    }

    fn student_model(age_years: Option<i32>) -> student::Model {
        let observed_at = observed_at();
        student::Model {
            id: Uuid::new_v4(),
            public_id: "student-123".to_string(),
            display_name: "Mina".to_string(),
            age_years,
            age_band: "11-13".to_string(),
            biography: Some("Loves reef systems.".to_string()),
            interests: json!(["marine biology"]),
            preferred_explanation_style: "visual".to_string(),
            level_context: "middle school".to_string(),
            suggested_topics: json!(["reef food webs", "ocean currents"]),
            xp_total: 50,
            created_at: observed_at,
            updated_at: observed_at,
        }
    }

    #[test]
    fn maps_age_to_australian_curriculum_year_level() {
        assert_eq!(curriculum_year_for_age(Some(5), "5-7").label, "Foundation");
        assert_eq!(curriculum_year_for_age(Some(6), "5-7").label, "Year 1");
        assert_eq!(curriculum_year_for_age(Some(15), "14-18").label, "Year 10");

        let senior = curriculum_year_for_age(Some(17), "14-18");
        assert_eq!(senior.label, "Year 10");
        assert!(senior.note.unwrap().contains("senior pathway review"));

        let estimated = curriculum_year_for_age(None, "8-10");
        assert_eq!(estimated.label, "Year 4");
        assert!(estimated.note.unwrap().contains("Estimated"));
    }

    #[test]
    fn aggregates_stagegate_attempts_from_book_entries() {
        let entries = vec![StudentBookEntryRecord {
            entry_id: "entry-1".to_string(),
            kind: "stagegate".to_string(),
            topic: Some("reef currents".to_string()),
            stage_level: Some("intuition".to_string()),
            position: 1,
            payload: json!({
                "request": {
                    "topic": "reef currents",
                    "stageLevel": "intuition",
                    "answer": "Forces push water."
                },
                "result": {
                    "passed": true,
                    "score": 0.88,
                    "feedbackToStudent": "Level 2 unlocked.",
                    "masteryEvidence": ["Named the cause."],
                    "gaps": ["Add one transfer example."]
                }
            }),
            created_at: "2026-04-29T00:00:00Z".to_string(),
        }];

        let summary = build_stagegate_summary(&entries);

        assert_eq!(summary.total_attempts, 1);
        assert_eq!(summary.passed_attempts, 1);
        assert_eq!(summary.average_score, 0.88);
        assert_eq!(
            summary.latest_attempt.unwrap().mastery_evidence,
            vec!["Named the cause."]
        );
    }

    #[test]
    fn curriculum_coverage_tracks_core_trio_status() {
        let learned_topics = vec![ReportLearnedTopic {
            topic: "reef current data patterns".to_string(),
            levels: vec!["intuition".to_string()],
            best_score: 0.88,
            status: "passed".to_string(),
            evidence: vec!["Used graph evidence.".to_string()],
            last_updated: "2026-04-29T00:00:00Z".to_string(),
        }];
        let stagegate_summary = StagegateSummary {
            total_attempts: 1,
            passed_attempts: 1,
            average_score: 0.88,
            latest_attempt: None,
            attempts: vec![StagegateAttempt {
                topic: "reef current data patterns".to_string(),
                stage_level: "intuition".to_string(),
                score: 0.88,
                passed: true,
                feedback: "Good reasoning.".to_string(),
                mastery_evidence: vec![],
                gaps: vec![],
                submitted_at: "2026-04-29T00:00:00Z".to_string(),
            }],
        };

        let coverage = build_curriculum_coverage(
            &curriculum_year_for_age(Some(12), "11-13"),
            &learned_topics,
            &stagegate_summary,
        );

        assert_eq!(coverage.len(), 3);
        assert!(coverage.iter().all(|item| item.year_level == "Year 7"));
        assert_eq!(coverage[0].learning_area, "Science");
        assert_eq!(coverage[0].status, "covered");
        assert_eq!(coverage[1].learning_area, "English");
        assert_eq!(coverage[1].status, "covered");
        assert_eq!(coverage[2].learning_area, "Mathematics");
        assert_eq!(coverage[2].status, "covered");
    }

    #[test]
    fn empty_report_has_stable_fallback_narrative() {
        let report =
            deterministic_report_from_parts(&student_model(Some(12)), vec![], vec![], vec![]);

        assert_eq!(report.learned_topics.len(), 0);
        assert_eq!(report.stagegate_summary.total_attempts, 0);
        assert!(report.student_summary.contains("ready to start"));
        assert!(
            report
                .curriculum_coverage
                .iter()
                .all(|coverage| coverage.status == "not_evidenced")
        );
    }
}
