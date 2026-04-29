use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            r#"CREATE TABLE IF NOT EXISTS student_books (
                id uuid PRIMARY KEY,
                student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
                title text NOT NULL,
                status text NOT NULL DEFAULT 'active',
                created_at timestamptz NOT NULL,
                updated_at timestamptz NOT NULL,
                UNIQUE (student_id)
            )"#,
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
            "DROP TABLE IF EXISTS student_book_entries",
            "DROP TABLE IF EXISTS student_books",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }
}
