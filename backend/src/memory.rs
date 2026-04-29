use std::collections::BTreeMap;

use crate::{
    domain::{
        MemoryAssertionRecord, MemoryEntityRecord, MemoryGraphEdge, MemoryGraphNode,
        MemoryGraphRequest, MemoryProfileRequest, StudentMemory, StudentMemoryGraph,
        StudentMemoryProfile,
    },
    entities::student,
};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryResult, Statement,
    TransactionTrait, Value,
};
use serde_json::{Value as JsonValue, json};
use uuid::Uuid;

const GRAPH_NAME: &str = "primer_memory";

pub async fn init_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    for sql in memory_schema_sql() {
        db.execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
            .await?;
    }

    Ok(())
}

pub async fn assert_student_memory(
    db: &DatabaseConnection,
    student: &student::Model,
    memory_type: &str,
    content: &str,
    confidence: f64,
    tags: JsonValue,
    source_ref: &str,
) -> Result<(), DbErr> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let source_id = ensure_source(
        db,
        student.id,
        "manual_note",
        source_ref,
        Some(trimmed),
        json!({ "path": "seed_or_ai_memory" }),
    )
    .await?;
    let subject = ensure_student_entity(db, student).await?;
    let predicate = predicate_for_memory_type(memory_type, &tags);
    let metadata = json!({
        "display_content": trimmed,
        "memory_type": memory_type,
    });
    insert_assertion_if_absent(
        db,
        student.id,
        source_id,
        subject,
        predicate,
        None,
        Some(trimmed),
        None,
        json!({ "tags": tags }),
        confidence,
        salience_for_memory_type(memory_type),
        metadata,
    )
    .await?;

    Ok(())
}

pub async fn record_lesson_started(
    db: &DatabaseConnection,
    student: &student::Model,
    topic: &str,
    lesson: &JsonValue,
) -> Result<(), DbErr> {
    let clean_topic = topic.trim();
    if clean_topic.is_empty() {
        return Ok(());
    }

    let source_id = ensure_source(
        db,
        student.id,
        "lesson_event",
        &format!("lesson-start:{clean_topic}"),
        lesson.get("plainExplanation").and_then(JsonValue::as_str),
        json!({ "topic": clean_topic, "event": "lesson_started" }),
    )
    .await?;
    let subject = ensure_student_entity(db, student).await?;
    let concept = ensure_entity(
        db,
        student.id,
        "concept",
        clean_topic,
        &format!("concept:{clean_topic}"),
        json!({ "topic": clean_topic }),
    )
    .await?;
    insert_assertion_if_absent(
        db,
        student.id,
        source_id,
        subject,
        "explored_topic",
        Some(concept),
        Some(&format!(
            "Learner started a guided exploration of {clean_topic}."
        )),
        None,
        json!({
            "tags": ["history", clean_topic],
            "topic": clean_topic,
            "stageLevel": lesson.get("stageLevel").and_then(JsonValue::as_str),
        }),
        0.82,
        0.62,
        json!({
            "display_content": format!("Learner started a guided exploration of {clean_topic}."),
            "memory_type": "history",
        }),
    )
    .await?;

    Ok(())
}

pub async fn record_stagegate_result(
    db: &DatabaseConnection,
    student: &student::Model,
    topic: &str,
    stage_level: &str,
    result: &JsonValue,
) -> Result<(), DbErr> {
    let score = result
        .get("score")
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let passed = result
        .get("passed")
        .and_then(JsonValue::as_bool)
        .unwrap_or(score >= 0.75);

    let source_id = ensure_source(
        db,
        student.id,
        "stagegate_event",
        &format!("stagegate:{topic}:{stage_level}:{passed}"),
        result.get("feedbackToStudent").and_then(JsonValue::as_str),
        json!({
            "topic": topic,
            "stageLevel": stage_level,
            "passed": passed,
            "score": score,
        }),
    )
    .await?;
    let subject = ensure_student_entity(db, student).await?;
    let concept = ensure_entity(
        db,
        student.id,
        "concept",
        topic,
        &format!("concept:{topic}"),
        json!({ "topic": topic }),
    )
    .await?;
    let status_text = if passed { "passed" } else { "practiced" };
    insert_assertion_if_absent(
        db,
        student.id,
        source_id,
        subject,
        "stagegate_result",
        Some(concept),
        Some(&format!(
            "Learner {status_text} {topic} at the {stage_level} level with score {score:.2}."
        )),
        None,
        json!({
            "tags": ["mastery", topic, stage_level],
            "topic": topic,
            "stageLevel": stage_level,
            "passed": passed,
            "score": score,
            "masteryEvidence": result.get("masteryEvidence").cloned().unwrap_or_else(|| json!([])),
        }),
        if passed { 0.92 } else { 0.78 },
        if passed { 0.9 } else { 0.72 },
        json!({
            "display_content": format!("Learner {status_text} {topic} at the {stage_level} level."),
            "memory_type": if passed { "knowledge" } else { "history" },
        }),
    )
    .await?;

    Ok(())
}

pub async fn student_memories(
    db: &DatabaseConnection,
    student_id: Uuid,
) -> Result<Vec<StudentMemory>, DbErr> {
    let rows = db
        .query_all(statement(
            r#"
            SELECT a.id AS assertion_id,
                   a.predicate,
                   a.object_text,
                   a.qualifiers,
                   a.metadata,
                   a.confidence,
                   lower(a.valid_period) AS valid_from,
                   upper(a.valid_period) AS valid_to,
                   lower(a.tx_period) AS known_from,
                   upper(a.tx_period) AS known_to,
                   a.observed_at,
                   src.source_type,
                   s.canonical_name AS subject_name,
                   o.canonical_name AS object_name
            FROM primer_current_beliefs a
            JOIN memory_entities s ON s.id = a.subject_entity_id
            LEFT JOIN memory_entities o ON o.id = a.object_entity_id
            LEFT JOIN memory_sources src ON src.id = a.source_id
            WHERE a.student_id = $1
              AND a.status = 'active'
            ORDER BY a.salience DESC, a.observed_at DESC, a.id
            LIMIT 20
            "#,
            vec![student_id.into()],
        ))
        .await?;

    rows.into_iter()
        .map(|row| student_memory_from_row(&row))
        .collect()
}

pub async fn profile_for_student(
    db: &DatabaseConnection,
    public_id: &str,
    request: MemoryProfileRequest,
) -> Result<Option<StudentMemoryProfile>, DbErr> {
    let student_row = db
        .query_one(statement(
            r#"
            SELECT id, public_id, display_name
            FROM students
            WHERE public_id = $1
            "#,
            vec![public_id.to_string().into()],
        ))
        .await?;
    let Some(student_row) = student_row else {
        return Ok(None);
    };
    let student_id: Uuid = student_row.try_get("", "id")?;

    let entity_row = db
        .query_one(statement(
            r#"
            SELECT id, kind, canonical_name, identity_key, normalized_key, properties
            FROM memory_entities
            WHERE student_id = $1
              AND kind = 'student'
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
            vec![student_id.into()],
        ))
        .await?;
    let Some(entity_row) = entity_row else {
        return Ok(None);
    };
    let profile_entity_id: Uuid = entity_row.try_get("", "id")?;
    let entity = entity_record_from_row(&entity_row)?;

    let valid_as_of = request.valid_as_of.unwrap_or_else(Utc::now).fixed_offset();
    let known_as_of = request.known_as_of.unwrap_or_else(Utc::now).fixed_offset();
    let max_facts = i64::from(request.max_facts.unwrap_or(24).clamp(1, 100));

    let subject_facts = profile_assertions(
        db,
        student_id,
        profile_entity_id,
        "subject_entity_id",
        valid_as_of,
        known_as_of,
        max_facts,
    )
    .await?;
    let inbound_facts = profile_assertions(
        db,
        student_id,
        profile_entity_id,
        "object_entity_id",
        valid_as_of,
        known_as_of,
        max_facts,
    )
    .await?;
    let timeline = timeline_assertions(db, student_id, valid_as_of, known_as_of, max_facts).await?;

    Ok(Some(StudentMemoryProfile {
        student_id: public_id.to_string(),
        entity,
        subject_facts,
        inbound_facts,
        timeline,
        valid_as_of: valid_as_of.to_rfc3339(),
        known_as_of: known_as_of.to_rfc3339(),
    }))
}

pub async fn graph_for_student(
    db: &DatabaseConnection,
    public_id: &str,
    request: MemoryGraphRequest,
) -> Result<Option<StudentMemoryGraph>, DbErr> {
    let Some(student_id) = student_id_for_public_id(db, public_id).await? else {
        return Ok(None);
    };
    let Some((root_entity_id, root_entity)) = root_student_entity(db, student_id).await? else {
        return Ok(None);
    };

    let valid_as_of = request.valid_as_of.unwrap_or_else(Utc::now).fixed_offset();
    let known_as_of = request.known_as_of.unwrap_or_else(Utc::now).fixed_offset();
    let max_edges = i64::from(request.max_edges.unwrap_or(24).clamp(1, 100));
    let root_node_id = entity_node_id(root_entity_id);
    let requested_node_id = request.node_id.as_deref().unwrap_or(&root_node_id);
    let selected = GraphSelection::from_node_id(requested_node_id)
        .unwrap_or(GraphSelection::Entity(root_entity_id));

    let rows = match selected {
        GraphSelection::Entity(entity_id) => {
            graph_assertions_for_entity(
                db,
                student_id,
                entity_id,
                valid_as_of,
                known_as_of,
                max_edges,
            )
            .await?
        }
        GraphSelection::Value(assertion_id) => {
            graph_assertions_for_value(db, student_id, assertion_id, valid_as_of, known_as_of)
                .await?
        }
    };

    let selected_node_id = selected.node_id();
    let mut nodes = BTreeMap::new();
    nodes.insert(root_node_id.clone(), root_entity);

    if let GraphSelection::Entity(entity_id) = selected {
        if let Some(selected_entity) = graph_entity_node(db, student_id, entity_id, true).await? {
            nodes.insert(entity_node_id(entity_id), selected_entity);
        }
    }

    let mut edges = Vec::new();
    for row in rows {
        let subject_node_id = entity_node_id(row.subject_entity_id);
        nodes.entry(subject_node_id.clone()).or_insert_with(|| {
            graph_node(
                &subject_node_id,
                "entity",
                &row.subject_kind,
                &row.subject_name,
                None,
                row.subject_entity_id == selected.entity_id().unwrap_or_default(),
            )
        });

        let target_node_id = if let Some(object_entity_id) = row.object_entity_id {
            let object_node_id = entity_node_id(object_entity_id);
            nodes.entry(object_node_id.clone()).or_insert_with(|| {
                graph_node(
                    &object_node_id,
                    "entity",
                    row.object_kind.as_deref().unwrap_or("memory"),
                    row.object_name.as_deref().unwrap_or("Memory node"),
                    None,
                    object_entity_id == selected.entity_id().unwrap_or_default(),
                )
            });
            object_node_id
        } else {
            let value_node_id = value_node_id(row.assertion_id);
            nodes.entry(value_node_id.clone()).or_insert_with(|| {
                graph_node(
                    &value_node_id,
                    "value",
                    &row.memory_type,
                    &shorten_label(&row.content),
                    Some(row.content.clone()),
                    row.assertion_id == selected.value_assertion_id().unwrap_or_default(),
                )
            });
            value_node_id
        };

        increment_fact_count(&mut nodes, &subject_node_id);
        increment_fact_count(&mut nodes, &target_node_id);

        edges.push(MemoryGraphEdge {
            id: row.assertion_id.to_string(),
            source: subject_node_id,
            target: target_node_id,
            label: row.predicate.replace('_', " "),
            assertion_id: row.assertion_id.to_string(),
            predicate: row.predicate,
            content: row.content,
            memory_type: row.memory_type,
            confidence: row.confidence,
            observed_at: row.observed_at.to_rfc3339(),
            valid_from: row.valid_from.map(|value| value.to_rfc3339()),
            known_from: row.known_from.map(|value| value.to_rfc3339()),
        });
    }

    Ok(Some(StudentMemoryGraph {
        student_id: public_id.to_string(),
        root_node_id,
        selected_node_id,
        nodes: nodes.into_values().collect(),
        edges,
        valid_as_of: valid_as_of.to_rfc3339(),
        known_as_of: known_as_of.to_rfc3339(),
    }))
}

pub async fn delete_student_graph_projection(
    conn: &impl ConnectionTrait,
    student_id: Uuid,
) -> Result<(), DbErr> {
    prepare_age(conn).await?;
    let query = delete_student_graph_projection_query(student_id);
    let cypher = format!(
        r#"
        SELECT *
        FROM ag_catalog.cypher({graph}, {query}) AS (deleted ag_catalog.agtype)
        "#,
        graph = sql_string(GRAPH_NAME),
        query = sql_dollar_string(&query),
    );
    conn.execute(raw_statement(&cypher)).await?;
    Ok(())
}

fn delete_student_graph_projection_query(student_id: Uuid) -> String {
    format!(
        r#"
        MATCH (n)
        WHERE n.student_id = {student_id}
        DETACH DELETE n
        "#,
        student_id = cypher_string(&student_id.to_string()),
    )
}

async fn ensure_student_entity(
    db: &DatabaseConnection,
    student: &student::Model,
) -> Result<Uuid, DbErr> {
    ensure_entity(
        db,
        student.id,
        "student",
        &student.display_name,
        &format!("student:{}", student.public_id),
        json!({
            "publicId": student.public_id,
            "ageBand": student.age_band,
            "biography": student.biography.clone(),
        }),
    )
    .await
}

async fn ensure_entity(
    db: &DatabaseConnection,
    student_id: Uuid,
    kind: &str,
    canonical_name: &str,
    identity_key: &str,
    properties: JsonValue,
) -> Result<Uuid, DbErr> {
    let normalized_key = normalize_key(identity_key);
    if let Some(row) = db
        .query_one(statement(
            r#"
            SELECT id
            FROM memory_entities
            WHERE student_id = $1
              AND kind = $2
              AND normalized_key = $3
            "#,
            vec![
                student_id.into(),
                kind.to_string().into(),
                normalized_key.clone().into(),
            ],
        ))
        .await?
    {
        return row.try_get("", "id");
    }

    let entity_id = Uuid::new_v4();
    db.execute(statement(
        r#"
        INSERT INTO memory_entities (
          id, student_id, kind, canonical_name, identity_key, normalized_key,
          aliases, properties, sensitivity, status, age_label, created_at, updated_at
        ) VALUES (
          $1, $2, $3, $4, $5, $6, '[]'::jsonb, $7, 'normal', 'active', $8, now(), now()
        )
        "#,
        vec![
            entity_id.into(),
            student_id.into(),
            kind.to_string().into(),
            canonical_name.to_string().into(),
            identity_key.to_string().into(),
            normalized_key.into(),
            properties.into(),
            age_label_for_kind(kind).into(),
        ],
    ))
    .await?;
    let _ = project_entity(
        db,
        entity_id,
        student_id,
        kind,
        canonical_name,
        identity_key,
    )
    .await;

    Ok(entity_id)
}

async fn ensure_source(
    db: &DatabaseConnection,
    student_id: Uuid,
    source_type: &str,
    external_ref: &str,
    raw_text: Option<&str>,
    metadata: JsonValue,
) -> Result<Uuid, DbErr> {
    if let Some(row) = db
        .query_one(statement(
            r#"
            SELECT id
            FROM memory_sources
            WHERE student_id = $1
              AND external_ref = $2
            "#,
            vec![student_id.into(), external_ref.to_string().into()],
        ))
        .await?
    {
        return row.try_get("", "id");
    }

    let source_id = Uuid::new_v4();
    db.execute(statement(
        r#"
        INSERT INTO memory_sources (
          id, student_id, source_type, external_ref, observed_at, source_time,
          raw_text, metadata, sensitivity, created_at
        ) VALUES (
          $1, $2, $3, $4, now(), now(), $5, $6, 'normal', now()
        )
        "#,
        vec![
            source_id.into(),
            student_id.into(),
            source_type.to_string().into(),
            external_ref.to_string().into(),
            raw_text.map(ToString::to_string).into(),
            metadata.into(),
        ],
    ))
    .await?;

    Ok(source_id)
}

#[allow(clippy::too_many_arguments)]
async fn insert_assertion_if_absent(
    db: &DatabaseConnection,
    student_id: Uuid,
    source_id: Uuid,
    subject_entity_id: Uuid,
    predicate: &str,
    object_entity_id: Option<Uuid>,
    object_text: Option<&str>,
    object_value: Option<JsonValue>,
    qualifiers: JsonValue,
    confidence: f64,
    salience: f64,
    metadata: JsonValue,
) -> Result<(), DbErr> {
    if let Some(row) = db
        .query_one(statement(
            r#"
            SELECT id
            FROM memory_assertions
            WHERE student_id = $1
              AND subject_entity_id = $2
              AND predicate = $3
              AND coalesce(object_entity_id::text, '') = coalesce($4::uuid::text, '')
              AND coalesce(object_text, '') = coalesce($5, '')
              AND tx_period @> now()
              AND status = 'active'
            LIMIT 1
            "#,
            vec![
                student_id.into(),
                subject_entity_id.into(),
                predicate.to_string().into(),
                object_entity_id.into(),
                object_text.map(ToString::to_string).into(),
            ],
        ))
        .await?
    {
        let _existing: Uuid = row.try_get("", "id")?;
        return Ok(());
    }

    let assertion_id = Uuid::new_v4();
    db.execute(statement(
        r#"
        INSERT INTO memory_assertions (
          id, student_id, subject_entity_id, predicate, object_entity_id,
          object_value, object_text, qualifiers, valid_period, tx_period,
          observed_at, source_id, confidence, salience, sensitivity, scope,
          status, metadata, age_edge_label, created_at
        ) VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8,
          tstzrange(now(), NULL, '[)'), tstzrange(now(), NULL, '[)'),
          now(), $9, $10, $11, 'normal', 'assistant',
          'active', $12, $13, now()
        )
        "#,
        vec![
            assertion_id.into(),
            student_id.into(),
            subject_entity_id.into(),
            predicate.to_string().into(),
            object_entity_id.into(),
            object_value.into(),
            object_text.map(ToString::to_string).into(),
            qualifiers.into(),
            source_id.into(),
            confidence.into(),
            salience.into(),
            metadata.into(),
            relation_label_for_predicate(predicate).into(),
        ],
    ))
    .await?;

    let _ = project_assertion(
        db,
        assertion_id,
        student_id,
        subject_entity_id,
        predicate,
        object_entity_id,
        object_text,
    )
    .await;

    Ok(())
}

async fn profile_assertions(
    db: &DatabaseConnection,
    student_id: Uuid,
    entity_id: Uuid,
    direction_column: &str,
    valid_as_of: DateTime<FixedOffset>,
    known_as_of: DateTime<FixedOffset>,
    limit: i64,
) -> Result<Vec<MemoryAssertionRecord>, DbErr> {
    let sql = format!(
        r#"
        SELECT a.id AS assertion_id,
               a.predicate,
               a.object_text,
               a.object_value,
               a.qualifiers,
               a.metadata,
               a.confidence,
               a.salience,
               lower(a.valid_period) AS valid_from,
               upper(a.valid_period) AS valid_to,
               lower(a.tx_period) AS known_from,
               upper(a.tx_period) AS known_to,
               a.observed_at,
               s.canonical_name AS subject_name,
               s.identity_key AS subject_identity_key,
               o.canonical_name AS object_name,
               o.identity_key AS object_identity_key,
               src.source_type
        FROM memory_assertions a
        JOIN memory_entities s ON s.id = a.subject_entity_id
        LEFT JOIN memory_entities o ON o.id = a.object_entity_id
        LEFT JOIN memory_sources src ON src.id = a.source_id
        WHERE a.student_id = $1
          AND a.{direction_column} = $2
          AND a.status IN ('active', 'superseded')
          AND a.valid_period @> $3
          AND a.tx_period @> $4
        ORDER BY a.salience DESC, a.observed_at DESC, a.id
        LIMIT $5
        "#
    );
    let rows = db
        .query_all(statement(
            &sql,
            vec![
                student_id.into(),
                entity_id.into(),
                valid_as_of.into(),
                known_as_of.into(),
                limit.into(),
            ],
        ))
        .await?;

    rows.into_iter()
        .map(|row| assertion_record_from_row(&row))
        .collect()
}

async fn timeline_assertions(
    db: &DatabaseConnection,
    student_id: Uuid,
    valid_as_of: DateTime<FixedOffset>,
    known_as_of: DateTime<FixedOffset>,
    limit: i64,
) -> Result<Vec<MemoryAssertionRecord>, DbErr> {
    let rows = db
        .query_all(statement(
            r#"
            SELECT a.id AS assertion_id,
                   a.predicate,
                   a.object_text,
                   a.object_value,
                   a.qualifiers,
                   a.metadata,
                   a.confidence,
                   a.salience,
                   lower(a.valid_period) AS valid_from,
                   upper(a.valid_period) AS valid_to,
                   lower(a.tx_period) AS known_from,
                   upper(a.tx_period) AS known_to,
                   a.observed_at,
                   s.canonical_name AS subject_name,
                   s.identity_key AS subject_identity_key,
                   o.canonical_name AS object_name,
                   o.identity_key AS object_identity_key,
                   src.source_type
            FROM memory_assertions a
            JOIN memory_entities s ON s.id = a.subject_entity_id
            LEFT JOIN memory_entities o ON o.id = a.object_entity_id
            LEFT JOIN memory_sources src ON src.id = a.source_id
            WHERE a.student_id = $1
              AND a.status IN ('active', 'superseded')
              AND a.valid_period @> $2
              AND a.tx_period @> $3
            ORDER BY a.observed_at DESC, a.salience DESC, a.id
            LIMIT $4
            "#,
            vec![
                student_id.into(),
                valid_as_of.into(),
                known_as_of.into(),
                limit.into(),
            ],
        ))
        .await?;

    rows.into_iter()
        .map(|row| assertion_record_from_row(&row))
        .collect()
}

async fn student_id_for_public_id(
    db: &DatabaseConnection,
    public_id: &str,
) -> Result<Option<Uuid>, DbErr> {
    db.query_one(statement(
        "SELECT id FROM students WHERE public_id = $1",
        vec![public_id.to_string().into()],
    ))
    .await?
    .map(|row| row.try_get("", "id"))
    .transpose()
}

async fn root_student_entity(
    db: &DatabaseConnection,
    student_id: Uuid,
) -> Result<Option<(Uuid, MemoryGraphNode)>, DbErr> {
    let Some(row) = db
        .query_one(statement(
            r#"
            SELECT id, kind, canonical_name, identity_key
            FROM memory_entities
            WHERE student_id = $1
              AND kind = 'student'
              AND status = 'active'
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
            vec![student_id.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    let entity_id: Uuid = row.try_get("", "id")?;
    let kind: String = row.try_get("", "kind")?;
    let canonical_name: String = row.try_get("", "canonical_name")?;

    Ok(Some((
        entity_id,
        graph_node(
            &entity_node_id(entity_id),
            "entity",
            &kind,
            &canonical_name,
            None,
            true,
        ),
    )))
}

async fn graph_entity_node(
    db: &DatabaseConnection,
    student_id: Uuid,
    entity_id: Uuid,
    expanded: bool,
) -> Result<Option<MemoryGraphNode>, DbErr> {
    let Some(row) = db
        .query_one(statement(
            r#"
            SELECT id, kind, canonical_name
            FROM memory_entities
            WHERE student_id = $1
              AND id = $2
              AND status = 'active'
            "#,
            vec![student_id.into(), entity_id.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    let kind: String = row.try_get("", "kind")?;
    let canonical_name: String = row.try_get("", "canonical_name")?;

    Ok(Some(graph_node(
        &entity_node_id(entity_id),
        "entity",
        &kind,
        &canonical_name,
        None,
        expanded,
    )))
}

async fn graph_assertions_for_entity(
    db: &DatabaseConnection,
    student_id: Uuid,
    entity_id: Uuid,
    valid_as_of: DateTime<FixedOffset>,
    known_as_of: DateTime<FixedOffset>,
    limit: i64,
) -> Result<Vec<GraphAssertionRow>, DbErr> {
    let rows = db
        .query_all(statement(
            graph_assertion_select_sql(
                r#"
                a.student_id = $1
                AND (a.subject_entity_id = $2 OR a.object_entity_id = $2)
                AND a.status IN ('active', 'superseded')
                AND a.valid_period @> $3
                AND a.tx_period @> $4
                ORDER BY a.salience DESC, a.observed_at DESC, a.id
                LIMIT $5
                "#,
            )
            .as_str(),
            vec![
                student_id.into(),
                entity_id.into(),
                valid_as_of.into(),
                known_as_of.into(),
                limit.into(),
            ],
        ))
        .await?;

    rows.into_iter()
        .map(|row| graph_row_from_row(&row))
        .collect()
}

async fn graph_assertions_for_value(
    db: &DatabaseConnection,
    student_id: Uuid,
    assertion_id: Uuid,
    valid_as_of: DateTime<FixedOffset>,
    known_as_of: DateTime<FixedOffset>,
) -> Result<Vec<GraphAssertionRow>, DbErr> {
    let rows = db
        .query_all(statement(
            graph_assertion_select_sql(
                r#"
                a.student_id = $1
                AND a.id = $2
                AND a.status IN ('active', 'superseded')
                AND a.valid_period @> $3
                AND a.tx_period @> $4
                LIMIT 1
                "#,
            )
            .as_str(),
            vec![
                student_id.into(),
                assertion_id.into(),
                valid_as_of.into(),
                known_as_of.into(),
            ],
        ))
        .await?;

    rows.into_iter()
        .map(|row| graph_row_from_row(&row))
        .collect()
}

fn graph_assertion_select_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT a.id AS assertion_id,
               a.predicate,
               a.object_entity_id,
               a.object_text,
               a.object_value,
               a.qualifiers,
               a.metadata,
               a.confidence,
               a.salience,
               lower(a.valid_period) AS valid_from,
               lower(a.tx_period) AS known_from,
               a.observed_at,
               s.id AS subject_entity_id,
               s.kind AS subject_kind,
               s.canonical_name AS subject_name,
               s.identity_key AS subject_identity_key,
               o.kind AS object_kind,
               o.canonical_name AS object_name,
               o.identity_key AS object_identity_key
        FROM memory_assertions a
        JOIN memory_entities s ON s.id = a.subject_entity_id
        LEFT JOIN memory_entities o ON o.id = a.object_entity_id
        WHERE {where_clause}
        "#
    )
}

#[derive(Clone)]
struct GraphAssertionRow {
    assertion_id: Uuid,
    predicate: String,
    subject_entity_id: Uuid,
    subject_kind: String,
    subject_name: String,
    object_entity_id: Option<Uuid>,
    object_kind: Option<String>,
    object_name: Option<String>,
    content: String,
    memory_type: String,
    confidence: f64,
    valid_from: Option<DateTime<FixedOffset>>,
    known_from: Option<DateTime<FixedOffset>>,
    observed_at: DateTime<FixedOffset>,
}

fn graph_row_from_row(row: &QueryResult) -> Result<GraphAssertionRow, DbErr> {
    let assertion_id: Uuid = row.try_get("", "assertion_id")?;
    let predicate: String = row.try_get("", "predicate")?;
    let object_text: Option<String> = row.try_get("", "object_text")?;
    let metadata: JsonValue = row.try_get("", "metadata")?;
    let object_name: Option<String> = row.try_get("", "object_name")?;
    let content = display_content(&metadata, object_text.as_deref(), object_name.as_deref());
    let memory_type = metadata
        .get("memory_type")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| memory_type_for_predicate(&predicate).to_string());

    Ok(GraphAssertionRow {
        assertion_id,
        predicate,
        subject_entity_id: row.try_get("", "subject_entity_id")?,
        subject_kind: row.try_get("", "subject_kind")?,
        subject_name: row.try_get("", "subject_name")?,
        object_entity_id: row.try_get("", "object_entity_id")?,
        object_kind: row.try_get("", "object_kind")?,
        object_name,
        content,
        memory_type,
        confidence: row.try_get("", "confidence")?,
        valid_from: row.try_get("", "valid_from")?,
        known_from: row.try_get("", "known_from")?,
        observed_at: row.try_get("", "observed_at")?,
    })
}

#[derive(Clone, Copy)]
enum GraphSelection {
    Entity(Uuid),
    Value(Uuid),
}

impl GraphSelection {
    fn from_node_id(node_id: &str) -> Option<Self> {
        parse_entity_node_id(node_id)
            .map(Self::Entity)
            .or_else(|| parse_value_node_id(node_id).map(Self::Value))
    }

    fn node_id(self) -> String {
        match self {
            Self::Entity(entity_id) => entity_node_id(entity_id),
            Self::Value(assertion_id) => value_node_id(assertion_id),
        }
    }

    fn entity_id(self) -> Option<Uuid> {
        match self {
            Self::Entity(entity_id) => Some(entity_id),
            Self::Value(_) => None,
        }
    }

    fn value_assertion_id(self) -> Option<Uuid> {
        match self {
            Self::Entity(_) => None,
            Self::Value(assertion_id) => Some(assertion_id),
        }
    }
}

fn graph_node(
    id: &str,
    node_type: &str,
    kind: &str,
    label: &str,
    summary: Option<String>,
    expanded: bool,
) -> MemoryGraphNode {
    MemoryGraphNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        kind: kind.to_string(),
        label: shorten_label(label),
        summary,
        expanded,
        fact_count: 0,
    }
}

fn increment_fact_count(nodes: &mut BTreeMap<String, MemoryGraphNode>, node_id: &str) {
    if let Some(node) = nodes.get_mut(node_id) {
        node.fact_count += 1;
    }
}

fn entity_node_id(entity_id: Uuid) -> String {
    format!("entity:{entity_id}")
}

fn value_node_id(assertion_id: Uuid) -> String {
    format!("value:{assertion_id}")
}

fn parse_entity_node_id(node_id: &str) -> Option<Uuid> {
    node_id
        .strip_prefix("entity:")
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn parse_value_node_id(node_id: &str) -> Option<Uuid> {
    node_id
        .strip_prefix("value:")
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn shorten_label(label: &str) -> String {
    const MAX_CHARS: usize = 58;
    let trimmed = label.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }

    let mut output = trimmed.chars().take(MAX_CHARS).collect::<String>();
    output.push_str("...");
    output
}

async fn project_entity(
    db: &DatabaseConnection,
    entity_id: Uuid,
    student_id: Uuid,
    kind: &str,
    canonical_name: &str,
    identity_key: &str,
) -> Result<(), DbErr> {
    let tx = db.begin().await?;
    prepare_age(&tx).await?;
    let label = validate_age_identifier(age_label_for_kind(kind))?;
    let query = format!(
        r#"
        MERGE (e:{label} {{uuid: {entity_id}, student_id: {student_id}}})
        SET e.kind = {kind},
            e.canonical_name = {canonical_name},
            e.identity_key = {identity_key},
            e.status = 'active'
        RETURN id(e) AS graph_id
        "#,
        entity_id = cypher_string(&entity_id.to_string()),
        student_id = cypher_string(&student_id.to_string()),
        kind = cypher_string(kind),
        canonical_name = cypher_string(canonical_name),
        identity_key = cypher_string(identity_key),
    );
    let cypher = format!(
        r#"
        SELECT graph_id::text AS graph_id
        FROM ag_catalog.cypher({graph}, {query}) AS (graph_id ag_catalog.agtype)
        "#,
        graph = sql_string(GRAPH_NAME),
        query = sql_dollar_string(&query),
    );
    let row = tx.query_one(raw_statement(&cypher)).await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(());
    };
    let graph_id: String = row.try_get("", "graph_id")?;
    tx.execute(statement(
        "UPDATE memory_entities SET age_graph_id = $1, age_label = $2 WHERE id = $3",
        vec![
            clean_agtype_string(&graph_id).into(),
            label.into(),
            entity_id.into(),
        ],
    ))
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn project_assertion(
    db: &DatabaseConnection,
    assertion_id: Uuid,
    student_id: Uuid,
    subject_entity_id: Uuid,
    predicate: &str,
    object_entity_id: Option<Uuid>,
    object_text: Option<&str>,
) -> Result<(), DbErr> {
    let tx = db.begin().await?;
    prepare_age(&tx).await?;
    let relation_label = relation_label_for_predicate(predicate);
    let label = validate_age_identifier(&relation_label)?;
    let query = if let Some(object_entity_id) = object_entity_id {
        format!(
            r#"
            MATCH (s {{uuid: {subject_id}, student_id: {student_id}}})
            MATCH (o {{uuid: {object_id}, student_id: {student_id}}})
            MERGE (s)-[r:{label} {{assertion_id: {assertion_id}}}]->(o)
            SET r.predicate = {predicate},
                r.status = 'active'
            RETURN {assertion_id} AS graph_id
            "#,
            subject_id = cypher_string(&subject_entity_id.to_string()),
            object_id = cypher_string(&object_entity_id.to_string()),
            student_id = cypher_string(&student_id.to_string()),
            assertion_id = cypher_string(&assertion_id.to_string()),
            predicate = cypher_string(predicate),
        )
    } else {
        let value_node_id = format!("value:{assertion_id}");
        format!(
            r#"
            MATCH (s {{uuid: {subject_id}, student_id: {student_id}}})
            MERGE (o:Value {{uuid: {value_node_id}, student_id: {student_id}}})
            SET o.kind = 'value',
                o.canonical_name = {object_text},
                o.assertion_id = {assertion_id},
                o.status = 'active'
            MERGE (s)-[r:{label} {{assertion_id: {assertion_id}}}]->(o)
            SET r.predicate = {predicate},
                r.status = 'active'
            RETURN {assertion_id} AS graph_id
            "#,
            subject_id = cypher_string(&subject_entity_id.to_string()),
            value_node_id = cypher_string(&value_node_id),
            student_id = cypher_string(&student_id.to_string()),
            assertion_id = cypher_string(&assertion_id.to_string()),
            object_text = cypher_string(object_text.unwrap_or("[value]")),
            predicate = cypher_string(predicate),
        )
    };
    let cypher = format!(
        r#"
        SELECT graph_id::text AS graph_id
        FROM ag_catalog.cypher({graph}, {query}) AS (graph_id ag_catalog.agtype)
        "#,
        graph = sql_string(GRAPH_NAME),
        query = sql_dollar_string(&query),
    );
    let _ = tx.query_one(raw_statement(&cypher)).await?;
    tx.execute(statement(
        "UPDATE memory_assertions SET age_graph_id = $1, age_edge_label = $2 WHERE id = $3",
        vec![
            assertion_id.to_string().into(),
            label.into(),
            assertion_id.into(),
        ],
    ))
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn prepare_age(conn: &impl ConnectionTrait) -> Result<(), DbErr> {
    conn.execute(raw_statement("LOAD 'age'")).await?;
    conn.execute(raw_statement(
        r#"SET search_path = ag_catalog, "$user", public"#,
    ))
    .await?;
    Ok(())
}

fn student_memory_from_row(row: &QueryResult) -> Result<StudentMemory, DbErr> {
    let assertion_id: Uuid = row.try_get("", "assertion_id")?;
    let predicate: String = row.try_get("", "predicate")?;
    let object_text: Option<String> = row.try_get("", "object_text")?;
    let object_name: Option<String> = row.try_get("", "object_name")?;
    let qualifiers: JsonValue = row.try_get("", "qualifiers")?;
    let metadata: JsonValue = row.try_get("", "metadata")?;
    let confidence: f64 = row.try_get("", "confidence")?;
    let valid_from: Option<DateTime<FixedOffset>> = row.try_get("", "valid_from")?;
    let valid_to: Option<DateTime<FixedOffset>> = row.try_get("", "valid_to")?;
    let known_from: Option<DateTime<FixedOffset>> = row.try_get("", "known_from")?;
    let known_to: Option<DateTime<FixedOffset>> = row.try_get("", "known_to")?;
    let source_type: Option<String> = row.try_get("", "source_type")?;
    let subject_name: String = row.try_get("", "subject_name")?;
    let content = display_content(&metadata, object_text.as_deref(), object_name.as_deref());

    Ok(StudentMemory {
        assertion_id: Some(assertion_id.to_string()),
        memory_type: metadata
            .get("memory_type")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| memory_type_for_predicate(&predicate).to_string()),
        content,
        confidence: confidence as f32,
        tags: tags_from_qualifiers(&qualifiers),
        subject: Some(subject_name),
        predicate: Some(predicate),
        valid_from: valid_from.map(|value| value.to_rfc3339()),
        valid_to: valid_to.map(|value| value.to_rfc3339()),
        known_from: known_from.map(|value| value.to_rfc3339()),
        known_to: known_to.map(|value| value.to_rfc3339()),
        source: source_type,
    })
}

fn assertion_record_from_row(row: &QueryResult) -> Result<MemoryAssertionRecord, DbErr> {
    let assertion_id: Uuid = row.try_get("", "assertion_id")?;
    let predicate: String = row.try_get("", "predicate")?;
    let object_text: Option<String> = row.try_get("", "object_text")?;
    let object_value: Option<JsonValue> = row.try_get("", "object_value")?;
    let qualifiers: JsonValue = row.try_get("", "qualifiers")?;
    let metadata: JsonValue = row.try_get("", "metadata")?;
    let confidence: f64 = row.try_get("", "confidence")?;
    let salience: f64 = row.try_get("", "salience")?;
    let valid_from: Option<DateTime<FixedOffset>> = row.try_get("", "valid_from")?;
    let valid_to: Option<DateTime<FixedOffset>> = row.try_get("", "valid_to")?;
    let known_from: Option<DateTime<FixedOffset>> = row.try_get("", "known_from")?;
    let known_to: Option<DateTime<FixedOffset>> = row.try_get("", "known_to")?;
    let observed_at: DateTime<FixedOffset> = row.try_get("", "observed_at")?;
    let subject_name: String = row.try_get("", "subject_name")?;
    let subject_identity_key: String = row.try_get("", "subject_identity_key")?;
    let object_name: Option<String> = row.try_get("", "object_name")?;
    let object_identity_key: Option<String> = row.try_get("", "object_identity_key")?;
    let source_type: Option<String> = row.try_get("", "source_type")?;
    let content = display_content(&metadata, object_text.as_deref(), object_name.as_deref());

    Ok(MemoryAssertionRecord {
        assertion_id: assertion_id.to_string(),
        subject: subject_name,
        subject_identity_key,
        predicate,
        object: object_name,
        object_identity_key,
        object_text,
        object_value,
        content,
        memory_type: metadata
            .get("memory_type")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| "knowledge".to_string()),
        confidence,
        salience,
        tags: tags_from_qualifiers(&qualifiers),
        qualifiers,
        valid_from: valid_from.map(|value| value.to_rfc3339()),
        valid_to: valid_to.map(|value| value.to_rfc3339()),
        known_from: known_from.map(|value| value.to_rfc3339()),
        known_to: known_to.map(|value| value.to_rfc3339()),
        observed_at: observed_at.to_rfc3339(),
        source: source_type,
    })
}

fn entity_record_from_row(row: &QueryResult) -> Result<MemoryEntityRecord, DbErr> {
    let entity_id: Uuid = row.try_get("", "id")?;
    Ok(MemoryEntityRecord {
        entity_id: entity_id.to_string(),
        kind: row.try_get("", "kind")?,
        canonical_name: row.try_get("", "canonical_name")?,
        identity_key: row.try_get("", "identity_key")?,
        normalized_key: row.try_get("", "normalized_key")?,
        properties: row.try_get("", "properties")?,
    })
}

fn display_content(
    metadata: &JsonValue,
    object_text: Option<&str>,
    object_name: Option<&str>,
) -> String {
    metadata
        .get("display_content")
        .and_then(JsonValue::as_str)
        .or(object_text)
        .or(object_name)
        .unwrap_or("Memory assertion")
        .to_string()
}

fn tags_from_qualifiers(qualifiers: &JsonValue) -> Vec<String> {
    qualifiers
        .get("tags")
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(JsonValue::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn predicate_for_memory_type(memory_type: &str, tags: &JsonValue) -> &'static str {
    match memory_type {
        "preference" if tags_contains(tags, "analogy") => "uses_analogy",
        "preference" => "prefers_learning_style",
        "misconception" => "has_misconception",
        "interest" => "is_interested_in",
        "history" => "experienced_learning_event",
        _ => "knows",
    }
}

fn salience_for_memory_type(memory_type: &str) -> f64 {
    match memory_type {
        "misconception" => 0.88,
        "preference" => 0.78,
        "knowledge" => 0.84,
        "history" => 0.66,
        _ => 0.7,
    }
}

fn memory_type_for_predicate(predicate: &str) -> &'static str {
    match predicate {
        "prefers_learning_style" | "uses_analogy" | "is_interested_in" => "preference",
        "has_misconception" => "misconception",
        "explored_topic" | "experienced_learning_event" => "history",
        _ => "knowledge",
    }
}

fn tags_contains(tags: &JsonValue, needle: &str) -> bool {
    tags.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(JsonValue::as_str)
                .any(|item| item.eq_ignore_ascii_case(needle))
        })
        .unwrap_or(false)
}

fn age_label_for_kind(kind: &str) -> &'static str {
    match kind {
        "student" => "Student",
        "concept" => "Concept",
        "preference" => "Preference",
        "misconception" => "Misconception",
        "event" => "LearningEvent",
        _ => "MemoryNode",
    }
}

fn relation_label_for_predicate(predicate: &str) -> String {
    predicate
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(":")
}

fn validate_age_identifier(identifier: &str) -> Result<&str, DbErr> {
    let valid = !identifier.is_empty()
        && identifier
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if valid {
        Ok(identifier)
    } else {
        Err(DbErr::Custom(format!(
            "invalid AGE identifier: {identifier}"
        )))
    }
}

fn sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sql_dollar_string(value: &str) -> String {
    for index in 0.. {
        let delimiter = if index == 0 {
            "$age$".to_string()
        } else {
            format!("$age_{index}$")
        };
        if !value.contains(&delimiter) {
            return format!("{delimiter}{value}{delimiter}");
        }
    }
    unreachable!("delimiter search always returns")
}

fn cypher_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('\'');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\'' => output.push_str("\\'"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(character),
        }
    }
    output.push('\'');
    output
}

fn clean_agtype_string(value: &str) -> String {
    value.trim_matches('"').to_string()
}

fn statement(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

fn raw_statement(sql: &str) -> Statement {
    Statement::from_string(DbBackend::Postgres, sql.to_string())
}

pub fn memory_schema_sql() -> Vec<&'static str> {
    vec![
        r#"CREATE TABLE IF NOT EXISTS memory_sources (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            source_type text NOT NULL,
            external_ref text NOT NULL,
            observed_at timestamptz NOT NULL,
            source_time timestamptz,
            raw_text text,
            metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
            sensitivity text NOT NULL DEFAULT 'normal',
            created_at timestamptz NOT NULL,
            UNIQUE (student_id, external_ref)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS memory_entities (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            kind text NOT NULL,
            canonical_name text NOT NULL,
            identity_key text NOT NULL,
            normalized_key text NOT NULL,
            aliases jsonb NOT NULL DEFAULT '[]'::jsonb,
            properties jsonb NOT NULL DEFAULT '{}'::jsonb,
            sensitivity text NOT NULL DEFAULT 'normal',
            status text NOT NULL DEFAULT 'active',
            age_label text,
            age_graph_id text,
            created_at timestamptz NOT NULL,
            updated_at timestamptz NOT NULL,
            UNIQUE (student_id, kind, normalized_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS memory_assertions (
            id uuid PRIMARY KEY,
            student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            subject_entity_id uuid NOT NULL REFERENCES memory_entities(id) ON DELETE CASCADE,
            predicate text NOT NULL,
            object_entity_id uuid REFERENCES memory_entities(id) ON DELETE SET NULL,
            object_value jsonb,
            object_text text,
            qualifiers jsonb NOT NULL DEFAULT '{}'::jsonb,
            valid_period tstzrange NOT NULL,
            tx_period tstzrange NOT NULL DEFAULT tstzrange(now(), NULL, '[)'),
            observed_at timestamptz NOT NULL,
            source_id uuid REFERENCES memory_sources(id) ON DELETE SET NULL,
            confidence double precision NOT NULL DEFAULT 0.75,
            salience double precision NOT NULL DEFAULT 0.5,
            sensitivity text NOT NULL DEFAULT 'normal',
            scope text NOT NULL DEFAULT 'assistant',
            status text NOT NULL DEFAULT 'active',
            supersedes_assertion_id uuid REFERENCES memory_assertions(id),
            contradicts_assertion_id uuid REFERENCES memory_assertions(id),
            metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
            age_edge_label text,
            age_graph_id text,
            created_at timestamptz NOT NULL,
            object_key text GENERATED ALWAYS AS (
                coalesce(
                    object_entity_id::text,
                    md5(coalesce(object_value::text, '') || '|' || coalesce(object_text, ''))
                )
            ) STORED,
            CHECK (NOT isempty(valid_period)),
            CHECK (object_entity_id IS NOT NULL OR object_value IS NOT NULL OR object_text IS NOT NULL)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_memory_sources_student_observed ON memory_sources (student_id, observed_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_memory_entities_student_kind_key ON memory_entities (student_id, kind, normalized_key)",
        "CREATE INDEX IF NOT EXISTS idx_memory_assertions_valid_period ON memory_assertions USING gist (valid_period)",
        "CREATE INDEX IF NOT EXISTS idx_memory_assertions_tx_period ON memory_assertions USING gist (tx_period)",
        "CREATE INDEX IF NOT EXISTS idx_memory_assertions_spo ON memory_assertions (student_id, subject_entity_id, predicate, object_key)",
        "CREATE INDEX IF NOT EXISTS idx_memory_assertions_object_profile ON memory_assertions (student_id, object_entity_id, salience DESC, observed_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_memory_assertions_source ON memory_assertions (source_id)",
        r#"CREATE OR REPLACE FUNCTION primer_assertions_at(valid_at timestamptz, known_at timestamptz)
           RETURNS SETOF memory_assertions
           LANGUAGE sql
           STABLE
           AS $$
             SELECT *
             FROM memory_assertions
             WHERE status IN ('active', 'superseded')
               AND valid_period @> valid_at
               AND tx_period @> known_at
           $$"#,
        r#"CREATE OR REPLACE VIEW primer_current_beliefs AS
           SELECT DISTINCT ON (
             student_id, subject_entity_id, predicate, object_key
           )
             *
           FROM primer_assertions_at(now(), now())
           ORDER BY
             student_id,
             subject_entity_id,
             predicate,
             object_key,
             confidence DESC,
             salience DESC,
             observed_at DESC"#,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_projection_delete_query_avoids_age_property_map_containment() {
        let query = delete_student_graph_projection_query(
            Uuid::parse_str("7854c3af-b0ff-42c8-9854-5bb14a37ea65").unwrap(),
        );

        assert!(query.contains("MATCH (n)"));
        assert!(query.contains("WHERE n.student_id = '7854c3af-b0ff-42c8-9854-5bb14a37ea65'"));
        assert!(query.contains("DETACH DELETE n"));
        assert!(!query.contains("MATCH (n {"));
    }
}
