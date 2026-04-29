pub use sea_orm_migration::prelude::*;

mod m20260429_000001_create_learning_tables;
mod m20260429_000002_create_narrative_characters;
mod m20260429_000003_create_student_books;
mod m20260429_000004_add_student_xp;
mod m20260429_000005_add_current_book_state;
mod m20260429_000006_create_infographic_voiceovers;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260429_000001_create_learning_tables::Migration),
            Box::new(m20260429_000002_create_narrative_characters::Migration),
            Box::new(m20260429_000003_create_student_books::Migration),
            Box::new(m20260429_000004_add_student_xp::Migration),
            Box::new(m20260429_000005_add_current_book_state::Migration),
            Box::new(m20260429_000006_create_infographic_voiceovers::Migration),
        ]
    }
}
