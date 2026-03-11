# Design Decisions

This document records significant design decisions using a lightweight Architecture Decision Record (ADR) format.

## Template

When adding a new decision, copy this template:

```markdown
## DD-NNN: <Title>

**Date:** YYYY-MM-DD  
**Status:** Proposed | Accepted | Superseded by DD-XXX

### Context

<What is the issue or question that needs a decision?>

### Decision

<What is the decision that was made?>

### Rationale

<Why was this decision made? What alternatives were considered?>

### Consequences

<What are the implications of this decision?>
```

---

## Decisions

<!-- Add decisions below, newest first -->

## DD-001: Docker Container Execution Model

**Date:** 2026-03-11
**Status:** Accepted

### Context

The system must be deployable in environments where operators do not manage Rust toolchains or CLI invocation scripts. The system requires persistent storage for generated reports and caches, and read-only access to project configuration. A mechanism is needed to define and enforce these runtime dependencies consistently across deployments.

### Decision

deltav runs as a Docker container managed by `docker-compose`. The container exposes a web interface rather than a CLI. Two volume mounts are required at runtime: `/data` (persistent read-write storage) and `/config` (read-only configuration). The container initializes the `/data` directory structure on first startup if it does not exist.

### Rationale

A container-based model bundles all runtime dependencies, removes operator toolchain requirements, and makes volume conventions explicit and enforceable via `docker-compose.yml`. A web interface is more appropriate than a CLI for a dashboard application accessed by multiple team members. Alternatives considered: a standalone CLI binary distributed via package manager (rejected — requires toolchain or pre-built binaries per platform); a serverless/function model (rejected — adds infrastructure complexity without benefit at this scale).

### Consequences

- All deployment documentation references `docker-compose` as the entry point.
- The `/data` and `/config` mount points are stable contracts; changing them is a breaking change requiring a new decision record.
- Integration tests must launch the container rather than invoking a CLI binary directly.
