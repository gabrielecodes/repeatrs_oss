use crate::error::{ContainerOptionsError, ValidationError};
use crate::{IsId, QueueId, SanitizedString};

use chrono::{DateTime, Duration, Utc};
use croner::Cron;
use repeatrs_proto::repeatrs::JobItem;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;

use std::{fmt::Display, ops::Deref, str::FromStr};
use uuid::Uuid;

const MAX_RETRIES: i32 = 10;
const DEFAULT_TIMEOUT_SECONDS: i64 = 7200;
const FORBIDDEN_CONTAINER_OPTIONS: [&str; 4] =
    ["--privileged", "--net=host", "cap-add=ALL", "-v /:"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobId(Uuid);
impl IsId for JobId {}

impl Deref for JobId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for JobId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<JobId> for Uuid {
    fn from(job_id: JobId) -> Self {
        job_id.0
    }
}

impl FromStr for JobId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(JobId(Uuid::from_str(s)?))
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl JobId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn inner(self) -> Uuid {
        self.0
    }
}

/// Describes the status of the job.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize)]
pub enum JobStatus {
    /// Job can be scheduled for execution
    #[default]
    Active,

    /// Job is not going to be scheduled for execution    
    Inactive,
}

impl Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Active => "ACTIVE",
            Self::Inactive => "INACTIVE",
        };
        f.write_str(s)
    }
}

impl FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ACTIVE" => Ok(Self::Active),
            "INACTIVE" => Ok(Self::Inactive),
            _ => Err("Job status not understood.".into()),
        }
    }
}

impl From<JobStatus> for String {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Active => "ACTIVE".to_string(),
            JobStatus::Inactive => "INACTIVE".to_string(),
        }
    }
}

pub trait ToOwnedString {
    fn as_map(&self) -> &HashMap<String, String>;

    fn to_owned_string(&self) -> String {
        let mut buf = String::new();
        let mut first = true;

        for (k, v) in self.as_map() {
            if !first {
                buf.push(' ');
            }
            first = false;

            write!(&mut buf, "{}={}", k, v).unwrap();
        }

        buf
    }
}

fn try_string_to_hashmap(value: String) -> Result<HashMap<String, String>, ValidationError> {
    let mut map = std::collections::HashMap::new();
    let mut iter = value.split(" ").peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with('-') {
            if arg.contains('=') {
                let (key, val) = arg.split_once('=').ok_or({
                    let err = ContainerOptionsError::MalformedArgument(arg.to_string());
                    ValidationError::InvalidContainerOption(err)
                })?;

                if val.is_empty() {
                    let err = ContainerOptionsError::MissingValue(key.into());
                    return Err(ValidationError::InvalidContainerOption(err));
                }

                map.insert(key.to_string(), val.to_string());
            } else {
                let key = arg;
                let has_value = iter.peek().is_some_and(|next| !next.starts_with('-'));

                if has_value {
                    map.insert(key.to_string(), iter.next().unwrap().to_string());
                } else {
                    map.insert(key.to_string(), "true".to_string());
                }
            }
        } else {
            let err = ContainerOptionsError::UnexpectedPositional(arg.into());
            return Err(ValidationError::InvalidContainerOption(err));
        }
    }

    Ok(map)
}

#[derive(Debug, Clone)]
pub struct ImageName(String);

impl ImageName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Returns the image name as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ImageName {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() || !value.contains('/') && !value.contains(':') {
            return Err(ValidationError::InvalidImageName(value));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[non_exhaustive]
pub struct ContainerOptions(HashMap<String, String>);

impl ContainerOptions {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self(map)
    }
}

impl ToOwnedString for ContainerOptions {
    fn as_map(&self) -> &HashMap<String, String> {
        &self.0
    }
}

impl TryFrom<String> for ContainerOptions {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let map = try_string_to_hashmap(value)?;

        Ok(ContainerOptions::new(map))
    }
}

/// Represents the command given to the entrypoint of the container.
/// If `None`, it is assumed that the image already defines a `CMD`
#[derive(Debug, Clone, PartialEq)]
pub struct RunCmd(Option<String>);

impl RunCmd {
    pub fn new(command: Option<String>) -> Self {
        Self(command)
    }

    /// Returns the run command. The command can be `None` in which
    /// case it is assumed that the image already defines a `CMD`
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CommandArgs(HashMap<String, String>);

impl CommandArgs {
    pub fn new(args: HashMap<String, String>) -> Self {
        Self(args)
    }
}

impl ToOwnedString for CommandArgs {
    fn as_map(&self) -> &HashMap<String, String> {
        &self.0
    }
}

impl TryFrom<String> for CommandArgs {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let map = try_string_to_hashmap(value)?;

        Ok(CommandArgs::new(map))
    }
}

/// Informations related to the identity of the job
#[derive(Debug, Clone)]
pub struct JobIdentity {
    /// Unique job name
    job_name: SanitizedString,

    /// The job description
    description: Option<String>,

    /// Identifier of the queue this job belongs to
    queue_name: SanitizedString,
}

impl JobIdentity {
    pub fn new(
        job_name: SanitizedString,
        description: Option<String>,
        queue_name: SanitizedString,
    ) -> Self {
        Self {
            job_name,
            description,
            queue_name,
        }
    }
}

/// Options regarding the job execution
#[derive(Debug, Clone)]
pub struct JobOptions {
    /// schedule of the cronjob or execution time
    schedule: Cron,

    /// Retry the job if last execution failed
    max_retries: i32,

    /// Job priority
    priority: i32,

    /// Maximum number of identical jobs running concurrently
    max_concurrency: i32,

    /// A hard limit on the duration of the job, after which the job is terminated.
    timeout_seconds: Duration,
}

impl JobOptions {
    pub fn new(
        schedule: Cron,
        max_retries: Option<i32>,
        priority: Option<i32>,
        max_concurrency: Option<i32>,
        timeout_seconds: Option<i64>,
    ) -> Self {
        let max_retries = match max_retries {
            Some(val) => val.clamp(0, MAX_RETRIES),
            None => 0,
        };

        let priority = match priority {
            Some(val) => val.clamp(1, i32::MAX),
            None => 1,
        };

        let duration = match timeout_seconds {
            Some(secs) => Duration::seconds(secs),
            None => Duration::seconds(DEFAULT_TIMEOUT_SECONDS),
        };

        Self {
            schedule,
            max_retries,
            priority,
            max_concurrency: max_concurrency.unwrap_or(0),
            timeout_seconds: duration,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContainerRunCommand {
    /// Options for running the container
    run_options: ContainerOptions,

    /// Image name as <optional_registry>/<image_name>:tag
    image_name: ImageName,

    /// The command to run the container
    cmd: RunCmd,

    /// Arguments for the command
    command_args: CommandArgs,
}

impl ContainerRunCommand {
    pub fn new(
        run_options: ContainerOptions,
        image_name: ImageName,
        cmd: RunCmd,
        command_args: CommandArgs,
    ) -> Self {
        // TODO: add validation
        Self {
            run_options,
            image_name,
            cmd,
            command_args,
        }
    }
}

/// Represents a job. This type has validation logic and methods to
/// enforce invariants before persisting the information in the database.
#[derive(Debug)]
pub struct Job {
    /// Informations related to the identity of the job
    identity: JobIdentity,

    /// Informations needed to reconstruct the run command
    run_command: ContainerRunCommand,

    /// Informations related to how the job is run
    options: JobOptions,
}

impl Job {
    /// Applies a set of validation ruels and instantiates a new [`Job`].
    pub fn new(
        identity: JobIdentity,
        run_command: ContainerRunCommand,
        options: JobOptions,
    ) -> Self {
        // NOTE: use "new" instead of TryFrom: keep definition and enforcement
        // of invariants in the domain layer and avoid coupling the service
        // layer to the domain layer

        Self {
            identity,
            run_command,
            options,
        }
    }

    pub fn identity(&self) -> &JobIdentity {
        &self.identity
    }

    pub fn run_command(&self) -> &ContainerRunCommand {
        &self.run_command
    }

    pub fn run_options(&self) -> &ContainerOptions {
        &self.run_command.run_options
    }

    pub fn image_name(&self) -> &ImageName {
        &self.run_command.image_name
    }

    pub fn command(&self) -> &RunCmd {
        &self.run_command.cmd
    }

    pub fn job_name(&self) -> &SanitizedString {
        &self.identity.job_name
    }

    pub fn description(&self) -> Option<&str> {
        self.identity.description.as_deref()
    }

    pub fn schedule(&self) -> &Cron {
        &self.options.schedule
    }

    pub fn command_args(&self) -> &CommandArgs {
        &self.run_command.command_args
    }

    pub fn options(&self) -> &JobOptions {
        &self.options
    }

    pub fn max_retries(&self) -> i32 {
        self.options.max_retries
    }

    pub fn priority(&self) -> i32 {
        self.options.priority
    }

    pub fn queue_name(&self) -> &SanitizedString {
        &self.identity.queue_name
    }

    pub fn max_concurrency(&self) -> i32 {
        self.options.max_concurrency
    }

    pub fn timeout_seconds(&self) -> Duration {
        self.options.timeout_seconds
    }
}

/// Represents a job as retrieved from the database.
#[derive(Debug)]
pub struct JobResponse {
    /// Unique ID of the job, primary key
    job_id: JobId,

    /// Unique job name
    job_name: String,

    /// The job description
    description: Option<String>,

    /// Identifier of the queue this job belongs to
    queue_id: QueueId,

    /// Name of the queue this job belongs to
    queue_name: String,

    /// Status of the job
    status: JobStatus,

    /// schedule of the cronjob or execution time
    schedule: Cron,

    /// Retry the job if last execution failed
    max_retries: i32,

    /// Job priority
    priority: i32,

    /// Maximum number of identical jobs running concurrently
    max_concurrency: i32,

    /// A hard limit on the duration of the job, after which the job is terminated.
    timeout_seconds: Option<Duration>,

    /// Options for running the container
    image_options: Option<String>,

    /// Image name as <optional_registry>/<image_name>:tag
    image_name: String,

    /// The command to run the container
    command: Option<String>,

    /// Arguments for the command
    command_args: Option<String>,

    /// Job creation timestamp
    created_at: DateTime<Utc>,

    /// Time this job was last updated
    updated_at: DateTime<Utc>,
}

impl JobResponse {
    // NOTE: we want to keep the fields of Job private and avoid
    // defining another DTO with public fields. With Builders
    // it's possible to partially instantiated objects. Job and
    // JobRow are internal types not exposed publicly so the
    // inconvenience of having too many fields does not leak.
    /// Builds a [`Job`] from database informations
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        job_id: JobId,
        job_name: String,
        description: Option<String>,
        schedule: Cron,
        image_options: Option<String>,
        image_name: String,
        command: Option<String>,
        command_args: Option<String>,
        max_retries: i32,
        status: JobStatus,
        priority: i32,
        queue_id: QueueId,
        queue_name: String,
        max_concurrency: i32,
        timeout_seconds: Option<Duration>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id,
            job_name,
            description,
            schedule,
            image_options,
            image_name,
            command,
            command_args,
            max_retries,
            status,
            priority,
            queue_id,
            queue_name,
            max_concurrency,
            timeout_seconds,
            created_at,
            updated_at,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn job_name(&self) -> &str {
        &self.job_name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn schedule(&self) -> &Cron {
        &self.schedule
    }

    pub fn image_options(&self) -> Option<&str> {
        self.image_options.as_deref()
    }

    pub fn image_name(&self) -> &str {
        &self.image_name
    }

    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    pub fn command_args(&self) -> Option<&str> {
        self.command_args.as_deref()
    }

    pub fn max_retries(&self) -> i32 {
        self.max_retries
    }

    pub fn status(&self) -> &JobStatus {
        &self.status
    }

    pub fn priority(&self) -> i32 {
        self.priority
    }

    pub fn queue_id(&self) -> &QueueId {
        &self.queue_id
    }

    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    pub fn max_concurrency(&self) -> i32 {
        self.max_concurrency
    }

    pub fn timeout_seconds(&self) -> Option<Duration> {
        self.timeout_seconds
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn to_job_item(&self, queue_name: &str) -> JobItem {
        JobItem {
            job_id: self.job_id.inner().into(),
            job_name: self.job_name.to_string(),
            status: self.status.to_string(),
            schedule: self.schedule.to_string(),
            image_name: self.image_name.to_string(),
            queue_name: queue_name.to_string(),
            max_retries: self.max_retries,
            priority: self.priority(),
        }
    }

    /// Returns the next occurrence of the job.
    pub fn calculate_next_occurrence(
        &self,
        inclusive_now: bool,
    ) -> Result<DateTime<Utc>, croner::errors::CronError> {
        let deadline = self
            .schedule
            .find_next_occurrence(&Utc::now(), inclusive_now)?;

        Ok(deadline)
    }
}

pub trait JobOperations<E>: Sync + Send + 'static {
    type Err: std::error::Error;

    fn add_job(
        &self,
        tx: &mut E,
        job: &Job,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    /// Returns true if the job with the given id already exists.
    fn get_job_by_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<JobResponse, Self::Err>> + Send;

    /// Returns true if the job with the given name already exists.
    fn get_job_by_name(
        &self,
        tx: &mut E,
        job_name: &str,
    ) -> impl std::future::Future<Output = Result<JobResponse, Self::Err>> + Send;

    fn get_jobs_by_queue_id(
        &self,
        tx: &mut E,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<Vec<JobResponse>, Self::Err>> + Send;

    fn get_jobs_by_queue_name(
        &self,
        tx: &mut E,
        queue_name: &str,
    ) -> impl std::future::Future<Output = Result<Vec<JobResponse>, Self::Err>> + Send;

    fn deactivate_job_by_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    fn deactivate_job_by_name(
        &self,
        tx: &mut E,
        job_name: &str,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    fn delete_job_by_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    fn delete_job_by_name(
        &self,
        tx: &mut E,
        job_name: &str,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    // fn get_earliest_deadline(
    //     &self,
    //     tx: &mut E,
    // ) -> impl std::future::Future<Output = Result<Instant, Self::Err>> + Send;

    fn get_due_jobs(
        &self,
        tx: &mut E,
    ) -> impl std::future::Future<Output = Result<Vec<JobResponse>, Self::Err>> + Send;

    // fn update_deadlines(
    //     &self,
    //     tx: &mut E,
    //     jobs: &[Job],
    // ) -> impl std::future::Future<Output = Result<(), Self::Err>> + Send;
}
