use crate::{
    domain::{
        ConceptProgress, DEFAULT_STUDENT_PUBLIC_ID, RegisterRequest, StagegateRequest,
        StudentMemory, StudentRecord,
    },
    entities::{concept_progress, local_user, student, student_memory},
    memory,
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr,
    EntityTrait, QueryFilter, Set, Statement,
};
use serde_json::{Value, json};
use uuid::Uuid;

pub async fn connect_database() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://primerlab:primerlab@127.0.0.1:5432/primerlab".to_string());
    Database::connect(url).await
}

pub async fn init_database(db: &DatabaseConnection) -> Result<(), DbErr> {
    for sql in [
        "CREATE EXTENSION IF NOT EXISTS vector",
        "CREATE EXTENSION IF NOT EXISTS age",
        "LOAD 'age'",
        r#"SET search_path = ag_catalog, "$user", public"#,
        r#"SELECT ag_catalog.create_graph('primer_memory')
           WHERE NOT EXISTS (
               SELECT 1 FROM ag_catalog.ag_graph WHERE name = 'primer_memory'
           )"#,
        "SET search_path TO public",
        r#"CREATE TABLE IF NOT EXISTS students (
            id uuid PRIMARY KEY,
            public_id text NOT NULL UNIQUE,
            display_name text NOT NULL,
            age_years integer,
            age_band text NOT NULL,
            biography text,
            interests jsonb NOT NULL DEFAULT '[]'::jsonb,
            preferred_explanation_style text NOT NULL,
            level_context text NOT NULL,
            suggested_topics jsonb NOT NULL DEFAULT '[]'::jsonb,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL
        )"#,
        "ALTER TABLE students ADD COLUMN IF NOT EXISTS age_years integer",
        "ALTER TABLE students ADD COLUMN IF NOT EXISTS biography text",
        r#"CREATE TABLE IF NOT EXISTS local_users (
            id uuid PRIMARY KEY,
            username text NOT NULL UNIQUE,
            password_hash text NOT NULL,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            created_at timestamptz NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS student_memories (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            memory_type text NOT NULL,
            content text NOT NULL,
            confidence double precision NOT NULL,
            tags jsonb NOT NULL DEFAULT '[]'::jsonb,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL,
            UNIQUE (student_id, content)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS concept_progress (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            topic text NOT NULL,
            level text NOT NULL,
            mastery_score double precision NOT NULL,
            status text NOT NULL,
            evidence jsonb NOT NULL DEFAULT '[]'::jsonb,
            updated_at timestamptz NOT NULL,
            UNIQUE (student_id, topic, level)
        )"#,
    ] {
        db.execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
            .await?;
    }

    memory::init_schema(db).await?;

    Ok(())
}

pub async fn seed_demo_student(db: &DatabaseConnection) -> Result<StudentRecord, DbErr> {
    let student = get_or_create_student(
        db,
        DEFAULT_STUDENT_PUBLIC_ID,
        "Mina",
        Some(12),
        "11-13",
        Some("Mina is curious about the ocean, likes drawing careful diagrams, and enjoys puzzle-like explanations that reveal hidden mechanisms.".to_string()),
        vec![
            "marine biology".to_string(),
            "drawing".to_string(),
            "puzzles".to_string(),
        ],
    )
    .await?;

    seed_memory(
        db,
        &student,
        "preference",
        "Learner likes visual puzzles and diagram-first explanations.",
        0.9,
        json!(["style", "visual"]),
    )
    .await?;
    seed_memory(
        db,
        &student,
        "preference",
        "Ocean-current analogies help the learner compare invisible forces.",
        0.84,
        json!(["analogy", "electricity"]),
    )
    .await?;
    seed_memory(
        db,
        &student,
        "misconception",
        "Learner may confuse voltage with current.",
        0.74,
        json!(["electricity", "misconception"]),
    )
    .await?;
    seed_progress(
        db,
        student.id,
        "energy",
        "intuition",
        0.81,
        "passed",
        json!(["Passed Energy: Intuition in the seeded demo state."]),
    )
    .await?;

    student_record(db, &student).await
}

pub async fn list_students(db: &DatabaseConnection) -> Result<Vec<StudentRecord>, DbErr> {
    let rows = student::Entity::find().all(db).await?;
    let mut students = Vec::with_capacity(rows.len());
    for row in rows {
        students.push(student_record(db, &row).await?);
    }
    Ok(students)
}

pub async fn find_student(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<Option<StudentRecord>, DbErr> {
    match student::Entity::find()
        .filter(student::Column::PublicId.eq(public_id))
        .one(db)
        .await?
    {
        Some(row) => Ok(Some(student_record(db, &row).await?)),
        None => Ok(None),
    }
}

pub async fn get_or_seed_student(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<StudentRecord, DbErr> {
    if let Some(student) = find_student(db, public_id).await? {
        return Ok(student);
    }

    let row = get_or_create_student(db, public_id, "Mina", None, "11-13", None, vec![]).await?;
    student_record(db, &row).await
}

pub async fn register_local_user(
    db: &DatabaseConnection,
    request: RegisterRequest,
) -> Result<StudentRecord, String> {
    let username = request.username.trim().to_lowercase();
    if username.len() < 3 || request.password.len() < 8 {
        return Err("Username must be at least 3 characters and password at least 8.".to_string());
    }
    if local_user::Entity::find()
        .filter(local_user::Column::Username.eq(username.clone()))
        .one(db)
        .await
        .map_err(|error| error.to_string())?
        .is_some()
    {
        return Err("Username is already taken.".to_string());
    }

    let public_id = format!("student-{}", Uuid::new_v4());
    let age_years = validate_signup_age(request.age)?;
    let biography = clean_biography(request.biography).ok_or_else(|| {
        "Add a short student biography so PrimerLab can guide the first lesson.".to_string()
    })?;
    let interests = clean_interests(request.interests);
    if interests.is_empty() {
        return Err("Add at least one interest so PrimerLab can personalize lessons.".to_string());
    }
    let display_name = request
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(username.as_str())
        .to_string();
    let age_band = request
        .age_band
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| age_band_for_age(age_years));
    let student = get_or_create_student(
        db,
        &public_id,
        &display_name,
        age_years,
        &age_band,
        Some(biography.clone()),
        interests.clone(),
    )
    .await
    .map_err(|error| error.to_string())?;

    local_user::ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(username),
        password_hash: Set(hash_password(&request.password)?),
        student_id: Set(student.id),
        created_at: Set(now()),
    }
    .insert(db)
    .await
    .map_err(|error| error.to_string())?;

    for interest in &interests {
        seed_memory(
            db,
            &student,
            "interest",
            &format!("Learner is interested in {interest}."),
            0.95,
            json!(["interest", interest]),
        )
        .await
        .map_err(|error| error.to_string())?;
    }

    seed_memory(
        db,
        &student,
        "preference",
        &format!("Signup biography: {biography}"),
        0.96,
        json!(["profile", "biography", "signup"]),
    )
    .await
    .map_err(|error| error.to_string())?;

    seed_memory(
        db,
        &student,
        "preference",
        &format!(
            "Use examples and story choices connected to {} when they fit the lesson.",
            format_interest_list(&interests)
        ),
        0.9,
        json!(["personalization", "interests"]),
    )
    .await
    .map_err(|error| error.to_string())?;

    student_record(db, &student)
        .await
        .map_err(|error| error.to_string())
}

pub async fn login_local_user(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
) -> Result<StudentRecord, String> {
    let user = local_user::Entity::find()
        .filter(local_user::Column::Username.eq(username.trim().to_lowercase()))
        .one(db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Invalid username or password.".to_string())?;

    if !verify_password(password, &user.password_hash)? {
        return Err("Invalid username or password.".to_string());
    }

    let student = student::Entity::find_by_id(user.student_id)
        .one(db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Student record is missing.".to_string())?;

    student_record(db, &student)
        .await
        .map_err(|error| error.to_string())
}

pub async fn update_progress_after_lesson(
    db: &DatabaseConnection,
    public_id: &str,
    topic: &str,
    lesson: &Value,
) -> Result<StudentRecord, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let suggested_topics = lesson
        .get("suggestedTopics")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .cloned();

    if let Some(suggested_topics) = suggested_topics {
        let mut active: student::ActiveModel = student.clone().into();
        active.suggested_topics = Set(Value::Array(suggested_topics));
        active.updated_at = Set(now());
        active.update(db).await?;
    }

    let stage_level = lesson
        .get("stageLevel")
        .and_then(Value::as_str)
        .unwrap_or("intuition");

    if concept_progress::Entity::find()
        .filter(concept_progress::Column::StudentId.eq(student.id))
        .filter(concept_progress::Column::Topic.eq(topic))
        .filter(concept_progress::Column::Level.eq(stage_level))
        .one(db)
        .await?
        .is_none()
    {
        concept_progress::ActiveModel {
            id: Set(Uuid::new_v4()),
            student_id: Set(student.id),
            topic: Set(topic.to_string()),
            level: Set(stage_level.to_string()),
            mastery_score: Set(0.0),
            status: Set("exploring".to_string()),
            evidence: Set(json!(["Learner started a guided exploration."])),
            updated_at: Set(now()),
        }
        .insert(db)
        .await?;
    }

    memory::record_lesson_started(db, &student, topic, lesson).await?;

    student_record(db, &get_or_seed_student_row(db, public_id).await?).await
}

pub async fn update_progress_after_stagegate(
    db: &DatabaseConnection,
    public_id: &str,
    request: &StagegateRequest,
    result: &Value,
) -> Result<StudentRecord, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let score = result
        .get("score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let passed = result
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(score >= 0.75);
    let stage_level = request.stage_level.as_deref().unwrap_or("intuition");

    match concept_progress::Entity::find()
        .filter(concept_progress::Column::StudentId.eq(student.id))
        .filter(concept_progress::Column::Topic.eq(&request.topic))
        .filter(concept_progress::Column::Level.eq(stage_level))
        .one(db)
        .await?
    {
        Some(row) => {
            let mut active: concept_progress::ActiveModel = row.into();
            active.mastery_score = Set(score);
            active.status = Set(if passed { "passed" } else { "practicing" }.to_string());
            active.evidence = Set(result
                .get("masteryEvidence")
                .cloned()
                .unwrap_or_else(|| json!([])));
            active.updated_at = Set(now());
            active.update(db).await?;
        }
        None => {
            concept_progress::ActiveModel {
                id: Set(Uuid::new_v4()),
                student_id: Set(student.id),
                topic: Set(request.topic.clone()),
                level: Set(stage_level.to_string()),
                mastery_score: Set(score),
                status: Set(if passed { "passed" } else { "practicing" }.to_string()),
                evidence: Set(result
                    .get("masteryEvidence")
                    .cloned()
                    .unwrap_or_else(|| json!(["Stagegate submitted."]))),
                updated_at: Set(now()),
            }
            .insert(db)
            .await?;
        }
    }

    memory::record_stagegate_result(db, &student, &request.topic, stage_level, result).await?;

    if let Some(memories) = result.get("newMemories").and_then(Value::as_array) {
        for memory in memories {
            let Some(content) = memory.get("content").and_then(Value::as_str) else {
                continue;
            };
            if content.trim().is_empty() {
                continue;
            }
            seed_memory(
                db,
                &student,
                memory
                    .get("memoryType")
                    .and_then(Value::as_str)
                    .unwrap_or("knowledge"),
                content,
                memory
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.7),
                memory.get("tags").cloned().unwrap_or_else(|| json!([])),
            )
            .await?;
        }
    }

    student_record(db, &student).await
}

async fn get_or_create_student(
    db: &DatabaseConnection,
    public_id: &str,
    display_name: &str,
    age_years: Option<u8>,
    age_band: &str,
    biography: Option<String>,
    interests: Vec<String>,
) -> Result<student::Model, DbErr> {
    if let Some(row) = student::Entity::find()
        .filter(student::Column::PublicId.eq(public_id))
        .one(db)
        .await?
    {
        if row.biography.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(biography) = biography {
                let mut active: student::ActiveModel = row.into();
                active.biography = Set(Some(biography));
                active.updated_at = Set(now());
                return active.update(db).await;
            }
        }

        return Ok(row);
    }

    let preferred_explanation_style = preferred_style_for_interests(&interests);
    let level_context = level_context_for_age(age_years, age_band);
    let suggested_topics = suggested_topics_for_interests(&interests);

    student::ActiveModel {
        id: Set(Uuid::new_v4()),
        public_id: Set(public_id.to_string()),
        display_name: Set(display_name.to_string()),
        age_years: Set(age_years.map(i32::from)),
        age_band: Set(age_band.to_string()),
        biography: Set(biography),
        interests: Set(json!(interests)),
        preferred_explanation_style: Set(preferred_explanation_style),
        level_context: Set(level_context),
        suggested_topics: Set(json!(suggested_topics)),
        created_at: Set(now()),
        updated_at: Set(now()),
    }
    .insert(db)
    .await
}

async fn get_or_seed_student_row(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<student::Model, DbErr> {
    if let Some(row) = student::Entity::find()
        .filter(student::Column::PublicId.eq(public_id))
        .one(db)
        .await?
    {
        return Ok(row);
    }
    get_or_create_student(db, public_id, "Mina", None, "11-13", None, vec![]).await
}

async fn seed_memory(
    db: &DatabaseConnection,
    student: &student::Model,
    memory_type: &str,
    content: &str,
    confidence: f64,
    tags: Value,
) -> Result<(), DbErr> {
    memory::assert_student_memory(
        db,
        student,
        memory_type,
        content,
        confidence,
        tags,
        &format!("student-memory:{}:{content}", student.public_id),
    )
    .await
}

async fn seed_progress(
    db: &DatabaseConnection,
    student_id: Uuid,
    topic: &str,
    level: &str,
    mastery_score: f64,
    status: &str,
    evidence: Value,
) -> Result<(), DbErr> {
    if concept_progress::Entity::find()
        .filter(concept_progress::Column::StudentId.eq(student_id))
        .filter(concept_progress::Column::Topic.eq(topic))
        .filter(concept_progress::Column::Level.eq(level))
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }

    concept_progress::ActiveModel {
        id: Set(Uuid::new_v4()),
        student_id: Set(student_id),
        topic: Set(topic.to_string()),
        level: Set(level.to_string()),
        mastery_score: Set(mastery_score),
        status: Set(status.to_string()),
        evidence: Set(evidence),
        updated_at: Set(now()),
    }
    .insert(db)
    .await?;

    Ok(())
}

async fn student_record(
    db: &DatabaseConnection,
    student: &student::Model,
) -> Result<StudentRecord, DbErr> {
    let mut memories = memory::student_memories(db, student.id).await?;
    if memories.is_empty() {
        memories = legacy_student_memories(db, student.id).await?;
    }

    let progress = concept_progress::Entity::find()
        .filter(concept_progress::Column::StudentId.eq(student.id))
        .all(db)
        .await?
        .into_iter()
        .map(|progress| ConceptProgress {
            topic: progress.topic,
            level: progress.level,
            mastery_score: progress.mastery_score as f32,
            status: progress.status,
            evidence: string_array(progress.evidence),
        })
        .collect();

    Ok(StudentRecord {
        student_id: student.public_id.clone(),
        display_name: student.display_name.clone(),
        age: student.age_years.and_then(|age| u8::try_from(age).ok()),
        age_band: student.age_band.clone(),
        biography: student.biography.clone(),
        interests: string_array(student.interests.clone()),
        preferred_explanation_style: student.preferred_explanation_style.clone(),
        level_context: student.level_context.clone(),
        memories,
        progress,
        suggested_topics: string_array(student.suggested_topics.clone()),
    })
}

async fn legacy_student_memories(
    db: &DatabaseConnection,
    student_id: Uuid,
) -> Result<Vec<StudentMemory>, DbErr> {
    student_memory::Entity::find()
        .filter(student_memory::Column::StudentId.eq(student_id))
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|memory| StudentMemory {
                    assertion_id: None,
                    memory_type: memory.memory_type,
                    content: memory.content,
                    confidence: memory.confidence as f32,
                    tags: string_array(memory.tags),
                    subject: None,
                    predicate: None,
                    valid_from: Some(memory.created_at.to_rfc3339()),
                    valid_to: None,
                    known_from: Some(memory.created_at.to_rfc3339()),
                    known_to: None,
                    source: Some("legacy_student_memories".to_string()),
                })
                .collect()
        })
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|error| error.to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn validate_signup_age(age: Option<u8>) -> Result<Option<u8>, String> {
    let Some(age) = age else {
        return Err("Age is required.".to_string());
    };
    if !(5..=18).contains(&age) {
        return Err("Age must be between 5 and 18 for this student demo.".to_string());
    }

    Ok(Some(age))
}

fn clean_interests(raw_interests: Vec<String>) -> Vec<String> {
    let mut interests: Vec<String> = Vec::new();
    for raw_interest in raw_interests {
        for part in raw_interest.split(',') {
            let clean = part.trim();
            if clean.is_empty() {
                continue;
            }

            let clean = clean.chars().take(48).collect::<String>();
            if interests
                .iter()
                .any(|interest| interest.eq_ignore_ascii_case(&clean))
            {
                continue;
            }

            interests.push(clean);
            if interests.len() >= 8 {
                return interests;
            }
        }
    }

    interests
}

fn clean_biography(raw_biography: Option<String>) -> Option<String> {
    let biography = raw_biography?
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if biography.is_empty() {
        return None;
    }

    Some(biography.chars().take(1200).collect())
}

fn age_band_for_age(age: Option<u8>) -> String {
    match age.unwrap_or(12) {
        5..=7 => "5-7",
        8..=10 => "8-10",
        11..=13 => "11-13",
        14..=18 => "14-18",
        _ => "11-13",
    }
    .to_string()
}

fn level_context_for_age(age: Option<u8>, age_band: &str) -> String {
    match age {
        Some(5..=7) => "early primary learner".to_string(),
        Some(8..=10) => "upper primary learner".to_string(),
        Some(11..=13) => "early middle-school science".to_string(),
        Some(14..=18) => "high-school learner".to_string(),
        _ => format!("{age_band} learner"),
    }
}

fn preferred_style_for_interests(interests: &[String]) -> String {
    if interests.is_empty() {
        return "visual, story-first, ocean-current analogies".to_string();
    }

    format!(
        "visual, story-first explanations anchored in {}",
        format_interest_list(interests)
    )
}

fn suggested_topics_for_interests(interests: &[String]) -> Vec<String> {
    let defaults = [
        "lightning",
        "coral reef ecosystems",
        "fractions through music",
        "photosynthesis",
    ];
    let mut topics: Vec<String> = interests
        .iter()
        .take(4)
        .map(|interest| format!("science in {interest}"))
        .collect();

    for topic in defaults {
        if topics.len() >= 4 {
            break;
        }
        if !topics.iter().any(|item| item.eq_ignore_ascii_case(topic)) {
            topics.push(topic.to_string());
        }
    }

    topics
}

fn format_interest_list(interests: &[String]) -> String {
    match interests {
        [] => "the learner's interests".to_string(),
        [single] => single.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut output = interests[..interests.len() - 1].join(", ");
            output.push_str(", and ");
            output.push_str(&interests[interests.len() - 1]);
            output
        }
    }
}

fn string_array(value: Value) -> Vec<String> {
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

fn now() -> DateTime<FixedOffset> {
    Utc::now().fixed_offset()
}
