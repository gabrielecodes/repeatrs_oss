# `repeatrs-cli`

The `repeatrs-cli` crate provides the command-line interface for interacting with the Repeat.rs decentralized job scheduler. It serves as the user's primary control and presentation layer.

## Overview

The CLI allows users to manage jobs, queues, and query the overall system health of the Repeat.rs cluster. All commands are sent via gRPC to the `repeatrs-scheduler` service, ensuring a clean separation of concerns and adherence to the project's 3-layer architecture.

## Features

- **Job Management**: Create, read, update, and delete job definitions.
- **Worker Provisioning**: Add and remove workers.
- **Queue Management**: Define and manage job queues.
- **Manual Job Execution**: Trigger immediate runs for scheduled jobs.
- **System Monitoring**: Query the `repeatrs-scheduler` for current system status, job execution logs, and worker information.

## Architecture Integration

- **Presentation Layer**: The `repeatrs-cli` is the presentation layer of the Repeat.rs system.
- **gRPC Communication**: It communicates exclusively with the `repeatrs-scheduler` via gRPC. It has **no** direct access to the PostgreSQL database or NATS message queues.

## Usage (Example)

_(Note: Specific commands will be detailed as they are implemented.)_

To add a new job:

```bash
repeatrs-cli job add --name "my-job" --image "my-repo/my-job-image:latest" --schedule "@daily"
```

To list all active jobs:

```bash
repeatrs-cli job list
```

To check the status of a specific worker:

```bash
repeatrs-cli worker status <worker-id>
```
