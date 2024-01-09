//  MAIN.rs
//    by Lut99
//
//  Created:
//    09 Jan 2024, 13:32:03
//  Last edited:
//    09 Jan 2024, 13:58:33
//  Auto updated?
//    Yes
//
//  Description:
//!   Entrypoint to the main `policy-reasoner` binary.
//

use std::env;
use std::fs::File;
use std::net::SocketAddr;

use clap::Parser;
use error_trace::ErrorTrace as _;
use humanlog::{DebugMode, HumanLogger};
use log::{error, info};
use srv::Srv;

use crate::auth::{JwtConfig, JwtResolver, KidResolver};
use crate::eflint::EFlintReasonerConnector;
use crate::logger::FileLogger;
use crate::sqlite::SqlitePolicyDataStore;
use crate::state::FileStateResolver;

pub mod auth;
pub mod eflint;
pub mod logger;
pub mod models;
pub mod schema;
pub mod sqlite;
pub mod state;


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





/***** ARGUMENTS *****/
/// Defines the arguments for the `policy-reasoner` server.
#[derive(Debug, Parser)]
struct Arguments {
    /// Whether to enable full debugging
    #[clap(long, global = true, help = "If given, enables more verbose debugging.")]
    trace: bool,

    /// The address on which to bind ourselves.
    #[clap(short, long, env, default_value = "127.0.0.1:3030", help = "The address on which to bind the server.")]
    address: SocketAddr,
}





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

    let pauthresolver = get_pauth_resolver();
    let logger = FileLogger::new("./audit-log.log");
    let dauthresolver = get_dauth_resolver();
    let pstore = SqlitePolicyDataStore::new("./lib/policy/data/policy.db");
    let rconn = EFlintReasonerConnector::new("http://localhost:8080".into());
    let sresolve = match FileStateResolver::new("./examples/eflint_reasonerconn/example-state.json") {
        Ok(sresolve) => sresolve,
        Err(err) => {
            error!("{}", err.trace());
            std::process::exit(1);
        },
    };
    let server = Srv::new(args.address, logger, rconn, pstore, sresolve, pauthresolver, dauthresolver);

    server.run().await;
}
