use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
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
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "DROP TABLE IF EXISTS infographic_voiceovers".to_string(),
            ))
            .await?;

        Ok(())
    }
}
