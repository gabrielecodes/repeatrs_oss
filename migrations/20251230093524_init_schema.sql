SET timezone = 'UTC';

CREATE OR REPLACE FUNCTION set_updated_at()
    RETURNS trigger AS
$$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION trigger_updated_at(tablename regclass)
    RETURNS void AS
$$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at
        BEFORE UPDATE
        ON %s
        FOR EACH ROW
        WHEN (OLD is distinct from NEW)
    EXECUTE FUNCTION set_updated_at();', tablename);
END;
$$ LANGUAGE plpgsql;

------------------------------------------------
-- QUEUES
------------------------------------------------
-- maps what worker is allowed to get what jobs
-- limit concurrency per domain
-- isolate workloads
-- prioritize traffic
-- assign workers logically

CREATE TYPE queue_status AS ENUM ('INACTIVE', 'ACTIVE');

CREATE TABLE
    IF NOT EXISTS queues (
        queue_id      UUID PRIMARY KEY DEFAULT uuidv7(),
        queue_name    TEXT UNIQUE NOT NULL,  -- NATS subject
        status        queue_status NOT NULL DEFAULT 'ACTIVE',
        capacity      INTEGER NOT NULL DEFAULT 1000,
        used_capacity INTEGER NOT NULL DEFAULT 0,
        created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

SELECT trigger_updated_at('queues');
INSERT INTO queues (queue_name) VALUES ('default');

------------------------------------------------
-- JOBS
------------------------------------------------
-- defines jobs

CREATE TYPE job_status AS ENUM ('INACTIVE', 'ACTIVE');

CREATE TABLE
    IF NOT EXISTS jobs (
        job_id                   UUID PRIMARY KEY DEFAULT uuidv7(),
        job_name                 TEXT NOT NULL UNIQUE,
        description              TEXT,
        schedule                 TEXT NOT NULL,
        options                  TEXT,
        image_name               TEXT NOT NULL,
        command                  TEXT,
        args                     TEXT,
        max_retries              INTEGER NOT NULL DEFAULT 0,
        status                   job_status NOT NULL DEFAULT 'ACTIVE',
        priority                 INTEGER NOT NULL DEFAULT 1,        
        queue_id                 UUID NOT NULL REFERENCES queues (queue_id),
        max_concurrency          INTEGER NOT NULL,
        timeout_seconds          INTEGER NOT NULL,
        created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

SELECT trigger_updated_at('jobs');
CREATE INDEX idx_job_scheduling ON jobs (status, priority DESC)
WHERE status = 'ACTIVE';

------------------------------------------------
-- JOB RUNS
------------------------------------------------
-- job_runs = queue + history + execution tracking 

CREATE TYPE job_run_status AS ENUM ('QUEUED', 'RUNNING', 'STOPPED', 'FAILED', 'COMPLETED');

CREATE TABLE
    IF NOT EXISTS job_runs (
        job_run_id           UUID PRIMARY KEY DEFAULT uuidv7(),
        job_id               UUID NOT NULL REFERENCES jobs (job_id),
        queue_id             UUID NOT NULL REFERENCES queues (queue_id),
        worker_id            UUID,
        claimed_at           TIMESTAMPTZ,
        status               job_run_status NOT NULL DEFAULT 'QUEUED',
        scheduled_time       TIMESTAMPTZ NOT NULL,
        attempt_count        INTEGER NOT NULL DEFAULT 1
        started_at           TIMESTAMPTZ,
        ended_at             TIMESTAMPTZ,
        duration_secs        INTEGER,
        exit_code            INTEGER,
        error                TEXT,
        created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

SELECT trigger_updated_at('job_runs');

-- workers always filter by queue
-- status = QUEUED is hot path
-- ordering matters for fairness
CREATE INDEX idx_job_runs_queue_pick
ON job_runs (queue_id, status, created_at);

CREATE UNIQUE INDEX job_runs_dedupe
ON job_runs(job_id, scheduled_time);

------------------------------------------------
-- JOB SCHEDULE STATE
------------------------------------------------
-- helper table to store the execution times. These 
-- are not part of Job since it is a static configuration

CREATE TABLE
    IF NOT EXISTS job_schedule_state (
        job_id               UUID PRIMARY KEY REFERENCES jobs(job_id),
        next_run_at          TIMESTAMPTZ,
        last_scheduled_at    TIMESTAMPTZ
    )

------------------------------------------------
-- WORKERS
------------------------------------------------

CREATE TYPE worker_status AS ENUM ('STARTING', 'ALIVE', 'TERMINATED', 'TERMINATING', 'STALE');

CREATE TABLE
    IF NOT EXISTS workers (
        worker_id    UUID PRIMARY KEY DEFAULT uuidv7(),
        queue_id     UUID NOT NULL REFERENCES queues (queue_id),
        status       worker_status NOT NULL DEFAULT 'STARTING',
        epoch        INTEGER NOT NULL DEFAULT 0,
        vpc_id       TEXT NOT NULL,
        dns          TEXT NOT NULL,
        private_ip   TEXT NOT NULL,
        created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );


-- workers job claim query
-- BEGIN
-- SELECT *
-- FROM job_runs
-- WHERE status = 'QUEUED'
--   AND queue_id = $1
-- FOR UPDATE SKIP LOCKED
-- LIMIT 1;

-- UPDATE job_runs
-- SET status = 'RUNNING',
--     claimed_by_worker_id = $worker_id,
--     claimed_at = now()
-- END

-- on completion:
    -- SUCCESS
-- UPDATE job_runs
-- SET status = 'COMPLETED',
--     ended_at = now(),
--     duration_secs = EXTRACT(EPOCH FROM now() - started_at)
-- WHERE job_run_id = $1;
     -- FAILURE
-- UPDATE job_runs
-- SET status = 'FAILED',
--     ended_at = now(),
--     duration_secs = EXTRACT(EPOCH FROM now() - started_at)
--     error = $error
-- WHERE job_run_id = $1;    

-- REAPER
-- UPDATE job_runs
-- SET status = 'QUEUED',
--     claimed_by_worker_id = NULL,
--     claimed_at = NULL
-- WHERE status = 'RUNNING'
--   AND claimed_at < now() - interval '10 minutes';