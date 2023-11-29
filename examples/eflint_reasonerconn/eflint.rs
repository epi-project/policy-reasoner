use std::fs;

use policy::Policy;
use reasonerconn::connector::ReasonerConnector;
use reasonerconn::eflint::EFlintReasonerConnector;
use state_resolver::State;
use workflow::spec::Workflow;

#[tokio::main]
async fn main() {
    let conn = EFlintReasonerConnector::new("http://localhost:8080".into());

    let policy = fs::read_to_string("/examples/eflint_reasonerconn/example-policy.json").unwrap();
    let workflow = fs::read_to_string("/examples/eflint_reasonerconn/example-workflow.json").unwrap();
    let state = fs::read_to_string("/examples/eflint_reasonerconn/example-state.json").unwrap();

    let workflow: Workflow = serde_json::from_str(&workflow).unwrap();
    let policy: Policy = serde_json::from_str(&policy).unwrap();
    let state: State = serde_json::from_str(&state).unwrap();

    conn.execute_task(policy, state, workflow, "X".into()).await.unwrap();
}
