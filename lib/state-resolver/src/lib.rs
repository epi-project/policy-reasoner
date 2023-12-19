use serde::{Deserialize, Serialize};
use workflow::spec::{Dataset, User};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    // Only scientists for now
    pub users:     Vec<User>,
    pub locations: Vec<User>,
    pub datasets:  Vec<Dataset>,
    pub functions: Vec<Dataset>,
    // TODO: Somehow add events / audit trail
    // TODO: Somehow add duties or duty policies, maybe encode in Dataset?
}

#[async_trait::async_trait]
pub trait StateResolver {
    async fn get_state(&self) -> State;
}
