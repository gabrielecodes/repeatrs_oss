# Repeat.rs: The infrastructure for Agentic AI

## Overview

Repeat.rs is a Zero-Trust Orchestration Layer that bridges AI with durable, asynchronous, and auditable jobs. It’s the difference between an AI that talks about doing work and a system that guarantees the work is done, auditable, repeated, and recovered—no matter the scale

Repeat.rs is designed to be a high-performance, decentralized job scheduler built on a worker-pull architecture. It's built for massive concurrency, agile scaling, and efficient workload management. It provides the structured environment and reliable execution guarantees that workflows need to perform tasks effectively, safely, and at scale.

## Features

1. Use your favorite AI platform

- Problem: giving agents total control and access.
- Solution:; You control the tools/workflows code and permissions. Define your tools and load them in the Repeatrs MCP server. Connect your agent to the MCP server and ask AI to execute workflows. The agent is woken up at every step of your workflow to check on results.

2. From "Prompting" to "Dispatching"

- Problem: Prompting an AI to "update the database" is a security and reliability nightmare.
- Solution: The AI acts as a Dispatcher. It identifies the intent and submits a formal Job Request to Repeat.rs. The job runs in a signed, versioned Wasm module. Logic is fixed; only parameters are dynamic.

3. Built-in "Industrial" Durability

- Problem: If an agentic script crashes mid-way, the state is lost, and the process hangs.
- Solution: Repeat.rs uses a Worker-Pull model. If a worker fails, the job remains in the queue. Another worker picks it up. The AI doesn't need to manage retries; the infrastructure handles the "physics" of execution.

4. The "Distributed MCP" Advantage

- Problem: MCP (Model Context Protocol) is great for local tools but doesn't scale to a thousand concurrent enterprise users.
- Solution: Repeat.rs is essentially "MCP at Scale." It provides the standardized interface for the AI to understand the tools, but moves the execution to a high-performance distributed cluster.

| Traditional Agents                           | Repeat.rs Infrastructure                                 |
| -------------------------------------------- | -------------------------------------------------------- |
| High Entropy: AI writes code on the fly      | Zero Entropy: AI triggers audited **Execution Units**.   |
| Synchronous: User waits for the LLM + Script | Asynchronous: AI submits Job → ID → Notification.        |
| Fragile: A network blip kills the agent task | Resilient: Worker-Pull ensures jobs survive failures.    |
| Opaque: Hard to audit what happened inside   | Transparent: Every tool call is a logged, versioned Job. |

## Features

- **High-Concurrency**: Workers run jobs in parallel with a high cardinality.
- **Agile Scaling**: Scale worker VMs up or down with a single command, paying only for what you use.
- **Intelligent Workload Management**: Easily assign jobs with different hardware requirements to dedicated queues.
- **AI-Ready**: Provides a structured and reliable execution engine for AI agents to use predefined, secure "tools" (jobs).

## Core Concepts

The system is built around three main concepts:

- **Jobs**: The fundamental unit of work, defined as a container or a Wasm module.
- **Queues**: Jobs are organized into queues, which allows for managing resource usage and prioritization. Each queue can be served by a dedicated pool of workers.
- **Workers**: The virtual machines that execute jobs. Each worker consumes jobs from a single queue at a time, allowing for hardware to be specifically provisioned for the type of work in that queue.

There is a 1-to-N relationship between a job queue and its associated workers.

```text
                     ┌────────────────────┐
                     │      queue N       │
                     │    ┌───────────┐   │
                     │    │ job queue │   │
                     │    └───────────┘   │
                     └────────────────────┘
                                │
        ┌────────────┰──────────┴──────────┰─────────────┐
        │            │                     │             │
        V            V                     V             V
  ┌──────────┐  ┌──────────┐          ┌──────────┐  ┌──────────┐
  │ Worker 1 │  │ Worker 2 │          │ Worker 3 │  │ Worker 4 │
  └──────────┘  └──────────┘          └──────────┘  └──────────┘
```

## System Architecture

The application follows a clean 3-layer architecture and is organized into a multi-crate workspace to ensure separation of concerns.

### Project Organization

```text
repeatrs/
├── crates/
│   ├── repeatrs-proto/     <-- gRPC service and message definitions.
│   ├── repeatrs-db/        <-- Data access layer for PostgreSQL.
│   ├── repeatrs-scheduler/ <-- Core logic: schedules jobs and manages state via gRPC.
│   ├── repeatrs-worker/    <-- Executes jobs received from NATS.
│   ├── repeatrs-cli/       <-- User control plane for managing jobs and queues.
│   ├── repeatrs-auditor/   <-- Monitors system health and reconciles state.
```

### Architectural Layers

1.  **Data Layer (`repeatrs-db`)**: A thin wrapper around the database schema. It handles SQL type mapping and basic queries but contains no business logic.
2.  **Logic Layer (`repeatrs-scheduler`, `repeatrs-auditor`)**: Implements and enforces all business rules and invariants. This layer is responsible for the system's core functionality but remains decoupled from the underlying query details.
3.  **Presentation Layer (`repeatrs-cli`)**: The user-facing interface that sends commands to the logic layer (via gRPC) and presents results.

### Component Responsibilities & Communication Flow

#### CLI (`repeatrs-cli`)

- **Role**: User Interface & Command Center.
- **Connectivity**: Communicates exclusively with the Scheduler via gRPC. It has **no** direct database access.
- **Tasks**:
  - Performs CRUD operations on jobs and queues.
  - Triggers manual "Run Now" signals for jobs.
  - Queries the Scheduler for system health and logs.

#### Scheduler (`repeatrs-scheduler`)

- **Role**: The central authority for job lifecycle and state management.
- **Connectivity**: Has read/write access to PostgreSQL (via `repeatrs-db`) and publishes messages to NATS.
- **Tasks**:
  - **Scheduling Loop**: Periodically queries the database for due jobs (`SELECT MIN(next_occurrence_at)`), calculates all jobs that need to run, and populates the appropriate `job_queues` in NATS.
  - **State Management**: Manages the lifecycle of jobs, queues, and workers through its gRPC API, which is called by the CLI.
  - **Waking Workers**: Does not directly wake workers. Instead, it publishes jobs to NATS topics, and NATS pushes these messages to subscribed workers, effectively waking them up to perform work.

#### Worker (`repeatrs-worker`)

- **Role**: A lightweight, autonomous job executor.
- **Connectivity**: Subscribes to NATS to receive jobs and publishes status updates back to NATS. It has **no** direct database access.
- **Tasks**:
  - **Subscription**: On startup, subscribes to a specific NATS topic representing its job queue.
  - **Job Execution**: Receives job messages from NATS, executes the job (e.g., runs a container), and monitors local resource usage via `cgroups`.
  - **Reporting**: Upon job completion, publishes results, peak CPU/memory usage, and status updates to a NATS topic for the Auditor to process.
  - **Self-Termination**: If a worker remains idle for a configured period (e.g., 15 minutes), it initiates a self-shutdown via the cloud provider's API to eliminate costs.

#### Auditor (`repeatrs-auditor`)

- **Role**: The system's truth and health reconciler. It can be scaled independently.
- **Connectivity**: Subscribes to NATS topics and has read/write access to PostgreSQL (via `repeatrs-db`).
- **Tasks**:
  - **Job Status Persistence**: Consumes `job_run_status` updates from NATS and persists them to the `job_runs` table in the database.
  - **Worker Liveness**: Monitors worker heartbeat subjects in NATS. If a worker becomes stale (no heartbeat within a given window), it marks the worker as `STALE` in the database.
  - **Fencing & Requeuing**: If a worker is confirmed dead, the Auditor triggers fencing by calling the cloud provider's API to terminate the VM instance. Once termination is confirmed, it safely requeues any of the worker's `RUNNING` jobs.

#### Architectural Diagram

```text
    ┌──── CLI ─────┐
    │              │
    │     cli      │    ┌────── Worker ────────┐           ┌───────── NATS ──────────┐
    │      │       │    │  NATS service ──────────────┬───────────> job queues <────────┐
    │      │       │    │                      │      │    │                         │  │
    │      │       │    │  gRPC layer -> main  │      │    ├─────────────────────────┤  │
    │  gRPC layer  │    │         │            │      └─────> job run status updates │  │
    │     │        │    │         │            │           │      │                  │  │
    │     │        │    └─────────│────────────┘           └──────│──────────────────┘  │
    └─────│────────┘              │ subscribe                     │                     │
          ├───────────────────────┘ worker                        │                     │
  ┌───────│─── scheduler ──────┐                                  │                     │
  │       │       (db)         │                                  │                     │
  │       V                    │                                  │                     │
  │   gRPC layer   scheduler   │                                  │                     │
  │       │          │         │                                  │                     │
  │       │          V         │                                  │                     │
  │       └──────> services    |                                  │                     │
  │                   │        │                                  │                     │
  └───────────────────│────────┘                      auditor <───┘                     │
           fetch jobs │                            db bulk updates                      │
                      │                          of job run statuses                    │
                      │          ┌───────────┐           │                              │
   update             │          │           │           │                              │
   next_occurrence_at │          │           │           │                              │
                      │          │           │           │                              │
                      ├────────> │  database │ <─────────┘                              │
                      │          └───────────┘                                          │
                      │                                                    load queues  │
                      └─────────────────────────────────────────────────────────────────┘
```

---

## Deep Dive: The State Machine (PostgreSQL)

PostgreSQL serves as the central source of truth for the system's state.

- **Primary Tables**:
  - **`jobs`**: The authoritative definition of jobs, including their name, image, schedule, and assigned queue.
  - **`queues`**: Defines the available job queues.
  - **`job_queues`**: A high-velocity, "thin" table containing only pending work. The Scheduler writes to this, workers consume from it.
  - **`job_runs`**: A historical and active-state table for tracking every job execution.
- **Concurrency Control**: The scheduler uses `SELECT ... FOR UPDATE SKIP LOCKED` when querying for due jobs. This allows multiple scheduler instances to query the same table without blocking each other, ensuring high availability.
- **Atomic Handoff**: The scheduler uses a CTE to DELETE jobs from `job_queues` and INSERT them into `job_runs` within a single transaction. This guarantees that a job is never "lost" between being queued and being marked for execution.

## Deep Dive: Event-Driven Communication (NATS)

NATS serves as the high-throughput, low-latency communication bus for the system.

- **Job Queues**: NATS _topics_ are used to represent job queues. The Scheduler publishes jobs to these topics, and workers subscribe to them to receive work.
- **Worker Heartbeats**: Each worker publishes a heartbeat message to a unique _subject_ (e.g., `workers.heartbeat.<worker_id>`). This stream is configured with a `Max Messages Per Subject` of 1, ensuring only the latest heartbeat is stored. The Auditor subscribes to this stream to monitor liveness.
- **Job Status Updates**: Workers publish job run status changes to a NATS topic, which the Auditor consumes to update the database.

---

## Architectural Advantages

### Design Principles

1.  **Relational Source of Truth**: Using PostgreSQL as the "Source of Truth" ensures that the CLI, Scheduler, and Auditor all operate on a consistent view of the system's state.
2.  **Worker Autonomy**: Workers are autonomous. They decide if they can handle a job based on local resource metrics (`cgroups`). This is far more scalable than a central scheduler trying to track the RAM of thousands of VMs in real-time.
3.  **Lean Binaries**: The multi-crate organization prevents the Worker binary from being bloated with CLI dependencies or the Scheduler's complex cron-parsing logic, keeping it lightweight and focused.

### Comparisons Overview

| Feature           | repeatrs (Projected)                 | Airflow                         | Kubernetes CronJob                     | Temporal                         |
| ----------------- | ------------------------------------ | ------------------------------- | -------------------------------------- | -------------------------------- |
| Core Paradigm     | High-throughput Job Scheduler        | DAG-based Workflow Orchestrator | Simple Cron Scheduler                  | Durable Workflow Execution       |
| Primary Use Case  | Massive, independent, recurring jobs | Complex data pipelines, ETL     | Basic scheduled tasks in a K8s env     | Long-running, stateful processes |
| Dependency Mgmt   | Not native (would require API calls) | Core Strength (DAGs)            | None (must be handled externally)      | Core Strength (sub-workflows)    |
| Scalability Model | Worker-pull, VM-level scaling        | Worker-push (via Executor)      | Pod-level scaling on fixed nodes       | Worker-pull, scales horizontally |
| Cost Efficiency   | Excellent (self-terminating VMs)     | Moderate (persistent workers)   | Good (shared cluster, nodes always up) | Good (scales to zero workers)    |
| Configuration     | API/SDK-driven                       | Python code (workflows-as-code) | YAML manifests                         | Language-native SDKs             |

### Comparison with Airflow

The worker-pull architecture and VM-level auto-scaling are designed for extreme cost-efficiency and performance in environments with spiky, high-volume workloads, a domain where Airflow can be complex and expensive to scale.

### Comparison to Kubernetes

| Challenge           | Kubernetes Approach             | Repeat.rs Approach                                       |
| ------------------- | ------------------------------- | -------------------------------------------------------- |
| **Resource Limits** | Manual "Guesses" in YAML        | **Auto-learning** via worker metrics reported to the DB. |
| **Idle Costs**      | 24/7 Nodes / Cluster Autoscaler | **Immediate Self-Termination** at the VM level.          |
| **Complexity**      | Thousands of lines of YAML      | **Simple SDK/API**; logic is in the code, not config.    |
| **State**           | Etcd (sensitive to scale)       | **Postgres** (scalable via partitioning hot tables).     |

## Core Technologies & Dependencies

- **PostgreSQL:** Serves as the central state machine and source of truth.
- **gRPC:** Used for synchronous, strongly-typed control plane communication (CLI to Scheduler).
- **NATS:** Used for asynchronous, high-throughput data plane communication (job distribution, heartbeats, status updates).
- **Vector:** A high-performance observability data pipeline used to ship logs (stdout/stderr) and metrics from workers.
- **Loki:** A log aggregation system that receives data from Vector, enabling deep debugging without cluttering the operational database.
