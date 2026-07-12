CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    job_type TEXT NOT NULL,
    created_at TEXT,
    started_at TEXT,
    completed_at TEXT,
    progress INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS analysis_results (
    job_id TEXT PRIMARY KEY,
    results TEXT NOT NULL,
    FOREIGN KEY (job_id) REFERENCES jobs(id)
);
