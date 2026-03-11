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
mod init;
mod report;
mod schema;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, OutputFormat};
use std::io::Write;
use std::path::PathBuf;

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
        Command::Serve {
            data_dir,
            config_dir,
            port,
        } => cmd_serve(data_dir, config_dir, port),
    }
}

/// Run as a long-lived container service implementing the Initializing → Running → Stopped lifecycle.
///
/// This is the primary container entrypoint. The process starts in the Initializing state,
/// creates the `/data` directory structure, validates configuration from `/config/project.toml`,
/// then binds the web server and transitions to the Running state. On SIGTERM/SIGINT or
/// unrecoverable error, the process transitions to Stopped.
///
/// # Arguments
///
/// * `data_dir` - Path to the persistent data directory (e.g. `/data`).
/// * `config_dir` - Path to the configuration directory containing `project.toml`.
/// * `port` - TCP port to bind the web server on.
///
/// # Errors
///
/// Returns an error if data directory initialization fails, configuration is invalid,
/// the web server cannot bind, or the async runtime cannot be created.
fn cmd_serve(data_dir: PathBuf, config_dir: PathBuf, port: u16) -> Result<()> {
    // Build the tokio runtime and block on the async serve routine
    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
    rt.block_on(async_serve(data_dir, config_dir, port))
}

/// Async implementation of the serve lifecycle.
///
/// # Arguments
///
/// * `data_dir` - Path to the persistent data directory.
/// * `config_dir` - Path to the configuration directory containing `project.toml`.
/// * `port` - TCP port to bind the web server on.
///
/// # Errors
///
/// Returns an error if initialization, config loading, or web server binding fails.
async fn async_serve(data_dir: PathBuf, config_dir: PathBuf, port: u16) -> Result<()> {
    // --- Initializing state ---
    eprintln!("deltav: entering Initializing state");

    // Step 1: Initialize data directory structure
    eprintln!(
        "deltav: initializing data directory at {}",
        data_dir.display()
    );
    init::initialize_data_dir(&data_dir)?;

    // Step 2: Load and validate configuration
    let config_path = config_dir.join("project.toml");
    eprintln!(
        "deltav: loading configuration from {}",
        config_path.display()
    );
    let _config = schema::ProjectConfig::load(&config_path).with_context(|| {
        format!(
            "Initialization failed: ensure a valid project.toml exists at {}",
            config_path.display()
        )
    })?;

    // Step 3: Build router with health and root endpoints
    let app = axum::Router::new()
        .route("/health", axum::routing::get(handle_health))
        .route("/", axum::routing::get(handle_root));

    // Step 4: Bind web server
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind web server on port {}", port))?;

    // --- Transition to Running state ---
    eprintln!("deltav: entering Running state");
    eprintln!("deltav running on port {}", port);

    // Step 5: Serve with graceful shutdown on SIGTERM/SIGINT
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Web server error")?;

    // --- Transition to Stopped state ---
    eprintln!("deltav: entering Stopped state");
    eprintln!("deltav: graceful shutdown complete");

    Ok(())
}

/// Wait for a SIGTERM or SIGINT signal for graceful shutdown.
///
/// If a signal handler cannot be installed, logs a warning and falls back
/// to waiting on the other signal indefinitely.
async fn shutdown_signal() {
    let ctrl_c = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => eprintln!("deltav: received SIGINT, shutting down"),
            Err(e) => {
                eprintln!("deltav: failed to install SIGINT handler: {e}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
                eprintln!("deltav: received SIGTERM, shutting down");
            }
            Err(e) => {
                eprintln!("deltav: failed to install SIGTERM handler: {e}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

/// Handler for `GET /health`.
///
/// # Returns
///
/// A 200 OK response with body `"OK"`.
async fn handle_health() -> &'static str {
    "OK"
}

/// Handler for `GET /`.
///
/// # Returns
///
/// A 200 OK HTML response with a placeholder status page.
async fn handle_root() -> axum::response::Html<&'static str> {
    axum::response::Html(
        "<html><head><title>deltav</title></head><body>\
         <h1>deltav</h1><p>DevSecOps metrics aggregator is running.</p>\
         </body></html>",
    )
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
    _token: Option<String>,
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

    // For now, generate a sample report without GitHub data
    // In a full implementation, we'd fetch from GitHub here
    let report_data = generate_sample_report(&config, &week_str, week_start, week_end)?;

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

/// Generate a sample report for testing (without GitHub API calls).
fn generate_sample_report(
    config: &schema::ProjectConfig,
    week_str: &str,
    week_start: chrono::NaiveDate,
    week_end: chrono::NaiveDate,
) -> Result<report::ReportData> {
    use report::data::*;

    let as_of = week_end;

    // Build meta
    let meta = ReportMeta {
        project_name: config.project.name.clone(),
        week: week_str.to_string(),
        week_start,
        week_end,
        generated_at: chrono::Utc::now(),
    };

    // Build weekly summary (placeholder data)
    let weekly = WeeklySummary {
        deliveries: vec![],
        tickets: TicketSummary {
            closed: 0,
            closed_prev: 0,
            opened: 0,
            net: 0,
            points_delivered: 0,
            velocity_avg: 0.0,
        },
        backlog: BacklogChange {
            start: 0,
            end: 0,
            new_work: 0,
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
                        capacity_percent: 0, // Would calculate based on overlap
                        reason: l.reason.clone(),
                    })
                })
                .collect(),
            expected_velocity: 0,
            actual_velocity: 0,
        },
        blocked: vec![],
        distractions: vec![],
    };

    // Build project status
    let project = ProjectStatus {
        timeline: ProjectTimeline {
            days_elapsed: config.project.days_elapsed(as_of),
            total_days: config.project.duration_days(),
            percent_elapsed: config.project.percent_elapsed(as_of),
        },
        cscis: config
            .deliverables
            .csci
            .iter()
            .map(|c| CsciStatus {
                id: c.id.clone(),
                name: c.name.clone(),
                target_date: c.target_date,
                days_until: c.days_until_target(as_of),
                total_tickets: 0, // Would fetch from GitHub
                tier1_complete: 0,
                tier2_complete: 0,
                completion_percent: 0.0,
                projection: Projection::OnTrack,
                buffer_days: 0,
            })
            .collect(),
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
                rc_received: false, // Would need manual tracking
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
