use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError as AppError;
use crate::api::state::WellbeingState;
use crate::app::auth::auth_service::AuthenticatedUser;
use crate::domain::wellbeing::{ActivityRecommendation, RecommendationStatus};

#[derive(Deserialize)]
pub struct RecommendationQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
}
fn default_limit() -> u64 {
    20
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationList {
    pub items: Vec<ActivityRecommendation>,
    pub poll_after_seconds: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRecommendation {
    pub status: RecommendationStatus,
    pub remind_at: Option<DateTime<Utc>>,
}

pub async fn list_recommendations(
    State(state): State<WellbeingState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(query): Query<RecommendationQuery>,
) -> Result<Json<RecommendationList>, AppError> {
    Ok(Json(RecommendationList {
        items: state.wellbeing.list_due(user.user_id, query.limit).await?,
        poll_after_seconds: 20,
    }))
}

pub async fn update_recommendation(
    State(state): State<WellbeingState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<u64>,
    Json(payload): Json<UpdateRecommendation>,
) -> Result<Json<ActivityRecommendation>, AppError> {
    Ok(Json(
        state
            .wellbeing
            .update(user.user_id, id, payload.status, payload.remind_at)
            .await?,
    ))
}
