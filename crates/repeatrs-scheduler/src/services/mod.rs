//! Logic layer. Responsible for implementing business rules and orchestrating
//! interactions between the database and the gRPC layer. Services are called by
//! the gRPC controllers, and they call the repositories to interact with the database.
//! Services are also responsible for publishing messages to NATS subjects.

pub mod job;
pub mod queue;
pub mod scheduling;
pub mod worker;
