use brane_ast::Workflow;
use brane_exe::pc::ProgramCounter;
use serde::{Deserialize, Serialize};

/// ExecuteTaskRequest represents the question if it is allowed to execute a
/// certain task on this node
#[derive(Serialize, Deserialize)]
pub struct ExecuteTaskRequest {
    /// Some identifier that allows the policy reasoner to assume a different context.
    ///
    /// Note that not any identifier is accepted. Which are depends on which plugins used.
    pub use_case: String,
    /// Workflow definition
    pub workflow: Workflow,
    /// The location of the task we're examining in the given `workflow`.
    pub task_id:  ProgramCounter,
}

/// AccessDataRequest represents the question if a certain dataset
/// can be accessed
#[derive(Serialize, Deserialize)]
pub struct AccessDataRequest {
    /// Some identifier that allows the policy reasoner to assume a different context.
    ///
    /// Note that not any identifier is accepted. Which are depends on which plugins used.
    pub use_case: String,
    /// Workflow definition
    pub workflow: Workflow,
    /// Identifier for the requested dataset
    pub data_id:  String,
    /// The location of the task for which we transfer in the given `workflow`. If omitted, then this transfer should be interpreted as transferring the final result of the workflow.
    pub task_id:  Option<ProgramCounter>,
}

/// WorkflowValidationRequest represents the question
/// if a workflow as a whole is considered valid by the checker.
/// Used on the 'central' side to enforce 'central' policies
#[derive(Serialize, Deserialize)]
pub struct WorkflowValidationRequest {
    /// Some identifier that allows the policy reasoner to assume a different context.
    ///
    /// Note that not any identifier is accepted. Which are depends on which plugins used.
    pub use_case: String,
    /// Workflow definition
    pub workflow: Workflow,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "verdict")]
pub enum Verdict {
    // Checker says yes
    #[serde(rename = "allow")]
    Allow(DeliberationAllowResponse),
    // Checker says no
    #[serde(rename = "deny")]
    Deny(DeliberationDenyResponse),
}

// DeliberationResponse represents the shared part of the the deliberation repsonses
// (Allow, Deny)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliberationResponse {
    pub verdict_reference: String,
}

// DeliberationResponse represents the answer the checker came up with
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliberationAllowResponse {
    #[serde(flatten)]
    pub shared:    DeliberationResponse,
    /// Signature by the checker
    pub signature: String,
}

// DeliberationResponse represents the answer the checker came up with
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliberationDenyResponse {
    #[serde(flatten)]
    pub shared: DeliberationResponse,
    /// A optional list that contains the reasons that the request is denied.
    /// Only present if the request is denied and it only contains reasons
    /// the checker wants to share.
    pub reasons_for_denial: Option<Vec<String>>,
}

pub type TaskExecResponse = DeliberationResponse;
pub type DataAccessResponse = DeliberationResponse;
pub type WorkflowValidationResponse = DeliberationResponse;

// POST /v1/deliberation/execute-task
// POST /v1/deliberation/access-data
// POST /v1/deliberation/execute-workflow
