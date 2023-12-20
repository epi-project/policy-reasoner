use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FResult};
use std::path::PathBuf;

use audit_logger::{AuditLogger, Error as AuditLoggerError, LogStatement, ReasonerConnectorAuditLogger};
use auth_resolver::AuthContext;
use deliberation::spec::Verdict;
use enum_debug::EnumDebug;
use error_trace::ErrorTrace as _;
use log::debug;
use policy::Policy;
use serde::Serialize;
use state_resolver::State;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt as _, AsyncWriteExt};
use workflow::Workflow;


/***** HELPER MACROS *****/
/// Wraps a [`write!`]-macro to return its error as a [`FileLoggerError`].
macro_rules! write_file {
    ($path:expr, $handle:expr, $($t:tt)+) => {
        // Psych we actually don't wrap that macro, since we're doing async ofc
        async {
            use tokio::io::AsyncWriteExt as _;
            let contents: String = format!($($t)+);
            $handle.write_all(contents.as_bytes()).await.map_err(|err| FileLoggerError::FileWrite { path: ($path), err })
        }
    };
}

/// Wraps a [`writeln!`]-macro to return its error as a [`FileLoggerError`].
macro_rules! writeln_file {
    ($path:expr, $handle:expr) => {
        // Psych we actually don't wrap that macro, since we're doing async ofc
        async {
            use tokio::io::AsyncWriteExt as _;
            $handle.write_all(b"\n").await.map_err(|err| FileLoggerError::FileWrite { path: ($path), err })
        }
    };
    ($path:expr, $handle:expr, $($t:tt)+) => {
        // Psych we actually don't wrap that macro, since we're doing async ofc
        async {
            use tokio::io::AsyncWriteExt as _;
            let mut contents: String = format!($($t)*);
            contents.push('\n');
            $handle.write_all(contents.as_bytes()).await.map_err(|err| FileLoggerError::FileWrite { path: ($path), err })
        }
    };
}





/***** ERRORS *****/
/// Defines errors originating from the [`FileLogger`].
#[derive(Debug)]
pub enum FileLoggerError {
    /// Failed to create a new logfile.
    FileCreate { path: PathBuf, err: std::io::Error },
    /// Failed to open an existing logfile.
    FileOpen { path: PathBuf, err: std::io::Error },
    /// Failed to seek in the logfile.
    FileSeek { path: PathBuf, err: std::io::Error },
    /// Failed to flush the given logfile.
    FileShutdown { path: PathBuf, err: std::io::Error },
    /// Failed to write to the logfile.
    FileWrite { path: PathBuf, err: std::io::Error },
    /// Failed to serialize a statement.
    StatementSerialize { kind: String, err: serde_json::Error },
}
impl Display for FileLoggerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use FileLoggerError::*;
        match self {
            FileCreate { path, .. } => write!(f, "Failed to create new log file '{}'", path.display()),
            FileOpen { path, .. } => write!(f, "Failed to open existing log file '{}'", path.display()),
            FileSeek { path, .. } => write!(f, "Failed to seek in log file '{}'", path.display()),
            FileShutdown { path, .. } => write!(f, "Failed to flush log file '{}'", path.display()),
            FileWrite { path, .. } => write!(f, "Failed to write to log file '{}'", path.display()),
            StatementSerialize { kind, .. } => write!(f, "Failed to serialize {kind}"),
        }
    }
}
impl Error for FileLoggerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use FileLoggerError::*;
        match self {
            FileCreate { err, .. } => Some(err),
            FileOpen { err, .. } => Some(err),
            FileSeek { err, .. } => Some(err),
            FileShutdown { err, .. } => Some(err),
            FileWrite { err, .. } => Some(err),
            StatementSerialize { err, .. } => Some(err),
        }
    }
}





/***** AUXILLARY *****/
/// Collects everything we might want to log in the [`FileLogger`].
#[derive(Clone, Copy, Debug, EnumDebug, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING-KEBAB-CASE")]
pub enum LogStatement<'a, C> {
    /// A request that asks if a task may be executed has been received.
    ExecuteTask {
        reference: &'a str,
        auth:      &'a AuthContext,
        policy:    i64,
        state:     &'a State,
        workflow:  &'a Workflow,
        task:      &'a str,
    },
    /// A request that asks if an asset may be accessed has been received.
    AssetAccess {
        reference: &'a str,
        auth:      &'a AuthContext,
        policy:    i64,
        state:     &'a State,
        workflow:  &'a Workflow,
        data:      &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        task:      &'a Option<String>,
    },
    /// A request that asks if a workflow is permitted has been received.
    WorkflowValidate {
        reference: &'a str,
        auth:      &'a AuthContext,
        policy:    i64,
        state:     &'a State,
        workflow:  &'a Workflow,
    },

    /// Logs the raw response of a reasoner.
    ReasonerResponse {
        reference: &'a str,
        response:  &'a str,
    },
    /// Logs the official response of a reasoner.
    ReasonerVerdict {
        reference: &'a str,
        verdict:   &'a Verdict,
    },

    /// Logs the reasoner backend for during startup.
    ReasonerContext {
        connector_context: &'a C,
    },
    /// Logs the arrival of a new policy.
    PolicyAdd {
        auth: &'a AuthContext,
        connector_context: &'a C,
        policy: &'a Policy,
    },
    /// Logs the activation of an existing policy.
    PolicyActivate {
        auth:   &'a AuthContext,
        policy: &'a Policy,
    },
    Init,
}





/***** LIBRARY *****/
/// A mock version of the logger that simply ignores all logged statements.
///
/// Just here for testing purposes.
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
        _reference: &str,
        _auth: &AuthContext,
        _policy: i64,
        _state: &State,
        _workflow: &Workflow,
        _task: &str,
    ) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_exec_task_request");
        Ok(())
    }

    async fn log_data_access_request(
        &self,
        _reference: &str,
        _auth: &AuthContext,
        _policy: i64,
        _state: &State,
        _workflow: &Workflow,
        _data: &str,
        _task: &Option<String>,
    ) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_data_access_request");
        Ok(())
    }

    async fn log_validate_workflow_request(
        &self,
        _reference: &str,
        _auth: &AuthContext,
        _policy: i64,
        _state: &State,
        _workflow: &Workflow,
    ) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_validate_workflow_request");
        Ok(())
    }

    async fn log_verdict(&self, _reference: &str, _verdict: &Verdict) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_verdict");
        Ok(())
    }

    async fn log_add_policy_request<C: Sync + Debug + Serialize>(
        &self,
        _auth: &AuthContext,
        _connector_context: &C,
        _policy: &Policy,
    ) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_add_policy_request");
        Ok(())
    }

    async fn log_set_active_version_policy(&self, _auth: &AuthContext, _policy: &Policy) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_set_active_version_policy");
        Ok(())
    }

    async fn log_reasoner_context<C: Sync + Debug + Serialize>(&self, _connector_context: &C) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_reasoner_context");
        Ok(())
    }
}

#[async_trait::async_trait]
impl ReasonerConnectorAuditLogger for MockLogger {
    async fn log_reasoner_response(&self, _reference: &str, _response: &str) -> Result<(), AuditLoggerError> {
        println!("AUDIT LOG: log_reasoner_response");
        Ok(())
    }
}

/// A more serious version of a logger that logs to a file.
///
/// Note that this logger is not exactly the perfect audit log, as it does nothing w.r.t. ensuring that the file is the same as last time or signing changes or w/e.
#[derive(Clone)]
pub struct FileLogger {
    /// The path of the file to log to.
    path: PathBuf,
}
impl FileLogger {
    /// Constructor for the FileLogger that initializes it pointing to the given file.
    ///
    /// # Arguments
    /// - `path`: The path to the file to log to.
    ///
    /// # Returns
    /// A new instance of self, ready for action.
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self { Self { path: path.into() } }

    /// Writes a log statement to the logging file.
    ///
    /// # Arguments
    /// - `stmt`: The [`LogStatement`] that determines what we're gonna log.
    ///
    /// # Errors
    /// This function errors if we failed to perform the logging completely (i.e., either write or flush).
    pub async fn log<C1: Clone + Serialize, C2: Clone + Serialize>(&self, stmt: LogStatement<'_, C1, C2>) -> Result<(), FileLoggerError> {
        // Step 1: Open the log file
        let mut handle: File = if !self.path.exists() {
            debug!("Creating new log file at '{}'...", self.path.display());
            match File::create(&self.path).await {
                Ok(handle) => handle,
                Err(err) => return Err(FileLoggerError::FileCreate { path: self.path.clone(), err }),
            }
        } else {
            debug!("Opening existing log file at '{}'...", self.path.display());
            match OpenOptions::new().write(true).append(true).open(&self.path).await {
                Ok(handle) => handle,
                Err(err) => return Err(FileLoggerError::FileOpen { path: self.path.clone(), err }),
            }
        };

        // // Navigate to the end of the file
        // let end_pos: u64 = match handle.seek(SeekFrom::End(0)).await {
        //     Ok(pos) => pos,
        //     Err(err) => return Err(FileLoggerError::FileSeek { path: self.path.clone(), err }),
        // };
        // debug!("End of file is after {end_pos} bytes");

        // Write the message
        debug!("Writing {}-statement to logfile...", stmt.variant());
        // Write who wrote it
        write_file!(self.path.clone(), &mut handle, concat!("[", env!("CARGO_BIN_NAME"), " v", env!("CARGO_PKG_VERSION"), "]")).await?;
        // Print the timestamp
        write_file!(self.path.clone(), &mut handle, "[{}]", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")).await?;
        // Then write the logged message
        match serde_json::to_string(&stmt) {
            Ok(message) => writeln_file!(self.path.clone(), &mut handle, " {message}").await?,
            Err(err) => return Err(FileLoggerError::StatementSerialize { kind: format!("{:?}", stmt.variant()), err }),
        }

        // Finally flush the file
        debug!("Flushing log file...");
        if let Err(err) = handle.shutdown().await {
            return Err(FileLoggerError::FileShutdown { path: self.path.clone(), err });
        }
        drop(handle);

        // Done, a smashing success
        Ok(())
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
        debug!("Handling request to log execute_task request");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), ()> = LogStatement::execute_task(reference, auth, policy, state, workflow, task);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
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
        debug!("Handling request to log data_access request");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), ()> = LogStatement::asset_access(reference, auth, policy, state, workflow, data, task);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }

    async fn log_validate_workflow_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
    ) -> Result<(), AuditLoggerError> {
        debug!("Handling request to log workflow_validate request");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), ()> = LogStatement::workflow_validate(reference, auth, policy, state, workflow);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }

    async fn log_verdict(&self, reference: &str, verdict: &Verdict) -> Result<(), AuditLoggerError> {
        debug!("Handling request to log reasoner verdict");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), ()> = LogStatement::reasoner_verdict(reference, verdict);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }

    async fn log_reasoner_context<C: Send + Sync + Clone + Debug + Serialize>(&self, connector_context: &C) -> Result<(), AuditLoggerError> {
        debug!("Handling request to log reasoner connector context");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<C, ()> = LogStatement::reasoner_context(connector_context);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }

    async fn log_add_policy_request<C: Send + Sync + Clone + Debug + Serialize>(
        &self,
        auth: &AuthContext,
        connector_context: &C,
        policy: &Policy,
    ) -> Result<(), AuditLoggerError> {
        debug!("Handling request to log policy add");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), C> = LogStatement::policy_add(auth, connector_context, policy);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }

    async fn log_set_active_version_policy(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> {
        debug!("Handling request to log policy activate");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), ()> = LogStatement::policy_activate(auth, policy);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }
}

#[async_trait::async_trait]
impl ReasonerConnectorAuditLogger for FileLogger {
    async fn log_reasoner_response(&self, reference: &str, response: &str) -> Result<(), AuditLoggerError> {
        debug!("Handling request to log reasoner response");

        // Construct the full message that we want to log, then log it (simple as that)
        let stmt: LogStatement<(), ()> = LogStatement::reasoner_response(reference, response);
        self.log(stmt).await.map_err(|err| AuditLoggerError::CouldNotDeliver(format!("{}", err.trace())))
    }
}
