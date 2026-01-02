//! Markdown report renderer.
//!
//! Generates self-contained markdown with embedded images as data URLs.

use super::data::*;
use std::fmt::Write;

/// Render a report to markdown.
pub fn render(data: &ReportData) -> String {
    let mut out = String::new();

    render_page1(&mut out, data);
    out.push_str("\n---\n\n");
    render_page2(&mut out, data);

    out
}

fn render_page1(out: &mut String, data: &ReportData) {
    // Header
    writeln!(out, "# DELTAV WEEKLY REPORT — {} — {}", data.meta.project_name, data.meta.week).unwrap();
    writeln!(out, "**{} to {}**\n", data.meta.week_start, data.meta.week_end).unwrap();

    // Deliveries this week
    writeln!(out, "## Deliveries This Week\n").unwrap();
    if data.weekly.deliveries.is_empty() {
        writeln!(out, "_No deliveries this week._\n").unwrap();
    } else {
        for delivery in &data.weekly.deliveries {
            let blocked_note = if delivery.was_blocked { " (was blocked)" } else { "" };
            writeln!(out, "- ☑ {} — {}{}", delivery.id, delivery.name, blocked_note).unwrap();
        }
        out.push('\n');
    }

    // Ticket summary
    writeln!(out, "## Ticket Summary\n").unwrap();
    let tickets = &data.weekly.tickets;
    writeln!(
        out,
        "| Metric | Value |",
    ).unwrap();
    writeln!(out, "|--------|-------|").unwrap();
    writeln!(
        out,
        "| Closed | {} ({} from {}) |",
        tickets.closed, tickets.closed_trend(), tickets.closed_prev
    ).unwrap();
    writeln!(out, "| Opened | {} |", tickets.opened).unwrap();
    writeln!(out, "| Net | {:+} toward done |", tickets.net).unwrap();
    writeln!(out, "| Points Delivered | {} |", tickets.points_delivered).unwrap();
    writeln!(out, "| Velocity (4wk avg) | {:.0} |", tickets.velocity_avg).unwrap();
    out.push('\n');

    // Backlog change
    writeln!(out, "## Backlog Change\n").unwrap();
    let backlog = &data.weekly.backlog;
    writeln!(out, "- Start of week: {} tickets", backlog.start).unwrap();
    writeln!(out, "- End of week: {} tickets", backlog.end).unwrap();
    if backlog.new_work > 0 {
        let note = backlog.new_work_note.as_deref().unwrap_or("new work added");
        writeln!(out, "- New work: {} tickets ({})", backlog.new_work, note).unwrap();
    }
    out.push('\n');

    // Capacity factors
    writeln!(out, "## Capacity Factors\n").unwrap();
    let capacity = &data.weekly.capacity;
    if capacity.leave.is_empty() {
        writeln!(out, "_No capacity adjustments this week._\n").unwrap();
    } else {
        for leave in &capacity.leave {
            let reason = leave.reason.as_deref().unwrap_or("leave");
            writeln!(out, "- **{}**: {} ({}% capacity)", leave.name, reason, leave.capacity_percent).unwrap();
        }
        out.push('\n');
    }
    writeln!(out, "- Adjusted velocity expectation: {} points", capacity.expected_velocity).unwrap();
    writeln!(
        out,
        "- Actual: {} points ({:.0}% of adjusted target) {}",
        capacity.actual_velocity,
        capacity.performance_percent(),
        if capacity.performance_percent() >= 100.0 { "✓" } else { "" }
    ).unwrap();
    out.push('\n');

    // Blocked tickets
    writeln!(out, "## Blocked / External\n").unwrap();
    if data.weekly.blocked.is_empty() {
        writeln!(out, "_No blocked tickets._\n").unwrap();
    } else {
        for blocked in &data.weekly.blocked {
            writeln!(
                out,
                "- **{}#{}**: {} — _blocked on: {}_",
                blocked.repo, blocked.number, blocked.title, blocked.blocked_on
            ).unwrap();
        }
        out.push('\n');
    }

    // Distractions
    writeln!(out, "## Non-Project Time Sinks\n").unwrap();
    if data.weekly.distractions.is_empty() {
        writeln!(out, "_No non-project work recorded._\n").unwrap();
    } else {
        for distraction in &data.weekly.distractions {
            let hours = distraction.estimated_hours
                .map(|h| format!(", ~{:.0} hours", h))
                .unwrap_or_default();
            let assignees = if distraction.assignees.is_empty() {
                String::new()
            } else {
                format!(" ({})", distraction.assignees.join(", "))
            };
            writeln!(
                out,
                "- **{}**: {} tickets{}{}",
                distraction.name, distraction.ticket_count, hours, assignees
            ).unwrap();
        }
        out.push('\n');
    }
}

fn render_page2(out: &mut String, data: &ReportData) {
    // Header
    writeln!(out, "# PROJECT STATUS — {}", data.meta.project_name).unwrap();
    writeln!(
        out,
        "**As of {} | Day {} of {} ({:.0}% elapsed)**\n",
        data.meta.week_end,
        data.project.timeline.days_elapsed,
        data.project.timeline.total_days,
        data.project.timeline.percent_elapsed
    ).unwrap();

    // CSCI completion
    writeln!(out, "## CSCI Completion\n").unwrap();
    writeln!(out, "_Adjusted for backlog completeness estimate._\n").unwrap();
    
    for csci in &data.project.cscis {
        writeln!(out, "### {} ({}) — Target: {}\n", csci.name, csci.id, csci.target_date).unwrap();
        writeln!(out, "```").unwrap();
        writeln!(out, "{} {:.0}% complete", csci.progress_bar(25), csci.completion_percent).unwrap();
        writeln!(out, "```\n").unwrap();
        writeln!(out, "- **Tier 1** (integration-ready): {}/{} tickets", csci.tier1_complete, csci.total_tickets).unwrap();
        writeln!(out, "- **Tier 2** (HIL-passed): {}/{} tickets", csci.tier2_complete, csci.total_tickets).unwrap();
        writeln!(
            out,
            "- **Projection**: {} {} ({} days {})",
            csci.projection.symbol(),
            csci.projection.as_str(),
            csci.buffer_days.abs(),
            if csci.buffer_days >= 0 { "buffer" } else { "behind" }
        ).unwrap();
        out.push('\n');
    }

    // External dependencies
    writeln!(out, "## External Dependencies\n").unwrap();
    if data.project.dependencies.is_empty() {
        writeln!(out, "_No external dependencies._\n").unwrap();
    } else {
        writeln!(out, "| ID | Name | Owner | RC | Final | Status |").unwrap();
        writeln!(out, "|----|------|-------|-----|-------|--------|").unwrap();
        for dep in &data.project.dependencies {
            let rc_status = if dep.rc_received { "✓" } else { dep.status.symbol() };
            let final_status = if dep.final_received { "✓" } else { "○" };
            writeln!(
                out,
                "| {} | {} | {} | {} {} | {} {} | {} |",
                dep.id, dep.name, dep.owner,
                rc_status, dep.rc_due,
                final_status, dep.final_due,
                if dep.status.is_at_risk() { "⚠ At Risk" } else { "" }
            ).unwrap();
        }
        out.push('\n');
    }

    // Documents
    writeln!(out, "## Documents\n").unwrap();
    if data.project.documents.is_empty() {
        writeln!(out, "_No documents tracked._\n").unwrap();
    } else {
        for doc in &data.project.documents {
            let status_str = match doc.status {
                DocumentStatusKind::Complete => {
                    let date = doc.completed_date.map(|d| d.to_string()).unwrap_or_default();
                    format!("✓ Complete ({})", date)
                }
                DocumentStatusKind::InProgress => {
                    let note = doc.note.as_deref().unwrap_or("in progress");
                    format!("◐ {}, due {}", note, doc.due_date)
                }
                DocumentStatusKind::NotStarted => {
                    format!("○ Not started, due {}", doc.due_date)
                }
                DocumentStatusKind::Overdue => {
                    format!("✗ Overdue (was due {})", doc.due_date)
                }
            };
            writeln!(out, "- **{}** ({}): {}", doc.name, doc.id, status_str).unwrap();
        }
        out.push('\n');
    }

    // Upcoming milestones
    writeln!(out, "## Upcoming Milestones\n").unwrap();
    if data.project.milestones.is_empty() {
        writeln!(out, "_No upcoming milestones._\n").unwrap();
    } else {
        for milestone in &data.project.milestones {
            writeln!(
                out,
                "- **{}**: {} ({} days)",
                milestone.date, milestone.name, milestone.days_until
            ).unwrap();
        }
        out.push('\n');
    }

    // Metrics glossary
    writeln!(out, "## Metrics Glossary\n").unwrap();
    writeln!(out, "**Tier 1 (Integration-Ready)**: Code complete, unit tested, CI passing, ready for HIL integration.\n").unwrap();
    writeln!(out, "**Tier 2 (HIL-Passed)**: Successfully tested in hardware-in-the-loop environment, approved for formal release.\n").unwrap();
    writeln!(out, "**T-Shirt Sizing**: XS=1pt (<2hr) | S=2pt (half day) | M=5pt (1-2 days) | L=8pt (3-5 days) | XL=13pt (week+)\n").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_report() -> ReportData {
        ReportData {
            meta: ReportMeta {
                project_name: "Test Project".to_string(),
                week: "2026-W02".to_string(),
                week_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                week_end: chrono::NaiveDate::from_ymd_opt(2026, 1, 11).unwrap(),
                generated_at: Utc::now(),
            },
            weekly: WeeklySummary {
                deliveries: vec![],
                tickets: TicketSummary {
                    closed: 10,
                    closed_prev: 8,
                    opened: 5,
                    net: 5,
                    points_delivered: 25,
                    velocity_avg: 22.0,
                },
                backlog: BacklogChange {
                    start: 100,
                    end: 95,
                    new_work: 3,
                    new_work_note: Some("discovery".to_string()),
                },
                capacity: CapacitySummary {
                    nominal: 3.0,
                    actual: 2.5,
                    leave: vec![],
                    expected_velocity: 20,
                    actual_velocity: 25,
                },
                blocked: vec![],
                distractions: vec![],
            },
            project: ProjectStatus {
                timeline: ProjectTimeline {
                    days_elapsed: 100,
                    total_days: 365,
                    percent_elapsed: 27.4,
                },
                cscis: vec![],
                dependencies: vec![],
                documents: vec![],
                milestones: vec![],
            },
        }
    }

    #[test]
    fn test_render_produces_output() {
        let report = sample_report();
        let md = render(&report);
        assert!(md.contains("Test Project"));
        assert!(md.contains("2026-W02"));
        assert!(md.contains("Ticket Summary"));
    }
}
