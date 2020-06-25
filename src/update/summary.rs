use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::{
    self,
    common::{Step, SyntheticGroupType, UpdateGroupSummary, UpdateStepSummary, UpdateSummary},
    error::Error,
};
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Summary {
    pub steps: Vec<UpdateStepSummary>,
    pub updated: bool,
    pub started_timestamp: DateTime<Utc>,
    pub finished_timestamp: Option<DateTime<Utc>>,
}

pub struct SummaryStepDelegate<'a> {
    _step: Step,
    summary: &'a mut Summary,
}

pub struct SummaryGroupDelegate<'a, 'b> {
    _group: GroupType,
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
            _step: step,
            summary: self,
        }
    }

    pub fn start(
        &mut self,
        connectors: &Connectors,
        synthetic_group: SyntheticGroupType,
        force: bool,
        data_only: bool,
    ) -> Result<(), Error> {
        update_metadata::launch_update(connectors, synthetic_group, force, data_only).map(
            |date| {
                self.started_timestamp = date;
                Ok(())
            },
        )?
    }

    pub fn finish(&mut self, connectors: &Connectors) -> Result<(), Error> {
        self.finished_timestamp = Some(Utc::now());

        update_metadata::finished_update(
            connectors,
            UpdateSummary {
                started_timestamp: self.started_timestamp,
                finished_timestamp: self.finished_timestamp,
                steps: self.steps,
                updated: self.steps.iter().find(|&s| s.updated).is_some(),
            },
        )
        .map(|_| Ok(()))?
    }
}

impl<'a> SummaryStepDelegate<'a> {
    pub fn group_delegate<'b>(&'b mut self, group: GroupType) -> SummaryGroupDelegate<'a, 'b> {
        if let Some(step) = self.summary.steps.first_mut() {
            let group_summary = UpdateGroupSummary {
                group_type: group,
                updated: false,
                status_label: String::from("Initialized"),
                started_timestamp: Utc::now(),
                finished_timestamp: None,
                planned_count: 0,
                done_count: 0,
                reference_timestamp: None,
            };

            step.groups.insert(0, group_summary);
        }

        SummaryGroupDelegate {
            _group: group,
            step_delegate: self,
        }
    }

    pub fn start(&self) {
        // Save DB
    }

    pub fn finish(&mut self) {
        // Get step in summary
        if let Some(step_summary) = self.summary.steps.first_mut() {
            // Modify it
            step_summary.finished_timestamp = Some(Utc::now());
        }

        // Save
    }
}

impl<'a, 'b> SummaryGroupDelegate<'a, 'b> {
    fn get_current_mut(&mut self) -> Option<&mut UpdateGroupSummary> {
        match self.step_delegate.summary.steps.first_mut() {
            Some(step_summary) => step_summary.groups.first_mut(),
            None => None,
        }
    }

    pub fn start(&mut self, reference_timestamp: Option<DateTime<Utc>>, planned_count: u32) {
        if let Some(group_summary) = self.get_current_mut() {
            group_summary.reference_timestamp = reference_timestamp;
            group_summary.planned_count = planned_count;
        }
    }

    pub fn progress(&mut self, done_count: u32) {
        if let Some(group_summary) = self.get_current_mut() {
            group_summary.done_count = done_count;
        }
    }

    pub fn finish(&mut self, status_label: String, done_count: u32, updated: bool) {
        if let Some(group_summary) = self.get_current_mut() {
            group_summary.status_label = status_label;
            group_summary.done_count = done_count;
            group_summary.updated = updated;
            group_summary.finished_timestamp = Some(Utc::now());
        }
    }
}
