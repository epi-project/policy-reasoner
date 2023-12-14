use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;

use audit_logger::{AuditLogger, Error as AuditLoggerError};
use auth_resolver::AuthContext;
use deliberation::spec::Verdict;
use policy::Policy;
use state_resolver::State;
use tokio::fs::File;
use workflow::Workflow;


pub struct MockLogger {}
impl MockLogger {
    pub fn new() -> Self { MockLogger {} }
}
impl Clone for MockLogger {
    fn clone(&self) -> Self { Self {} }
}
#[async_trait::async_trait]
impl AuditLogger for MockLogger {
    async fn log_exec_task_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        task: &str,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_data_access_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        data: &str,
        task: &Option<String>,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_validate_workflow_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_reasoner_response(&self, reference: &str, response: &str) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_verdict(&self, reference: &str, verdict: &Verdict) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_add_policy_request(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_set_active_version_policy(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> { todo!() }
}



/// Defines errors originating from the [`FileLogger`].
#[derive(Debug)]
pub enum FileLoggerError {
    /// Failed to create a new logfile.
    FileCreate { path: PathBuf, err: std::io::Error },
    /// Failed to open an existing logfile.
    FileOpen { path: PathBuf, err: std::io::Error },
}
impl Display for FileLoggerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use FileLoggerError::*;
        match self {
            FileCreate { path, .. } => write!(f, "Failed to create new log file '{}'", path.display()),
            FileOpen { path, .. } => write!(f, "Failed to open existing log file '{}'", path.display()),
        }
    }
}
impl Error for FileLoggerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use FileLoggerError::*;
        match self {
            FileCreate { err, .. } => Some(err),
            FileOpen { err, .. } => Some(err),
        }
    }
}



/// A more serious version of a logger that logs to a file.
///
/// Note that this logger is not exactly the perfect audit log, as it does nothing w.r.t. ensuring that the file is the same as last time or signing changes or w/e.
pub struct FileLogger {
    /// The path of the file to log to (used for debugging purposes).
    path:   PathBuf,
    /// A handle to the file to which we actually log.
    handle: File,
}
impl FileLogger {
    /// Constructor for the FileLogger that initializes it pointing to the given file.
    ///
    /// # Arguments
    /// - `path`: The path to the file to log to.
    /// - `overwrite`: If true, always creates a new file instead of opening an existing one.
    ///
    /// # Returns
    /// A new instance of self, ready for action.
    ///
    /// # Errors
    /// This function may error if the target file could not be created/opened to.
    pub async fn new(path: impl Into<PathBuf>, overwrite: bool) -> Result<Self, FileLoggerError> {
        let path: PathBuf = path.into();

        // See if we create or open the file
        let handle: File = if overwrite || !path.exists() {
            match File::create(&path).await {
                Ok(handle) => handle,
                Err(err) => return Err(FileLoggerError::FileCreate { path, err }),
            }
        } else {
            match File::open(&path).await {
                Ok(handle) => handle,
                Err(err) => return Err(FileLoggerError::FileOpen { path, err }),
            }
        };

        // OK, create ourselves with that
        Ok(Self { path, handle })
    }
}
#[async_trait::async_trait]
impl AuditLogger for FileLogger {
    async fn log_exec_task_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        task: &str,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_data_access_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        data: &str,
        task: &Option<String>,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_validate_workflow_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_reasoner_response(&self, reference: &str, response: &str) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_verdict(&self, reference: &str, verdict: &Verdict) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_add_policy_request(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_set_active_version_policy(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> { todo!() }
}
