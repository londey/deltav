//! Report data structures.
//!
//! These structures hold the computed metrics that will be rendered into reports.

use chrono::NaiveDate;

/// Complete report data for a week.
#[derive(Debug, Clone)]
pub struct ReportData {
    /// Report metadata.
    pub meta: ReportMeta,

    /// Weekly summary (page 1).
    pub weekly: WeeklySummary,

    /// Project status (page 2).
    pub project: ProjectStatus,
}

/// Report metadata.
#[derive(Debug, Clone)]
pub struct ReportMeta {
    /// Project name.
    pub project_name: String,

    /// ISO week being reported (e.g., "2026-W02").
    pub week: String,

    /// Start date of the week (Monday).
    pub week_start: NaiveDate,

    /// End date of the week (Sunday).
    pub week_end: NaiveDate,

    /// When the report was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Weekly summary data (page 1).
#[derive(Debug, Clone)]
pub struct WeeklySummary {
    /// Deliveries completed this week.
    pub deliveries: Vec<DeliveryItem>,

    /// Ticket statistics.
    pub tickets: TicketSummary,

    /// Backlog changes.
    pub backlog: BacklogChange,

    /// Capacity factors (leave, etc.).
    pub capacity: CapacitySummary,

    /// Blocked tickets.
    pub blocked: Vec<BlockedTicket>,

    /// Non-project time sinks.
    pub distractions: Vec<DistractionSummary>,
}

/// A delivered item.
#[derive(Debug, Clone)]
pub struct DeliveryItem {
    /// Item ID.
    pub id: String,

    /// Item name/description.
    pub name: String,

    /// Type of delivery.
    pub kind: DeliveryKind,

    /// Whether it was previously blocked.
    pub was_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryKind {
    Document,
    Csci,
    Dependency,
    Milestone,
    Release,
}

/// Ticket statistics for the week.
#[derive(Debug, Clone)]
pub struct TicketSummary {
    /// Tickets closed this week.
    pub closed: u32,

    /// Tickets closed last week (for comparison).
    pub closed_prev: u32,

    /// Tickets opened this week.
    pub opened: u32,

    /// Net change (closed - opened).
    pub net: i32,

    /// Points delivered this week.
    pub points_delivered: u32,

    /// Rolling average velocity (4 weeks).
    pub velocity_avg: f64,
}

impl TicketSummary {
    /// Trend indicator for closed tickets.
    pub fn closed_trend(&self) -> &'static str {
        if self.closed > self.closed_prev {
            "▲"
        } else if self.closed < self.closed_prev {
            "▼"
        } else {
            "─"
        }
    }
}

/// Backlog changes.
#[derive(Debug, Clone)]
pub struct BacklogChange {
    /// Backlog size at start of week.
    pub start: u32,

    /// Backlog size at end of week.
    pub end: u32,

    /// New work added (discovery vs scope creep).
    pub new_work: u32,

    /// Description of new work.
    pub new_work_note: Option<String>,
}

impl BacklogChange {
    /// Net change in backlog.
    pub fn net(&self) -> i32 {
        self.end as i32 - self.start as i32
    }
}

/// Capacity summary for the week.
#[derive(Debug, Clone)]
pub struct CapacitySummary {
    /// Nominal team capacity (without leave).
    pub nominal: f64,

    /// Actual capacity (after leave adjustments).
    pub actual: f64,

    /// Leave entries affecting this week.
    pub leave: Vec<LeaveEntry>,

    /// Expected velocity given capacity.
    pub expected_velocity: u32,

    /// Actual velocity achieved.
    pub actual_velocity: u32,
}

impl CapacitySummary {
    /// Capacity as percentage of nominal.
    pub fn capacity_percent(&self) -> f64 {
        if self.nominal > 0.0 {
            (self.actual / self.nominal) * 100.0
        } else {
            0.0
        }
    }

    /// Performance vs expectation.
    pub fn performance_percent(&self) -> f64 {
        if self.expected_velocity > 0 {
            (self.actual_velocity as f64 / self.expected_velocity as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// A leave entry.
#[derive(Debug, Clone)]
pub struct LeaveEntry {
    /// Team member name.
    pub name: String,

    /// Capacity during the week (as percentage of normal).
    pub capacity_percent: u32,

    /// Reason if provided.
    pub reason: Option<String>,
}

/// A blocked ticket.
#[derive(Debug, Clone)]
pub struct BlockedTicket {
    /// Repository (org/repo).
    pub repo: String,

    /// Issue number.
    pub number: u64,

    /// Issue title.
    pub title: String,

    /// What it's blocked on.
    pub blocked_on: String,
}

/// Summary of distraction/non-project work.
#[derive(Debug, Clone)]
pub struct DistractionSummary {
    /// Name of the distraction category.
    pub name: String,

    /// Number of tickets worked on.
    pub ticket_count: u32,

    /// Estimated hours (if available).
    pub estimated_hours: Option<f64>,

    /// Who worked on it.
    pub assignees: Vec<String>,
}

/// Project status data (page 2).
#[derive(Debug, Clone)]
pub struct ProjectStatus {
    /// Project timeline info.
    pub timeline: ProjectTimeline,

    /// CSCI completion status.
    pub cscis: Vec<CsciStatus>,

    /// External dependency status.
    pub dependencies: Vec<DependencyStatus>,

    /// Document status.
    pub documents: Vec<DocumentStatus>,

    /// Upcoming milestones.
    pub milestones: Vec<MilestoneStatus>,
}

/// Project timeline information.
#[derive(Debug, Clone)]
pub struct ProjectTimeline {
    /// Days into project.
    pub days_elapsed: i64,

    /// Total project days.
    pub total_days: i64,

    /// Percentage elapsed.
    pub percent_elapsed: f64,
}

/// CSCI completion status.
#[derive(Debug, Clone)]
pub struct CsciStatus {
    /// CSCI ID.
    pub id: String,

    /// CSCI name.
    pub name: String,

    /// Target date.
    pub target_date: NaiveDate,

    /// Days until target.
    pub days_until: i64,

    /// Total tickets for this CSCI.
    pub total_tickets: u32,

    /// Tier 1 (integration-ready) completion.
    pub tier1_complete: u32,

    /// Tier 2 (HIL-passed) completion.
    pub tier2_complete: u32,

    /// Adjusted completion percentage (accounting for backlog completeness).
    pub completion_percent: f64,

    /// Projection status.
    pub projection: Projection,

    /// Buffer days (positive = ahead, negative = behind).
    pub buffer_days: i64,
}

impl CsciStatus {
    /// Raw completion percentage (not adjusted).
    pub fn raw_completion_percent(&self) -> f64 {
        if self.total_tickets > 0 {
            (self.tier1_complete as f64 / self.total_tickets as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Progress bar representation.
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = ((self.completion_percent / 100.0) * width as f64) as usize;
        let empty = width.saturating_sub(filled);
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
}

/// Projection status for a deliverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    OnTrack,
    AtRisk,
    Behind,
    Complete,
}

impl Projection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Projection::OnTrack => "On track",
            Projection::AtRisk => "At risk",
            Projection::Behind => "Behind",
            Projection::Complete => "Complete",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Projection::OnTrack => "✓",
            Projection::AtRisk => "⚠",
            Projection::Behind => "✗",
            Projection::Complete => "✓",
        }
    }
}

/// External dependency status.
#[derive(Debug, Clone)]
pub struct DependencyStatus {
    /// Dependency ID.
    pub id: String,

    /// Dependency name.
    pub name: String,

    /// Owner team.
    pub owner: String,

    /// RC due date.
    pub rc_due: NaiveDate,

    /// Final due date.
    pub final_due: NaiveDate,

    /// RC received?
    pub rc_received: bool,

    /// Final received?
    pub final_received: bool,

    /// Status.
    pub status: DependencyStatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyStatusKind {
    Pending,
    RcOverdue,
    RcReceived,
    FinalOverdue,
    Complete,
}

impl DependencyStatusKind {
    pub fn symbol(&self) -> &'static str {
        match self {
            DependencyStatusKind::Pending => "○",
            DependencyStatusKind::RcOverdue => "✗",
            DependencyStatusKind::RcReceived => "◐",
            DependencyStatusKind::FinalOverdue => "✗",
            DependencyStatusKind::Complete => "✓",
        }
    }

    pub fn is_at_risk(&self) -> bool {
        matches!(
            self,
            DependencyStatusKind::RcOverdue | DependencyStatusKind::FinalOverdue
        )
    }
}

/// Document status.
#[derive(Debug, Clone)]
pub struct DocumentStatus {
    /// Document ID.
    pub id: String,

    /// Document name.
    pub name: String,

    /// Due date.
    pub due_date: NaiveDate,

    /// Status.
    pub status: DocumentStatusKind,

    /// Completion date if complete.
    pub completed_date: Option<NaiveDate>,

    /// Progress note (e.g., "3 sections remaining").
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentStatusKind {
    NotStarted,
    InProgress,
    Complete,
    Overdue,
}

impl DocumentStatusKind {
    pub fn symbol(&self) -> &'static str {
        match self {
            DocumentStatusKind::NotStarted => "○",
            DocumentStatusKind::InProgress => "◐",
            DocumentStatusKind::Complete => "✓",
            DocumentStatusKind::Overdue => "✗",
        }
    }
}

/// Milestone status.
#[derive(Debug, Clone)]
pub struct MilestoneStatus {
    /// Milestone ID.
    pub id: String,

    /// Milestone name.
    pub name: String,

    /// Date.
    pub date: NaiveDate,

    /// Days until milestone.
    pub days_until: i64,
}

/// Builder for creating ReportData from raw inputs.
#[derive(Debug, Default)]
pub struct ReportBuilder {
    pub meta: Option<ReportMeta>,
    pub weekly: Option<WeeklySummary>,
    pub project: Option<ProjectStatus>,
}

impl ReportBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn meta(mut self, meta: ReportMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn weekly(mut self, weekly: WeeklySummary) -> Self {
        self.weekly = Some(weekly);
        self
    }

    pub fn project(mut self, project: ProjectStatus) -> Self {
        self.project = Some(project);
        self
    }

    pub fn build(self) -> anyhow::Result<ReportData> {
        Ok(ReportData {
            meta: self
                .meta
                .ok_or_else(|| anyhow::anyhow!("Missing report meta"))?,
            weekly: self
                .weekly
                .ok_or_else(|| anyhow::anyhow!("Missing weekly summary"))?,
            project: self
                .project
                .ok_or_else(|| anyhow::anyhow!("Missing project status"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csci_progress_bar() {
        let csci = CsciStatus {
            id: "CSCI-001".to_string(),
            name: "Test".to_string(),
            target_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            days_until: 100,
            total_tickets: 100,
            tier1_complete: 50,
            tier2_complete: 25,
            completion_percent: 50.0,
            projection: Projection::OnTrack,
            buffer_days: 5,
        };

        let bar = csci.progress_bar(20);
        assert_eq!(bar, "██████████░░░░░░░░░░");
    }

    #[test]
    fn test_ticket_summary_trend() {
        let up = TicketSummary {
            closed: 15,
            closed_prev: 10,
            opened: 5,
            net: 10,
            points_delivered: 30,
            velocity_avg: 25.0,
        };
        assert_eq!(up.closed_trend(), "▲");

        let down = TicketSummary {
            closed: 8,
            closed_prev: 10,
            opened: 5,
            net: 3,
            points_delivered: 20,
            velocity_avg: 25.0,
        };
        assert_eq!(down.closed_trend(), "▼");
    }
}
