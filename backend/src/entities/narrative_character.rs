use sea_orm::entity::prelude::*;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "narrative_characters")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub student_id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub role: Option<String>,
    pub current_biography: String,
    pub topic_affinities: Value,
    pub consistency_notes: Value,
    pub status: String,
    pub introduced_at: DateTimeWithTimeZone,
    pub last_seen_at: DateTimeWithTimeZone,
    pub last_seen_topic: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
