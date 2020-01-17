use crate::models::metadata::common::GroupType;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct UpdateGroupSummary {
    pub group_type: GroupType,
    pub updated: bool,
    pub status_label: String,
    pub started_timestamp: DateTime<Utc>,
    pub ended_timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
pub enum Step {
    DownloadFile,
    UnzipFile,
    InsertData,
    SwapData,
    CleanFile,
}

#[derive(Serialize)]
pub struct UpdateStepSummary {
    pub step: Step,
    pub updated: bool,
    pub started_timestamp: DateTime<Utc>,
    pub ended_timestamp: DateTime<Utc>,
    pub groups: Vec<UpdateGroupSummary>,
}

#[derive(Serialize)]
pub struct UpdateSummary {
    pub updated: bool,
    pub started_timestamp: DateTime<Utc>,
    pub ended_timestamp: DateTime<Utc>,
    pub steps: Vec<UpdateStepSummary>,
}
