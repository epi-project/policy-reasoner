use std::convert::Infallible;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use ::policy::{PolicyDataAccess, PolicyDataError};
use audit_logger::AuditLogger;
use auth_resolver::AuthResolver;
use error_trace::trace;
use log::{debug, error, info, warn};
use reasonerconn::ReasonerConnector;
use serde::{Deserialize, Serialize};
use state_resolver::StateResolver;
use tokio::signal::unix::{signal, Signal, SignalKind};
use warp::reject::Rejection;
use warp::Filter;

pub mod deliberation;
pub mod models;
pub mod policy;

/// Function that returns a future that only returns if either SIGTERM or SIGINT has been sent to this process.
///
/// This is used to gracefully shut down the warp server, which takes an async function and will run until it returns. This mostly improves Docker-compatability, as it responds to `docker stop` and all that.
///
/// Only works on Unix.
async fn graceful_signal() {
    // Register a SIGTERM handler to be Docker-friendly
    let term_handler: Option<Signal> = match signal(SignalKind::terminate()) {
        Ok(handler) => Some(handler),
        Err(err) => {
            error!("{}", trace!(("Failed to register SIGTERM signal handler"), err));
            warn!("Service will NOT shutdown gracefully on SIGTERM");
            None
        },
    };

    // Also register a SIGINT handler to be manual-friendly
    let int_handler: Option<Signal> = match signal(SignalKind::interrupt()) {
        Ok(handler) => Some(handler),
        Err(err) => {
            error!("{}", trace!(("Failed to register SIGINT signal handler"), err));
            warn!("Service will NOT shutdown gracefully on SIGINT");
            None
        },
    };

    // Wait until we receive such a signal after which we terminate the server
    match (term_handler, int_handler) {
        (Some(mut term), Some(mut int)) => tokio::select! {
            _ = term.recv() => {
                info!("Received SIGTERM, shutting down gracefully...");
            },

            _ = int.recv() => {
                info!("Received SIGINT, shutting down gracefully...");
            },
        },

        (Some(mut term), None) => {
            term.recv().await;
            info!("Received SIGTERM, shutting down gracefully...");
        },
        (None, Some(mut int)) => {
            int.recv().await;
            info!("Received SIGINT, shutting down gracefully...");
        },

        // Just wait forever to not stop the warp server
        (None, None) => loop {
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        },
    }
}

pub struct Srv<L, C, P, S, PA, DA> {
    addr: SocketAddr,
    logger: L,
    reasonerconn: C,
    policystore: P,
    stateresolver: S,
    pauthresolver: PA,
    dauthresolver: DA,
}

#[derive(Serialize, Deserialize)]
struct PingResponse {
    success: bool,
    ping:    String,
}

impl<L, C, P, S, PA, DA> Srv<L, C, P, S, PA, DA>
where
    L: 'static + AuditLogger + Send + Sync + Clone,
    C: 'static + ReasonerConnector<L> + Send + Sync,
    P: 'static + PolicyDataAccess + Send + Sync,
    S: 'static + StateResolver + Send + Sync,
    PA: 'static + AuthResolver + Send + Sync,
    DA: 'static + AuthResolver + Send + Sync,
    C::Context: Send + Sync + Debug + Serialize,
{
    pub fn new(
        addr: impl Into<SocketAddr>,
        logger: L,
        reasonerconn: C,
        policystore: P,
        stateresolver: S,
        pauthresolver: PA,
        dauthresolver: DA,
    ) -> Self {
        Srv { addr: addr.into(), logger, reasonerconn, policystore, stateresolver, pauthresolver, dauthresolver }
    }

    fn with_self(this: Arc<Self>) -> impl Filter<Extract = (Arc<Self>,), Error = Infallible> + Clone { warp::any().map(move || this.clone()) }

    pub async fn run(self) {
        let addr: SocketAddr = self.addr;
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
            } else if let Some(audit_logger::Error::CouldNotDeliver { .. }) = err.find() {
                Ok(warp::reply::with_status(warp::reply::json(&()), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
            } else {
                Err(err)
            }
        });

        let (addr, srv) = warp::serve(index).bind_with_graceful_shutdown(addr, graceful_signal());
        info!("Now serving at {addr}; ready for requests");
        srv.await;
    }
}
