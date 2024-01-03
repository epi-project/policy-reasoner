use core::fmt::Debug;
use std::fmt::Display;
use std::future::Future;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PolicyContent {
    pub reasoner: String,
    pub reasoner_version: String,
    pub content: Box<serde_json::value::RawValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PolicyVersion {
    pub creator: Option<String>,
    pub created_at: DateTime<Local>,
    pub version: Option<i64>,
    pub version_description: String,
    // TODO Add base def hash
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivePolicy {
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    pub description: String,
    #[serde(flatten)]
    pub version:     PolicyVersion,
    pub content:     Vec<PolicyContent>,
}

#[derive(Debug)]
pub enum PolicyDataError {
    NotFound,
    GeneralError(String),
}

impl Display for PolicyDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyDataError::NotFound => write!(f, "PolicyData error: Item not found"),
            PolicyDataError::GeneralError(err) => write!(f, "PolicyData general error: {}", err),
        }
    }
}

impl std::error::Error for PolicyDataError {}

impl warp::reject::Reject for PolicyDataError {}

// impl std::error::Error for PolicyDataError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             PolicyDataError::NotFound => Some(err),
//             PolicyDataError::GeneralError() => todo!(),
//         }
//     }
// }


// pub trait Transaction {
//     pub fn cancel(self) -> Result<(), Error> {
//         // Do the rollback
//         //...

//         // Drop self _without_ calling its `drop()`
//         std::mem::forget(self);
//     }
//     pub fn accept(self) -> Result<Policy, Error> {
//         // Do the rollback
//         //...

//         // Drop self _without_ calling its `drop()`
//         std::mem::forget(self);
//     }
// }
// impl Drop {
//     fn drop(&mut self) {
//         // Do commit
//         panic!("should not be called implicit, user should execute cancel or accept")
//     }
// }

pub struct Context {
    pub initiator: String,
}

#[async_trait::async_trait]
pub trait PolicyDataAccess {
    type Error;
    #[must_use]
    async fn add_version<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        version: Policy,
        context: Context,
        transaction: impl 'static + Send + FnOnce(Policy) -> F,
    ) -> Result<Policy, PolicyDataError>;
    async fn get_version(&self, version: i64) -> Result<Policy, PolicyDataError>;
    async fn get_most_recent(&self) -> Result<Policy, PolicyDataError>;
    async fn get_versions(&self) -> Result<Vec<PolicyVersion>, PolicyDataError>;
    async fn get_active(&self) -> Result<Policy, PolicyDataError>;
    #[must_use]
    async fn set_active<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        version: i64,
        context: Context,
        transaction: impl 'static + Send + FnOnce(Policy) -> F,
    ) -> Result<Policy, PolicyDataError>;
    #[must_use]
    async fn deactivate_policy<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        context: Context,
        transaction: impl 'static + Send + FnOnce() -> F,
    ) -> Result<(), PolicyDataError>;
}
