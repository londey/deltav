//! Team composition and capacity schema.

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Team configuration including members and planned leave.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Team {
    /// List of team members.
    pub members: Vec<TeamMember>,

    /// Planned leave periods.
    #[serde(default)]
    pub leave: Vec<Leave>,
}

impl Team {
    /// Calculate total team capacity for a given date.
    ///
    /// Returns a value between 0.0 and the sum of all member capacities,
    /// reduced by any members on leave.
    pub fn capacity_on(&self, date: NaiveDate) -> f64 {
        self.members
            .iter()
            .map(|member| {
                if self.is_on_leave(&member.github, date) {
                    0.0
                } else {
                    member.capacity
                }
            })
            .sum()
    }

    /// Calculate average capacity over a date range (inclusive).
    pub fn average_capacity(&self, start: NaiveDate, end: NaiveDate) -> f64 {
        let mut current = start;
        let mut total = 0.0;
        let mut days = 0;

        while current <= end {
            total += self.capacity_on(current);
            days += 1;
            current = current.succ_opt().unwrap_or(current);
        }

        if days > 0 {
            total / days as f64
        } else {
            0.0
        }
    }

    /// Check if a team member is on leave on a given date.
    pub fn is_on_leave(&self, github: &str, date: NaiveDate) -> bool {
        self.leave
            .iter()
            .any(|l| l.github == github && date >= l.start && date <= l.end)
    }

    /// Get all leave periods affecting a date range.
    pub fn leave_in_range(&self, start: NaiveDate, end: NaiveDate) -> Vec<&Leave> {
        self.leave
            .iter()
            .filter(|l| l.end >= start && l.start <= end)
            .collect()
    }

    /// Get team member by GitHub username.
    pub fn member_by_github(&self, github: &str) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.github == github)
    }

    /// Total headcount.
    pub fn headcount(&self) -> usize {
        self.members.len()
    }

    /// Total nominal capacity (without leave adjustments).
    pub fn nominal_capacity(&self) -> f64 {
        self.members.iter().map(|m| m.capacity).sum()
    }
}

/// A team member.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamMember {
    /// Human-readable name.
    pub name: String,

    /// GitHub username.
    pub github: String,

    /// Capacity multiplier (1.0 = full time, 0.5 = half time, etc.).
    #[serde(default = "default_capacity")]
    pub capacity: f64,
}

fn default_capacity() -> f64 {
    1.0
}

/// A period of planned leave for a team member.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Leave {
    /// GitHub username of the team member.
    pub github: String,

    /// First day of leave (inclusive).
    pub start: NaiveDate,

    /// Last day of leave (inclusive).
    pub end: NaiveDate,

    /// Optional reason (for report context).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Leave {
    /// Duration of leave in days (inclusive).
    pub fn duration_days(&self) -> i64 {
        (self.end - self.start).num_days() + 1
    }

    /// Check if this leave period overlaps with a date range.
    pub fn overlaps(&self, start: NaiveDate, end: NaiveDate) -> bool {
        self.start <= end && self.end >= start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_team() -> Team {
        Team {
            members: vec![
                TeamMember {
                    name: "Alice".to_string(),
                    github: "alice".to_string(),
                    capacity: 1.0,
                },
                TeamMember {
                    name: "Bob".to_string(),
                    github: "bob".to_string(),
                    capacity: 0.5,
                },
            ],
            leave: vec![Leave {
                github: "alice".to_string(),
                start: NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 1, 7).unwrap(),
                reason: Some("PTO".to_string()),
            }],
        }
    }

    #[test]
    fn test_capacity_no_leave() {
        let team = sample_team();
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert_eq!(team.capacity_on(date), 1.5);
    }

    #[test]
    fn test_capacity_with_leave() {
        let team = sample_team();
        let date = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
        assert_eq!(team.capacity_on(date), 0.5); // Only Bob available
    }

    #[test]
    fn test_leave_duration() {
        let leave = Leave {
            github: "test".to_string(),
            start: NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
            end: NaiveDate::from_ymd_opt(2025, 1, 7).unwrap(),
            reason: None,
        };
        assert_eq!(leave.duration_days(), 3);
    }
}
