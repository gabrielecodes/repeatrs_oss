# `repeatrs-scheduler`

This crate is the central nervous system of the Repeat.rs ecosystem. It serves as the authoritative service for job lifecycle management, state tracking, and scheduling. It is designed to be highly available and horizontally scalable.

## Role and Responsibilities

The `repeatrs-scheduler` has two primary responsibilities:

1.  **State Management (via gRPC):** It exposes a gRPC API that serves as the control plane for the entire system. The `repeatrs-cli` interacts with this API to perform CRUD operations on jobs, queues, and workers. This layer is responsible for enforcing all business rules and invariants.

2.  **Job Scheduling Loop:** It contains the core scheduling logic. It periodically queries the PostgreSQL database to identify jobs that are due to run based on their schedule (`next_occurrence_at`).

## Core Logic

- **Scheduling Loop**: The scheduler wakes up based on the time of the next scheduled job or when it is notified of a change (e.g., a new job is added via the gRPC API).
- **Job Dispatch**: When jobs are due, the scheduler fetches them from the `jobs` table, calculates their next run time, and atomically enqueues them (same transaction) into the `job_queues` table to be picked up by workers.
- **Publishing to NATS**: After enqueuing jobs, it publishes a message to the relevant NATS topic (representing the job's queue). This message serves as a "wake-up call" for any subscribed workers, prompting them to check the database for new work.

The scheduler is based on 5 invariants:
1. A job occurrence is enqueued exactly once. Jobs are fetched with a lock and updated with their next occurrence in a single transaction
2. After jobs are updated the NATS job queues are loaded. If publishing fails, the scheduler at least executes the next iteration of those jobs.
3. Schedulers is coordination-free. Locks rows (...`FOR UPDATE SKIP LOCKED`). Avoids dealing with leader elections.
4. Scheduler only deals with scheduling. It doesn't own job execution state. Scheduler doesn't know about Worker's capacity either.

## Architecture

```text
      ┌──────────────────────┐        ┌─────────────────────┐
      │  gRPC Requests       │        │   Scheduler Loop    │         Controller Layer
      │  (from repeatrs-cli) │        │   (Internal Timer)  │         (Source of queries)
      └────────┬─────────────┘        └─────────┬───────────┘
               │                                │
               V                                V
       ┌───────────────┐             ┌─────────────────────┐
       │  JobService   │             │  SchedulingService  │          Service Layer
       └───────┬───────┘             └──────────┬──────────┘
               │                                │
               ├──────────────┐       ┌─────────┘
               │              │       │
               V              V       V
       ┌───────────────┐  ┌──────────────────┐
       │ JobRepository │  │ JetStreamClient  │                        Repository Layer
       └───────┬───────┘  └──────────────────┘                        (Execution of queries)
               │
               V
       ┌───────────────┐
       │   Database    │
       └───────────────┘
```

## Connectivity

- **PostgreSQL (`repeatrs-db`)**: The scheduler has direct read/write access to the PostgreSQL database. It uses this as the single source of truth for the state of all jobs, queues, and workers.
- **NATS**: It is a NATS publisher. It publishes messages to job queue topics to signal that new work is available.
- **gRPC**: It is a gRPC server. It listens for commands from clients, primarily the `repeatrs-cli`, to manage the system's state.

## Architectural Principles

The scheduler is designed to be stateless where possible, relying on PostgreSQL for durable state. The state is recorded in the database and NATS, it's not owned by the scheduler. It uses database locks to allow multiple scheduler instances to run concurrently without race conditions, ensuring high availability.

The internal API follows a standard 3-layer architecture:

1.  **Controllers**: Handle incoming gRPC requests, validation, and serialization.
2.  **Services**: Implement the core business logic and invariants.
3.  **Repositories**: Provide an abstraction over data access, interacting directly with the `repeatrs-db` crate.

## Layer Organization

The api follows a service-centric approach and is organized in 3 layers.

2. Services layer: it's the uppermost layer. It owns the interface types between controllers and services.
1. Controllers layer: its input are types defined by the proto definition. These types are rigidly auto-generated therefore they are used only at the **controllers** layer. The controller translates the gRPC types into service types.
3. Repositories layer: imports the interface types from the services layer and translates it into types usable by the database.
4. Database models: types directly necessary for queries.


## TODO
- Multi-tenant queues
- Queue rate limiting
- Queue pause/resume
- Per-job retry policies
- Dead letter queues
- Job backfilling
- Event-triggered jobs (not just cron)