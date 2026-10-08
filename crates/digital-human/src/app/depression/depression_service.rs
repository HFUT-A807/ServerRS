use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::depression::{
    DepressionAssessment, DepressionRepoT, DepressionScale, NewDepressionAssessment,
};
use crate::shared::error::AppError;

use super::scoring::{compute_score, determine_severity};

pub struct DepressionService {
    pub repo: Arc<dyn DepressionRepoT>,
}

impl DepressionService {
    pub fn new(repo: Arc<dyn DepressionRepoT>) -> Self {
        Self { repo }
    }

    pub async fn list_scales(&self) -> Result<Vec<DepressionScale>, AppError> {
        self.repo.list_scales().await
    }

    pub async fn get_scale(&self, scale_id: u16) -> Result<DepressionScale, AppError> {
        self.repo
            .find_scale_by_id(scale_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("scale {scale_id} not found")))
    }

    /// Creates an assessment. Computes total_score from answers, determines severity_level
    /// from the scale's severity_ranges, and returns an AssessmentDetail DTO that includes
    /// severity_level as a computed field (NOT persisted in the database).
    pub async fn create_assessment(
        &self,
        user_id: u64,
        scale_id: u16,
        answers: serde_json::Value,
        notes: Option<String>,
    ) -> Result<AssessmentDetail, AppError> {
        let scale = self.get_scale(scale_id).await?;

        let total_score = compute_score(&scale, &answers)?;
        let severity_level = determine_severity(&scale, total_score)?;

        let assessment = self
            .repo
            .save_assessment(
                NewDepressionAssessment {
                    user_id,
                    scale_id,
                    answers,
                    notes,
                },
                total_score as i16,
            )
            .await?;

        Ok(AssessmentDetail {
            assessment,
            severity_level,
        })
    }

    pub async fn list_assessments(
        &self,
        user_id: u64,
        page: u64,
        size: u64,
    ) -> Result<(Vec<AssessmentDetail>, u64), AppError> {
        let page = page.max(1);
        let size = size.clamp(1, 100);
        let offset = page.saturating_sub(1) * size;
        let (assessments, total) = self
            .repo
            .find_assessments_by_user_id(user_id, size, offset)
            .await?;
        let scales: HashMap<u16, DepressionScale> = self
            .repo
            .list_scales()
            .await?
            .into_iter()
            .map(|scale| (scale.scale_id, scale))
            .collect();
        let items = assessments
            .into_iter()
            .map(|assessment| {
                let severity_level = scales
                    .get(&assessment.scale_id)
                    .map(|scale| determine_severity(scale, assessment.total_score as i32))
                    .unwrap_or_else(|| Ok("unknown".to_string()))?;
                Ok(AssessmentDetail {
                    assessment,
                    severity_level,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok((items, total))
    }

    pub async fn get_assessment(
        &self,
        user_id: u64,
        assessment_id: u64,
    ) -> Result<AssessmentDetail, AppError> {
        let a = self
            .repo
            .find_assessment_by_id(assessment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("assessment not found".into()))?;
        if a.user_id != user_id {
            return Err(AppError::Forbidden("not your assessment".into()));
        }
        let scale = self.get_scale(a.scale_id).await?;
        let severity_level = determine_severity(&scale, a.total_score as i32)?;
        Ok(AssessmentDetail {
            assessment: a,
            severity_level,
        })
    }

    pub async fn delete_assessment(
        &self,
        user_id: u64,
        assessment_id: u64,
    ) -> Result<(), AppError> {
        let a = self
            .repo
            .find_assessment_by_id(assessment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("assessment not found".into()))?;
        if a.user_id != user_id {
            return Err(AppError::Forbidden("not your assessment".into()));
        }
        self.repo.delete_assessment(assessment_id).await?;
        Ok(())
    }
}

/// DTO for assessment creation response — includes computed severity_level that
/// is NOT persisted in the depression_assessments table.
pub struct AssessmentDetail {
    pub assessment: DepressionAssessment,
    pub severity_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex;

    struct TestRepo {
        scale: DepressionScale,
        saved: Mutex<Vec<DepressionAssessment>>,
    }

    fn repo() -> Arc<TestRepo> {
        Arc::new(TestRepo {
            scale: DepressionScale {
                scale_id: 1,
                scale_name: "test".into(),
                scale_description: None,
                min_score: 0,
                max_score: 3,
                questions: json!([{"id":9,"options":[{"score":0},{"score":1},{"score":2},{"score":3}]}]),
                severity_ranges: json!([{"range":"0-1","label":"minimal"},{"range":"2-3","label":"elevated"}]),
                created_at: None,
                updated_at: None,
            },
            saved: Mutex::new(Vec::new()),
        })
    }

    #[async_trait]
    impl DepressionRepoT for TestRepo {
        async fn find_scale_by_id(&self, id: u16) -> Result<Option<DepressionScale>, AppError> {
            Ok((id == self.scale.scale_id).then(|| self.scale.clone()))
        }
        async fn list_scales(&self) -> Result<Vec<DepressionScale>, AppError> {
            Ok(vec![self.scale.clone()])
        }
        async fn save_assessment(
            &self,
            new: NewDepressionAssessment,
            total_score: i16,
        ) -> Result<DepressionAssessment, AppError> {
            let a = DepressionAssessment {
                assessment_id: 1,
                user_id: new.user_id,
                scale_id: new.scale_id,
                assessment_date: chrono::Utc::now().date_naive(),
                answers: new.answers,
                total_score,
                notes: new.notes,
                created_at: None,
                updated_at: None,
            };
            self.saved.lock().unwrap().push(a.clone());
            Ok(a)
        }
        async fn find_assessment_by_id(
            &self,
            id: u64,
        ) -> Result<Option<DepressionAssessment>, AppError> {
            Ok(self
                .saved
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.assessment_id == id)
                .cloned())
        }
        async fn find_assessments_by_user_id(
            &self,
            user_id: u64,
            _: u64,
            _: u64,
        ) -> Result<(Vec<DepressionAssessment>, u64), AppError> {
            let records: Vec<_> = self
                .saved
                .lock()
                .unwrap()
                .iter()
                .filter(|a| a.user_id == user_id)
                .cloned()
                .collect();
            let count = records.len() as u64;
            Ok((records, count))
        }
        async fn update_assessment(
            &self,
            _: u64,
            _: Option<String>,
        ) -> Result<DepressionAssessment, AppError> {
            unreachable!()
        }
        async fn delete_assessment(&self, _: u64) -> Result<u64, AppError> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn invalid_answers_and_missing_scale_never_save() {
        let r = repo();
        let service = DepressionService::new(r.clone());
        for answers in [json!([]), json!([4]), json!({"1":0}), json!([0, 0])] {
            assert!(matches!(
                service.create_assessment(7, 1, answers, None).await,
                Err(AppError::Validation(_))
            ));
        }
        assert!(matches!(
            service.create_assessment(7, 99, json!([0]), None).await,
            Err(AppError::NotFound(_))
        ));
        assert!(r.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn invalid_ranges_never_save() {
        let mut r = repo();
        Arc::get_mut(&mut r).unwrap().scale.severity_ranges = json!([{"label":"minimal"}]);
        let service = DepressionService::new(r.clone());
        assert!(matches!(
            service.create_assessment(7, 1, json!([0]), None).await,
            Err(AppError::Internal(_))
        ));
        assert!(r.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn creation_detail_and_list_use_the_same_grade_and_preserve_ownership() {
        let r = repo();
        let service = DepressionService::new(r.clone());
        let a = service
            .create_assessment(7, 1, json!({"9":3}), None)
            .await
            .unwrap();
        assert_eq!(a.assessment.total_score, 3);
        assert_eq!(a.severity_level, "elevated");
        assert_eq!(
            service.get_assessment(7, 1).await.unwrap().severity_level,
            "elevated"
        );
        let (records, total) = service.list_assessments(7, 1, 20).await.unwrap();
        assert_eq!(total, 1);
        assert_eq!(records[0].severity_level, "elevated");
        assert!(matches!(
            service.get_assessment(8, 1).await,
            Err(AppError::Forbidden(_))
        ));
        assert_eq!(service.list_assessments(8, 1, 20).await.unwrap().1, 0);
    }
}
