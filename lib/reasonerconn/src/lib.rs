use std::fmt;

use audit_logger::{ConnectorWithContext, ReasonerConnectorAuditLogger, SessionedConnectorAuditLogger};
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
pub trait ReasonerConnector<L: ReasonerConnectorAuditLogger>: ConnectorWithContext {
    async fn execute_task(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: Policy,
        state: State,
        workflow: Workflow,
        task: String,
    ) -> Result<ReasonerResponse, ReasonerConnError>;
    async fn access_data_request(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: Policy,
        state: State,
        workflow: Workflow,
        data: String,
        task: Option<String>,
    ) -> Result<ReasonerResponse, ReasonerConnError>;
    async fn workflow_validation_request(
        &self,
        logger: SessionedConnectorAuditLogger<L>,
        policy: Policy,
        state: State,
        workflow: Workflow,
    ) -> Result<ReasonerResponse, ReasonerConnError>;
}

// #[async_trait::async_trait]
// pub trait LoggingReasonerConnector: ReasonerConnector + ReasonerConnectorAuditLogger {
//     fn reference(&self) -> String;
//     async fn log_raw_result(&self, raw_result: &str) -> Result<(), ReasonerConnError> {
//         self.log_reasoner_response(&self.reference(), raw_result).await.map_err(|err| ReasonerConnError { err: "test".into() })
//     }
//     fn new_session(&self, session_id: String) -> Self;
// }


// pub struct DefaultLoggingReasonerConnector<Connector: ReasonerConnector, Logger: ReasonerConnectorAuditLogger> {
//     session:   Option<String>,
//     connector: Arc<Connector>,
//     logger:    Arc<Logger>,
// }


// impl<Connector: ReasonerConnector, Logger: ReasonerConnectorAuditLogger> DefaultLoggingReasonerConnector<Connector, Logger> {
//     fn new(connector: Connector, logger: Logger) -> Self { Self { session: None, connector: Arc::new(connector), logger: Arc::new(logger) } }
// }

// #[async_trait::async_trait]
// impl<Connector: ReasonerConnector + Send + Sync, Logger: ReasonerConnectorAuditLogger + Send + Sync> LoggingReasonerConnector
//     for DefaultLoggingReasonerConnector<Connector, Logger>
// {
//     fn reference(&self) -> String { return self.session.clone().unwrap() }

//     fn new_session(&self, session_id: String) -> Self {
//         Self { session: Some(session_id), connector: self.connector.clone(), logger: self.logger.clone() }
//     }
// }

// #[async_trait::async_trait]
// impl<Connector: ReasonerConnector + Send + Sync, Logger: ReasonerConnectorAuditLogger + Send + Sync> ReasonerConnectorAuditLogger
//     for DefaultLoggingReasonerConnector<Connector, Logger>
// {
//     async fn log_reasoner_response(&self, reference: &str, response: &str) -> Result<(), AuditLoggerError> {
//         self.logger.log_reasoner_response(reference, response).await
//     }
// }

// #[async_trait::async_trait]
// impl<Connector: ReasonerConnector + Send + Sync, Logger: ReasonerConnectorAuditLogger + Send + Sync> ReasonerConnector
//     for DefaultLoggingReasonerConnector<Connector, Logger>
// {
//     async fn execute_task(&self, policy: Policy, state: State, workflow: Workflow, task: String) -> Result<ReasonerResponse, ReasonerConnError> {
//         self.connector.execute_task(policy, state, workflow, task).await
//     }

//     async fn access_data_request(
//         &self,
//         policy: Policy,
//         state: State,
//         workflow: Workflow,
//         data: String,
//         task: Option<String>,
//     ) -> Result<ReasonerResponse, ReasonerConnError> {
//         self.connector.access_data_request(policy, state, workflow, data, task).await
//     }

//     async fn workflow_validation_request(&self, policy: Policy, state: State, workflow: Workflow) -> Result<ReasonerResponse, ReasonerConnError> {
//         self.connector.workflow_validation_request(policy, state, workflow).await
//     }
// }
