use std::fs;
use std::sync::Arc;

use deliberation::spec::{DeliberationAllowResponse, DeliberationDenyResponse, ExecuteTaskRequest, TaskExecResponse};
use log::{debug, info};
use policy::policy::{Policy, PolicyDataAccess};
use reasonerconn::ReasonerConnector;
use serde::{Deserialize, Serialize};
use state_resolver::StateResolver;
use warp::Filter;
use workflow::spec::Workflow;
use workflow::utils::ProgramCounter;

pub struct Srv<C, P, S> {
    reasonerconn:  C,
    policystore:   P,
    stateresolver: S,
}

#[derive(Serialize, Deserialize)]
struct PingResponse {
    success: bool,
    ping:    String,
}

impl<C, P, S> Srv<C, P, S>
where
    C: 'static + ReasonerConnector + Send + Sync,
    P: 'static + PolicyDataAccess + Send + Sync,
    S: 'static + StateResolver + Send + Sync,
{
    pub fn new(reasonerconn: C, policystore: P, stateresolver: S) -> Self { Srv { reasonerconn, policystore, stateresolver } }

    pub async fn handle_exec_task_request(
        this: Arc<Self>,
        body: ExecuteTaskRequest,
    ) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::reject::Rejection> {
        info!("Handling exec-task request");

        // First, resolve the task ID in the workflow to the ProgramCounter ID needed for `task_id` below (and before we pass it by ownership to be converted)
        debug!("Compiling WIR workflow to Checker Workflow...");
        let task_pc: String = ProgramCounter(body.task_id.0, body.task_id.1).display(&body.workflow.table).to_string();

        // Read the body's workflow as a Checker Workflow
        let workflow: Workflow = match Workflow::try_from(body.workflow) {
            Ok(workflow) => workflow,
            Err(err) => {
                return Ok(warp::reply::with_status(warp::reply::json(&err.to_string()), warp::hyper::StatusCode::BAD_REQUEST));
            },
        };
        // Get the task ID based on the request's target ID
        let task_id = format!("{}-{}-task", workflow.id, task_pc);
        debug!("Considering task '{}' in workflow '{}'", task_id, workflow.id);

        debug!("Retrieving state...");
        let state = this.stateresolver.get_state().await;
        debug!(
            "Got state with {} datasets, {} functions, {} locations and {} users",
            state.datasets.len(),
            state.functions.len(),
            state.locations.len(),
            state.users.len()
        );

        debug!("Retrieving active policy...");
        let policy = this.policystore.get_active().unwrap();
        debug!("Got policy with {} bodies", policy.content.len());

        let verdict_reference = uuid::Uuid::nil().into();

        debug!("Consulting reasoner connector...");
        match this.reasonerconn.execute_task(policy, state, workflow, task_id).await {
            Ok(v) => {
                if !v.success {
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&deliberation::spec::Verdict::Deny(DeliberationDenyResponse {
                            shared: TaskExecResponse { verdict_reference },
                            reasons_for_denial: Some(v.errors),
                        })),
                        warp::hyper::StatusCode::OK,
                    ));
                }

                Ok(warp::reply::with_status(
                    warp::reply::json(&deliberation::spec::Verdict::Allow(DeliberationAllowResponse {
                        shared:    TaskExecResponse { verdict_reference },
                        signature: "signature".into(),
                    })),
                    warp::hyper::StatusCode::OK,
                ))
            },
            Err(err) => Ok(warp::reply::with_status(warp::reply::json(&format!("{}", err)), warp::hyper::StatusCode::OK)),
        }
    }

    pub async fn run(self) {
        let this: Arc<Self> = Arc::new(self);

        let deliberation_exec_task = warp::post()
        .and(warp::path!("deliberation" / "exec-task"))
        // .and(Self::with_policy_repo(self))
        .and(warp::any().map(move || this.clone()))
        .and(warp::body::json())
        .and_then(Self::handle_exec_task_request);

        let ping = warp::get().and(warp::path("ping")).map(|| warp::reply::json(&PingResponse { success: true, ping: String::from("pong") }));

        let index = warp::any().and(deliberation_exec_task.or(ping));

        info!("Now serving at 127.0.0.1:3030; ready for requests");
        warp::serve(index).run(([127, 0, 0, 1], 3030)).await;
    }
}
