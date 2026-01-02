//! Project metadata schema.

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Core project metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    /// Human-readable project name.
    pub name: String,

    /// Project start date (for progress calculations).
    pub start_date: NaiveDate,

    /// Project target end date.
    pub end_date: NaiveDate,

    /// Estimated fraction of total work captured in tickets (0.0 to 1.0).
    ///
    /// Used to adjust completion percentages. For example, if you estimate
    /// that only 85% of work is currently ticketed, set this to 0.85.
    #[serde(default = "default_backlog_completeness")]
    pub backlog_completeness: f64,
}

fn default_backlog_completeness() -> f64 {
    1.0
}

impl Project {
    /// Calculate the total project duration in days.
    pub fn duration_days(&self) -> i64 {
        (self.end_date - self.start_date).num_days()
    }

    /// Calculate days elapsed since project start.
    pub fn days_elapsed(&self, as_of: NaiveDate) -> i64 {
        (as_of - self.start_date).num_days()
    }

    /// Calculate percentage of project timeline elapsed.
    pub fn percent_elapsed(&self, as_of: NaiveDate) -> f64 {
        let elapsed = self.days_elapsed(as_of) as f64;
        let total = self.duration_days() as f64;
        (elapsed / total * 100.0).clamp(0.0, 100.0)
    }

    /// Calculate days remaining until project end.
    pub fn days_remaining(&self, as_of: NaiveDate) -> i64 {
        (self.end_date - as_of).num_days()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration() {
        let project = Project {
            name: "Test".to_string(),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            backlog_completeness: 1.0,
        };
        assert_eq!(project.duration_days(), 364);
    }

    #[test]
    fn test_percent_elapsed() {
        let project = Project {
            name: "Test".to_string(),
            start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 1, 11).unwrap(),
            backlog_completeness: 1.0,
        };
        let midpoint = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
        assert_eq!(project.percent_elapsed(midpoint), 50.0);
    }
}
