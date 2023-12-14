use std::fmt;

use audit_logger::AuditLogger;
use policy::Policy;
use serde::{Deserialize, Serialize};
use state_resolver::State;
use workflow::spec::Workflow;

#[derive(Debug)]
pub struct ReasonerConnError {
    err: String,
}

impl fmt::Display for ReasonerConnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.err.fmt(f) }
}

impl ReasonerConnError {
    pub fn new<T: Into<String>>(t: T) -> Self { Self { err: t.into() } }

    pub fn from<T: std::error::Error>(t: T) -> Self { Self { err: format!("{}", t) } }
}

impl std::error::Error for ReasonerConnError {
    fn description(&self) -> &str { &self.err }
}

#[derive(Serialize, Deserialize)]
pub struct ReasonerResponse {
    pub success: bool,
    pub errors:  Vec<String>,
}

impl ReasonerResponse {
    pub fn new(success: bool, errors: Vec<String>) -> Self { ReasonerResponse { success, errors } }
}

#[async_trait::async_trait]
pub trait ReasonerConnector {
    /// The type returned by [`ReasonerConnector::context()`].
    type Context;
    /// The type returned by [`ReasonerConnector::full_context()`].
    type FullContext;

    /// Returns context about the reasoner connector that is relevant for the audit log.
    ///
    /// In particular, this should contain stuff like the name of the reasoner used, its version, base spec hash, etc.
    fn context(&self) -> Self::Context;
    /// Returns so-called "full context" about the reasoner connector that is relevant for the audit log.
    ///
    /// In particular, this should contain stuff like the name of the reasoner used, its version, base spec hash, etc, but also more details like the actual full base spec itself.
    fn full_context(&self) -> Self::FullContext;

    async fn execute_task<L: AuditLogger + Send + Sync>(
        &self,
        logger: &L,
        policy: Policy,
        state: State,
        workflow: Workflow,
        task: String,
    ) -> Result<ReasonerResponse, ReasonerConnError>;
    async fn access_data_request<L: AuditLogger + Send + Sync>(
        &self,
        logger: &L,
        policy: Policy,
        state: State,
        workflow: Workflow,
        data: String,
        task: Option<String>,
    ) -> Result<ReasonerResponse, ReasonerConnError>;
    async fn workflow_validation_request<L: AuditLogger + Send + Sync>(
        &self,
        logger: &L,
        policy: Policy,
        state: State,
        workflow: Workflow,
    ) -> Result<ReasonerResponse, ReasonerConnError>;
}
