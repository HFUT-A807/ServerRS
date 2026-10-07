use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationStatus {
    Pending,
    Completed,
    Dismissed,
}

impl RecommendationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Dismissed => "dismissed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRecommendation {
    pub id: u64,
    pub assessment_id: u64,
    pub activity: String,
    pub title: String,
    pub description: String,
    pub status: RecommendationStatus,
    pub available_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// General wellness choices offered after an assessment, independent of its score.
/// These are not clinical prescriptions or outputs from the risk audit pipeline.
pub const ASSESSMENT_ACTIVITIES: &[(&str, &str, &str)] = &[
    (
        "breathing",
        "呼吸放松练习",
        "可以选择一段呼吸放松练习，也可以跳过或稍后提醒。",
    ),
    (
        "emotion_diary",
        "记录此刻的心情",
        "可以写一段情绪日记，记录此刻的感受，也可以跳过或稍后提醒。",
    ),
];

#[async_trait]
pub trait WellbeingRepoT: Send + Sync {
    async fn list_due(
        &self,
        user_id: u64,
        now: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<ActivityRecommendation>, AppError>;
    async fn list_for_assessment(
        &self,
        user_id: u64,
        assessment_id: u64,
    ) -> Result<Vec<ActivityRecommendation>, AppError>;
    /// Atomically update only this user's pending recommendation; terminal states stay terminal.
    async fn update_pending(
        &self,
        user_id: u64,
        id: u64,
        status: RecommendationStatus,
        available_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<ActivityRecommendation, AppError>;
}
