use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use super::super::entities::wellbeing_recommendations as recommendations;
use crate::domain::wellbeing::{ActivityRecommendation, RecommendationStatus, WellbeingRepoT};
use crate::shared::error::AppError;

pub struct WellbeingRepo {
    db: DatabaseConnection,
}

impl WellbeingRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn map_err(e: sea_orm::DbErr) -> AppError {
    AppError::Internal(e.to_string())
}

fn map_model(m: recommendations::Model) -> Result<ActivityRecommendation, AppError> {
    let status = match m.status.as_str() {
        "pending" => RecommendationStatus::Pending,
        "completed" => RecommendationStatus::Completed,
        "dismissed" => RecommendationStatus::Dismissed,
        _ => return Err(AppError::Internal("invalid recommendation status".into())),
    };
    Ok(ActivityRecommendation {
        id: m.id,
        assessment_id: m.assessment_id,
        activity: m.activity,
        title: m.title,
        description: m.description,
        status,
        available_at: m.available_at.and_utc(),
        created_at: m.created_at.and_utc(),
        updated_at: m.updated_at.and_utc(),
    })
}

#[async_trait]
impl WellbeingRepoT for WellbeingRepo {
    async fn list_due(
        &self,
        user_id: u64,
        now: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<ActivityRecommendation>, AppError> {
        recommendations::Entity::find()
            .filter(recommendations::Column::UserId.eq(user_id))
            .filter(recommendations::Column::Status.eq("pending"))
            .filter(recommendations::Column::AvailableAt.lte(now.naive_utc()))
            .order_by_asc(recommendations::Column::AvailableAt)
            .order_by_asc(recommendations::Column::Id)
            .limit(limit.clamp(1, 100))
            .all(&self.db)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(map_model)
            .collect()
    }

    async fn list_for_assessment(
        &self,
        user_id: u64,
        assessment_id: u64,
    ) -> Result<Vec<ActivityRecommendation>, AppError> {
        recommendations::Entity::find()
            .filter(recommendations::Column::UserId.eq(user_id))
            .filter(recommendations::Column::AssessmentId.eq(assessment_id))
            .order_by_asc(recommendations::Column::Id)
            .all(&self.db)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(map_model)
            .collect()
    }

    async fn update_pending(
        &self,
        user_id: u64,
        id: u64,
        status: RecommendationStatus,
        available_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<ActivityRecommendation, AppError> {
        let mut changes = recommendations::ActiveModel::default();
        changes.status = Set(status.as_str().to_owned());
        changes.updated_at = Set(now.naive_utc());
        if let Some(time) = available_at {
            changes.available_at = Set(time.naive_utc());
        }
        let result = recommendations::Entity::update_many()
            .set(changes)
            .filter(recommendations::Column::Id.eq(id))
            .filter(recommendations::Column::UserId.eq(user_id))
            .filter(recommendations::Column::Status.eq("pending"))
            .exec(&self.db)
            .await
            .map_err(map_err)?;
        let item = recommendations::Entity::find_by_id(id)
            .filter(recommendations::Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(map_err)?
            .ok_or_else(|| AppError::NotFound("recommendation not found".into()))?;
        let item = map_model(item)?;
        if result.rows_affected == 0
            && (status == RecommendationStatus::Pending || item.status != status)
        {
            return Err(AppError::Conflict(
                "recommendation has already been handled".into(),
            ));
        }
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    fn model(status: &str) -> recommendations::Model {
        let now = Utc::now().naive_utc();
        recommendations::Model {
            id: 11,
            user_id: 7,
            assessment_id: 9,
            activity: "breathing".into(),
            title: "呼吸放松练习".into(),
            description: "optional".into(),
            status: status.into(),
            available_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn due_query_filters_owner_status_time_and_caps_results() {
        let db = MockDatabase::new(DatabaseBackend::MySql)
            .append_query_results([vec![model("pending")]])
            .into_connection();
        let repo = WellbeingRepo::new(db.clone());
        let items = repo.list_due(7, Utc::now(), 1000).await.unwrap();
        assert_eq!(items.len(), 1);
        let log = format!("{:?}", db.into_transaction_log());
        assert!(log.contains("`user_id` = ?"), "{log}");
        assert!(log.contains("`status` = ?"), "{log}");
        assert!(log.contains("`available_at` <= ?"), "{log}");
        assert!(log.contains("Unsigned(Some(7))"), "{log}");
        assert!(log.contains("Unsigned(Some(100))"), "{log}");
    }

    #[tokio::test]
    async fn updating_another_users_record_is_not_found_and_scopes_both_queries() {
        let db = MockDatabase::new(DatabaseBackend::MySql)
            .append_exec_results([MockExecResult {
                rows_affected: 0,
                last_insert_id: 0,
            }])
            .append_query_results([Vec::<recommendations::Model>::new()])
            .into_connection();
        let repo = WellbeingRepo::new(db.clone());
        assert!(matches!(
            repo.update_pending(8, 11, RecommendationStatus::Completed, None, Utc::now())
                .await,
            Err(AppError::NotFound(_))
        ));
        let log = format!("{:?}", db.into_transaction_log());
        assert_eq!(log.matches("`user_id` = ?").count(), 2, "{log}");
        assert!(log.contains("`status` = ?"), "{log}");
    }

    #[tokio::test]
    async fn terminal_state_is_idempotent_but_cannot_be_reopened_or_changed() {
        for (status, success) in [
            (RecommendationStatus::Completed, true),
            (RecommendationStatus::Pending, false),
            (RecommendationStatus::Dismissed, false),
        ] {
            let db = MockDatabase::new(DatabaseBackend::MySql)
                .append_exec_results([MockExecResult {
                    rows_affected: 0,
                    last_insert_id: 0,
                }])
                .append_query_results([vec![model("completed")]])
                .into_connection();
            let result = WellbeingRepo::new(db)
                .update_pending(7, 11, status, None, Utc::now())
                .await;
            if success {
                assert_eq!(result.unwrap().status, RecommendationStatus::Completed);
            } else {
                assert!(matches!(result, Err(AppError::Conflict(_))));
            }
        }
    }

    #[tokio::test]
    async fn postponing_updates_the_due_time_without_completing_the_activity() {
        let time = Utc::now() + chrono::Duration::minutes(30);
        let mut record = model("pending");
        record.available_at = time.naive_utc();
        let db = MockDatabase::new(DatabaseBackend::MySql)
            .append_exec_results([MockExecResult {
                rows_affected: 1,
                last_insert_id: 0,
            }])
            .append_query_results([vec![record]])
            .into_connection();
        let result = WellbeingRepo::new(db.clone())
            .update_pending(7, 11, RecommendationStatus::Pending, Some(time), Utc::now())
            .await
            .unwrap();
        assert_eq!(result.status, RecommendationStatus::Pending);
        assert_eq!(result.available_at, time);
        let log = format!("{:?}", db.into_transaction_log());
        assert!(log.contains("`available_at` = ?"), "{log}");
    }
}
