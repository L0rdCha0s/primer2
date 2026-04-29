use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in narrative_character_sql() {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "DROP TABLE IF EXISTS narrative_character_biographies",
            "DROP TABLE IF EXISTS narrative_characters",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }
}

fn narrative_character_sql() -> Vec<&'static str> {
    vec![
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
    ]
}
