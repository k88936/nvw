CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    status TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE task_leases (
    lease_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    leased_at TIMESTAMP NOT NULL,
    lease_expires_at TIMESTAMP NOT NULL,
    attempt INTEGER NOT NULL,
    active BOOLEAN NOT NULL
);

CREATE TABLE task_results (
    lease_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    best_cost REAL NULL,
    best_param_json TEXT NULL,
    iters INTEGER NOT NULL,
    best_iters INTEGER NOT NULL,
    termination TEXT NOT NULL,
    error_message TEXT NULL,
    finished_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_leases_task_active ON task_leases(task_id, active);
CREATE INDEX idx_leases_expiry_active ON task_leases(lease_expires_at, active);
