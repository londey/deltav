# Concept of Execution

This document describes the runtime behavior of the system: how it starts up, how data flows through it, and how it responds to events.

## System Overview

deltav runs as a long-lived Docker container process. On startup it initializes persistent storage, then enters a serving loop that handles web requests from operators. Each request triggers a fetch-compute-render cycle: configuration is read from `/config`, live data is fetched from the GitHub Enterprise API, delivery metrics are computed, and a report is rendered and returned. Rendered reports are also persisted to `/data/reports/`.

## Operational Modes

Reference: `doc/requirements/states_and_modes.md`

The container passes through three states at runtime: Initializing, Running, and Stopped. See states_and_modes.md for entry/exit conditions and capabilities per state.

## Startup Sequence

1. Container process starts; the Rust binary is invoked as the container entrypoint.
2. The binary checks for the existence of the `/data` directory structure. If any required subdirectories are absent (`/data/reports`, `/data/cache`), they are created.
3. The binary reads and validates `project.toml` from `/config`. If the file is absent or invalid, the process logs an error and exits with a non-zero status.
4. The web server binds to the configured port (default: 8080) and begins accepting requests.
5. The system transitions to the Running state.

## Data Flow

```
┌──────────┐  HTTP request  ┌─────────────────┐  project.toml  ┌─────────┐
│ Operator │───────────────▶│  Web Handler    │◀───────────────│ /config │
│ Browser  │                │  (Rust/leptos)  │                └─────────┘
└──────────┘                └────────┬────────┘
                                     │ GitHub API calls
                                     ▼
                            ┌─────────────────┐
                            │  GitHub         │
                            │  Enterprise API │
                            └────────┬────────┘
                                     │ issue/PR data
                                     ▼
                            ┌─────────────────┐   rendered report   ┌───────┐
                            │  Metrics Engine │────────────────────▶│ /data │
                            │  + Renderer     │                      └───────┘
                            └─────────────────┘
```

## Event Handling

### Event: Web request for report

- **Source:** Operator browser
- **Handler:** Web handler (design unit TBD)
- **Response:** Read configuration, fetch GitHub data, compute metrics, render report, write to `/data/reports/`, return rendered document.

### Event: `/data` directory missing on startup

- **Source:** Container filesystem check during initialization
- **Handler:** Startup initialization routine
- **Response:** Create required subdirectories under `/data`. Log creation. Continue startup.

### Event: `project.toml` absent or invalid on startup

- **Source:** Configuration loader during initialization
- **Handler:** Startup initialization routine
- **Response:** Log a descriptive error message identifying the missing or invalid field. Exit with non-zero status. Container stops (transitions to Stopped state).

## Timing and Synchronization

GitHub API calls are made synchronously per request in the initial implementation. No background polling or scheduled fetches occur in the skeleton milestone.

## Error Handling

GitHub API errors (network failure, rate limiting, authentication failure) are surfaced to the operator as an error page with a descriptive message. The container remains in the Running state and continues to accept subsequent requests.

Configuration errors detected at startup cause the container to exit immediately (see Startup Sequence, step 3).

## Resource Management

The container uses no in-process database. GitHub data is fetched per request and held in memory only for the duration of that request. Generated reports written to `/data/reports/` accumulate over time; operators are responsible for managing disk usage of the `/data` volume.
