use crate::models::update_metadata::common::UpdateStepSummary;
use super::error::Error;

pub struct Summary {
    pub error: Option<Error>,
    pub steps: Vec<UpdateStepSummary>,
}

impl Summary {
    pub fn new() -> Self {
        Summary {
            error: None,
            steps: vec![],
        }
    }
}
