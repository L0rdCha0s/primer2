pub use sea_orm_migration::prelude::*;

mod m20260429_000001_create_learning_tables;
mod m20260429_000002_create_narrative_characters;
mod m20260429_000003_create_student_books;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260429_000001_create_learning_tables::Migration),
            Box::new(m20260429_000002_create_narrative_characters::Migration),
            Box::new(m20260429_000003_create_student_books::Migration),
        ]
    }
}
