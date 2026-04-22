//! 1. Interface Layer (gRPC / Controller)
//!
//! Responsibility: Structural & Type Integrity
//! This layer validates that the incoming request is "well-formed."
//! It ensures that the data can exist as an object at all.
//!
//! - Syntax & Parsing: Can this string be parsed into a Cron expression? Is this i32 actually a valid JobPriority enum variant?
//! - Presence: Are required fields present?
//! - Format: Does the image_name follow the regex registry/image:tag?
//! - Range: Is max_retries a positive number?

pub mod job;
pub mod queue;
