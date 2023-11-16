use std::fmt;

use serde::{Serialize, Deserialize};
use workflow::spec::{Workflow};
use state_resolver::{State};
use policy::policy::Policy;

#[derive(Debug)]
pub struct ReasonerConnError {
    err: String
}

impl fmt::Display for ReasonerConnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.err.fmt(f)
    }
}

impl ReasonerConnError {
    pub fn new<T: Into<String>>(t: T) -> Self {
        Self{ err: t.into() }
    }
 
    pub fn from<T: std::error::Error>(t: T) -> Self {
        Self{ err: format!("{}", t) }
    }
}

impl std::error::Error for ReasonerConnError {
    fn description(&self) -> &str {
        &self.err
    }
}

#[derive(Serialize, Deserialize)]
pub struct ReasonerResponse {
    pub success: bool,
    pub errors: Vec<String>,
}

impl ReasonerResponse {
    pub fn new(success: bool, errors: Vec<String>) -> Self {
        ReasonerResponse { success, errors }
    }
}

#[async_trait::async_trait]
pub trait ReasonerConnector {
    async fn execute_task(&self, policy: Policy, state: State, workflow: Workflow, task: String) -> Result<ReasonerResponse, Box<dyn std::error::Error>>;
    async fn access_data_request(&self, policy: Policy, state: State, workflow: Workflow, data: String, task: Option<String>) -> Result<ReasonerResponse, Box<dyn std::error::Error>>;
    async fn workflow_validation_request(&self, policy: Policy, state: State, workflow: Workflow) -> Result<ReasonerResponse, Box<dyn std::error::Error>>;
}