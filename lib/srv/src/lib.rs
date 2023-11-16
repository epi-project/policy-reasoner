use std::{fs};
use std::sync::Arc;

use deliberation::spec::{ExecuteTaskRequest, TaskExecResponse, DeliberationDenyResponse, DeliberationAllowResponse};
use reasonerconn::connector::{ReasonerConnector};
use state_resolver::StateResolver;
use workflow::spec::Workflow;
use policy::policy::{PolicyDataAccess, Policy};
use serde::{Serialize, Deserialize};
use warp::Filter;

pub struct Srv<C, P, S>  {
    reasonerconn : C,
    policystore: P,
    stateresolver: S
}

#[derive(Serialize, Deserialize)]
struct PingResponse {
    success: bool,
    ping: String
}

impl<C,P,S> Srv<C,P,S>
where C: 'static + ReasonerConnector + Send + Sync, P: 'static + PolicyDataAccess + Send + Sync, S: 'static + StateResolver + Send + Sync
{
    pub fn new(reasonerconn : C, policystore : P, stateresolver: S) -> Self {
        Srv{
            reasonerconn,
            policystore,
            stateresolver
        }
    }
    
    pub async fn handle_exec_task_request(this: Arc<Self>, body: ExecuteTaskRequest) -> Result<warp::reply::Json, warp::reject::Rejection> {
        let workflow = fs::read_to_string("./lib/reasonerconn/examples/example-workflow.json").unwrap();

        let state = this.stateresolver.get_state().await;

        // TODO: actually transform workflow from body (brane) to workflow (checker)
        let workflow : Workflow = serde_json::from_str(&workflow).unwrap();

        // TODO: actually transform task_id from body (brane) to task_id in workflow (checker)
        let task_id = "X".into();

        let policy = this.policystore.get_active().unwrap();

        let verdict_reference = uuid::Uuid::nil().into();

        match this.reasonerconn.execute_task(policy, state, workflow, task_id).await {
            Ok(v) => {
                if !v.success {
                    return Ok(warp::reply::json(&deliberation::spec::Verdict::Deny(DeliberationDenyResponse{
                        shared: TaskExecResponse{
                            verdict_reference,
                        },
                        reasons_for_denial: Some(v.errors),
                    })));
                }

                Ok(warp::reply::json(&deliberation::spec::Verdict::Allow(DeliberationAllowResponse{
                    shared: TaskExecResponse{
                        verdict_reference,
                    },
                    signature: "signature".into(),
                })))
                    
            },
            Err(err) => Ok(warp::reply::json(&format!("{}", err)))
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

        let ping = warp::get()
            .and(warp::path("ping"))
            .map(|| warp::reply::json(&PingResponse{success:true, ping: String::from("pong")}));

        let index = warp::any().and(
            deliberation_exec_task
                .or(ping)
        );
    
        warp::serve(index)
            .run(([127, 0, 0, 1], 3030))
            .await;
    }
}