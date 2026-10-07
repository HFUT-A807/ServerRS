use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::domain::wellbeing::{ActivityRecommendation, RecommendationStatus, WellbeingRepoT};
use crate::shared::error::AppError;

pub struct WellbeingService {
    repo: Arc<dyn WellbeingRepoT>,
}

impl WellbeingService {
    pub fn new(repo: Arc<dyn WellbeingRepoT>) -> Self {
        Self { repo }
    }

    pub async fn list_due(
        &self,
        user_id: u64,
        limit: u64,
    ) -> Result<Vec<ActivityRecommendation>, AppError> {
        self.repo
            .list_due(user_id, Utc::now(), limit.clamp(1, 100))
            .await
    }

    pub async fn for_assessment(
        &self,
        user_id: u64,
        assessment_id: u64,
    ) -> Result<Vec<ActivityRecommendation>, AppError> {
        self.repo.list_for_assessment(user_id, assessment_id).await
    }

    pub async fn update(
        &self,
        user_id: u64,
        id: u64,
        status: RecommendationStatus,
        remind_at: Option<DateTime<Utc>>,
    ) -> Result<ActivityRecommendation, AppError> {
        let now = Utc::now();
        validate_update(status, remind_at, now)?;
        self.repo
            .update_pending(user_id, id, status, remind_at, now)
            .await
    }
}

fn validate_update(
    status: RecommendationStatus,
    remind_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    match (status, remind_at) {
        (RecommendationStatus::Pending, Some(time)) if time > now && time <= now + Duration::days(7) => Ok(()),
        (RecommendationStatus::Completed | RecommendationStatus::Dismissed, None) => Ok(()),
        _ => Err(AppError::Validation("use completed/dismissed without remindAt, or pending with a reminder within the next 7 days".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_require_a_future_time_and_terminal_actions_cannot_schedule() {
        let now = Utc::now();
        assert!(
            validate_update(
                RecommendationStatus::Pending,
                Some(now + Duration::minutes(10)),
                now
            )
            .is_ok()
        );
        for time in [now, now - Duration::minutes(1), now + Duration::days(8)] {
            assert!(validate_update(RecommendationStatus::Pending, Some(time), now).is_err());
        }
        assert!(validate_update(RecommendationStatus::Pending, None, now).is_err());
        assert!(
            validate_update(
                RecommendationStatus::Completed,
                Some(now + Duration::minutes(1)),
                now
            )
            .is_err()
        );
        assert!(validate_update(RecommendationStatus::Dismissed, None, now).is_ok());
    }
}
