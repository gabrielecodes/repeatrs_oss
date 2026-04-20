# `repeatrs-proto`

This crate defines the official gRPC API contract for the Repeat.rs system using Protocol Buffers. It is the single source of truth for all inter-service communication.

## Overview

`repeatrs-proto` contains the `.proto` files that define all services and messages for the system. A `build.rs` script uses `tonic-build` to generate the necessary Rust code for both server implementations and client stubs.

This crate doesn't contain any logic itself; it is purely for API definition and code generation.

## Architectural Role: The API Contract

This crate is the critical link between the presentation layer (the CLI) and the logic layer (the Scheduler). It formalizes their interaction.

### Server Implementation (`repeatrs-scheduler`)

The `repeatrs-scheduler` crate depends on `repeatrs-proto` to generate the server-side service traits (e.g., `trait RepeatrsService`). The scheduler is responsible for **implementing** these traits, providing the actual business logic that powers the API.

### Client Implementation (`repeatrs-cli`)

The `repeatrs-cli` crate uses the generated client stubs (e.g., `struct RepeatrsServiceClient`). It acts as a **client**, making gRPC calls to the services defined in this contract. It does not need to know any implementation details of the scheduler; it only needs to adhere to the API defined here.

## Development Workflow

When making a change to the gRPC API:

1.  Modify the service or message definitions in the `proto/repeatrs.proto` file.
2.  Recompile the `repeatrs-proto` crate. The `build.rs` script will automatically regenerate the corresponding Rust code in `target/`.
3.  Implement the necessary logic for the changes in the `repeatrs-scheduler` (server).
4.  Update the `repeatrs-cli` (client) to use the new or modified services.

---

## Service Definitions

### GrpcCliService

This service is implemented by the **Scheduler** (server) and consumed by the **CLI** (client). It handles management commands from the user.

- **`AddJob`**
  - **Request (`AddJobRequest`)**: Contains all the necessary details to create a new job, including name, schedule, container image, and queue.
  - **Response (`JobServiceResponse`)**: Returns the unique ID of the newly created job and a success status.

- **`DeactivateJob`**
  - **Request (`JobIdentifierRequest`)**: Specifies the job to deactivate, either by its unique ID or its name.
  - **Response (`JobServiceResponse`)**: Returns the ID of the affected job and a success status.

- **`DeleteJob`**
  - **Request (`JobIdentifierRequest`)**: Specifies the job to permanently delete, either by its unique ID or its name.
  - **Response (`JobServiceResponse`)**: Returns the ID of the deleted job and a success status.

### GrpcWorkerService

This service is implemented by the **Scheduler** (server) and consumed by the **Worker** (client). It handles the worker lifecycle and communication.

- **`SubscribeWorker`**
  - **Request (`WorkerReadyMessage`)**: Sent by a worker on startup to signal that it is ready to receive work. Authentication tokens should be sent in the gRPC metadata.
  - **Response (`SchedulerSignal`)**: The scheduler's acknowledgment, which may contain initial configuration or signals for the worker.
