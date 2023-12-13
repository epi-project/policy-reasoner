use ::policy::{Context, Policy, PolicyContent, PolicyDataAccess, PolicyDataError, PolicyVersion};
use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use policy::Transactionable;

use crate::models::{SqliteActiveVersion, SqlitePolicy};
pub struct SqlitePolicyDataStore {
    pool: Pool<ConnectionManager<SqliteConnection>>,
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

pub struct SqliteTransaction {}

impl SqliteTransaction {
    fn new(policy: Policy) -> Self { SqliteTransaction {} }
}

#[async_trait::async_trait]
impl Transactionable for SqliteTransaction {
    type Error = String;

    async fn cancel(self) -> Result<(), String> {
        todo!();
        std::mem::forget(self);
    }

    async fn accept(self) -> Result<Policy, String> {
        todo!();
        std::mem::forget(self);
    }
}

impl Drop for SqliteTransaction {
    fn drop(&mut self) { panic!("should not be called implicit, user should execute cancel or accept") }
}

#[async_trait::async_trait]
impl PolicyDataAccess for SqlitePolicyDataStore {
    type Transaction = SqliteTransaction;

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

    async fn add_version(&self, mut version: Policy, ctx: Context) -> Result<Self::Transaction, PolicyDataError> {
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
            creator: ctx.initiator,
            created_at: version.version.created_at.timestamp_micros(),
            content: str_content,
        };

        match diesel::insert_into(policies).values(&model).execute(&mut conn) {
            Ok(_) => {
                version.version.version = Some(next_version);
                return Ok(SqliteTransaction::new(version));
            },
            Err(err) => {
                return Err(PolicyDataError::GeneralError(err.to_string()));
            },
        }
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

    async fn set_active(&self, version: i64, ctx: Context) -> Result<Self::Transaction, PolicyDataError> {
        use crate::schema::active_version::dsl::active_version;
        let mut conn = self.pool.get().unwrap();

        let policy = match self.get_version(version).await {
            Ok(policy) => SqliteTransaction::new(policy),
            Err(err) => return Err(err),
        };

        let model = SqliteActiveVersion { version, activated_on: Utc::now().naive_local(), activated_by: ctx.initiator };

        match diesel::insert_into(active_version).values(&model).execute(&mut conn) {
            Ok(_) => {
                return Ok(policy);
            },
            Err(err) => Err(match err {
                Error::NotFound => PolicyDataError::NotFound,
                _ => PolicyDataError::GeneralError(err.to_string()),
            }),
        }
    }
}
