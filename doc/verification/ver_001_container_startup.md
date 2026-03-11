# VER-001: Container Startup Behavior

## Verification Method

- **Demonstration:** Verified by operating the container and observing state transitions.

## Verifies Requirements

- states_and_modes.md — State: Initializing (entry, exit conditions, capabilities, restrictions)
- states_and_modes.md — State: Running (entry conditions, capabilities)
- states_and_modes.md — State: Stopped (entry conditions from init failure and graceful shutdown)
- states_and_modes.md — State Transition Table (all five transitions)
- DD-001: Docker Container Execution Model (volume mounts, `/data` initialization, web interface)

## Verified Design Units

- TBD — Design units for the startup initialization routine and web server binding have not yet been created.

## Preconditions

- Docker and docker-compose are installed and functional.
- The deltav container image has been built (`docker-compose build` or equivalent).
- A valid `project.toml` configuration file is available for Scenario A and C.
- The `/data` host-side volume directory is writable by the container process.
- No other process is bound to the container's configured port (default: 8080).

## Procedure

### Scenario A — Success Path (Initializing to Running)

Verifies: Initializing state entry, `/data` structure creation, config loading, transition to Running.

1. Prepare a valid `project.toml` in the host directory mapped to `/config`.
2. Ensure the host directory mapped to `/data` is empty (or remove any prior `/data/reports` and `/data/cache` subdirectories to confirm creation).
3. Start the container: `docker-compose up -d`.
4. Observe container logs: `docker-compose logs -f deltav`.
5. Confirm the following log entries appear in order:
   a. An initialization message indicating the process has started (Initializing state entered).
   b. Creation of `/data/reports` and `/data/cache` subdirectories (or confirmation they already exist).
   c. Configuration loaded from `/config/project.toml` without errors.
   d. Web server bound to the configured port.
   e. A message indicating the system is now serving (Running state entered).
6. Send an HTTP request to the health endpoint: `curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health`.
7. Record the HTTP status code.

### Scenario B — Missing Configuration (Initializing to Stopped)

Verifies: Initializing state exit on config absence, transition to Stopped with non-zero exit.

1. Ensure the host directory mapped to `/config` does **not** contain `project.toml` (remove or rename it).
2. Start the container: `docker-compose up deltav`.
3. Observe container logs.
4. Confirm the following:
   a. An initialization message indicating the process has started (Initializing state entered).
   b. An error log message identifying that the configuration file is absent or cannot be read.
   c. The container process exits.
5. Check the container exit code: `docker inspect --format='{{.State.ExitCode}}' <container_id>`.
6. Confirm the web server was never reachable: `curl -s http://localhost:8080/health` should fail to connect (connection refused or no response).
7. Record the exit code.

### Scenario C — Graceful Shutdown (Running to Stopped)

Verifies: Running state exit on SIGTERM, transition to Stopped with zero exit.

1. Complete Scenario A to reach the Running state (container up and healthy).
2. Send SIGTERM to the container: `docker-compose stop deltav` (or `docker kill --signal=SIGTERM <container_id>`).
3. Observe container logs.
4. Confirm the following:
   a. A log message indicating a shutdown signal was received.
   b. The container process exits.
5. Check the container exit code: `docker inspect --format='{{.State.ExitCode}}' <container_id>`.
6. Record the exit code.

## Expected Results

### Scenario A — Success Path

- **Pass Criteria:**
  - Container logs show the Initializing, directory setup, config loaded, and Running sequence without errors.
  - `/data/reports` and `/data/cache` directories exist inside the container (or on the host volume).
  - `curl` to `/health` returns HTTP 200.
- **Fail Criteria:**
  - Container exits during startup.
  - `/data` subdirectories are not created.
  - `/health` returns a non-200 status or is unreachable.

### Scenario B — Missing Configuration

- **Pass Criteria:**
  - Container logs show an error message that clearly identifies the missing configuration file.
  - Container exits with a non-zero exit code.
  - The web server never accepts connections (no HTTP response from `/health`).
- **Fail Criteria:**
  - Container enters Running state without valid configuration.
  - Container exits with exit code 0.
  - Error message does not identify the cause (missing config).

### Scenario C — Graceful Shutdown

- **Pass Criteria:**
  - Container logs show a shutdown message after SIGTERM.
  - Container exits with exit code 0.
- **Fail Criteria:**
  - Container does not respond to SIGTERM within a reasonable timeout (e.g., 10 seconds).
  - Container exits with a non-zero exit code after SIGTERM.
  - No shutdown log message is produced.

## Test Implementation

Automated tests are TBD. The current verification method is manual execution of the procedure described above. When automated integration tests are added in a future milestone, this document should be updated to reference the test files and the Verification Method changed to "Test".

## Notes

- This verification covers the container skeleton milestone. The startup routine and web server are minimal at this stage; the procedure validates the state machine scaffolding, not full business logic.
- Scenario B only tests absence of the config file. Validation of malformed configuration content is a separate verification concern.
- The unrecoverable runtime error transition (Running to Stopped, non-zero exit) is not covered here as it requires fault injection beyond the skeleton milestone scope.
