use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, Statement};

const PUBLIC_SCHEMA: &str = "public";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(public_table(Students::Table))
                    .if_not_exists()
                    .col(ColumnDef::new(Students::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Students::PublicId).string().not_null())
                    .col(ColumnDef::new(Students::DisplayName).string().not_null())
                    .col(ColumnDef::new(Students::AgeYears).integer())
                    .col(ColumnDef::new(Students::AgeBand).string().not_null())
                    .col(ColumnDef::new(Students::Biography).text())
                    .col(ColumnDef::new(Students::Interests).json_binary().not_null())
                    .col(
                        ColumnDef::new(Students::PreferredExplanationStyle)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Students::LevelContext).string().not_null())
                    .col(
                        ColumnDef::new(Students::SuggestedTopics)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Students::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Students::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_students_public_id")
                    .table(public_table(Students::Table))
                    .col(Students::PublicId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(public_table(LocalUsers::Table))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LocalUsers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LocalUsers::Username).string().not_null())
                    .col(ColumnDef::new(LocalUsers::PasswordHash).string().not_null())
                    .col(ColumnDef::new(LocalUsers::StudentId).uuid().not_null())
                    .col(
                        ColumnDef::new(LocalUsers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_local_users_student_id")
                            .from(public_table(LocalUsers::Table), LocalUsers::StudentId)
                            .to(public_table(Students::Table), Students::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_local_users_username")
                    .table(public_table(LocalUsers::Table))
                    .col(LocalUsers::Username)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_local_users_student_id")
                    .table(public_table(LocalUsers::Table))
                    .col(LocalUsers::StudentId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(public_table(StudentMemories::Table))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StudentMemories::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(StudentMemories::StudentId).uuid().not_null())
                    .col(
                        ColumnDef::new(StudentMemories::MemoryType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(StudentMemories::Content).text().not_null())
                    .col(
                        ColumnDef::new(StudentMemories::Confidence)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StudentMemories::Tags)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StudentMemories::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StudentMemories::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_student_memories_student_id")
                            .from(
                                public_table(StudentMemories::Table),
                                StudentMemories::StudentId,
                            )
                            .to(public_table(Students::Table), Students::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_student_memories_student_id")
                    .table(public_table(StudentMemories::Table))
                    .col(StudentMemories::StudentId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(public_table(ConceptProgress::Table))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ConceptProgress::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ConceptProgress::StudentId).uuid().not_null())
                    .col(ColumnDef::new(ConceptProgress::Topic).string().not_null())
                    .col(ColumnDef::new(ConceptProgress::Level).string().not_null())
                    .col(
                        ColumnDef::new(ConceptProgress::MasteryScore)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ConceptProgress::Status).string().not_null())
                    .col(
                        ColumnDef::new(ConceptProgress::Evidence)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ConceptProgress::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_concept_progress_student_id")
                            .from(
                                public_table(ConceptProgress::Table),
                                ConceptProgress::StudentId,
                            )
                            .to(public_table(Students::Table), Students::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_concept_progress_student_topic_level")
                    .table(public_table(ConceptProgress::Table))
                    .col(ConceptProgress::StudentId)
                    .col(ConceptProgress::Topic)
                    .col(ConceptProgress::Level)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        for sql in memory_graph_sql() {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "DROP VIEW IF EXISTS primer_current_beliefs",
            "DROP FUNCTION IF EXISTS primer_assertions_at(timestamptz, timestamptz)",
            "DROP TABLE IF EXISTS memory_assertions",
            "DROP TABLE IF EXISTS memory_entities",
            "DROP TABLE IF EXISTS memory_sources",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        manager
            .drop_table(
                Table::drop()
                    .table(public_table(ConceptProgress::Table))
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(public_table(StudentMemories::Table))
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(public_table(LocalUsers::Table))
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(public_table(Students::Table))
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

fn public_table<T>(table: T) -> (Alias, T)
where
    T: IntoIden,
{
    (Alias::new(PUBLIC_SCHEMA), table)
}

#[derive(DeriveIden)]
enum Students {
    Table,
    Id,
    PublicId,
    DisplayName,
    AgeYears,
    AgeBand,
    Biography,
    Interests,
    PreferredExplanationStyle,
    LevelContext,
    SuggestedTopics,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum LocalUsers {
    Table,
    Id,
    Username,
    PasswordHash,
    StudentId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum StudentMemories {
    Table,
    Id,
    StudentId,
    MemoryType,
    Content,
    Confidence,
    Tags,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ConceptProgress {
    Table,
    Id,
    StudentId,
    Topic,
    Level,
    MasteryScore,
    Status,
    Evidence,
    UpdatedAt,
}

fn memory_graph_sql() -> Vec<&'static str> {
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
