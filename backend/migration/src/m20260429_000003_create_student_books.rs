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
                current_lesson_id uuid,
                created_at timestamptz NOT NULL,
                updated_at timestamptz NOT NULL,
                UNIQUE (student_id)
            )"#,
            r#"CREATE TABLE IF NOT EXISTS student_book_lessons (
                id uuid PRIMARY KEY,
                book_id uuid NOT NULL REFERENCES student_books(id) ON DELETE CASCADE,
                student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
                topic text NOT NULL,
                stage_level text,
                position integer NOT NULL,
                lesson jsonb NOT NULL DEFAULT '{}'::jsonb,
                latest_infographic jsonb,
                latest_stagegate jsonb,
                latest_answer text,
                created_at timestamptz NOT NULL,
                updated_at timestamptz NOT NULL,
                UNIQUE (book_id, position)
            )"#,
            r#"CREATE TABLE IF NOT EXISTS lesson_pages (
                id uuid PRIMARY KEY,
                lesson_id uuid NOT NULL REFERENCES student_book_lessons(id) ON DELETE CASCADE,
                book_id uuid NOT NULL REFERENCES student_books(id) ON DELETE CASCADE,
                student_id uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
                page_kind text NOT NULL,
                topic text,
                stage_level text,
                position integer NOT NULL,
                payload jsonb NOT NULL DEFAULT '{}'::jsonb,
                created_at timestamptz NOT NULL,
                UNIQUE (lesson_id, position)
            )"#,
            r#"DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM pg_constraint
                    WHERE conname = 'fk_student_books_current_lesson'
                ) THEN
                    ALTER TABLE student_books
                        ADD CONSTRAINT fk_student_books_current_lesson
                        FOREIGN KEY (current_lesson_id)
                        REFERENCES student_book_lessons(id)
                        ON DELETE SET NULL;
                END IF;
            END $$"#,
            "CREATE INDEX IF NOT EXISTS idx_student_book_lessons_book_position ON student_book_lessons (book_id, position)",
            "CREATE INDEX IF NOT EXISTS idx_student_book_lessons_student_position ON student_book_lessons (student_id, position)",
            "CREATE INDEX IF NOT EXISTS idx_lesson_pages_lesson_position ON lesson_pages (lesson_id, position)",
            "CREATE INDEX IF NOT EXISTS idx_lesson_pages_book_kind_created ON lesson_pages (book_id, page_kind, created_at DESC)",
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
            "DROP TABLE IF EXISTS lesson_pages",
            "DROP TABLE IF EXISTS student_book_lessons",
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
