use std::sync::Arc;

use audit_logger::AuditLogger;
use auth_resolver::{AuthContext, AuthResolver};
use deliberation::spec::{
    AccessDataRequest, DataAccessResponse, DeliberationAllowResponse, DeliberationDenyResponse, ExecuteTaskRequest, TaskExecResponse, Verdict,
    WorkflowValidationRequest, WorkflowValidationResponse,
};
use log::{debug, info};
use policy::PolicyDataAccess;
use reasonerconn::ReasonerConnector;
use state_resolver::StateResolver;
use warp::Filter;
use workflow::utils::ProgramCounter;
use workflow::Workflow;

use crate::Srv;

impl<L, C, P, S, PA, DA> Srv<L, C, P, S, PA, DA>
where
    L: 'static + AuditLogger + Send + Sync + Clone,
    C: 'static + ReasonerConnector + Send + Sync,
    P: 'static + PolicyDataAccess + Send + Sync,
    S: 'static + StateResolver + Send + Sync,
    PA: 'static + AuthResolver + Send + Sync,
    DA: 'static + AuthResolver + Send + Sync,
{
    // POST /v1/deliberation/execute-task
    async fn handle_execute_task_request(
        auth_ctx: AuthContext,
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
        let policy = this.policystore.get_active().await.unwrap();
        debug!("Got policy with {} bodies", policy.content.len());

        let verdict_reference: String = uuid::Uuid::new_v4().into();

        this.logger
            .log_exec_task_request(&verdict_reference, &auth_ctx, policy.version.version.unwrap(), &state, &workflow, &task_id)
            .await
            .map_err(|err| {
                debug!("Could not log exec task request to audit log : {:?} | request id: {}", err, verdict_reference);
                warp::reject::custom(err)
            })?;

        debug!("Consulting reasoner connector...");
        match this.reasonerconn.execute_task(&this.logger, policy, state, workflow, task_id).await {
            Ok(v) => {
                let resp: Verdict;
                if !v.success {
                    resp = Verdict::Deny(DeliberationDenyResponse {
                        shared: TaskExecResponse { verdict_reference: verdict_reference.clone() },
                        reasons_for_denial: Some(v.errors),
                    });
                } else {
                    resp = Verdict::Allow(DeliberationAllowResponse {
                        shared:    TaskExecResponse { verdict_reference: verdict_reference.clone() },
                        // TODO implement signature
                        signature: "signature".into(),
                    })
                }

                this.logger.log_verdict(&verdict_reference, &resp).await.map_err(|err| {
                    debug!("Could not log execute task verdict to audit log : {:?} | request id: {}", err, verdict_reference);
                    warp::reject::custom(err)
                })?;

                Ok(warp::reply::with_status(warp::reply::json(&resp), warp::hyper::StatusCode::OK))
            },
            Err(err) => Ok(warp::reply::with_status(warp::reply::json(&format!("{}", err)), warp::hyper::StatusCode::OK)),
        }
    }

    // POST /v1/deliberation/access-data
    async fn handle_access_data_request(
        auth_ctx: AuthContext,
        this: Arc<Self>,
        body: AccessDataRequest,
    ) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::reject::Rejection> {
        info!("Handling access-data request");

        debug!("Compiling WIR workflow to Checker Workflow...");

        let table = body.workflow.table.clone();
        // Read the body's workflow as a Checker Workflow
        let workflow: Workflow = match Workflow::try_from(body.workflow) {
            Ok(workflow) => workflow,
            Err(err) => {
                return Ok(warp::reply::with_status(warp::reply::json(&err.to_string()), warp::hyper::StatusCode::BAD_REQUEST));
            },
        };

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
        let policy = this.policystore.get_active().await.unwrap();
        debug!("Got policy with {} bodies", policy.content.len());

        let verdict_reference: String = uuid::Uuid::new_v4().into();

        let task_id: Option<String> = match body.task_id {
            Some(task_id) => {
                // First, resolve the task ID in the workflow to the ProgramCounter ID needed for `task_id` below (and before we pass it by ownership to be converted)
                let task_pc: String = ProgramCounter(task_id.0, task_id.1).display(&table).to_string();

                // Get the task ID based on the request's target ID
                let task_id = format!("{}-{}-task", workflow.id, task_pc);
                debug!("Considering task '{}' in workflow '{}'", task_id, workflow.id);
                Some(task_id)
            },
            None => None,
        };

        this.logger
            .log_data_access_request(&verdict_reference, &auth_ctx, policy.version.version.unwrap(), &state, &workflow, &body.data_id, &task_id)
            .await
            .map_err(|err| {
                debug!("Could not log data access request to audit log : {:?} | request id: {}", err, verdict_reference);
                warp::reject::custom(err)
            })?;

        match this.reasonerconn.access_data_request(&this.logger, policy, state, workflow, body.data_id, task_id).await {
            Ok(v) => {
                let resp: Verdict;
                if !v.success {
                    resp = Verdict::Deny(DeliberationDenyResponse {
                        shared: DataAccessResponse { verdict_reference: verdict_reference.clone() },
                        reasons_for_denial: Some(v.errors),
                    });
                } else {
                    resp = Verdict::Allow(DeliberationAllowResponse {
                        shared:    DataAccessResponse { verdict_reference: verdict_reference.clone() },
                        // TODO implement signature
                        signature: "signature".into(),
                    })
                }

                this.logger.log_verdict(&verdict_reference, &resp).await.map_err(|err| {
                    debug!("Could not log data access verdict to audit log : {:?} | request id: {}", err, verdict_reference);
                    warp::reject::custom(err)
                })?;

                Ok(warp::reply::with_status(warp::reply::json(&resp), warp::hyper::StatusCode::OK))
            },
            Err(err) => Ok(warp::reply::with_status(warp::reply::json(&format!("{}", err)), warp::hyper::StatusCode::OK)),
        }
    }

    // POST /v1/deliberation/validate-workflow
    async fn handle_validate_workflow_request(
        auth_ctx: AuthContext,
        this: Arc<Self>,
        body: WorkflowValidationRequest,
    ) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::reject::Rejection> {
        info!("Handling validate request");

        debug!("Compiling WIR workflow to Checker Workflow...");
        // Read the body's workflow as a Checker Workflow
        let workflow: Workflow = match Workflow::try_from(body.workflow) {
            Ok(workflow) => workflow,
            Err(err) => {
                return Ok(warp::reply::with_status(warp::reply::json(&err.to_string()), warp::hyper::StatusCode::BAD_REQUEST));
            },
        };

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
        let policy = this.policystore.get_active().await.unwrap();
        debug!("Got policy with {} bodies", policy.content.len());

        let verdict_reference: String = uuid::Uuid::new_v4().into();

        this.logger.log_validate_workflow_request(&verdict_reference, &auth_ctx, policy.version.version.unwrap(), &state, &workflow).await.map_err(
            |err| {
                debug!("Could not log validate workflow request to audit log : {:?} | request id: {}", err, verdict_reference);
                warp::reject::custom(err)
            },
        )?;

        match this.reasonerconn.workflow_validation_request(&this.logger, policy, state, workflow).await {
            Ok(v) => {
                let resp: Verdict;
                if !v.success {
                    resp = Verdict::Deny(DeliberationDenyResponse {
                        shared: WorkflowValidationResponse { verdict_reference: verdict_reference.clone() },
                        reasons_for_denial: Some(v.errors),
                    });
                } else {
                    resp = Verdict::Allow(DeliberationAllowResponse {
                        shared:    WorkflowValidationResponse { verdict_reference: verdict_reference.clone() },
                        // TODO implement signature
                        signature: "signature".into(),
                    })
                }

                this.logger.log_verdict(&verdict_reference, &resp).await.map_err(|err| {
                    debug!("Could not log workflow validation verdict to audit log : {:?} | request id: {}", err, verdict_reference);
                    warp::reject::custom(err)
                })?;

                Ok(warp::reply::with_status(warp::reply::json(&resp), warp::hyper::StatusCode::OK))
            },
            Err(err) => Ok(warp::reply::with_status(warp::reply::json(&format!("{}", err)), warp::hyper::StatusCode::OK)),
        }
    }

    pub fn deliberation_handlers(this: Arc<Self>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let exec_task = warp::post()
            .and(warp::path!("execute-task"))
            .and(Self::with_deliberation_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and(warp::body::json())
            .and_then(Self::handle_execute_task_request);

        let access_data = warp::post()
            .and(warp::path!("access-data"))
            .and(Self::with_deliberation_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and(warp::body::json())
            .and_then(Self::handle_access_data_request);

        let execute_workflow = warp::post()
            .and(warp::path!("execute-workflow"))
            .and(Self::with_deliberation_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and(warp::body::json())
            .and_then(Self::handle_validate_workflow_request);

        warp::path("v1").and(warp::path("deliberation")).and(exec_task.or(access_data).or(execute_workflow))
    }

    pub fn with_deliberation_api_auth(this: Arc<Self>) -> impl Filter<Extract = (AuthContext,), Error = warp::Rejection> + Clone {
        Self::with_self(this.clone()).and(warp::header::headers_cloned()).and_then(|this: Arc<Self>, headers| async move {
            match this.dauthresolver.authenticate(headers).await {
                Ok(v) => Ok(v),
                Err(err) => Err(warp::reject::custom(err)),
            }
        })
    }
}
