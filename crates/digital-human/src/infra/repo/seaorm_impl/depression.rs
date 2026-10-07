use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};

use crate::domain::depression::{
    DepressionAssessment, DepressionRepoT, DepressionScale, NewDepressionAssessment,
};
use crate::shared::error::AppError;

use super::super::entities::{
    depression_assessments, depression_scales, wellbeing_recommendations,
};
use crate::domain::wellbeing::ASSESSMENT_ACTIVITIES;

fn map_scale(m: depression_scales::Model) -> DepressionScale {
    DepressionScale {
        scale_id: m.scale_id,
        scale_name: m.scale_name,
        scale_description: m.scale_description,
        min_score: m.min_score,
        max_score: m.max_score,
        questions: m.questions,
        severity_ranges: m.severity_ranges,
        created_at: m.created_at.map(|v| v.and_utc()),
        updated_at: m.updated_at.map(|v| v.and_utc()),
    }
}

fn map_assessment(m: depression_assessments::Model) -> DepressionAssessment {
    DepressionAssessment {
        assessment_id: m.assessment_id,
        user_id: m.user_id,
        scale_id: m.scale_id,
        assessment_date: m.assessment_date,
        answers: m.answers,
        total_score: m.total_score,
        notes: m.notes,
        created_at: m.created_at.map(|v| v.and_utc()),
        updated_at: m.updated_at.map(|v| v.and_utc()),
    }
}

fn map_err(e: sea_orm::DbErr) -> AppError {
    AppError::Internal(e.to_string())
}

pub struct DepressionRepo {
    db: DatabaseConnection,
}

impl DepressionRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DepressionRepoT for DepressionRepo {
    async fn find_scale_by_id(&self, id: u16) -> Result<Option<DepressionScale>, AppError> {
        depression_scales::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(map_err)
            .map(|o| o.map(map_scale))
    }

    async fn list_scales(&self) -> Result<Vec<DepressionScale>, AppError> {
        depression_scales::Entity::find()
            .order_by_asc(depression_scales::Column::ScaleId)
            .all(&self.db)
            .await
            .map_err(map_err)
            .map(|v| v.into_iter().map(map_scale).collect())
    }

    async fn save_assessment(
        &self,
        new: NewDepressionAssessment,
        total_score: i16,
    ) -> Result<DepressionAssessment, AppError> {
        let txn = self.db.begin().await.map_err(map_err)?;
        let now = chrono::Utc::now();
        let am: depression_assessments::ActiveModel =
            depression_assessments::ActiveModel::builder()
                .set_user_id(new.user_id)
                .set_scale_id(new.scale_id)
                .set_assessment_date(chrono::Utc::now().date_naive())
                .set_answers(new.answers)
                .set_total_score(total_score)
                .set_notes(new.notes)
                .set_created_at(Some(now.naive_utc()))
                .set_updated_at(Some(now.naive_utc()))
                .into();
        let assessment = am.insert(&txn).await.map_err(map_err)?;
        for &(activity, title, description) in ASSESSMENT_ACTIVITIES {
            let suggestion: wellbeing_recommendations::ActiveModel =
                wellbeing_recommendations::ActiveModel::builder()
                    .set_user_id(assessment.user_id)
                    .set_assessment_id(assessment.assessment_id)
                    .set_activity(activity.to_owned())
                    .set_title(title.to_owned())
                    .set_description(description.to_owned())
                    .set_status("pending".to_owned())
                    .set_available_at(now.naive_utc())
                    .set_created_at(now.naive_utc())
                    .set_updated_at(now.naive_utc())
                    .into();
            suggestion.insert(&txn).await.map_err(map_err)?;
        }
        txn.commit().await.map_err(map_err)?;
        Ok(map_assessment(assessment))
    }

    async fn find_assessment_by_id(
        &self,
        id: u64,
    ) -> Result<Option<DepressionAssessment>, AppError> {
        depression_assessments::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(map_err)
            .map(|o| o.map(map_assessment))
    }

    async fn find_assessments_by_user_id(
        &self,
        user_id: u64,
        limit: u64,
        offset: u64,
    ) -> Result<(Vec<DepressionAssessment>, u64), AppError> {
        let paginator = depression_assessments::Entity::find()
            .filter(depression_assessments::Column::UserId.eq(user_id))
            .order_by_desc(depression_assessments::Column::AssessmentDate)
            .paginate(&self.db, limit);
        let count = paginator.num_items().await.map_err(map_err)?;
        let page_num = offset / limit;
        let items = paginator.fetch_page(page_num).await.map_err(map_err)?;
        Ok((items.into_iter().map(map_assessment).collect(), count))
    }

    async fn update_assessment(
        &self,
        id: u64,
        notes: Option<String>,
    ) -> Result<DepressionAssessment, AppError> {
        let existing = depression_assessments::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(map_err)?
            .ok_or(AppError::NotFound("assessment not found".into()))?;
        let mut am: depression_assessments::ActiveModel = existing.into();
        am.notes = Set(notes);
        am.updated_at = Set(Some(chrono::Utc::now().naive_utc()));
        Ok(map_assessment(am.update(&self.db).await.map_err(map_err)?))
    }

    async fn delete_assessment(&self, id: u64) -> Result<u64, AppError> {
        Ok(depression_assessments::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(map_err)?
            .rows_affected)
    }
}

#[cfg(test)]
mod wellbeing_tests {
    use super::*;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult, Value};
    use std::collections::BTreeMap;

    fn assessment_row() -> BTreeMap<&'static str, Value> {
        let now = chrono::Utc::now().naive_utc();
        BTreeMap::from([
            ("assessment_id", 9_u64.into()),
            ("user_id", 7_u64.into()),
            ("scale_id", 1_u16.into()),
            ("assessment_date", now.date().into()),
            ("answers", serde_json::json!([0]).into()),
            ("total_score", 0_i16.into()),
            ("notes", Option::<String>::None.into()),
            ("created_at", now.into()),
            ("updated_at", now.into()),
        ])
    }

    fn suggestion_row(id: u64, activity: &str) -> BTreeMap<&'static str, Value> {
        let now = chrono::Utc::now().naive_utc();
        BTreeMap::from([
            ("id", id.into()),
            ("user_id", 7_u64.into()),
            ("assessment_id", 9_u64.into()),
            ("activity", activity.into()),
            ("title", "activity".into()),
            ("description", "optional activity".into()),
            ("status", "pending".into()),
            ("available_at", now.into()),
            ("created_at", now.into()),
            ("updated_at", now.into()),
        ])
    }

    fn request() -> NewDepressionAssessment {
        NewDepressionAssessment {
            user_id: 7,
            scale_id: 1,
            answers: serde_json::json!([0]),
            notes: None,
        }
    }

    #[tokio::test]
    async fn assessment_and_both_suggestions_commit_together() {
        let db = MockDatabase::new(DatabaseBackend::MySql)
            .append_exec_results([9, 11, 12].map(|id| MockExecResult {
                last_insert_id: id,
                rows_affected: 1,
            }))
            .append_query_results([
                vec![assessment_row()],
                vec![suggestion_row(11, "breathing")],
                vec![suggestion_row(12, "emotion_diary")],
            ])
            .into_connection();
        let result = DepressionRepo::new(db.clone())
            .save_assessment(request(), 0)
            .await
            .unwrap();
        assert_eq!(result.assessment_id, 9);
        let log = format!("{:?}", db.into_transaction_log());
        assert!(log.contains("BEGIN"), "{log}");
        assert!(log.contains("COMMIT"), "{log}");
        assert_eq!(
            log.matches("INSERT INTO `wellbeing_recommendations`")
                .count(),
            2,
            "{log}"
        );
        assert!(log.contains("breathing"), "{log}");
        assert!(log.contains("emotion_diary"), "{log}");
    }

    #[tokio::test]
    async fn recommendation_failure_rolls_back_the_assessment() {
        let db = MockDatabase::new(DatabaseBackend::MySql)
            .append_exec_results([MockExecResult {
                last_insert_id: 9,
                rows_affected: 1,
            }])
            .append_exec_errors([DbErr::Custom("suggestion insert failed".into())])
            .append_query_results([vec![assessment_row()]])
            .into_connection();
        let result = DepressionRepo::new(db.clone())
            .save_assessment(request(), 0)
            .await;
        assert!(result.is_err());
        let log = format!("{:?}", db.into_transaction_log());
        assert!(log.contains("ROLLBACK"), "{log}");
        assert!(!log.contains("COMMIT"), "{log}");
    }
}
