//! deltav - DevSecOps metrics aggregator for GitHub Enterprise
//!
//! Tracks delivery metrics for systems engineering projects using data from
//! GitHub Enterprise. Generates weekly reports showing team velocity, CSCI
//! completion, and external dependency status.

#![deny(unsafe_code)]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::all))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(clippy::pedantic))]
#![cfg_attr(all(not(debug_assertions), not(test)), deny(missing_docs))]
// Allow some pedantic lints that are too strict for this project
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(unused_imports)]
#![allow(dead_code)]

mod cli;
mod github;
mod report;
mod schema;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, OutputFormat};
use std::io::Write;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { output } => cmd_init(output),
        Command::Schema { output } => cmd_schema(output),
        Command::Validate { config } => cmd_validate(config),
        Command::Report {
            config,
            week,
            format,
            output,
            token,
        } => cmd_report(config, week, format, output, token),
    }
}

/// Generate a stub project.toml with example values.
fn cmd_init(output: Option<std::path::PathBuf>) -> Result<()> {
    let stub = schema::ProjectConfig::stub();
    let toml = toml::to_string_pretty(&stub).context("Failed to serialize stub config")?;

    // Add helpful comments
    let commented = add_stub_comments(&toml);

    if let Some(path) = output {
        std::fs::write(&path, &commented)
            .with_context(|| format!("Failed to write to {}", path.display()))?;
        eprintln!("Created {}", path.display());
    } else {
        println!("{}", commented);
    }

    Ok(())
}

/// Add helpful comments to the stub TOML.
fn add_stub_comments(toml: &str) -> String {
    let header = r#"# deltav Project Configuration
# =============================
#
# This file defines your project for metrics tracking.
# Edit the values below to match your project structure.
#
# For JSON schema (editor autocomplete): deltav schema > project.schema.json
#

"#;

    let mut out = String::from(header);
    out.push_str(toml);
    out
}

/// Output JSON schema for project.toml.
fn cmd_schema(output: Option<std::path::PathBuf>) -> Result<()> {
    let schema = schema::ProjectConfig::json_schema();
    let json = serde_json::to_string_pretty(&schema).context("Failed to serialize schema")?;

    if let Some(path) = output {
        std::fs::write(&path, &json)
            .with_context(|| format!("Failed to write to {}", path.display()))?;
        eprintln!("Created {}", path.display());
    } else {
        println!("{}", json);
    }

    Ok(())
}

/// Validate a project.toml file.
fn cmd_validate(config: std::path::PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(&config)
        .with_context(|| format!("Failed to read {}", config.display()))?;

    let parsed: schema::ProjectConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", config.display()))?;

    // Validate regex patterns
    for org in &parsed.github.organisations {
        org.validate_pattern()
            .with_context(|| format!("Invalid repo_pattern for org '{}'", org.name))?;
    }

    // Validate date ranges
    if parsed.project.end_date <= parsed.project.start_date {
        anyhow::bail!("Project end_date must be after start_date");
    }

    // Validate backlog completeness
    if parsed.project.backlog_completeness <= 0.0 || parsed.project.backlog_completeness > 1.0 {
        anyhow::bail!("backlog_completeness must be between 0.0 (exclusive) and 1.0 (inclusive)");
    }

    // Check for duplicate IDs
    let mut ids = std::collections::HashSet::new();
    for id in parsed.deliverables.all_ids() {
        if !ids.insert(id) {
            anyhow::bail!("Duplicate deliverable ID: {}", id);
        }
    }

    eprintln!("✓ {} is valid", config.display());
    eprintln!("  Project: {}", parsed.project.name);
    eprintln!(
        "  Duration: {} to {} ({} days)",
        parsed.project.start_date,
        parsed.project.end_date,
        parsed.project.duration_days()
    );
    eprintln!("  Team: {} members", parsed.team.headcount());
    eprintln!("  Organisations: {}", parsed.github.org_names().join(", "));
    eprintln!("  CSCIs: {}", parsed.deliverables.csci.len());
    eprintln!("  Documents: {}", parsed.deliverables.documents.len());
    eprintln!(
        "  External dependencies: {}",
        parsed.dependencies.external.len()
    );

    Ok(())
}

/// Generate a weekly report.
fn cmd_report(
    config_path: std::path::PathBuf,
    week: Option<String>,
    format: OutputFormat,
    output: Option<std::path::PathBuf>,
    token: Option<String>,
) -> Result<()> {
    // Load and validate config
    let config = schema::ProjectConfig::load(&config_path)
        .with_context(|| format!("Failed to load {}", config_path.display()))?;

    // Parse week or use current
    let (year, week_num) = if let Some(w) = week {
        cli::parse_iso_week(&w)?
    } else {
        cli::current_iso_week()
    };

    let week_str = cli::format_iso_week(year, week_num);
    eprintln!("Generating report for {}", week_str);

    // Calculate week boundaries
    let week_start = iso_week_to_date(year, week_num)?;
    let week_end = week_start + chrono::Duration::days(6);

    // Resolve GitHub token
    let hostname = github::extract_hostname(&config.github.enterprise_url);
    let resolved_token = github::resolve_token(token.as_deref(), hostname.as_deref())?;

    // Create GitHub client
    let client = github::GitHubClient::new(config.github.clone(), &resolved_token)?;

    // Test connection
    let user = client
        .test_connection()
        .context("Failed to connect to GitHub")?;
    eprintln!("Authenticated as: {}", user.login);

    // Fetch data and generate report
    let report_data =
        generate_report_from_github(&client, &config, &week_str, week_start, week_end)?;

    // Render based on format
    match format {
        OutputFormat::Markdown => {
            let md = report::markdown::render(&report_data);
            write_output(&md, output.as_ref(), "md")?;
        }
        OutputFormat::Html => {
            let html = report::html::render(&report_data);
            write_output(&html, output.as_ref(), "html")?;
        }
        OutputFormat::Pdf => {
            // Generate HTML first, then convert to PDF
            // For now, just output HTML with a note
            let html = report::html::render(&report_data);
            eprintln!("Note: PDF generation requires additional setup. Generating HTML instead.");
            write_output(&html, output.as_ref(), "html")?;
        }
        OutputFormat::All => {
            let base_path = output.unwrap_or_else(|| std::path::PathBuf::from("."));
            let base_name = format!("report-{}", week_str);

            let md = report::markdown::render(&report_data);
            let md_path = base_path.join(format!("{}.md", base_name));
            std::fs::write(&md_path, &md)?;
            eprintln!("Created {}", md_path.display());

            let html = report::html::render(&report_data);
            let html_path = base_path.join(format!("{}.html", base_name));
            std::fs::write(&html_path, &html)?;
            eprintln!("Created {}", html_path.display());
        }
    }

    Ok(())
}

/// Write output to file or stdout.
fn write_output(content: &str, output: Option<&std::path::PathBuf>, ext: &str) -> Result<()> {
    if let Some(path) = output {
        let final_path = if path.is_dir() {
            path.join(format!("report.{}", ext))
        } else {
            path.clone()
        };
        std::fs::write(&final_path, content)?;
        eprintln!("Created {}", final_path.display());
    } else {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(content.as_bytes())?;
    }
    Ok(())
}

/// Convert ISO week to Monday date.
fn iso_week_to_date(year: i32, week: u32) -> Result<chrono::NaiveDate> {
    use chrono::{Datelike, NaiveDate};

    // Find Jan 4 of the year (always in week 1)
    let jan4 = NaiveDate::from_ymd_opt(year, 1, 4).context("Invalid year")?;

    // Find Monday of week 1
    let week1_monday = jan4 - chrono::Duration::days(jan4.weekday().num_days_from_monday() as i64);

    // Add weeks
    let target = week1_monday + chrono::Duration::weeks((week - 1) as i64);

    Ok(target)
}

/// Generate a report by fetching data from GitHub.
fn generate_report_from_github(
    client: &github::GitHubClient,
    config: &schema::ProjectConfig,
    week_str: &str,
    week_start: chrono::NaiveDate,
    week_end: chrono::NaiveDate,
) -> Result<report::ReportData> {
    use chrono::{TimeZone, Utc};
    use report::data::*;

    let as_of = week_end;

    // Convert week boundaries to UTC timestamps for API queries
    let week_start_utc = Utc.from_utc_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap());
    let _week_end_utc = Utc.from_utc_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap());

    // Fetch issues from all configured organizations
    let mut all_issues: Vec<github::Issue> = Vec::new();
    let mut repos_fetched = 0;

    for org in &config.github.organisations {
        eprintln!("Fetching repositories for org: {}", org.name);
        let repos = client.fetch_repos(&org.name)?;
        eprintln!("  Found {} matching repositories", repos.len());

        for repo in &repos {
            eprintln!("  Fetching issues from {}/{}", org.name, repo.name);
            // Fetch issues updated since start of week (to catch closed issues)
            let issues = client.fetch_issues(
                &org.name,
                &repo.name,
                Some(week_start_utc),
                github::IssueState::All,
            )?;
            eprintln!("    Found {} issues", issues.len());
            all_issues.extend(issues);
            repos_fetched += 1;
        }
    }

    eprintln!(
        "Total: {} issues from {} repositories",
        all_issues.len(),
        repos_fetched
    );

    // Calculate ticket metrics
    let (closed_this_week, opened_this_week, points_delivered) =
        calculate_ticket_metrics(&all_issues, week_start, week_end, &config.sizing);

    // Find blocked issues (issues with "blocked" label)
    let blocked_tickets = find_blocked_tickets(&all_issues);

    // Count open issues for backlog
    let open_issues = all_issues.iter().filter(|i| i.is_open()).count() as u32;

    // Build meta
    let meta = ReportMeta {
        project_name: config.project.name.clone(),
        week: week_str.to_string(),
        week_start,
        week_end,
        generated_at: chrono::Utc::now(),
    };

    // Build weekly summary
    let weekly = WeeklySummary {
        deliveries: vec![], // Would need to track completed CSCIs/documents
        tickets: TicketSummary {
            closed: closed_this_week,
            closed_prev: 0, // Would need to fetch previous week
            opened: opened_this_week,
            net: closed_this_week as i32 - opened_this_week as i32,
            points_delivered,
            velocity_avg: points_delivered as f64, // Would need historical data
        },
        backlog: BacklogChange {
            start: open_issues + closed_this_week, // Estimate
            end: open_issues,
            new_work: opened_this_week,
            new_work_note: None,
        },
        capacity: CapacitySummary {
            nominal: config.team.nominal_capacity(),
            actual: config.team.average_capacity(week_start, week_end),
            leave: config
                .team
                .leave_in_range(week_start, week_end)
                .iter()
                .filter_map(|l| {
                    config.team.member_by_github(&l.github).map(|m| LeaveEntry {
                        name: m.name.clone(),
                        capacity_percent: 0,
                        reason: l.reason.clone(),
                    })
                })
                .collect(),
            expected_velocity: 0,
            actual_velocity: points_delivered,
        },
        blocked: blocked_tickets,
        distractions: fetch_distractions(client, config, week_start_utc)?,
    };

    // Build project status
    let project = ProjectStatus {
        timeline: ProjectTimeline {
            days_elapsed: config.project.days_elapsed(as_of),
            total_days: config.project.duration_days(),
            percent_elapsed: config.project.percent_elapsed(as_of),
        },
        cscis: calculate_csci_status(
            &all_issues,
            &config.deliverables.csci,
            as_of,
            config.project.backlog_completeness,
        ),
        dependencies: config
            .dependencies
            .external
            .iter()
            .map(|d| DependencyStatus {
                id: d.id.clone(),
                name: d.name.clone(),
                owner: d.owner.clone(),
                rc_due: d.rc_due,
                final_due: d.final_due,
                rc_received: false,
                final_received: false,
                status: if d.is_rc_overdue(as_of) {
                    DependencyStatusKind::RcOverdue
                } else {
                    DependencyStatusKind::Pending
                },
            })
            .collect(),
        documents: config
            .deliverables
            .documents
            .iter()
            .map(|doc| DocumentStatus {
                id: doc.id.clone(),
                name: doc.name.clone(),
                due_date: doc.due_date,
                status: if doc.is_overdue(as_of) {
                    DocumentStatusKind::Overdue
                } else {
                    DocumentStatusKind::NotStarted
                },
                completed_date: None,
                note: None,
            })
            .collect(),
        milestones: config
            .deliverables
            .upcoming_milestones(as_of, 90)
            .iter()
            .map(|m| MilestoneStatus {
                id: m.id.clone(),
                name: m.name.clone(),
                date: m.date,
                days_until: m.days_until(as_of),
            })
            .collect(),
    };

    Ok(ReportData {
        meta,
        weekly,
        project,
    })
}

/// Calculate ticket metrics for the week.
fn calculate_ticket_metrics(
    issues: &[github::Issue],
    week_start: chrono::NaiveDate,
    week_end: chrono::NaiveDate,
    sizing: &schema::Sizing,
) -> (u32, u32, u32) {
    let mut closed = 0u32;
    let mut opened = 0u32;
    let mut points = 0u32;

    for issue in issues {
        let created_date = issue.created_at.date_naive();
        let closed_date = issue.closed_at.map(|d| d.date_naive());

        // Count opened this week
        if created_date >= week_start && created_date <= week_end {
            opened += 1;
        }

        // Count closed this week and sum points
        let closed_in_range = closed_date
            .filter(|&d| d >= week_start && d <= week_end)
            .is_some();

        if closed_in_range {
            closed += 1;

            // Get points from size label
            let issue_points = issue
                .size_label()
                .and_then(|label| sizing.points_for(label))
                .unwrap_or(0);
            points += issue_points;
        }
    }

    (closed, opened, points)
}

/// Find blocked tickets (issues with "blocked" label).
fn find_blocked_tickets(issues: &[github::Issue]) -> Vec<report::data::BlockedTicket> {
    issues
        .iter()
        .filter(|i| i.is_open() && i.has_label_prefix("blocked"))
        .map(|i| {
            let (org, repo) = i.org_repo().unwrap_or(("unknown", "unknown"));
            report::data::BlockedTicket {
                repo: format!("{}/{}", org, repo),
                number: i.number,
                title: i.title.clone(),
                blocked_on: i
                    .labels
                    .iter()
                    .find(|l| l.name.starts_with("blocked"))
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
            }
        })
        .collect()
}

/// Fetch distraction work from configured repositories.
fn fetch_distractions(
    client: &github::GitHubClient,
    config: &schema::ProjectConfig,
    since: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<report::data::DistractionSummary>> {
    let mut distractions = Vec::new();

    for distraction in &config.github.distractions {
        let issues = client.fetch_issues(
            &distraction.org,
            &distraction.repo,
            Some(since),
            github::IssueState::All,
        )?;

        // Filter by label if specified
        let filtered: Vec<_> = if let Some(ref label) = distraction.label {
            issues.into_iter().filter(|i| i.has_label(label)).collect()
        } else {
            issues
        };

        if !filtered.is_empty() {
            let assignees: Vec<String> = filtered
                .iter()
                .flat_map(|i| i.assignee_logins())
                .map(|s| s.to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            distractions.push(report::data::DistractionSummary {
                name: distraction.name.clone(),
                ticket_count: filtered.len() as u32,
                estimated_hours: None,
                assignees,
            });
        }
    }

    Ok(distractions)
}

/// Calculate CSCI status by matching issues to CSCIs.
fn calculate_csci_status(
    issues: &[github::Issue],
    cscis: &[schema::Csci],
    as_of: chrono::NaiveDate,
    backlog_completeness: f64,
) -> Vec<report::data::CsciStatus> {
    use report::data::{CsciStatus, Projection};

    cscis
        .iter()
        .map(|csci| {
            // Count issues belonging to this CSCI's repositories
            let csci_issues: Vec<_> = issues
                .iter()
                .filter(|i| {
                    i.org_repo()
                        .map(|(org, repo)| csci.contains_repo(org, repo))
                        .unwrap_or(false)
                })
                .collect();

            let total = csci_issues.len() as u32;
            let closed = csci_issues.iter().filter(|i| i.is_closed()).count() as u32;

            // Tier 1 = closed, Tier 2 would require label checking
            let tier1 = closed;
            let tier2 = csci_issues
                .iter()
                .filter(|i| i.is_closed() && i.has_label("tier2-complete"))
                .count() as u32;

            // Adjusted completion (accounting for undiscovered work)
            let raw_completion = if total > 0 {
                (tier1 as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            let adjusted_completion = raw_completion * backlog_completeness;

            // Simple projection based on completion vs time elapsed
            let days_until = csci.days_until_target(as_of);
            let projection = if adjusted_completion >= 100.0 {
                Projection::Complete
            } else if days_until < 0 {
                Projection::Behind
            } else if adjusted_completion < 50.0 && days_until < 30 {
                Projection::AtRisk
            } else {
                Projection::OnTrack
            };

            CsciStatus {
                id: csci.id.clone(),
                name: csci.name.clone(),
                target_date: csci.target_date,
                days_until,
                total_tickets: total,
                tier1_complete: tier1,
                tier2_complete: tier2,
                completion_percent: adjusted_completion,
                projection,
                buffer_days: 0, // Would need velocity calculation
            }
        })
        .collect()
}
