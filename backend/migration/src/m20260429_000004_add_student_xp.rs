use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "ALTER TABLE students ADD COLUMN IF NOT EXISTS xp_total integer NOT NULL DEFAULT 0",
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
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "DROP TABLE IF EXISTS student_xp_events",
            "ALTER TABLE students DROP COLUMN IF EXISTS xp_total",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }
}
