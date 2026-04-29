use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, DbBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for sql in [
            "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS active_lesson jsonb",
            "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS latest_infographic jsonb",
            "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS latest_stagegate jsonb",
            "ALTER TABLE student_books ADD COLUMN IF NOT EXISTS latest_answer text",
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
            "ALTER TABLE student_books DROP COLUMN IF EXISTS latest_answer",
            "ALTER TABLE student_books DROP COLUMN IF EXISTS latest_stagegate",
            "ALTER TABLE student_books DROP COLUMN IF EXISTS latest_infographic",
            "ALTER TABLE student_books DROP COLUMN IF EXISTS active_lesson",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }

        Ok(())
    }
}
