use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "wellbeing_recommendations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub user_id: u64,
    pub assessment_id: u64,
    pub activity: String,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub status: String,
    pub available_at: DateTime,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl ActiveModelBehavior for ActiveModel {}
