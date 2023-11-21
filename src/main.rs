use std::{env, fs};

use humanlog::{DebugMode, HumanLogger};
use log::info;
use policy::sqlite::SqlitePolicyDataStore;
use reasonerconn::eflint::EFlintReasonerConnector;
use srv::Srv;
use state_resolver::{State, StateResolver};

struct FileStateResolver {}

#[async_trait::async_trait]
impl StateResolver for FileStateResolver {
    async fn get_state(&self) -> State {
        let state = fs::read_to_string("./lib/reasonerconn/examples/example-state.json").unwrap();
        let state: State = serde_json::from_str(&state).unwrap();

        return state;
    }
}

#[tokio::main]
async fn main() {
    // Very simply argarser looking for the `--trace` flag
    let mut debug_mode: DebugMode = DebugMode::Debug;
    for arg in env::args() {
        if arg == "--trace" {
            debug_mode = DebugMode::Full;
        }
    }

    // Setup a logger
    if let Err(err) = HumanLogger::terminal(debug_mode).init() {
        eprintln!("WARNING: Failed to setup logger: {err} (no logging for this session)");
    }
    info!("{} - v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let pstore = SqlitePolicyDataStore::new("./lib/policy/data/policy.db");
    let rconn = EFlintReasonerConnector::new("http://localhost:8080".into());
    let sresolve = FileStateResolver {};
    let server = Srv::new(rconn, pstore, sresolve);

    server.run().await;
}
