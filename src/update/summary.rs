use crate::models::update_metadata::common::UpdateStepSummary;
use chrono::{DateTime, Utc};
use crate::models::update_metadata::common::Step;
use crate::models::group_metadata::common::GroupType;

pub struct Summary {
    pub steps: Vec<UpdateStepSummary>,
    pub updated: bool,
    pub started_timestamp: DateTime<Utc>,
    pub finished_timestamp: Option<DateTime<Utc>>,
}

pub struct SummaryStepDelegate<'a> {
    step: Step,

    summary: &'a mut Summary,
}

pub struct SummaryGroupDelegate<'a, 'b> {
    group: GroupType,
    step_delegate: &'b mut SummaryStepDelegate<'a>,
}

impl Summary {
    pub fn new() -> Self {
        Summary {
            steps: vec![],
            updated: false,
            started_timestamp: Utc::now(),
            finished_timestamp: None,
        }
    }

    pub fn step_delegate<'a>(&'a mut self, step: Step) -> SummaryStepDelegate<'a> {
        let step_summary = UpdateStepSummary {
            step,
            updated: false,
            started_timestamp: Utc::now(),
            finished_timestamp: None,
            groups: vec![],
        };

        self.steps.insert(0, step_summary);

        SummaryStepDelegate {
            step,
            summary: self,
        }
    }
}

impl<'a> SummaryStepDelegate<'a> {
    pub fn group_delegate<'b>(&'b mut self, group: GroupType) -> SummaryGroupDelegate<'a, 'b> {
        // TODO: Insert GroupSummary
        SummaryGroupDelegate {
            group,
            step_delegate: self,
        }
    }

    pub fn start(self) {
        if let Some(step_summary) = self.summary.steps.get_mut(0) {

        }

        // Save DB
    }

    pub fn finish(self) {
        // Get step in summary
        // Modify it
        // Save
    }
}
