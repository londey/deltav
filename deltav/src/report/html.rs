//! HTML report renderer.
//!
//! Generates self-contained HTML with inline CSS.

use super::data::*;
use std::fmt::Write;

/// Render a report to self-contained HTML.
pub fn render(data: &ReportData) -> String {
    let mut out = String::new();

    // HTML header with inline CSS
    out.push_str(HTML_HEADER);

    // Page 1: Weekly Summary
    render_page1(&mut out, data);

    // Page break for printing
    out.push_str(r#"<div class="page-break"></div>"#);

    // Page 2: Project Status
    render_page2(&mut out, data);

    // HTML footer
    out.push_str(HTML_FOOTER);

    out
}

fn render_page1(out: &mut String, data: &ReportData) {
    writeln!(out, r#"<div class="page">"#).unwrap();

    // Header
    writeln!(out, r#"<header>"#).unwrap();
    writeln!(
        out,
        r#"<h1>DELTAV WEEKLY REPORT — {} — {}</h1>"#,
        html_escape(&data.meta.project_name),
        data.meta.week
    )
    .unwrap();
    writeln!(
        out,
        r#"<p class="subtitle">{} to {}</p>"#,
        data.meta.week_start, data.meta.week_end
    )
    .unwrap();
    writeln!(out, r#"</header>"#).unwrap();

    // Deliveries
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Deliveries This Week</h2>"#).unwrap();
    if data.weekly.deliveries.is_empty() {
        writeln!(out, r#"<p class="empty">No deliveries this week.</p>"#).unwrap();
    } else {
        writeln!(out, r#"<ul class="deliveries">"#).unwrap();
        for delivery in &data.weekly.deliveries {
            let blocked_note = if delivery.was_blocked {
                r#" <span class="was-blocked">(was blocked)</span>"#
            } else {
                ""
            };
            writeln!(
                out,
                r#"<li><span class="check">☑</span> <strong>{}</strong> — {}{}</li>"#,
                html_escape(&delivery.id),
                html_escape(&delivery.name),
                blocked_note
            )
            .unwrap();
        }
        writeln!(out, r#"</ul>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Ticket Summary
    let tickets = &data.weekly.tickets;
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Ticket Summary</h2>"#).unwrap();
    writeln!(out, r#"<table class="metrics">"#).unwrap();
    writeln!(out, r#"<tr><td>Closed</td><td><strong>{}</strong> <span class="trend">{}</span> from {}</td></tr>"#,
        tickets.closed, tickets.closed_trend(), tickets.closed_prev).unwrap();
    writeln!(
        out,
        r#"<tr><td>Opened</td><td>{}</td></tr>"#,
        tickets.opened
    )
    .unwrap();
    writeln!(
        out,
        r#"<tr><td>Net</td><td><strong>{:+}</strong> toward done</td></tr>"#,
        tickets.net
    )
    .unwrap();
    writeln!(
        out,
        r#"<tr><td>Points Delivered</td><td><strong>{}</strong></td></tr>"#,
        tickets.points_delivered
    )
    .unwrap();
    writeln!(
        out,
        r#"<tr><td>Velocity (4wk avg)</td><td>{:.0}</td></tr>"#,
        tickets.velocity_avg
    )
    .unwrap();
    writeln!(out, r#"</table>"#).unwrap();
    writeln!(out, r#"</section>"#).unwrap();

    // Backlog Change
    let backlog = &data.weekly.backlog;
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Backlog Change</h2>"#).unwrap();
    writeln!(out, r#"<p>Start of week: <strong>{}</strong> tickets &rarr; End: <strong>{}</strong> tickets</p>"#,
        backlog.start, backlog.end).unwrap();
    if backlog.new_work > 0 {
        let note = html_escape(backlog.new_work_note.as_deref().unwrap_or("new work added"));
        writeln!(
            out,
            r#"<p>New work: {} tickets ({})</p>"#,
            backlog.new_work, note
        )
        .unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Capacity
    let capacity = &data.weekly.capacity;
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Capacity Factors</h2>"#).unwrap();
    if !capacity.leave.is_empty() {
        writeln!(out, r#"<ul>"#).unwrap();
        for leave in &capacity.leave {
            let reason = html_escape(leave.reason.as_deref().unwrap_or("leave"));
            writeln!(
                out,
                r#"<li><strong>{}</strong>: {} ({}% capacity)</li>"#,
                html_escape(&leave.name),
                reason,
                leave.capacity_percent
            )
            .unwrap();
        }
        writeln!(out, r#"</ul>"#).unwrap();
    }
    let perf_class = if capacity.performance_percent() >= 100.0 {
        "good"
    } else {
        "warn"
    };
    writeln!(
        out,
        r#"<p>Adjusted velocity expectation: <strong>{}</strong> points</p>"#,
        capacity.expected_velocity
    )
    .unwrap();
    writeln!(
        out,
        r#"<p>Actual: <strong class="{}">{}</strong> points ({:.0}% of target)</p>"#,
        perf_class,
        capacity.actual_velocity,
        capacity.performance_percent()
    )
    .unwrap();
    writeln!(out, r#"</section>"#).unwrap();

    // Blocked
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Blocked / External</h2>"#).unwrap();
    if data.weekly.blocked.is_empty() {
        writeln!(out, r#"<p class="empty">No blocked tickets.</p>"#).unwrap();
    } else {
        writeln!(out, r#"<ul class="blocked">"#).unwrap();
        for blocked in &data.weekly.blocked {
            writeln!(out, r#"<li><strong>{}#{}</strong>: {} <span class="blocked-on">blocked on: {}</span></li>"#,
                html_escape(&blocked.repo), blocked.number,
                html_escape(&blocked.title), html_escape(&blocked.blocked_on)).unwrap();
        }
        writeln!(out, r#"</ul>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Distractions
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Non-Project Time Sinks</h2>"#).unwrap();
    if data.weekly.distractions.is_empty() {
        writeln!(out, r#"<p class="empty">No non-project work recorded.</p>"#).unwrap();
    } else {
        writeln!(out, r#"<ul>"#).unwrap();
        for d in &data.weekly.distractions {
            let hours = d
                .estimated_hours
                .map(|h| format!(", ~{:.0}h", h))
                .unwrap_or_default();
            writeln!(
                out,
                r#"<li><strong>{}</strong>: {} tickets{}</li>"#,
                html_escape(&d.name),
                d.ticket_count,
                hours
            )
            .unwrap();
        }
        writeln!(out, r#"</ul>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    writeln!(out, r#"</div>"#).unwrap();
}

fn render_page2(out: &mut String, data: &ReportData) {
    writeln!(out, r#"<div class="page">"#).unwrap();

    // Header
    writeln!(out, r#"<header>"#).unwrap();
    writeln!(
        out,
        r#"<h1>PROJECT STATUS — {}</h1>"#,
        html_escape(&data.meta.project_name)
    )
    .unwrap();
    writeln!(
        out,
        r#"<p class="subtitle">As of {} | Day {} of {} ({:.0}% elapsed)</p>"#,
        data.meta.week_end,
        data.project.timeline.days_elapsed,
        data.project.timeline.total_days,
        data.project.timeline.percent_elapsed
    )
    .unwrap();
    writeln!(out, r#"</header>"#).unwrap();

    // CSCI Status
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>CSCI Completion</h2>"#).unwrap();
    writeln!(
        out,
        r#"<p class="note">Adjusted for backlog completeness estimate.</p>"#
    )
    .unwrap();

    for csci in &data.project.cscis {
        writeln!(out, r#"<div class="csci">"#).unwrap();
        writeln!(
            out,
            r#"<h3>{} ({}) <span class="target">Target: {}</span></h3>"#,
            html_escape(&csci.name),
            html_escape(&csci.id),
            csci.target_date
        )
        .unwrap();

        // Progress bar
        writeln!(out, r#"<div class="progress-container">"#).unwrap();
        writeln!(
            out,
            r#"<div class="progress-bar" style="width: {}%"></div>"#,
            csci.completion_percent.min(100.0)
        )
        .unwrap();
        writeln!(
            out,
            r#"<span class="progress-label">{:.0}%</span>"#,
            csci.completion_percent
        )
        .unwrap();
        writeln!(out, r#"</div>"#).unwrap();

        writeln!(out, r#"<p><strong>Tier 1</strong> (integration-ready): {}/{} | <strong>Tier 2</strong> (HIL-passed): {}/{}</p>"#,
            csci.tier1_complete, csci.total_tickets,
            csci.tier2_complete, csci.total_tickets).unwrap();

        let proj_class = match csci.projection {
            Projection::OnTrack | Projection::Complete => "good",
            Projection::AtRisk => "warn",
            Projection::Behind => "bad",
        };
        writeln!(
            out,
            r#"<p class="{}">Projection: {} {} ({} days {})</p>"#,
            proj_class,
            csci.projection.symbol(),
            csci.projection.as_str(),
            csci.buffer_days.abs(),
            if csci.buffer_days >= 0 {
                "buffer"
            } else {
                "behind"
            }
        )
        .unwrap();
        writeln!(out, r#"</div>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Dependencies
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>External Dependencies</h2>"#).unwrap();
    if data.project.dependencies.is_empty() {
        writeln!(out, r#"<p class="empty">No external dependencies.</p>"#).unwrap();
    } else {
        writeln!(out, r#"<table class="deps">"#).unwrap();
        writeln!(
            out,
            r#"<tr><th>ID</th><th>Name</th><th>Owner</th><th>RC</th><th>Final</th></tr>"#
        )
        .unwrap();
        for dep in &data.project.dependencies {
            let rc_class = if dep.rc_received {
                "good"
            } else if dep.status.is_at_risk() {
                "bad"
            } else {
                ""
            };
            let rc_sym = if dep.rc_received {
                "✓"
            } else {
                dep.status.symbol()
            };
            writeln!(out, r#"<tr><td>{}</td><td>{}</td><td>{}</td><td class="{}">{} {}</td><td>{} {}</td></tr>"#,
                html_escape(&dep.id), html_escape(&dep.name), html_escape(&dep.owner),
                rc_class, rc_sym, dep.rc_due,
                if dep.final_received { "✓" } else { "○" }, dep.final_due).unwrap();
        }
        writeln!(out, r#"</table>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Documents
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Documents</h2>"#).unwrap();
    if data.project.documents.is_empty() {
        writeln!(out, r#"<p class="empty">No documents tracked.</p>"#).unwrap();
    } else {
        writeln!(out, r#"<ul>"#).unwrap();
        for doc in &data.project.documents {
            let (class, status_str) = match doc.status {
                DocumentStatusKind::Complete => ("good", "✓ Complete".to_string()),
                DocumentStatusKind::InProgress => {
                    ("", format!("◐ In progress, due {}", doc.due_date))
                }
                DocumentStatusKind::NotStarted => {
                    ("", format!("○ Not started, due {}", doc.due_date))
                }
                DocumentStatusKind::Overdue => {
                    ("bad", format!("✗ Overdue (was due {})", doc.due_date))
                }
            };
            writeln!(
                out,
                r#"<li class="{}"><strong>{}</strong> ({}): {}</li>"#,
                class,
                html_escape(&doc.name),
                html_escape(&doc.id),
                status_str
            )
            .unwrap();
        }
        writeln!(out, r#"</ul>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Milestones
    writeln!(out, r#"<section>"#).unwrap();
    writeln!(out, r#"<h2>Upcoming Milestones</h2>"#).unwrap();
    if data.project.milestones.is_empty() {
        writeln!(out, r#"<p class="empty">No upcoming milestones.</p>"#).unwrap();
    } else {
        writeln!(out, r#"<ul>"#).unwrap();
        for m in &data.project.milestones {
            writeln!(
                out,
                r#"<li><strong>{}</strong>: {} ({} days)</li>"#,
                m.date,
                html_escape(&m.name),
                m.days_until
            )
            .unwrap();
        }
        writeln!(out, r#"</ul>"#).unwrap();
    }
    writeln!(out, r#"</section>"#).unwrap();

    // Glossary
    writeln!(out, r#"<section class="glossary">"#).unwrap();
    writeln!(out, r#"<h2>Metrics Glossary</h2>"#).unwrap();
    writeln!(out, r#"<dl>"#).unwrap();
    writeln!(out, r#"<dt>Tier 1 (Integration-Ready)</dt><dd>Code complete, unit tested, CI passing, ready for HIL integration.</dd>"#).unwrap();
    writeln!(out, r#"<dt>Tier 2 (HIL-Passed)</dt><dd>Successfully tested in hardware-in-the-loop environment, approved for formal release.</dd>"#).unwrap();
    writeln!(out, r#"<dt>T-Shirt Sizing</dt><dd>XS=1pt (&lt;2hr) | S=2pt (half day) | M=5pt (1-2 days) | L=8pt (3-5 days) | XL=13pt (week+)</dd>"#).unwrap();
    writeln!(out, r#"</dl>"#).unwrap();
    writeln!(out, r#"</section>"#).unwrap();

    writeln!(out, r#"</div>"#).unwrap();
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const HTML_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DeltaV Report</title>
    <style>
        :root {
            --bg: #ffffff;
            --fg: #1a1a1a;
            --accent: #2563eb;
            --good: #16a34a;
            --warn: #ca8a04;
            --bad: #dc2626;
            --muted: #6b7280;
            --border: #e5e7eb;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 14px;
            line-height: 1.5;
            color: var(--fg);
            background: var(--bg);
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
        }
        .page { margin-bottom: 3rem; }
        .page-break { page-break-after: always; }
        header { margin-bottom: 2rem; border-bottom: 2px solid var(--accent); padding-bottom: 1rem; }
        h1 { font-size: 1.5rem; font-weight: 600; }
        h2 { font-size: 1.1rem; font-weight: 600; margin: 1.5rem 0 0.75rem; color: var(--accent); }
        h3 { font-size: 1rem; font-weight: 600; margin-bottom: 0.5rem; }
        .subtitle { color: var(--muted); margin-top: 0.25rem; }
        section { margin-bottom: 1.5rem; }
        table { width: 100%; border-collapse: collapse; margin: 0.5rem 0; }
        th, td { padding: 0.5rem; text-align: left; border-bottom: 1px solid var(--border); }
        th { font-weight: 600; background: #f9fafb; }
        ul { list-style: none; }
        li { padding: 0.25rem 0; }
        .check { color: var(--good); }
        .trend { color: var(--muted); }
        .good { color: var(--good); }
        .warn { color: var(--warn); }
        .bad { color: var(--bad); }
        .empty { color: var(--muted); font-style: italic; }
        .note { color: var(--muted); font-size: 0.9rem; }
        .was-blocked { color: var(--muted); font-size: 0.9rem; }
        .blocked-on { color: var(--bad); font-style: italic; }
        .target { font-weight: normal; color: var(--muted); float: right; }
        .csci { background: #f9fafb; padding: 1rem; border-radius: 0.5rem; margin: 1rem 0; }
        .progress-container {
            background: var(--border);
            border-radius: 0.25rem;
            height: 1.5rem;
            position: relative;
            margin: 0.5rem 0;
        }
        .progress-bar {
            background: var(--accent);
            height: 100%;
            border-radius: 0.25rem;
            transition: width 0.3s;
        }
        .progress-label {
            position: absolute;
            right: 0.5rem;
            top: 50%;
            transform: translateY(-50%);
            font-weight: 600;
            font-size: 0.8rem;
        }
        .glossary { background: #f9fafb; padding: 1rem; border-radius: 0.5rem; margin-top: 2rem; }
        .glossary dt { font-weight: 600; margin-top: 0.5rem; }
        .glossary dd { color: var(--muted); margin-left: 1rem; }
        @media print {
            body { padding: 0; max-width: none; }
            .page-break { page-break-after: always; }
        }
    </style>
</head>
<body>
"#;

const HTML_FOOTER: &str = r#"
</body>
</html>
"#;

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
    fn test_render_produces_valid_html() {
        let report = sample_report();
        let html = render(&report);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("Test Project"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }
}
