use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;

use crate::schema::{active_version, policies};

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = policies)]
pub struct SqlitePolicy {
    pub description: String,
    pub version: i64,
    pub version_description: String,
    pub creator: String,
    pub created_at: i64,
    pub content: String,
    pub reasoner_connector_context: String,
}

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = active_version)]
pub struct SqliteActiveVersion {
    pub version: i64,
    pub activated_on: NaiveDateTime,
    pub activated_by: String,
    pub deactivated_on: Option<NaiveDateTime>,
    pub deactivated_by: Option<String>,
}

impl SqliteActiveVersion {
    pub fn new(version: i64, activated_by: String) -> Self {
        Self { version, activated_by, activated_on: Utc::now().naive_local(), deactivated_by: None, deactivated_on: None }
    }
}
