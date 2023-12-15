use std::future::Future;

use ::policy::{Context, Policy, PolicyContent, PolicyDataAccess, PolicyDataError, PolicyVersion};
use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use tokio::runtime::Handle;

use crate::models::{SqliteActiveVersion, SqlitePolicy};
pub struct SqlitePolicyDataStore {
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

struct SqlitePolicyDataStoreError {
    msg: String,
}

impl From<PolicyDataError> for SqlitePolicyDataStoreError {
    fn from(value: PolicyDataError) -> Self {
        match value {
            PolicyDataError::NotFound => SqlitePolicyDataStoreError { msg: "Not Found".into() },
            PolicyDataError::GeneralError(msg) => SqlitePolicyDataStoreError { msg },
        }
    }
}

impl From<diesel::result::Error> for SqlitePolicyDataStoreError {
    fn from(value: diesel::result::Error) -> Self { Self { msg: value.to_string() } }
}

impl From<SqlitePolicyDataStoreError> for PolicyDataError {
    fn from(value: SqlitePolicyDataStoreError) -> Self { PolicyDataError::GeneralError(value.msg) }
}

impl SqlitePolicyDataStore {
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        // Refer to the `r2d2` documentation for more methods to use
        // when building a connection pool
        let pool = Pool::builder().test_on_check_out(true).build(manager).expect("Could not build connection pool");
        return Self { pool };
    }
}

#[async_trait::async_trait]
impl PolicyDataAccess for SqlitePolicyDataStore {
    type Error = String;

    async fn get_most_recent(&self) -> Result<Policy, PolicyDataError> {
        use crate::schema::policies::dsl::policies;

        let mut conn = self.pool.get().unwrap();
        match policies.limit(1).order_by(crate::schema::policies::dsl::created_at.desc()).select(SqlitePolicy::as_select()).load(&mut conn) {
            Ok(mut r) => {
                if r.len() != 1 {
                    return Err(PolicyDataError::NotFound);
                }
                let item = r.remove(0);
                let content = serde_json::from_str::<Vec<PolicyContent>>(item.content.as_str()).expect("error");
                let created_at = Local.from_utc_datetime(&NaiveDateTime::from_timestamp_micros(item.created_at).unwrap());
                let policy = Policy {
                    description: item.description,
                    version: PolicyVersion {
                        creator: Some(item.creator),
                        created_at,
                        version: Some(item.version),
                        version_description: item.version_description,
                    },
                    content,
                };
                return Ok(policy);
            },
            Err(err) => Err(match err {
                Error::NotFound => PolicyDataError::NotFound,
                _ => PolicyDataError::GeneralError(err.to_string()),
            }),
        }
    }

    async fn add_version<F: Future<Output = Result<(), PolicyDataError>>>(
        &self,
        mut version: Policy,
        context: Context,
        transaction: impl 'static + Send + FnOnce(Policy) -> F,
    ) -> Result<Policy, PolicyDataError> {
        use crate::schema::policies::dsl::policies;
        let mut conn = self.pool.get().unwrap();

        // get last version
        let v: Result<Vec<i64>, Error> = policies::select(policies, crate::schema::policies::dsl::version)
            .order_by(crate::schema::policies::dsl::created_at.desc())
            .limit(1)
            .load(&mut conn);

        let mut latest_version = 0;
        match v {
            Ok(versions) => {
                if versions.len() == 1 {
                    latest_version = versions[0];
                }
            },
            Err(_) => todo!(),
        }

        // up to next version
        let next_version = latest_version + 1;
        let str_content = serde_json::to_string(&version.content).unwrap();

        let model = SqlitePolicy {
            description: version.description.clone(),
            version: next_version.clone(),
            version_description: version.version.version_description.clone(),
            creator: context.initiator,
            created_at: version.version.created_at.timestamp_micros(),
            content: str_content,
        };

        let rt_handle: Handle = Handle::current();
        match tokio::task::spawn_blocking(move || {
            conn.exclusive_transaction(|conn| -> Result<Policy, SqlitePolicyDataStoreError> {
                let policy = match diesel::insert_into(policies).values(&model).execute(conn.into()) {
                    Ok(_) => {
                        version.version.version = Some(next_version);
                        version
                    },
                    Err(err) => return Err(SqlitePolicyDataStoreError { msg: err.to_string() }),
                };

                rt_handle.block_on(transaction(policy.clone())).map_err(|err| SqlitePolicyDataStoreError::from(err))?;

                Ok(policy)
            })
        })
        .await
        {
            Ok(res) => res,
            Err(err) => panic!(),
        }
        .map_err(|err: SqlitePolicyDataStoreError| err.into())
    }

    async fn get_version(&self, version: i64) -> Result<Policy, PolicyDataError> {
        use crate::schema::policies::dsl::policies;
        let mut conn = self.pool.get().unwrap();

        match policies
            .limit(1)
            .filter(crate::schema::policies::dsl::version.eq(version))
            .order_by(crate::schema::policies::dsl::created_at.desc())
            .select(SqlitePolicy::as_select())
            .load::<SqlitePolicy>(&mut conn)
        {
            Ok(mut r) => {
                if r.len() != 1 {
                    return Err(PolicyDataError::NotFound);
                }

                let item: SqlitePolicy = r.remove(0);
                let content = serde_json::from_str::<Vec<PolicyContent>>(item.content.as_str()).expect("error");
                let created_at = Local.from_utc_datetime(&NaiveDateTime::from_timestamp_micros(item.created_at).unwrap());
                let policy = Policy {
                    description: item.description,
                    version: PolicyVersion {
                        creator: Some(item.creator),
                        created_at,
                        version: Some(item.version),
                        version_description: item.version_description,
                    },
                    content,
                };

                return Ok(policy);
            },
            Err(err) => Err(match err {
                Error::NotFound => PolicyDataError::NotFound,
                _ => PolicyDataError::GeneralError(err.to_string()),
            }),
        }
    }

    async fn get_versions(&self) -> Result<Vec<PolicyVersion>, PolicyDataError> {
        use crate::schema::policies::dsl::{created_at, creator, policies, version, version_description};
        let mut conn = self.pool.get().unwrap();



        match policies.order_by(crate::schema::policies::dsl::created_at.desc()).select((version, version_description, creator, created_at)).load::<(
            i64,
            String,
            String,
            i64,
        )>(
            &mut conn,
        ) {
            Ok(r) => {
                let items: Vec<PolicyVersion> = r
                    .into_iter()
                    .map(|x| PolicyVersion {
                        version: Some(x.0),
                        version_description: x.1,
                        creator: Some(x.2),
                        created_at: Local.from_utc_datetime(&NaiveDateTime::from_timestamp_micros(x.3).unwrap()),
                    })
                    .collect();

                return Ok(items);
            },
            Err(err) => Err(match err {
                Error::NotFound => PolicyDataError::NotFound,
                _ => PolicyDataError::GeneralError(err.to_string()),
            }),
        }
    }

    async fn get_active(&self) -> Result<Policy, PolicyDataError> {
        use crate::schema::active_version::dsl::active_version;
        let mut conn = self.pool.get().unwrap();
        let av = match active_version
            .limit(1)
            .order_by(crate::schema::active_version::dsl::activated_on.desc())
            .select(crate::schema::active_version::dsl::version)
            .load::<i64>(&mut conn)
        {
            Ok(mut r) => {
                if r.len() != 1 {
                    return Err(PolicyDataError::NotFound);
                }

                r.remove(0)
            },
            Err(err) => return Err(PolicyDataError::GeneralError(err.to_string())),
        };

        self.get_version(av).await
    }

    async fn set_active<F: 'static + Send + Future<Output = Result<(), PolicyDataError>>>(
        &self,
        version: i64,
        context: Context,
        transaction: impl 'static + Send + FnOnce(Policy) -> F,
    ) -> Result<Policy, PolicyDataError> {
        use crate::schema::active_version::dsl::active_version;
        let mut conn = self.pool.get().unwrap();

        let policy = self.get_version(version).await?;

        let model = SqliteActiveVersion { version, activated_on: Utc::now().naive_local(), activated_by: context.initiator };

        let rt_handle: Handle = Handle::current();
        match tokio::task::spawn_blocking(move || {
            conn.exclusive_transaction(|conn| {
                diesel::insert_into(active_version).values(&model).execute(conn.into())?;

                rt_handle.block_on(transaction(policy.clone())).map_err(|err| SqlitePolicyDataStoreError::from(err))?;

                Ok(policy)
            })
        })
        .await
        {
            Ok(res) => res,
            Err(err) => panic!(),
        }
        .map_err(|err: SqlitePolicyDataStoreError| err.into())
    }
}
