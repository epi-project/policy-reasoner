use std::fmt::Debug;

use auth_resolver::AuthContext;
use deliberation::spec::Verdict;
use policy::Policy;
use serde::Serialize;
use state_resolver::State;
use workflow::Workflow;

#[derive(Debug)]
pub enum Error {
    CouldNotDeliver(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CouldNotDeliver(msg) => {
                write!(f, "Could not deliver: {}", msg)
            },
        }
    }
}

impl std::error::Error for Error {}

impl warp::reject::Reject for Error {}

#[async_trait::async_trait]
pub trait AuditLogger {
    async fn log_exec_task_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        task: &str,
    ) -> Result<(), Error>;

    async fn log_data_access_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        data: &str,
        task: &Option<String>,
    ) -> Result<(), Error>;

    async fn log_validate_workflow_request(
        &self,
        reference: &str,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
    ) -> Result<(), Error>;

    async fn log_reasoner_response(&self, reference: &str, response: &str) -> Result<(), Error>;
    async fn log_verdict(&self, reference: &str, verdict: &Verdict) -> Result<(), Error>;

    /// Dumps the full context of the reasoner on startup.
    ///
    /// Note that it's recommended to use `ReasonerConnector::FullContext` for this, to include the full base specification.
    async fn log_reasoner_context<C: Sync + Debug + Serialize>(&self, connector_context: &C) -> Result<(), Error>;
    /// Logs that a new policy has been added, including the full policy.
    ///
    /// Note that it's recommended to use `ReasonerConnector::Context` for this, as the full base spec as already been logged at startup.
    async fn log_add_policy_request<C: Sync + Debug + Serialize>(
        &self,
        auth: &AuthContext,
        connector_context: &C,
        policy: &Policy,
    ) -> Result<(), Error>;
    async fn log_set_active_version_policy(&self, auth: &AuthContext, policy: &Policy) -> Result<(), Error>;
}
