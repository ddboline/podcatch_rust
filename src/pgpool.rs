use anyhow::{format_err, Error};
use deadpool_postgres::{Client, Config, Pool};
use std::fmt;
use tokio_postgres::{Config as PgConfig, NoTls};

use stack_string::StackString;

/// Wrapper around `r2d2::Pool`, two pools are considered equal if they have the
/// same connection string The only way to use `PgPool` is through the get
/// method, which returns a `PooledConnection` object
#[derive(Clone, Default)]
pub struct PgPool {
    pgurl: StackString,
    pool: Option<Pool>,
}

impl fmt::Debug for PgPool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PgPool {}", self.pgurl)
    }
}

impl PartialEq for PgPool {
    fn eq(&self, other: &Self) -> bool {
        self.pgurl == other.pgurl
    }
}

impl PgPool {
    /// # Errors
    /// Return error if pool setup fails
    pub fn new(pgurl: &str) -> Result<Self, Error> {
        let pgconf: PgConfig = pgurl.parse()?;

        let mut config = Config::default();

        if let tokio_postgres::config::Host::Tcp(s) = &pgconf.get_hosts()[0] {
            config.host.replace(s.to_string());
        }
        if let Some(u) = pgconf.get_user() {
            config.user.replace(u.to_string());
        }
        if let Some(p) = pgconf.get_password() {
            config
                .password
                .replace(String::from_utf8_lossy(p).to_string());
        }
        if let Some(db) = pgconf.get_dbname() {
            config.dbname.replace(db.to_string());
        }

        let pool = config.builder(NoTls)?.max_size(4).build()?;

        Ok(Self {
            pgurl: pgurl.into(),
            pool: Some(pool),
        })
    }

    /// # Errors
    /// Return error if we fail to grab connection from pool
    pub async fn get(&self) -> Result<Client, Error> {
        self.pool
            .as_ref()
            .ok_or_else(|| format_err!("No Pool Exists"))?
            .get()
            .await
            .map_err(Into::into)
    }
}
