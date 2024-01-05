use std::fs::File;
use std::net::SocketAddr;
use std::{env, fs};

use clap::Parser;
use humanlog::{DebugMode, HumanLogger};
use log::info;
use srv::Srv;
use state_resolver::{State, StateResolver};

use crate::auth::{JwtConfig, JwtResolver, KidResolver};
use crate::eflint::EFlintReasonerConnector;
use crate::logger::FileLogger;
use crate::sqlite::SqlitePolicyDataStore;

pub mod auth;
pub mod eflint;
pub mod logger;
pub mod models;
pub mod schema;
pub mod sqlite;


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



struct FileStateResolver {}

#[async_trait::async_trait]
impl StateResolver for FileStateResolver {
    async fn get_state(&self) -> State {
        let state = fs::read_to_string("./examples/eflint_reasonerconn/example-state.json").unwrap();
        let state: State = serde_json::from_str(&state).unwrap();

        return state;
    }
}

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
    let sresolve = FileStateResolver {};
    let server = Srv::new(args.address, logger, rconn, pstore, sresolve, pauthresolver, dauthresolver);

    server.run().await;
}
