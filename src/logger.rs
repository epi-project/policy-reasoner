use audit_logger::{AuditLogger, Error as AuditLoggerError};
use auth_resolver::AuthContext;
use deliberation::spec::Verdict;
use policy::Policy;
use state_resolver::State;
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
        reference: &String,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        task: &String,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_data_access_request(
        &self,
        reference: &String,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
        data: &String,
        task: &Option<String>,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_validate_workflow_request(
        &self,
        reference: &String,
        auth: &AuthContext,
        policy: i64,
        state: &State,
        workflow: &Workflow,
    ) -> Result<(), AuditLoggerError> {
        todo!()
    }

    async fn log_reasoner_response(&self, reference: &String, response: &serde_json::Value) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_verdict(&self, reference: &String, verdict: &Verdict) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_add_policy_request(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> { todo!() }

    async fn log_set_active_version_policy(&self, auth: &AuthContext, policy: &Policy) -> Result<(), AuditLoggerError> { todo!() }
}
