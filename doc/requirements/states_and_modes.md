# States and Modes

This document defines the operational states and modes of the system.

## Definitions

- **State:** A condition of the system characterized by specific behaviors and capabilities
- **Mode:** A variant of operation that affects how the system behaves within a state

## System States

### State: Initializing

- **Description:** The container process has started and is performing pre-service checks and setup. The web interface is not yet available.
- **Entry Conditions:** Container process starts (Docker entrypoint invoked).
- **Exit Conditions:** Initialization succeeds (all checks pass, `/data` structure confirmed, configuration valid) — transitions to Running. Initialization fails (configuration absent or invalid, unrecoverable filesystem error) — transitions to Stopped.
- **Capabilities:** Filesystem access to `/data` and `/config`; ability to create directories under `/data`.
- **Restrictions:** The system does not accept web requests. No GitHub API calls are made.

### State: Running

- **Description:** The web server is bound and accepting requests. The system is fully operational.
- **Entry Conditions:** Successful completion of Initializing state.
- **Exit Conditions:** Unrecoverable runtime error; SIGTERM or SIGINT received from the container runtime. Transitions to Stopped.
- **Capabilities:** Accept and handle web requests; read configuration from `/config`; make GitHub API calls; compute metrics; render and persist reports to `/data/reports/`.
- **Restrictions:** None under normal operation.

### State: Stopped

- **Description:** The container process has exited. No requests are handled.
- **Entry Conditions:** Initialization failure; graceful shutdown signal (SIGTERM/SIGINT); unrecoverable runtime error.
- **Exit Conditions:** N/A — the container must be restarted by the orchestrator to re-enter Initializing.
- **Capabilities:** None.
- **Restrictions:** All operations unavailable.

## Operational Modes

No distinct operational modes are defined for the current implementation. All web requests are handled identically. Future milestones may introduce a read-only mode (when GitHub credentials are absent) or a maintenance mode.

## State Transition Diagram

```
                    ┌──────────────────┐
    [container  ]   │                  │  [init success]
    [started    ]──▶│   Initializing   │───────────────────▶┌─────────────┐
                    │                  │                     │             │
                    └──────────────────┘                     │   Running   │
                             │                               │             │
                    [init    │                               └──────┬──────┘
                    [failed  │                                      │
                             │                          [SIGTERM /  │
                             │                          runtime err]│
                             ▼                                      │
                    ┌──────────────────┐                            │
                    │     Stopped      │◀───────────────────────────┘
                    └──────────────────┘
```

## State Transition Table

| Current State | Event / Condition | Next State | Actions |
|---------------|-------------------|------------|---------|
| Initializing | `/data` structure confirmed, config valid | Running | Bind web server port, begin accepting requests |
| Initializing | Config file absent or invalid | Stopped | Log error, exit non-zero |
| Initializing | Unrecoverable filesystem error | Stopped | Log error, exit non-zero |
| Running | SIGTERM or SIGINT received | Stopped | Complete in-flight requests, close connections, exit zero |
| Running | Unrecoverable runtime error | Stopped | Log error, exit non-zero |
