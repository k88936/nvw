## Build, Test, and Lint Commands

This is a Rust workspace with three members: `server`, `worker`, and `proto`.

- **Build Workspace:** `cargo build --workspace`
- **Lint:** `cargo clippy --workspace -- -D warnings`
- **Format:** `cargo fmt --all`
- **Database Migrations:** `cd server && diesel migration run` (Requires `diesel_cli`)

## High-Level Architecture

This project implements a distributed optimization system where a central server manages tasks and distributed workers process them.

### components
1. **Server (`server/`)**:
    -   Axum-based REST API that manages the task queue.
    -   Uses SQLite (via Diesel ORM) for persistence.
    -   Handles task distribution via a leasing mechanism to ensure exactly-once processing (or at-least-once with retries).
    -   Exposes endpoints: `POST /v1/tasks/claim` (workers get tasks) and `POST /v1/tasks/result` (workers submit results).

2. **Worker (`worker/`)**:
    -   Independent service that polls the server for pending tasks.
    -   Executes optimization logic using the `argmin` crate based on `TaskPayload`.
    -   Reports success/failure back to the server.
    -   Designed to be horizontally scalable; multiple workers can poll the same server.

3. **Proto (`proto/`)**:
    - Shared library containing the domain model and API contracts.
    - Defines `TaskStatus`, `TaskPayload`, `ClaimTaskRequest`, `SubmitTaskResultRequest`, etc.
    - Ensures type safety between the Server and Worker. **Changes to API types must be made here first.**
4. **Cli (`cli/`)**;
    - cli manage client for server.
    - support basic crud for task.
    - 