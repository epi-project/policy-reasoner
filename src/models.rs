use chrono::{NaiveDateTime};
use diesel::prelude::*;
use crate::schema::{policies, active_version};

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = policies)]
pub struct SqlitePolicy {
    pub description: String,
    pub version: i64,
    pub version_description: String,
    pub creator: String,
    pub created_at: i64, 
    pub content: String,
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = active_version)]
pub struct SqliteActiveVersion {
    pub version: i64,
    pub activated_on: NaiveDateTime,
    pub activated_by: String,
}