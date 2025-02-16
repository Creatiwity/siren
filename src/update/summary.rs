use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::{
    self,
    common::{Step, SyntheticGroupType, UpdateGroupSummary, UpdateStepSummary, UpdateSummary},
    error::Error,
};
use chrono::{DateTime, Utc};

pub struct SummaryStepDelegate<'a> {
    _step: Step,
    summary: &'a mut UpdateSummary,
}

pub struct SummaryGroupDelegate<'a, 'b> {
    _group: GroupType,
    step_delegate: &'b mut SummaryStepDelegate<'a>,
}

impl Default for UpdateSummary {
    fn default() -> Self {
        UpdateSummary {
            steps: vec![],
            updated: false,
            started_timestamp: Utc::now(),
            finished_timestamp: None,
        }
    }
}

impl UpdateSummary {
    pub fn step_delegate(&'_ mut self, step: Step) -> SummaryStepDelegate<'_> {
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
        self.updated = self.steps.iter().any(|s| s.updated);

        update_metadata::finished_update(connectors, self.clone()).map(|_| Ok(()))?
    }
}

impl<'a> SummaryStepDelegate<'a> {
    pub fn group_delegate<'b>(&'b mut self, group: GroupType) -> SummaryGroupDelegate<'a, 'b> {
        if let Some(step) = self.summary.steps.first_mut() {
            let group_summary = UpdateGroupSummary {
                group_type: group,
                updated: false,
                status_label: String::from("initialized"),
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

    pub fn start(&self, connectors: &Connectors) -> Result<(), Error> {
        update_metadata::progress_update(connectors, self.summary.clone()).map(|_| Ok(()))?
    }

    pub fn finish(&mut self, connectors: &Connectors) -> Result<(), Error> {
        if let Some(step_summary) = self.summary.steps.first_mut() {
            step_summary.finished_timestamp = Some(Utc::now());
            step_summary.updated = step_summary.groups.iter().any(|g| g.updated);
        }

        update_metadata::progress_update(connectors, self.summary.clone()).map(|_| Ok(()))?
    }
}

impl SummaryGroupDelegate<'_, '_> {
    fn get_current_mut(&mut self) -> Option<&mut UpdateGroupSummary> {
        match self.step_delegate.summary.steps.first_mut() {
            Some(step_summary) => step_summary.groups.first_mut(),
            None => None,
        }
    }

    pub fn start(
        &mut self,
        connectors: &Connectors,
        reference_timestamp: Option<DateTime<Utc>>,
        planned_count: u32,
    ) -> Result<(), Error> {
        if let Some(group_summary) = self.get_current_mut() {
            group_summary.reference_timestamp = reference_timestamp;
            group_summary.planned_count = planned_count;
            group_summary.status_label = String::from("in progress")
        }

        update_metadata::progress_update(connectors, self.step_delegate.summary.clone())
            .map(|_| Ok(()))?
    }

    pub fn progress(&mut self, connectors: &Connectors, done_count: u32) -> Result<(), Error> {
        if let Some(group_summary) = self.get_current_mut() {
            group_summary.done_count = done_count;
        }

        update_metadata::progress_update(connectors, self.step_delegate.summary.clone())
            .map(|_| Ok(()))?
    }

    pub fn finish(
        &mut self,
        connectors: &Connectors,
        status_label: String,
        done_count: u32,
        updated: bool,
    ) -> Result<(), Error> {
        if let Some(group_summary) = self.get_current_mut() {
            group_summary.status_label = status_label;
            group_summary.done_count = done_count;
            group_summary.updated = updated;
            group_summary.finished_timestamp = Some(Utc::now());
        }

        update_metadata::progress_update(connectors, self.step_delegate.summary.clone())
            .map(|_| Ok(()))?
    }
}
