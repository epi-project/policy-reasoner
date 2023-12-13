use std::sync::Arc;

use auth_resolver::{AuthContext, AuthResolver};
use policy::{Context, PolicyDataAccess, Transactionable};
use reasonerconn::ReasonerConnector;
use state_resolver::StateResolver;
use warp::Filter;

use crate::{models, Srv};

impl<C, P, S, PA> Srv<C, P, S, PA>
where
    C: 'static + ReasonerConnector + Send + Sync,
    P: 'static + PolicyDataAccess + Send + Sync,
    S: 'static + StateResolver + Send + Sync,
    PA: 'static + AuthResolver + Send + Sync,
{
    // Get Policy, default latest version
    // GET /v1/policies

    async fn handle_get_latest_policy(_auth_ctx: AuthContext, this: Arc<Self>) -> Result<warp::reply::Json, warp::reject::Rejection> {
        match this.policystore.get_most_recent().await {
            Ok(v) => Ok(warp::reply::json(&v)),
            Err(err) => Ok(warp::reply::json(&format!("{}", err))),
        }
    }

    // GET specific version
    // GET /v1/policies/:version
    // out:
    // - 200 Policy
    // - 404

    async fn handle_get_policy_version(_auth_ctx: AuthContext, version: i64, this: Arc<Self>) -> Result<warp::reply::Json, warp::reject::Rejection> {
        match this.policystore.get_version(version).await {
            Ok(v) => Ok(warp::reply::json(&v)),
            Err(err) => Ok(warp::reply::json(&format!("{}", err))),
        }
    }

    // List policy's versions
    // GET /v1/policies/versions (version, version_description, created_at)
    // out:
    // - 200 Vec<PolicyVersionDescription>

    async fn handle_get_all_policies(_auth_ctx: AuthContext, this: Arc<Self>) -> Result<warp::reply::Json, warp::reject::Rejection> {
        match this.policystore.get_versions().await {
            Ok(v) => Ok(warp::reply::json(&v)),
            Err(err) => Ok(warp::reply::json(&format!("{}", err))),
        }
    }

    // Create new version of policy
    // POST /v1/policies
    // in: Policy
    // out:
    //  - 201 Policy. version in body
    //  - 400 problem+json

    async fn handle_add_policy(
        auth_ctx: AuthContext,
        this: Arc<Self>,
        body: models::AddPolicyPostModel,
    ) -> Result<warp::reply::Json, warp::reject::Rejection> {
        match this.policystore.add_version(body.to_domain(), Context { initiator: auth_ctx.initiator }).await {
            Ok(transaction) => {
                // TODO try to log, if it fails reject
                let policy = match transaction.accept().await {
                    Ok(policy) => policy,
                    Err(_) => {
                        // Log and crash server, something ie really wrong
                        todo!()
                    },
                };
                Ok(warp::reply::json(&policy))
            },
            Err(err) => Ok(warp::reply::json(&format!("{}", err))),
        }
    }

    // Show active policy
    // GET /v1/policies/active
    // out: 200 {version: string}

    async fn handle_get_active_policy(_auth_ctx: AuthContext, this: Arc<Self>) -> Result<warp::reply::Json, warp::reject::Rejection> {
        match this.policystore.get_active().await {
            Ok(v) => Ok(warp::reply::json(&v)),
            Err(err) => Ok(warp::reply::json(&format!("{}", err))),
        }
    }

    // Set active policy
    // PUT /v1/policies/active
    // in: {version: string}
    // out:
    //  200 {version: string}
    //  400 problem+json

    async fn handle_set_active_policy(
        auth_ctx: AuthContext,
        this: Arc<Self>,
        body: models::SetVersionPostModel,
    ) -> Result<warp::reply::Json, warp::reject::Rejection> {
        match this.policystore.set_active(body.version, Context { initiator: auth_ctx.initiator }).await {
            Ok(transaction) => {
                // TODO try to log, if it fails reject
                let policy = match transaction.accept().await {
                    Ok(policy) => policy,
                    Err(_) => {
                        // Log and crash server, something ie really wrong
                        todo!()
                    },
                };
                Ok(warp::reply::json(&policy))
            },
            Err(err) => Ok(warp::reply::json(&format!("{}", err))),
        }
    }

    pub fn policy_handlers(this: Arc<Self>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let add_version = warp::post()
            .and(warp::path::end())
            .and(Self::with_policy_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and(warp::body::json())
            .and_then(Self::handle_add_policy);

        let get_latest = warp::get()
            .and(warp::path::end())
            .and(Self::with_policy_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and_then(Self::handle_get_latest_policy);

        let get_version = warp::get()
            .and(Self::with_policy_api_auth(this.clone()))
            .and(warp::path!(i64))
            .and(Self::with_self(this.clone()))
            .and_then(Self::handle_get_policy_version);

        let get_all = warp::get()
            .and(warp::path!("versions"))
            .and(Self::with_policy_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and_then(Self::handle_get_all_policies);

        let get_active = warp::get()
            .and(warp::path!("active"))
            .and(Self::with_policy_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and_then(Self::handle_get_active_policy);

        let set_active = warp::put()
            .and(warp::path!("active"))
            .and(Self::with_policy_api_auth(this.clone()))
            .and(Self::with_self(this.clone()))
            .and(warp::body::json())
            .and_then(Self::handle_set_active_policy);

        warp::path("v1").and(warp::path("policies")).and(get_latest.or(get_version).or(get_all).or(get_active).or(set_active).or(add_version))
    }

    fn with_policy_api_auth(this: Arc<Self>) -> impl Filter<Extract = (AuthContext,), Error = warp::Rejection> + Clone {
        Self::with_self(this.clone()).and(warp::header::headers_cloned()).and_then(|this: Arc<Self>, headers| async move {
            match this.pauthresolver.authenticate(headers).await {
                Ok(v) => Ok(v),
                Err(err) => Err(warp::reject::custom(err)),
            }
        })
    }
}
