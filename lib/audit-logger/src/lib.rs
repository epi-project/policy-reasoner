use auth_resolver::AuthContext;
use deliberation::spec::Verdict;
use policy::Policy;
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
        reference: &String,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        task: &String,
    ) -> Result<(), Error>;

    async fn log_data_access_request(
        &self,
        reference: &String,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        data: &String,
        task: &Option<String>,
    ) -> Result<(), Error>;

    async fn log_validate_workflow_request(
        &self,
        reference: &String,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
    ) -> Result<(), Error>;

    async fn log_reasoner_response(&self, reference: &String, response: &serde_json::Value) -> Result<(), Error>;
    async fn log_verdict(&self, reference: &String, verdict: &Verdict) -> Result<(), Error>;

    // Log base defs on startup

    async fn log_add_policy_request(&self, auth: &AuthContext, policy: &Policy) -> Result<(), Error>;
    async fn log_set_active_version_policy(&self, auth: &AuthContext, policy: &Policy) -> Result<(), Error>;
}
