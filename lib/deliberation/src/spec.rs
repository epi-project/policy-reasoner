use brane_ast::Workflow;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// ExecuteTaskRequest represents the question if it is allowed to execute a
/// certain task on this node
#[derive(Serialize, Deserialize)]
pub struct ExecuteTaskRequest {
    /// Workflow definition
    pub workflow: Workflow,
    /// Structured as follows:
    /// - `0`: Pointer to the particular function, where there are two cases:
    ///   - `usize::MAX` means main function (workflow.graph)
    ///   - otherwise, index into function table (workflow.funcs[...])
    /// - `1`: Pointer to the instruction (Edge) within the function indicated by `0`.
    pub task_id: (usize, usize),
}

/// AccessDataRequest represents the question if a certain dataset
/// can be accessed
#[derive(Serialize, Deserialize)]
pub struct AccessDataRequest {
    pub workflow: Workflow,
    /// Identifier for the requested dataset
    pub data_id: String,
    /// Structured as follows:
    /// - `0`: Pointer to the particular function, where there are two cases:
    ///   - `usize::MAX` means main function (workflow.graph)
    ///   - otherwise, index into function table (workflow.funcs[...])
    /// - `1`: Pointer to the instruction (Edge) within the function indicated by `0`.
    /// Empty if the requested dataset is the
    /// result of the workflow
    pub task_id: Option<(usize, usize)>,
}

/// WorkflowValidationRequest represents the question
/// if a workflow as a whole is considered valid by the checker.
/// Used on the 'central' side to enforce 'central' policies
#[derive(Serialize, Deserialize)]
pub struct WorkflowValidationRequest {
    /// Workflow definition
    pub workflow: Workflow,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "verdict")]
pub enum Verdict {
    // Checker says yes
    #[serde(rename="allow")]
    Allow(DeliberationAllowResponse),
    // Checker says no
    #[serde(rename="deny")]
    Deny(DeliberationDenyResponse)
}

// DeliberationResponse represents the shared part of the the deliberation repsonses 
// (Allow, Deny)
#[derive(Serialize, Deserialize)]
pub struct DeliberationResponse {
    pub verdict_reference: String,
}

// DeliberationResponse represents the answer the checker came up with
#[derive(Serialize, Deserialize)]
pub struct DeliberationAllowResponse {
    #[serde(flatten)]
    pub shared: DeliberationResponse,
    /// Signature by the checker 
    pub signature: String,
}

// DeliberationResponse represents the answer the checker came up with
#[derive(Serialize, Deserialize)]
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

