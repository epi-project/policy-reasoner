use std::convert::Infallible;
use std::sync::Arc;

use ::policy::{PolicyDataAccess, PolicyDataError};
use auth_resolver::AuthResolver;
use log::{debug, info};
use reasonerconn::ReasonerConnector;
use serde::{Deserialize, Serialize};
use state_resolver::StateResolver;
use warp::reject::Rejection;
use warp::Filter;

pub mod deliberation;
pub mod models;
pub mod policy;

pub struct Srv<C, P, S, PA> {
    reasonerconn:  C,
    policystore:   P,
    stateresolver: S,
    pauthresolver: PA,
}

#[derive(Serialize, Deserialize)]
struct PingResponse {
    success: bool,
    ping:    String,
}

impl<C, P, S, PA> Srv<C, P, S, PA>
where
    C: 'static + ReasonerConnector + Send + Sync,
    P: 'static + PolicyDataAccess + Send + Sync,
    S: 'static + StateResolver + Send + Sync,
    PA: 'static + AuthResolver + Send + Sync,
{
    pub fn new(reasonerconn: C, policystore: P, stateresolver: S, pauthresolver: PA) -> Self {
        Srv { reasonerconn, policystore, stateresolver, pauthresolver }
    }

    fn with_self(this: Arc<Self>) -> impl Filter<Extract = (Arc<Self>,), Error = Infallible> + Clone { warp::any().map(move || this.clone()) }

    pub async fn run(self) {
        let this_arc: Arc<Self> = Arc::new(self);

        let ping = warp::get().and(warp::path("ping")).map(|| warp::reply::json(&PingResponse { success: true, ping: String::from("pong") }));
        let policy_api = Self::policy_handlers(this_arc.clone());
        let deliberation_api = Self::deliberation_handlers(this_arc.clone());

        let index = warp::any().and(deliberation_api.or(policy_api).or(ping)).recover(|err: Rejection| async move {
            debug!("err: {:?}", err);
            if let Some(auth_resolver::AuthResolverError { .. }) = err.find() {
                Ok(warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::UNAUTHORIZED))
            } else if let Some(PolicyDataError::GeneralError(msg)) = err.find() {
                // TODO implement problem+json for general error
                Ok(warp::reply::with_status(warp::reply::json(msg), warp::http::StatusCode::BAD_REQUEST))
            } else {
                Err(err)
            }
        });

        info!("Now serving at 127.0.0.1:3030; ready for requests");
        warp::serve(index).run(([127, 0, 0, 1], 3030)).await;
    }
}
