//  MAIN.rs
//    by Lut99
//
//  Created:
//    09 Jan 2024, 13:32:03
//  Last edited:
//    07 Feb 2024, 18:08:43
//  Auto updated?
//    Yes
//
//  Description:
//!   Entrypoint to the main `policy-reasoner` binary.
//

pub mod implementation;

use std::env;
use std::fs::File;

use clap::Parser;
use error_trace::ErrorTrace as _;
use humanlog::{DebugMode, HumanLogger};
#[cfg(not(feature = "leak-public-errors"))]
use implementation::eflint::EFlintLeakNoErrors;
#[cfg(feature = "leak-public-errors")]
use implementation::eflint::EFlintLeakPrefixErrors;
use implementation::eflint::EFlintReasonerConnector;
use implementation::interface::Arguments;
use log::{error, info};
use policy_reasoner::auth::{JwtConfig, JwtResolver, KidResolver};
use policy_reasoner::logger::FileLogger;
use policy_reasoner::sqlite::SqlitePolicyDataStore;
use srv::Srv;

/***** HELPER FUNCTIONS *****/
fn get_pauth_resolver() -> JwtResolver<KidResolver> {
    let kid_resolver = KidResolver::new("./examples/config/jwk_set_expert.json").unwrap();
    let r = File::open("./examples/config/jwt_resolver.yaml").unwrap();
    let jwt_cfg: JwtConfig = serde_yaml::from_reader(r).unwrap();
    JwtResolver::new(jwt_cfg, kid_resolver).unwrap()
}
fn get_dauth_resolver() -> JwtResolver<KidResolver> {
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
type PolicyStorePlugin = SqlitePolicyDataStore;

/// The plugin used to interact with the backend reasoner.
#[cfg(feature = "leak-public-errors")]
type ReasonerConnectorPlugin = EFlintReasonerConnector<EFlintLeakPrefixErrors>;
#[cfg(not(feature = "leak-public-errors"))]
type ReasonerConnectorPlugin = EFlintReasonerConnector<EFlintLeakNoErrors>;

/// The plugin used to resolve policy input state.
#[cfg(feature = "brane-api-resolver")]
type StateResolverPlugin = policy_reasoner::state::BraneApiResolver;
#[cfg(not(feature = "brane-api-resolver"))]
type StateResolverPlugin = policy_reasoner::state::FileStateResolver;

/***** ENTRYPOINT *****/
#[tokio::main]
async fn main() {
    // Parse arguments
    let args = Arguments::parse();

    // Setup a logger
    if let Err(err) = HumanLogger::terminal(if args.trace { DebugMode::Full } else { DebugMode::Debug }).init() {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging for this session)");
    }
    info!("{} - v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // Handle help
    let mut exit: bool = false;
    if args.help_reasoner_connector {
        println!("{}", ReasonerConnectorPlugin::help('r', "reasoner-connector"));
        exit = true;
    }
    if args.help_state_resolver {
        println!("{}", StateResolverPlugin::help('s', "state-resolver"));
        exit = true;
    }
    if exit {
        std::process::exit(0);
    }

    // Initialize the plugins
    let log_identifier = format!("{binary} v{version}", binary = env!("CARGO_BIN_NAME"), version = env!("CARGO_PKG_VERSION"));
    let logger: AuditLogPlugin = FileLogger::new(log_identifier, "./audit-log.log");
    let pauthresolver: PolicyAuthResolverPlugin = get_pauth_resolver();
    let dauthresolver: DeliberationAuthResolverPlugin = get_dauth_resolver();
    let pstore: PolicyStorePlugin = SqlitePolicyDataStore::new("./data/policy.db");
    let rconn: ReasonerConnectorPlugin = match ReasonerConnectorPlugin::new(args.reasoner_connector.unwrap_or_else(String::new)) {
        Ok(rconn) => rconn,
        Err(err) => {
            error!("{}", err.trace());
            std::process::exit(1);
        },
    };

    let sresolve: StateResolverPlugin = match StateResolverPlugin::new(args.state_resolver.unwrap_or_else(String::new)) {
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
