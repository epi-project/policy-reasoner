use std::fs;

use policy::sqlite::SqlitePolicyDataStore;
use reasonerconn::eflint::EFlintReasonerConnector;
use srv::Srv;
use state_resolver::{StateResolver, State};

struct FileStateResolver {}

#[async_trait::async_trait]
impl StateResolver for FileStateResolver {
    async fn get_state(&self) -> State {
        let state = fs::read_to_string("./lib/reasonerconn/examples/example-state.json").unwrap();
        let state : State = serde_json::from_str(&state).unwrap();

        return state;
    }
}

#[tokio::main]
async fn main() {
    let pstore = SqlitePolicyDataStore::new("./lib/policy/data/policy.db");
    let rconn = EFlintReasonerConnector::new("http://localhost:8080".into());
    let sresolve = FileStateResolver{};
    let server = Srv::new(
        rconn, pstore, sresolve
    );

    server.run().await;
}
