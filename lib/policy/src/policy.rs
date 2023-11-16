use chrono::{Local, DateTime};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PolicyContent {
    pub reasoner: String,
    pub reasoner_version: String,
    pub content: Box<serde_json::value::RawValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolicyVersion {
    pub creator: Option<String>,
    pub created_at: DateTime<Local>, 
    pub version: Option<i64>,
    pub version_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivePolicy{
    pub version: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Policy {
    pub description: String,
    #[serde(flatten)]
    pub version: PolicyVersion,
    pub content: Vec<PolicyContent>,
}

#[derive(Debug)]
pub enum PolicyDataError {
    
}

// impl Display for PolicyDataError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             PolicyDataError::SqlError(err) => write!(f, "Cannot connect to SQL database: {err}"),
//         }   
//     }
// }
// impl std::error::Error for PolicyDataError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             PolicyDataError::SqlError(err) => Some(err),
//         }
//     }
// }

pub struct Context {
    pub initiator: String,
}

pub trait PolicyDataAccess {
    type Error: std::error::Error;

    fn add_version(&self, version: Policy, context: Context) -> Result<Policy, Self::Error>;
    fn get_version(&self, version: i64) -> Result<Policy, Self::Error>;
    fn get_most_recent(&self) -> Result<Policy, Self::Error>;
    fn get_versions(&self) -> Result<Vec<PolicyVersion>, Self::Error>;
    fn get_active(&self) -> Result<Policy, Self::Error>;
    fn set_active(&self, version: i64, context: Context) -> Result<Policy, Self::Error>;
}

// Get Policy, default latest version
// GET /v1/policy

// GET specific version
// GET /v1/policy/:version
// out:
// - 200 Policy
// - 404 

// List policy's versions
// GET /v1/policy/versions (version, version_description, created_at)
// out:
// - 200 Vec<PolicyVersionDescription>

// Create new version of policy
// POST /v1/policy
// in: Policy
// out: 
//  - 201 Policy. version in body
//  - 400 problem+json

// Show active policy
// GET /v1/policy/active
// out: 200 {version: string}

// Set active policy
// PUT /v1/policy/active
// in: {version: string}
// out:
//  200 {version: string}
//  400 problem+json