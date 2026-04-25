//! Infrastructure related errors

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("{0}")]
    Cron(#[from] croner::errors::CronError),

    #[error("{0}")]
    Database(#[from] sqlx::Error),

    #[error("{0}")]
    Internal(String),
}

// impl std::fmt::Display for DbError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Internal(msg) => write!(f, "{}", msg),
//             Self::Database(e) => match e {
//                 sqlx::Error::Database(dberr) => {
//                     if dberr.is_unique_violation() {
//                         match dberr.constraint() {
//                             Some("jobs_job_name_key") => write!(f, "Job name already in use."),
//                             Some("pools_pool_name_key") => {
//                                 write!(f, "Pool name already in use.")
//                             }
//                             _ => write!(f, "Internal error: {}", dberr),
//                         }
//                     } else {
//                         write!(f, "Internal error: {}", dberr)
//                     }
//                 }
//                 _ => write!(f, "{}", e),
//             },
//             Self::DuplicateValue(msg) => write!(f, "Duplicate value. {}", msg),
//             Self::Cron(e) => write!(f, "Invalid cron expression. {}", e),
//             Self::Io(e) => write!(f, "{}", e),
//             Self::Uuid(e) => write!(f, "Invalid UUID. {}", e),
//             Self::Domain(e) => write!(f, "Internal Error, {}", e),
//         }
//     }
// }

// pub trait QueryResultExt<T> {
//     fn map_query_err(self) -> std::result::Result<T, Error>;
// }

// impl<T> QueryResultExt<T> for std::result::Result<T, sqlx::Error> {
//     fn map_query_err(self) -> std::result::Result<T, Error> {
//         self.map_err(|e| {
//             if let Some(db_err) = e.as_database_error() {
//                 if db_err.is_unique_violation() {
//                     match db_err.constraint() {
//                         Some("jobs_name_key") => {
//                             return Error::DuplicateValue("Job name already in use.".into());
//                         }
//                         Some("pools_name_key") => {
//                             return Error::DuplicateValue("Pool name already in use.".into());
//                         }
//                         _ => {
//                             tracing::error!("Constraint violation: {}", db_err.to_string());
//                         }
//                     }
//                     return Error::DuplicateValue("Duplicate value".into());
//                 }
//             }
//             Error::Database(e)
//         })
//     }
// }
