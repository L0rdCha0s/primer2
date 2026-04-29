use sea_orm::entity::prelude::*;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "infographic_voiceovers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub student_id: Uuid,
    pub cache_key: String,
    pub topic: String,
    pub title: Option<String>,
    pub alt: Option<String>,
    pub image_hash: String,
    pub image_length: i64,
    pub explanation: Value,
    pub speech_model: Option<String>,
    pub voice: Option<String>,
    pub content_type: String,
    pub file_path: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
