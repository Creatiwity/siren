use super::error::Error;
use crate::models::update_metadata::common::UpdateStepSummary;

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
