use crate::{
    domain::{
        ConceptProgress, InfographicExplanationRequest, InfographicRequest, LessonStartRequest,
        NarrativeCharacter, RegisterRequest, StagegateRequest, StudentBookEntryRecord,
        StudentBookState, StudentMemory, StudentRecord,
    },
    entities::{
        concept_progress, infographic_voiceover, local_user, narrative_character,
        narrative_character_biography, student, student_book, student_book_entry, student_memory,
        student_xp_event,
    },
    memory,
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr,
    EntityTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait, Value as SeaOrmValue,
};
use serde_json::{Value, json};
use uuid::Uuid;

const LESSON_STARTED_XP: i32 = 10;
const STAGEGATE_SUBMITTED_XP: i32 = 10;
const STAGEGATE_PASSED_XP: i32 = 40;

#[derive(Clone, Debug)]
pub struct InfographicVoiceoverRecord {
    pub explanation: Value,
    pub content_type: String,
    pub file_path: String,
}

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
            xp_total integer NOT NULL DEFAULT 0,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL
        )"#,
        "ALTER TABLE students ADD COLUMN IF NOT EXISTS age_years integer",
        "ALTER TABLE students ADD COLUMN IF NOT EXISTS biography text",
        "ALTER TABLE students ADD COLUMN IF NOT EXISTS xp_total integer NOT NULL DEFAULT 0",
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
        r#"CREATE TABLE IF NOT EXISTS student_xp_events (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            event_key text NOT NULL UNIQUE,
            points integer NOT NULL,
            source_type text NOT NULL,
            topic text,
            stage_level text,
            metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
            created_at timestamptz NOT NULL
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_student_xp_events_student_created ON student_xp_events (student_id, created_at DESC)",
        r#"CREATE TABLE IF NOT EXISTS student_books (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            title text NOT NULL,
            status text NOT NULL DEFAULT 'active',
            active_lesson jsonb,
            latest_infographic jsonb,
            latest_stagegate jsonb,
            latest_answer text,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL,
            UNIQUE (student_id)
        )"#,
        "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS active_lesson jsonb",
        "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS latest_infographic jsonb",
        "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS latest_stagegate jsonb",
        "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS latest_answer text",
        r#"CREATE TABLE IF NOT EXISTS student_book_entries (
            id uuid PRIMARY KEY,
            book_id uuid NOT NULL REFERENCES student_books(id) ON DELETE CASCADE,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            entry_kind text NOT NULL,
            topic text,
            stage_level text,
            position integer NOT NULL,
            payload jsonb NOT NULL DEFAULT '{}'::jsonb,
            created_at timestamptz NOT NULL,
            UNIQUE (book_id, position)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_student_book_entries_student_position ON student_book_entries (student_id, position)",
        "CREATE INDEX IF NOT EXISTS idx_student_book_entries_book_kind_created ON student_book_entries (book_id, entry_kind, created_at DESC)",
        r#"CREATE TABLE IF NOT EXISTS infographic_voiceovers (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            cache_key text NOT NULL,
            topic text NOT NULL,
            title text,
            alt text,
            image_hash text NOT NULL,
            image_length bigint NOT NULL,
            explanation jsonb NOT NULL DEFAULT '{}'::jsonb,
            speech_model text,
            voice text,
            content_type text NOT NULL,
            file_path text NOT NULL,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL,
            UNIQUE (student_id, cache_key)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_infographic_voiceovers_student_updated ON infographic_voiceovers (student_id, updated_at DESC)",
        r#"CREATE TABLE IF NOT EXISTS narrative_characters (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            name text NOT NULL,
            normalized_name text NOT NULL,
            role text,
            current_biography text NOT NULL,
            topic_affinities jsonb NOT NULL DEFAULT '[]'::jsonb,
            consistency_notes jsonb NOT NULL DEFAULT '[]'::jsonb,
            status text NOT NULL DEFAULT 'active',
            introduced_at timestamptz NOT NULL,
            last_seen_at timestamptz NOT NULL,
            last_seen_topic text,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL,
            UNIQUE (student_id, normalized_name)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_narrative_characters_student_seen ON narrative_characters (student_id, last_seen_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_narrative_characters_topic_affinities ON narrative_characters USING gin (topic_affinities)",
        r#"CREATE TABLE IF NOT EXISTS narrative_character_biographies (
            id uuid PRIMARY KEY,
            character_id uuid NOT NULL REFERENCES narrative_characters(id) ON DELETE CASCADE,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            biography text NOT NULL,
            source_topic text,
            revision_note text,
            created_at timestamptz NOT NULL
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_narrative_character_bios_character_created ON narrative_character_biographies (character_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_narrative_character_bios_student_created ON narrative_character_biographies (student_id, created_at DESC)",
    ] {
        db.execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
            .await?;
    }

    memory::init_schema(db).await?;

    Ok(())
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

    let row =
        get_or_create_student(db, public_id, "Guest learner", None, "11-13", None, vec![]).await?;
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
        "Add a short student biography so Primer can guide the first lesson.".to_string()
    })?;
    let interests = clean_interests(request.interests);
    if interests.is_empty() {
        return Err("Add at least one interest so Primer can personalize lessons.".to_string());
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
    upsert_lesson_characters_for_student(db, &student, topic, lesson).await?;

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

    for award in stagegate_xp_awards(result) {
        award_xp_event(
            db,
            &student,
            &stagegate_xp_event_key(student.id, award.source_type, &request.topic, stage_level),
            award.points,
            award.source_type,
            Some(&request.topic),
            Some(stage_level),
            json!({
                "score": score,
                "passed": passed,
                "stageLevel": stage_level
            }),
        )
        .await?;
    }

    let refreshed_student = get_or_seed_student_row(db, public_id).await?;
    student_record(db, &refreshed_student).await
}

pub async fn book_state_for_student(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<Option<StudentBookState>, DbErr> {
    let Some(student) = student::Entity::find()
        .filter(student::Column::PublicId.eq(public_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let Some(book) = student_book::Entity::find()
        .filter(student_book::Column::StudentId.eq(student.id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    book_state_for_book(db, &student, &book).await.map(Some)
}

pub async fn update_lesson_book_state(
    db: &DatabaseConnection,
    public_id: &str,
    topic: &str,
    request: &LessonStartRequest,
    lesson: &Value,
) -> Result<StudentBookState, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let book = get_or_create_student_book(db, &student).await?;
    let observed_at = now();
    let mut active_book: student_book::ActiveModel = book.into();
    active_book.active_lesson = Set(Some(lesson.clone()));
    active_book.latest_infographic = Set(None);
    active_book.latest_stagegate = Set(None);
    active_book.latest_answer = Set(None);
    active_book.updated_at = Set(observed_at);
    let updated_book = active_book.update(db).await?;

    award_xp_event(
        db,
        &student,
        &lesson_xp_event_key(Uuid::new_v4()),
        LESSON_STARTED_XP,
        "lesson_started",
        Some(topic),
        lesson.get("stageLevel").and_then(Value::as_str),
        json!({
            "bookId": updated_book.id,
            "request": {
                "topic": request.topic.clone(),
                "question": request.question.clone()
            }
        }),
    )
    .await?;

    book_state_for_book(db, &student, &updated_book).await
}

pub async fn update_infographic_book_state(
    db: &DatabaseConnection,
    public_id: &str,
    _request: &InfographicRequest,
    artifact: &Value,
) -> Result<StudentBookState, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let book = get_or_create_student_book(db, &student).await?;
    let observed_at = now();
    let mut active_book: student_book::ActiveModel = book.into();
    active_book.latest_infographic = Set(Some(artifact.clone()));
    active_book.updated_at = Set(observed_at);
    let updated_book = active_book.update(db).await?;

    book_state_for_book(db, &student, &updated_book).await
}

pub async fn find_infographic_voiceover(
    db: &DatabaseConnection,
    public_id: &str,
    cache_key: &str,
) -> Result<Option<InfographicVoiceoverRecord>, DbErr> {
    let Some(student) = student::Entity::find()
        .filter(student::Column::PublicId.eq(public_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let voiceover = infographic_voiceover::Entity::find()
        .filter(infographic_voiceover::Column::StudentId.eq(student.id))
        .filter(infographic_voiceover::Column::CacheKey.eq(cache_key))
        .one(db)
        .await?;

    Ok(voiceover.map(|voiceover| InfographicVoiceoverRecord {
        explanation: voiceover.explanation,
        content_type: voiceover.content_type,
        file_path: voiceover.file_path,
    }))
}

pub async fn save_infographic_voiceover(
    db: &DatabaseConnection,
    public_id: &str,
    request: &InfographicExplanationRequest,
    cache_key: &str,
    image_hash: &str,
    image_length: i64,
    explanation: &Value,
    content_type: &str,
    file_path: &str,
) -> Result<InfographicVoiceoverRecord, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let observed_at = now();
    let existing = infographic_voiceover::Entity::find()
        .filter(infographic_voiceover::Column::StudentId.eq(student.id))
        .filter(infographic_voiceover::Column::CacheKey.eq(cache_key))
        .one(db)
        .await?;

    let model = match existing {
        Some(existing) => {
            let mut active: infographic_voiceover::ActiveModel = existing.into();
            active.topic = Set(request.topic.clone());
            active.title = Set(request.title.clone());
            active.alt = Set(request.alt.clone());
            active.image_hash = Set(image_hash.to_string());
            active.image_length = Set(image_length);
            active.explanation = Set(explanation.clone());
            active.speech_model = Set(explanation
                .get("speechModel")
                .and_then(Value::as_str)
                .map(ToString::to_string));
            active.voice = Set(explanation
                .get("voice")
                .and_then(Value::as_str)
                .map(ToString::to_string));
            active.content_type = Set(content_type.to_string());
            active.file_path = Set(file_path.to_string());
            active.updated_at = Set(observed_at);
            active.update(db).await?
        }
        None => {
            infographic_voiceover::ActiveModel {
                id: Set(Uuid::new_v4()),
                student_id: Set(student.id),
                cache_key: Set(cache_key.to_string()),
                topic: Set(request.topic.clone()),
                title: Set(request.title.clone()),
                alt: Set(request.alt.clone()),
                image_hash: Set(image_hash.to_string()),
                image_length: Set(image_length),
                explanation: Set(explanation.clone()),
                speech_model: Set(explanation
                    .get("speechModel")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)),
                voice: Set(explanation
                    .get("voice")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)),
                content_type: Set(content_type.to_string()),
                file_path: Set(file_path.to_string()),
                created_at: Set(observed_at),
                updated_at: Set(observed_at),
            }
            .insert(db)
            .await?
        }
    };

    Ok(InfographicVoiceoverRecord {
        explanation: model.explanation,
        content_type: model.content_type,
        file_path: model.file_path,
    })
}

pub async fn append_stagegate_book_entry(
    db: &DatabaseConnection,
    public_id: &str,
    request: &StagegateRequest,
    result: &Value,
) -> Result<StudentBookState, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let book = get_or_create_student_book(db, &student).await?;
    let stage_level = request.stage_level.as_deref().unwrap_or("intuition");
    append_book_entry(
        db,
        &book,
        &student,
        "stagegate",
        Some(&request.topic),
        Some(stage_level),
        json!({
            "result": result.clone(),
            "request": {
                "topic": request.topic.clone(),
                "stageLevel": stage_level,
                "answer": request.answer.clone()
            }
        }),
    )
    .await?;
    let observed_at = now();
    let mut active_book: student_book::ActiveModel = book.into();
    active_book.latest_stagegate = Set(Some(result.clone()));
    active_book.latest_answer = Set(Some(request.answer.clone()));
    active_book.updated_at = Set(observed_at);
    let updated_book = active_book.update(db).await?;

    book_state_for_book(db, &student, &updated_book).await
}

pub async fn reset_student_learning_state(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<StudentRecord, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    let student_id = student.id;

    let tx = db.begin().await?;

    student_book_entry::Entity::delete_many()
        .filter(student_book_entry::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    student_book::Entity::delete_many()
        .filter(student_book::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    narrative_character_biography::Entity::delete_many()
        .filter(narrative_character_biography::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    narrative_character::Entity::delete_many()
        .filter(narrative_character::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    concept_progress::Entity::delete_many()
        .filter(concept_progress::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    student_xp_event::Entity::delete_many()
        .filter(student_xp_event::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    infographic_voiceover::Entity::delete_many()
        .filter(infographic_voiceover::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;
    student_memory::Entity::delete_many()
        .filter(student_memory::Column::StudentId.eq(student_id))
        .exec(&tx)
        .await?;

    tx.execute(sql_statement(
        r#"
        UPDATE memory_assertions
        SET supersedes_assertion_id = NULL,
            contradicts_assertion_id = NULL
        WHERE student_id = $1
        "#,
        vec![student_id.into()],
    ))
    .await?;
    tx.execute(sql_statement(
        "DELETE FROM memory_assertions WHERE student_id = $1",
        vec![student_id.into()],
    ))
    .await?;
    tx.execute(sql_statement(
        "DELETE FROM memory_entities WHERE student_id = $1",
        vec![student_id.into()],
    ))
    .await?;
    tx.execute(sql_statement(
        "DELETE FROM memory_sources WHERE student_id = $1",
        vec![student_id.into()],
    ))
    .await?;

    let interests = string_array(student.interests.clone());
    let mut active: student::ActiveModel = student.into();
    active.suggested_topics = Set(json!(suggested_topics_for_interests(&interests)));
    active.xp_total = Set(0);
    active.updated_at = Set(now());
    active.update(&tx).await?;

    tx.commit().await?;
    if let Err(error) = memory::delete_student_graph_projection(db, student_id).await {
        println!(
            "[primerlab-api] AGE graph reset failed for student={public_id}; SQL reset already completed: {error}"
        );
    }
    let refreshed_student = get_or_seed_student_row(db, public_id).await?;
    student_record(db, &refreshed_student).await
}

pub async fn relevant_narrative_characters(
    db: &DatabaseConnection,
    public_id: &str,
    topic: Option<&str>,
) -> Result<Vec<NarrativeCharacter>, DbErr> {
    let student = get_or_seed_student_row(db, public_id).await?;
    relevant_narrative_characters_for_student(db, &student, topic).await
}

async fn relevant_narrative_characters_for_student(
    db: &DatabaseConnection,
    student: &student::Model,
    topic: Option<&str>,
) -> Result<Vec<NarrativeCharacter>, DbErr> {
    let rows = narrative_character::Entity::find()
        .filter(narrative_character::Column::StudentId.eq(student.id))
        .filter(narrative_character::Column::Status.eq("active"))
        .order_by_desc(narrative_character::Column::LastSeenAt)
        .all(db)
        .await?;
    let terms = topic_terms(topic);
    let mut scored_rows = rows
        .into_iter()
        .map(|row| (character_relevance_score(&row, &terms), row))
        .collect::<Vec<_>>();

    scored_rows.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.last_seen_at.cmp(&left.1.last_seen_at))
            .then_with(|| left.1.name.cmp(&right.1.name))
    });

    let has_topic_match = !terms.is_empty() && scored_rows.iter().any(|(score, _)| *score > 0);
    Ok(scored_rows
        .into_iter()
        .filter(|(score, _)| !has_topic_match || *score > 0)
        .take(4)
        .map(|(_, row)| narrative_character_record(row))
        .collect())
}

async fn upsert_lesson_characters_for_student(
    db: &DatabaseConnection,
    student: &student::Model,
    topic: &str,
    lesson: &Value,
) -> Result<(), DbErr> {
    let Some(characters) = lesson.get("narrativeCharacters").and_then(Value::as_array) else {
        return Ok(());
    };

    for character in characters {
        if character
            .get("usedInScene")
            .and_then(Value::as_bool)
            .is_some_and(|used| !used)
        {
            continue;
        }

        let Some(name) = clean_character_text(character.get("name").and_then(Value::as_str), 80)
        else {
            continue;
        };
        let normalized_name = normalize_character_name(&name);
        if normalized_name.is_empty()
            || normalized_name == normalize_character_name(&student.display_name)
            || matches!(normalized_name.as_str(), "student" | "learner" | "you")
        {
            continue;
        }

        let Some(biography) =
            clean_character_text(character.get("biography").and_then(Value::as_str), 1600)
        else {
            continue;
        };
        let role = clean_character_text(character.get("role").and_then(Value::as_str), 120);
        let mut topic_affinities = clean_character_array(character.get("topicAffinities"), 10, 80);
        push_unique_clean_string(&mut topic_affinities, topic, 80);
        let consistency_notes = clean_character_array(character.get("consistencyNotes"), 10, 200);
        let revision_note =
            clean_character_text(character.get("revisionNote").and_then(Value::as_str), 240);

        upsert_narrative_character(
            db,
            student,
            &name,
            &normalized_name,
            role,
            &biography,
            topic_affinities,
            consistency_notes,
            topic,
            revision_note,
        )
        .await?;
    }

    Ok(())
}

async fn upsert_narrative_character(
    db: &DatabaseConnection,
    student: &student::Model,
    name: &str,
    normalized_name: &str,
    role: Option<String>,
    biography: &str,
    topic_affinities: Vec<String>,
    consistency_notes: Vec<String>,
    topic: &str,
    revision_note: Option<String>,
) -> Result<(), DbErr> {
    let observed_at = now();
    let existing = narrative_character::Entity::find()
        .filter(narrative_character::Column::StudentId.eq(student.id))
        .filter(narrative_character::Column::NormalizedName.eq(normalized_name))
        .one(db)
        .await?;

    match existing {
        Some(row) => {
            let character_id = row.id;
            let bio_changed = row.current_biography.trim() != biography.trim();
            let next_role = role.or_else(|| row.role.clone());
            let merged_topic_affinities =
                merge_character_arrays(row.topic_affinities.clone(), topic_affinities, 12);
            let merged_consistency_notes =
                merge_character_arrays(row.consistency_notes.clone(), consistency_notes, 12);
            let mut active: narrative_character::ActiveModel = row.into();
            active.name = Set(name.to_string());
            active.role = Set(next_role);
            active.current_biography = Set(biography.to_string());
            active.topic_affinities = Set(json!(merged_topic_affinities));
            active.consistency_notes = Set(json!(merged_consistency_notes));
            active.status = Set("active".to_string());
            active.last_seen_at = Set(observed_at);
            active.last_seen_topic = Set(Some(topic.to_string()));
            active.updated_at = Set(observed_at);
            active.update(db).await?;

            if bio_changed {
                insert_character_biography(
                    db,
                    character_id,
                    student.id,
                    biography,
                    Some(topic),
                    revision_note.as_deref(),
                    observed_at,
                )
                .await?;
            }
        }
        None => {
            let character_id = Uuid::new_v4();
            narrative_character::ActiveModel {
                id: Set(character_id),
                student_id: Set(student.id),
                name: Set(name.to_string()),
                normalized_name: Set(normalized_name.to_string()),
                role: Set(role),
                current_biography: Set(biography.to_string()),
                topic_affinities: Set(json!(topic_affinities)),
                consistency_notes: Set(json!(consistency_notes)),
                status: Set("active".to_string()),
                introduced_at: Set(observed_at),
                last_seen_at: Set(observed_at),
                last_seen_topic: Set(Some(topic.to_string())),
                created_at: Set(observed_at),
                updated_at: Set(observed_at),
            }
            .insert(db)
            .await?;

            insert_character_biography(
                db,
                character_id,
                student.id,
                biography,
                Some(topic),
                revision_note.as_deref(),
                observed_at,
            )
            .await?;
        }
    }

    Ok(())
}

async fn insert_character_biography(
    db: &DatabaseConnection,
    character_id: Uuid,
    student_id: Uuid,
    biography: &str,
    source_topic: Option<&str>,
    revision_note: Option<&str>,
    created_at: DateTime<FixedOffset>,
) -> Result<(), DbErr> {
    narrative_character_biography::ActiveModel {
        id: Set(Uuid::new_v4()),
        character_id: Set(character_id),
        student_id: Set(student_id),
        biography: Set(biography.to_string()),
        source_topic: Set(source_topic.map(ToString::to_string)),
        revision_note: Set(revision_note.map(ToString::to_string)),
        created_at: Set(created_at),
    }
    .insert(db)
    .await?;

    Ok(())
}

async fn book_state_for_book(
    db: &DatabaseConnection,
    student: &student::Model,
    book: &student_book::Model,
) -> Result<StudentBookState, DbErr> {
    let rows = student_book_entry::Entity::find()
        .filter(student_book_entry::Column::BookId.eq(book.id))
        .order_by_asc(student_book_entry::Column::Position)
        .all(db)
        .await?;

    Ok(book_state_from_rows(student, book, rows))
}

async fn get_or_create_student_book(
    db: &DatabaseConnection,
    student: &student::Model,
) -> Result<student_book::Model, DbErr> {
    if let Some(book) = student_book::Entity::find()
        .filter(student_book::Column::StudentId.eq(student.id))
        .one(db)
        .await?
    {
        return Ok(book);
    }

    let observed_at = now();
    student_book::ActiveModel {
        id: Set(Uuid::new_v4()),
        student_id: Set(student.id),
        title: Set(format!("{}'s Primer", student.display_name)),
        status: Set("active".to_string()),
        active_lesson: Set(None),
        latest_infographic: Set(None),
        latest_stagegate: Set(None),
        latest_answer: Set(None),
        created_at: Set(observed_at),
        updated_at: Set(observed_at),
    }
    .insert(db)
    .await
}

async fn append_book_entry(
    db: &DatabaseConnection,
    book: &student_book::Model,
    student: &student::Model,
    entry_kind: &str,
    topic: Option<&str>,
    stage_level: Option<&str>,
    payload: Value,
) -> Result<student_book_entry::Model, DbErr> {
    let observed_at = now();
    let position = next_book_position(db, book.id).await?;
    let entry = student_book_entry::ActiveModel {
        id: Set(Uuid::new_v4()),
        book_id: Set(book.id),
        student_id: Set(student.id),
        entry_kind: Set(entry_kind.to_string()),
        topic: Set(topic.map(ToString::to_string)),
        stage_level: Set(stage_level.map(ToString::to_string)),
        position: Set(position),
        payload: Set(payload),
        created_at: Set(observed_at),
    }
    .insert(db)
    .await?;

    let mut active_book: student_book::ActiveModel = book.clone().into();
    active_book.updated_at = Set(observed_at);
    active_book.update(db).await?;

    Ok(entry)
}

async fn next_book_position(db: &DatabaseConnection, book_id: Uuid) -> Result<i32, DbErr> {
    let latest = student_book_entry::Entity::find()
        .filter(student_book_entry::Column::BookId.eq(book_id))
        .order_by_desc(student_book_entry::Column::Position)
        .one(db)
        .await?;

    Ok(latest.map(|entry| entry.position + 1).unwrap_or(1))
}

async fn award_xp_event(
    db: &DatabaseConnection,
    student: &student::Model,
    event_key: &str,
    points: i32,
    source_type: &str,
    topic: Option<&str>,
    stage_level: Option<&str>,
    metadata: Value,
) -> Result<i32, DbErr> {
    if points <= 0 {
        return Ok(0);
    }

    let observed_at = now();
    let tx = db.begin().await?;
    let inserted = tx
        .query_one(sql_statement(
            r#"
            INSERT INTO student_xp_events (
                id, student_id, event_key, points, source_type, topic,
                stage_level, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9)
            ON CONFLICT (event_key) DO NOTHING
            RETURNING points
            "#,
            vec![
                Uuid::new_v4().into(),
                student.id.into(),
                event_key.to_string().into(),
                points.into(),
                source_type.to_string().into(),
                topic.map(ToString::to_string).into(),
                stage_level.map(ToString::to_string).into(),
                metadata.to_string().into(),
                observed_at.into(),
            ],
        ))
        .await?;

    if inserted.is_some() {
        tx.execute(sql_statement(
            "UPDATE students SET xp_total = xp_total + $1, updated_at = $2 WHERE id = $3",
            vec![points.into(), observed_at.into(), student.id.into()],
        ))
        .await?;
        tx.commit().await?;
        return Ok(points);
    }

    tx.commit().await?;
    Ok(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XpAward {
    source_type: &'static str,
    points: i32,
}

fn stagegate_xp_awards(result: &Value) -> Vec<XpAward> {
    let score = result
        .get("score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let passed = result
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(score >= 0.75);
    let mut awards = vec![XpAward {
        source_type: "stagegate_submitted",
        points: STAGEGATE_SUBMITTED_XP,
    }];

    if passed {
        awards.push(XpAward {
            source_type: "stagegate_passed",
            points: STAGEGATE_PASSED_XP,
        });
    }

    awards
}

fn lesson_xp_event_key(entry_id: Uuid) -> String {
    format!("lesson_started:{entry_id}")
}

fn stagegate_xp_event_key(
    student_id: Uuid,
    source_type: &str,
    topic: &str,
    stage_level: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        source_type,
        student_id,
        xp_key_component(topic),
        xp_key_component(stage_level)
    )
}

fn xp_key_component(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;

    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            last_was_separator = false;
        } else if !last_was_separator && !output.is_empty() {
            output.push('-');
            last_was_separator = true;
        }

        if output.len() >= 96 {
            break;
        }
    }

    let normalized = output.trim_matches('-').to_string();
    if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized
    }
}

fn book_state_from_rows(
    student: &student::Model,
    book: &student_book::Model,
    rows: Vec<student_book_entry::Model>,
) -> StudentBookState {
    let all_entries = rows
        .into_iter()
        .map(student_book_entry_record)
        .collect::<Vec<_>>();
    let has_current_state = book.active_lesson.is_some()
        || book.latest_infographic.is_some()
        || book.latest_stagegate.is_some()
        || book.latest_answer.is_some();
    let (active_lesson, latest_infographic, latest_stagegate, latest_answer) = if has_current_state
    {
        (
            book.active_lesson.clone(),
            book.latest_infographic.clone(),
            book.latest_stagegate.clone(),
            book.latest_answer.clone(),
        )
    } else {
        let mut active_lesson = None;
        let mut latest_infographic = None;
        let mut latest_stagegate = None;
        let mut latest_answer = None;

        for entry in &all_entries {
            match entry.kind.as_str() {
                "lesson" => {
                    active_lesson = entry
                        .payload
                        .get("lesson")
                        .cloned()
                        .or_else(|| Some(entry.payload.clone()));
                    latest_infographic = None;
                    latest_stagegate = None;
                    latest_answer = None;
                }
                "infographic" => {
                    latest_infographic = entry
                        .payload
                        .get("artifact")
                        .cloned()
                        .or_else(|| Some(entry.payload.clone()));
                }
                "stagegate" => {
                    latest_stagegate = entry
                        .payload
                        .get("result")
                        .cloned()
                        .or_else(|| Some(entry.payload.clone()));
                    latest_answer = entry
                        .payload
                        .get("request")
                        .and_then(|request| request.get("answer"))
                        .and_then(Value::as_str)
                        .map(ToString::to_string);
                }
                _ => {}
            }
        }

        (
            active_lesson,
            latest_infographic,
            latest_stagegate,
            latest_answer,
        )
    };
    let entries = all_entries
        .into_iter()
        .filter(|entry| entry.kind == "stagegate")
        .collect::<Vec<_>>();

    let has_passed_stagegate = latest_stagegate
        .as_ref()
        .and_then(|stagegate| stagegate.get("passed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    StudentBookState {
        student_id: student.public_id.clone(),
        book_id: book.id.to_string(),
        entries,
        active_lesson,
        latest_infographic,
        latest_stagegate,
        latest_answer,
        has_passed_stagegate,
    }
}

fn student_book_entry_record(row: student_book_entry::Model) -> StudentBookEntryRecord {
    StudentBookEntryRecord {
        entry_id: row.id.to_string(),
        kind: row.entry_kind,
        topic: row.topic,
        stage_level: row.stage_level,
        position: row.position,
        payload: row.payload,
        created_at: row.created_at.to_rfc3339(),
    }
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
        xp_total: Set(0),
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
    get_or_create_student(db, public_id, "Guest learner", None, "11-13", None, vec![]).await
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
        xp_total: student.xp_total,
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
        return Err("Age must be between 5 and 18 for this student profile.".to_string());
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
        return "visual, story-first explanations".to_string();
    }

    format!(
        "visual, story-first explanations anchored in {}",
        format_interest_list(interests)
    )
}

fn suggested_topics_for_interests(interests: &[String]) -> Vec<String> {
    let defaults = [
        "cause and effect in everyday systems",
        "building models from observations",
        "how evidence changes an explanation",
        "patterns that predict what happens next",
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

fn narrative_character_record(character: narrative_character::Model) -> NarrativeCharacter {
    NarrativeCharacter {
        character_id: character.id.to_string(),
        name: character.name,
        role: character.role,
        current_biography: character.current_biography,
        topic_affinities: string_array(character.topic_affinities),
        consistency_notes: string_array(character.consistency_notes),
        introduced_at: character.introduced_at.to_rfc3339(),
        last_seen_at: character.last_seen_at.to_rfc3339(),
        last_seen_topic: character.last_seen_topic,
    }
}

fn character_relevance_score(character: &narrative_character::Model, terms: &[String]) -> i32 {
    if terms.is_empty() {
        return 1;
    }

    let topic_affinities = string_array(character.topic_affinities.clone());
    let haystack = format!(
        "{} {} {} {} {}",
        character.name,
        character.role.as_deref().unwrap_or_default(),
        character.current_biography,
        topic_affinities.join(" "),
        character.last_seen_topic.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();

    terms
        .iter()
        .map(|term| {
            let affinity_score = topic_affinities
                .iter()
                .filter(|affinity| affinity.to_ascii_lowercase().contains(term))
                .count() as i32
                * 3;
            let text_score = if haystack.contains(term) { 1 } else { 0 };
            affinity_score + text_score
        })
        .sum()
}

fn topic_terms(topic: Option<&str>) -> Vec<String> {
    let mut terms = Vec::new();
    for term in topic
        .unwrap_or_default()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|term| term.trim().to_ascii_lowercase())
        .filter(|term| term.len() >= 3)
    {
        if !terms.contains(&term) {
            terms.push(term);
        }
    }
    terms
}

fn clean_character_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    let text = value?
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return None;
    }

    Some(text.chars().take(max_chars).collect())
}

fn clean_character_array(value: Option<&Value>, max_items: usize, max_chars: usize) -> Vec<String> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut output = Vec::new();
    for item in items {
        let Some(text) = clean_character_text(item.as_str(), max_chars) else {
            continue;
        };
        push_unique_string(&mut output, text);
        if output.len() >= max_items {
            break;
        }
    }
    output
}

fn merge_character_arrays(existing: Value, incoming: Vec<String>, max_items: usize) -> Vec<String> {
    let mut merged = string_array(existing)
        .into_iter()
        .filter_map(|item| clean_character_text(Some(&item), 200))
        .collect::<Vec<_>>();

    for item in incoming {
        push_unique_string(&mut merged, item);
        if merged.len() >= max_items {
            break;
        }
    }

    merged
}

fn push_unique_clean_string(items: &mut Vec<String>, value: &str, max_chars: usize) {
    if let Some(clean) = clean_character_text(Some(value), max_chars) {
        push_unique_string(items, clean);
    }
}

fn push_unique_string(items: &mut Vec<String>, value: String) {
    if value.trim().is_empty() || items.iter().any(|item| item.eq_ignore_ascii_case(&value)) {
        return;
    }
    items.push(value);
}

fn normalize_character_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn sql_statement(sql: &str, values: Vec<SeaOrmValue>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn now() -> DateTime<FixedOffset> {
    Utc::now().fixed_offset()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signup_age_validation_requires_supported_student_age() {
        assert_eq!(validate_signup_age(Some(5)).unwrap(), Some(5));
        assert_eq!(validate_signup_age(Some(18)).unwrap(), Some(18));
        assert_eq!(validate_signup_age(None).unwrap_err(), "Age is required.");
        assert_eq!(
            validate_signup_age(Some(4)).unwrap_err(),
            "Age must be between 5 and 18 for this student profile."
        );
        assert_eq!(
            validate_signup_age(Some(19)).unwrap_err(),
            "Age must be between 5 and 18 for this student profile."
        );
    }

    #[test]
    fn profile_text_cleaners_trim_dedupe_and_bound_inputs() {
        let long_interest = "x".repeat(80);
        let interests = clean_interests(vec![
            " marine biology, drawing ,, Marine Biology ".to_string(),
            long_interest,
            "puzzles".to_string(),
        ]);

        assert_eq!(interests.len(), 4);
        assert_eq!(interests[0], "marine biology");
        assert_eq!(interests[1], "drawing");
        assert_eq!(interests[2].chars().count(), 48);
        assert_eq!(interests[3], "puzzles");

        assert_eq!(
            clean_biography(Some("  Loves   tide pools\nand machines.  ".to_string())).unwrap(),
            "Loves tide pools and machines."
        );
        assert_eq!(clean_biography(Some("   ".to_string())), None);
        assert_eq!(
            clean_biography(Some("a".repeat(1300)))
                .unwrap()
                .chars()
                .count(),
            1200
        );
    }

    #[test]
    fn profile_defaults_create_age_and_interest_sensitive_learning_context() {
        assert_eq!(age_band_for_age(Some(6)), "5-7");
        assert_eq!(age_band_for_age(Some(12)), "11-13");
        assert_eq!(age_band_for_age(None), "11-13");
        assert_eq!(
            level_context_for_age(Some(15), "14-18"),
            "high-school learner"
        );
        assert_eq!(level_context_for_age(None, "custom"), "custom learner");

        let interests = vec!["marine biology".to_string(), "drawing".to_string()];
        assert_eq!(
            preferred_style_for_interests(&interests),
            "visual, story-first explanations anchored in marine biology and drawing"
        );
        assert_eq!(
            suggested_topics_for_interests(&interests),
            vec![
                "science in marine biology",
                "science in drawing",
                "cause and effect in everyday systems",
                "building models from observations",
            ]
        );
    }

    #[test]
    fn stagegate_xp_awards_attempts_and_passes_separately() {
        assert_eq!(
            stagegate_xp_awards(&json!({ "passed": false, "score": 0.4 })),
            vec![XpAward {
                source_type: "stagegate_submitted",
                points: STAGEGATE_SUBMITTED_XP,
            }]
        );

        assert_eq!(
            stagegate_xp_awards(&json!({ "passed": true, "score": 0.9 })),
            vec![
                XpAward {
                    source_type: "stagegate_submitted",
                    points: STAGEGATE_SUBMITTED_XP,
                },
                XpAward {
                    source_type: "stagegate_passed",
                    points: STAGEGATE_PASSED_XP,
                },
            ]
        );
    }

    #[test]
    fn stagegate_xp_event_keys_are_student_topic_and_level_scoped() {
        let student_id = Uuid::parse_str("7854c3af-b0ff-42c8-9854-5bb14a37ea65").unwrap();

        assert_eq!(
            stagegate_xp_event_key(
                student_id,
                "stagegate_passed",
                "  Reef Currents! ",
                "Intuition",
            ),
            "stagegate_passed:7854c3af-b0ff-42c8-9854-5bb14a37ea65:reef-currents:intuition"
        );
        assert_eq!(xp_key_component("!!!"), "unknown");
    }

    #[test]
    fn student_record_serializes_xp_total_as_camel_case() {
        let record = StudentRecord {
            student_id: "student-123".to_string(),
            display_name: "Mina".to_string(),
            age: Some(12),
            age_band: "11-13".to_string(),
            biography: None,
            interests: vec![],
            preferred_explanation_style: "visual".to_string(),
            level_context: "middle school".to_string(),
            memories: vec![],
            progress: vec![],
            suggested_topics: vec![],
            xp_total: 60,
        };

        let value = serde_json::to_value(record).unwrap();

        assert_eq!(value["xpTotal"], json!(60));
    }

    #[test]
    fn password_hashes_verify_only_the_original_secret() {
        let hash = hash_password("correct horse battery staple").unwrap();

        assert_ne!(hash, "correct horse battery staple");
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_password("wrong password", &hash).unwrap());
    }

    #[test]
    fn character_helpers_normalize_topics_and_arrays_for_memory_updates() {
        assert_eq!(
            topic_terms(Some("Marine-biology, marine biology, AI!")),
            vec!["marine", "biology"]
        );
        assert_eq!(
            clean_character_text(Some("  Tala   the reef guide  "), 12).unwrap(),
            "Tala the ree"
        );
        assert_eq!(
            clean_character_array(
                Some(&json!([" Tala ", "", 42, "Tala", "Storm guide"])),
                4,
                20,
            ),
            vec!["Tala", "Storm guide"]
        );
        assert_eq!(
            merge_character_arrays(
                json!(["Tala", "Reef guide"]),
                vec!["tala".to_string(), "Storm guide".to_string(),],
                3
            ),
            vec!["Tala", "Reef guide", "Storm guide"]
        );
    }

    #[test]
    fn book_state_from_rows_exposes_latest_persisted_book_content() {
        let observed_at = now();
        let student_id = Uuid::new_v4();
        let book_id = Uuid::new_v4();
        let student = student::Model {
            id: student_id,
            public_id: "student-123".to_string(),
            display_name: "Mina".to_string(),
            age_years: Some(11),
            age_band: "11-13".to_string(),
            biography: Some("Loves tide pools.".to_string()),
            interests: json!(["marine biology"]),
            preferred_explanation_style: "visual".to_string(),
            level_context: "middle school".to_string(),
            suggested_topics: json!([]),
            xp_total: 50,
            created_at: observed_at,
            updated_at: observed_at,
        };
        let book = student_book::Model {
            id: book_id,
            student_id,
            title: "Mina's Primer".to_string(),
            status: "active".to_string(),
            active_lesson: None,
            latest_infographic: None,
            latest_stagegate: None,
            latest_answer: None,
            created_at: observed_at,
            updated_at: observed_at,
        };
        let rows = vec![
            student_book_entry::Model {
                id: Uuid::new_v4(),
                book_id,
                student_id,
                entry_kind: "lesson".to_string(),
                topic: Some("reef currents".to_string()),
                stage_level: Some("intuition".to_string()),
                position: 1,
                payload: json!({
                    "lesson": {
                        "topic": "reef currents",
                        "stageLevel": "intuition"
                    }
                }),
                created_at: observed_at,
            },
            student_book_entry::Model {
                id: Uuid::new_v4(),
                book_id,
                student_id,
                entry_kind: "infographic".to_string(),
                topic: Some("reef currents".to_string()),
                stage_level: None,
                position: 2,
                payload: json!({
                    "artifact": {
                        "generated": true,
                        "model": "gpt-image-2"
                    }
                }),
                created_at: observed_at,
            },
            student_book_entry::Model {
                id: Uuid::new_v4(),
                book_id,
                student_id,
                entry_kind: "stagegate".to_string(),
                topic: Some("reef currents".to_string()),
                stage_level: Some("intuition".to_string()),
                position: 3,
                payload: json!({
                    "request": {
                        "answer": "Currents move when forces push water."
                    },
                    "result": {
                        "passed": true,
                        "score": 0.88
                    }
                }),
                created_at: observed_at,
            },
        ];

        let state = book_state_from_rows(&student, &book, rows);

        assert_eq!(state.student_id, "student-123");
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].kind, "stagegate");
        assert_eq!(
            state.active_lesson.unwrap()["topic"],
            json!("reef currents")
        );
        assert_eq!(
            state.latest_infographic.unwrap()["model"],
            json!("gpt-image-2")
        );
        assert!(state.has_passed_stagegate);
        assert_eq!(
            state.latest_answer.as_deref(),
            Some("Currents move when forces push water.")
        );
    }

    #[test]
    fn current_book_state_keeps_non_stagegate_rows_out_of_the_visible_book() {
        let observed_at = now();
        let student_id = Uuid::new_v4();
        let book_id = Uuid::new_v4();
        let student = student::Model {
            id: student_id,
            public_id: "student-123".to_string(),
            display_name: "Mina".to_string(),
            age_years: Some(11),
            age_band: "11-13".to_string(),
            biography: Some("Loves tide pools.".to_string()),
            interests: json!(["marine biology"]),
            preferred_explanation_style: "visual".to_string(),
            level_context: "middle school".to_string(),
            suggested_topics: json!([]),
            xp_total: 50,
            created_at: observed_at,
            updated_at: observed_at,
        };
        let book = student_book::Model {
            id: book_id,
            student_id,
            title: "Mina's Primer".to_string(),
            status: "active".to_string(),
            active_lesson: Some(json!({
                "topic": "basketball arcs",
                "stageLevel": "intuition"
            })),
            latest_infographic: None,
            latest_stagegate: Some(json!({
                "passed": false,
                "score": 0.4
            })),
            latest_answer: Some("It loops back.".to_string()),
            created_at: observed_at,
            updated_at: observed_at,
        };
        let rows = vec![
            student_book_entry::Model {
                id: Uuid::new_v4(),
                book_id,
                student_id,
                entry_kind: "lesson".to_string(),
                topic: Some("old lesson".to_string()),
                stage_level: Some("intuition".to_string()),
                position: 1,
                payload: json!({ "lesson": { "topic": "old lesson" } }),
                created_at: observed_at,
            },
            student_book_entry::Model {
                id: Uuid::new_v4(),
                book_id,
                student_id,
                entry_kind: "stagegate".to_string(),
                topic: Some("basketball arcs".to_string()),
                stage_level: Some("intuition".to_string()),
                position: 2,
                payload: json!({
                    "request": {
                        "answer": "It loops back."
                    },
                    "result": {
                        "passed": false,
                        "score": 0.4
                    }
                }),
                created_at: observed_at,
            },
        ];

        let state = book_state_from_rows(&student, &book, rows);

        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].kind, "stagegate");
        assert_eq!(
            state.active_lesson.unwrap()["topic"],
            json!("basketball arcs")
        );
        assert!(!state.has_passed_stagegate);
        assert_eq!(state.latest_answer.as_deref(), Some("It loops back."));
    }
}
