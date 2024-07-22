//! No-op policy reasoner
//! This reasoner does a little as possible and functions as base for other implementations of the
//! policy reasoner.
use std::env;
use std::fs::File;
use std::future::Future;

pub mod implementation;

use async_trait::async_trait;
use clap::Parser;
use error_trace::ErrorTrace as _;
use humanlog::{DebugMode, HumanLogger};
use implementation::interface::Arguments;
use implementation::no_op::NoOpReasonerConnector;
use log::{error, info};
use policy::{Context, Policy, PolicyDataAccess, PolicyDataError, PolicyVersion};
use policy_reasoner::auth::{JwtConfig, JwtResolver, KidResolver};
use policy_reasoner::logger::FileLogger;
use policy_reasoner::state;
use reasonerconn::ReasonerConnector;
use srv::Srv;

/***** HELPER FUNCTIONS *****/
fn get_pauth_resolver() -> policy_reasoner::auth::JwtResolver<KidResolver> {
    let kid_resolver = KidResolver::new("./examples/config/jwk_set_expert.json").unwrap();
    let r = File::open("./examples/config/jwt_resolver.yaml").unwrap();
    let jwt_cfg: JwtConfig = serde_yaml::from_reader(r).unwrap();
    JwtResolver::new(jwt_cfg, kid_resolver).unwrap()
}
fn get_dauth_resolver() -> policy_reasoner::auth::JwtResolver<KidResolver> {
    let kid_resolver = KidResolver::new("./examples/config/jwk_set_delib.json").unwrap();
    let r = File::open("./examples/config/jwt_resolver.yaml").unwrap();
    let jwt_cfg: JwtConfig = serde_yaml::from_reader(r).unwrap();
    JwtResolver::new(jwt_cfg, kid_resolver).unwrap()
}

/***** PLUGINS *****/
/// The plugin used to do the audit logging.
type AuditLogPlugin = FileLogger;

/// The plugin used to do authentication for the policy expert API.
type PolicyAuthResolverPlugin = JwtResolver<KidResolver>;
/// The plugin used to do authentication for the deliberation API.
type DeliberationAuthResolverPlugin = JwtResolver<KidResolver>;

/// The plugin used to interact with the policy store.
type PolicyStorePlugin = DummyPolicyStore;

/// The plugin used to resolve policy input state.
#[cfg(feature = "brane-api-resolver")]
type StateResolverPlugin = crate::state::BraneApiResolver;
#[cfg(not(feature = "brane-api-resolver"))]
type StateResolverPlugin = state::FileStateResolver;

struct DummyPolicyStore;

#[async_trait]
impl PolicyDataAccess for DummyPolicyStore {
    type Error = std::convert::Infallible;

    async fn add_version<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        _version: Policy,
        _context: Context,
        _transaction: impl 'static + Send + FnOnce(Policy) -> F,
    ) -> Result<Policy, PolicyDataError> {
        #[allow(unreachable_code)]
        Ok(Policy {
            description: String::from("This is a dummy policy"),
            version:     policy::PolicyVersion {
                creator: None,
                created_at: chrono::DateTime::from_timestamp_nanos(0).into(),
                version: Some(1),
                version_description: String::from("This is a dummy version of a dummy policy"),
                // TODO: Compute hash by hand
                reasoner_connector_context: String::from("Asd"),
            },
            content:     Vec::new(),
        })
    }

    async fn get_version(&self, _version: i64) -> Result<Policy, PolicyDataError> {
        #[allow(unreachable_code)]
        Ok(Policy {
            description: String::from("This is a dummy policy"),
            version:     policy::PolicyVersion {
                creator: None,
                created_at: chrono::DateTime::from_timestamp_nanos(0).into(),
                version: Some(1),
                version_description: String::from("This is a dummy version of a dummy policy"),
                // TODO: Compute hash by hand
                reasoner_connector_context: String::from("Asd"),
            },
            content:     Vec::new(),
        })
    }

    async fn get_most_recent(&self) -> Result<Policy, PolicyDataError> {
        #[allow(unreachable_code)]
        Ok(Policy {
            description: String::from("This is a dummy policy"),
            version:     policy::PolicyVersion {
                creator: None::<String>,
                created_at: chrono::DateTime::from_timestamp_nanos(0).into(),
                version: Some(1),
                version_description: String::from("This is a dummy version of a dummy policy"),
                // TODO: Compute hash by hand
                reasoner_connector_context: String::from("Asd"),
            },
            content:     Vec::new(),
        })
    }

    async fn get_versions(&self) -> Result<Vec<PolicyVersion>, PolicyDataError> {
        #[allow(unreachable_code)]
        Ok(vec![PolicyVersion {
            creator: None,
            created_at: chrono::DateTime::from_timestamp_nanos(0).into(),
            version: Some(1),
            version_description: String::from("This is a dummy version of a dummy policy"),
            // TODO: Compute hash by hand
            reasoner_connector_context: String::from("Asd"),
        }])
    }

    async fn get_active(&self) -> Result<Policy, PolicyDataError> {
        #[allow(unreachable_code)]
        Ok(Policy {
            description: String::from("This is a dummy policy"),
            version:     policy::PolicyVersion {
                creator: None,
                created_at: chrono::DateTime::from_timestamp_nanos(0).into(),
                version: Some(1),
                version_description: String::from("This is a dummy version of a dummy policy"),
                // TODO: Compute hash by hand
                reasoner_connector_context: String::from("Asd"),
            },
            content:     Vec::new(),
        })
    }

    async fn set_active<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        _version: i64,
        _context: Context,
        _transaction: impl 'static + Send + FnOnce(Policy) -> F,
    ) -> Result<Policy, PolicyDataError> {
        #[allow(unreachable_code)]
        Ok(Policy {
            description: String::from("This is a dummy policy"),
            version:     policy::PolicyVersion {
                creator: None,
                created_at: chrono::DateTime::from_timestamp_nanos(0).into(),
                version: Some(1),
                version_description: String::from("This is a dummy version of a dummy policy"),
                // TODO: Compute hash by hand
                reasoner_connector_context: String::from("Asd"),
            },
            content:     Vec::new(),
        })
    }

    async fn deactivate_policy<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        _context: Context,
        _transaction: impl 'static + Send + FnOnce() -> F,
    ) -> Result<(), PolicyDataError> {
        // Nothing to do
        Ok(())
    }
}

/***** ENTRYPOINT *****/
#[tokio::main]
async fn main() {
    let args: Arguments = Arguments::parse();

    let rconn = NoOpReasonerConnector::new();

    // Setup a logger
    if let Err(err) = HumanLogger::terminal(if args.trace { DebugMode::Full } else { DebugMode::Debug }).init() {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging for this session)");
    }

    info!("{} - v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    run_app(args, rconn).await;
}

async fn run_app<R>(args: Arguments, rconn: R)
where
    R: ReasonerConnector<AuditLogPlugin> + Send + Sync + 'static,
{
    // Initialize the plugins
    let log_identifier = format!("{binary} v{version}", binary = env!("CARGO_BIN_NAME"), version = env!("CARGO_PKG_VERSION"));
    let logger: AuditLogPlugin = FileLogger::new(log_identifier, "./audit-log.log");
    let pauthresolver: PolicyAuthResolverPlugin = get_pauth_resolver();
    let dauthresolver: DeliberationAuthResolverPlugin = get_dauth_resolver();
    let pstore: PolicyStorePlugin = DummyPolicyStore {};

    let sresolve: StateResolverPlugin = match StateResolverPlugin::new(String::new()) {
        Ok(sresolve) => sresolve,
        Err(err) => {
            error!("{}", err.trace());
            std::process::exit(1);
        },
    };

    // Run them!
    let server = Srv::new(args.address, logger, rconn, pstore, sresolve, pauthresolver, dauthresolver);

    server.run().await;
}
