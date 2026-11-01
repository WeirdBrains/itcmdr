//! Store on Diesel: Postgres or SQLite, chosen by the database URL (`postgres://…` or
//! `postgresql://…` is Postgres; `sqlite://<path>`, `sqlite:<path>` or a plain path is SQLite).
//! Migrations for each backend are embedded (migrations/postgres, migrations/sqlite).
//! Calls run on a blocking thread (`Pool::run`); SQLite uses one connection (writes are serial).
use anyhow::{anyhow, Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{self, ManageConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const SCHEMA_VERSION: i64 = 3;
const PG_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgres");
const SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

#[derive(diesel::MultiConnection)]
pub enum Db {
    Pg(diesel::PgConnection),
    Sqlite(diesel::SqliteConnection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Postgres(String),
    Sqlite(String),
}

/// Which backend a database URL names.
pub fn parse_url(url: &str) -> Result<Kind> {
    let u = url.trim();
    if u.is_empty() {
        return Err(anyhow!("database URL is empty"));
    }
    if u.starts_with("postgres://") || u.starts_with("postgresql://") {
        return Ok(Kind::Postgres(u.into()));
    }
    let p = u
        .strip_prefix("sqlite://")
        .or_else(|| u.strip_prefix("sqlite:"))
        .unwrap_or(u);
    if p.is_empty() || p.contains("://") {
        return Err(anyhow!(
            "database URL must be postgres://… or sqlite://<path> (or a file path)"
        ));
    }
    Ok(Kind::Sqlite(p.into()))
}

pub struct Manager(Kind);

impl ManageConnection for Manager {
    type Connection = Db;
    type Error = r2d2::Error;
    fn connect(&self) -> Result<Db, r2d2::Error> {
        match &self.0 {
            Kind::Postgres(u) => diesel::PgConnection::establish(u)
                .map(Db::Pg)
                .map_err(r2d2::Error::ConnectionError),
            Kind::Sqlite(p) => {
                let mut c =
                    diesel::SqliteConnection::establish(p).map_err(r2d2::Error::ConnectionError)?;
                use diesel::connection::SimpleConnection;
                c.batch_execute(
                    "PRAGMA busy_timeout = 10000; PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;",
                )
                .map_err(r2d2::Error::QueryError)?;
                Ok(Db::Sqlite(c))
            }
        }
    }
    fn is_valid(&self, c: &mut Db) -> Result<(), r2d2::Error> {
        use diesel::connection::SimpleConnection;
        c.batch_execute("SELECT 1").map_err(r2d2::Error::QueryError)
    }
    fn has_broken(&self, _: &mut Db) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct Pool {
    inner: r2d2::Pool<Manager>,
    pub sqlite: bool,
}

impl Pool {
    /// Run `f` with a connection on a blocking thread.
    pub async fn run<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Db) -> Result<T> + Send + 'static,
    {
        let p = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let mut c = p.get().context("database connection")?;
            f(&mut c)
        })
        .await
        .context("database task")?
    }
    /// Plain SQL without parameters (console and tests).
    pub async fn exec(&self, sql: &str) -> Result<usize> {
        let s = sql.to_string();
        self.run(move |c| Ok(diesel::sql_query(s).execute(c)?))
            .await
    }
}

pub async fn connect(url: &str) -> Result<Pool> {
    let kind = parse_url(url)?;
    let sqlite = matches!(kind, Kind::Sqlite(_));
    if let Kind::Sqlite(p) = &kind {
        let path = std::path::Path::new(p);
        if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d).with_context(|| format!("create {}", d.display()))?;
        }
        if !path.exists() {
            std::fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
            }
        }
    }
    tokio::task::spawn_blocking(move || {
        let inner = r2d2::Pool::builder()
            .max_size(if sqlite { 1 } else { 10 })
            .connection_timeout(std::time::Duration::from_secs(10))
            .build(Manager(kind))
            .context("connect to the database (db_url)")?;
        Ok(Pool { inner, sqlite })
    })
    .await?
}

pub async fn migrate(pool: &Pool) -> Result<()> {
    pool.run(|c| {
        match c {
            Db::Pg(c) => c.run_pending_migrations(PG_MIGRATIONS).map(|_| ()),
            Db::Sqlite(c) => c.run_pending_migrations(SQLITE_MIGRATIONS).map(|_| ()),
        }
        .map_err(|e| anyhow!("migrations: {e}"))?;
        use crate::schema::meta::dsl::*;
        let n = diesel::update(meta.filter(k.eq("schema")))
            .set(v.eq(SCHEMA_VERSION.to_string()))
            .execute(c)?;
        if n == 0 {
            diesel::insert_into(meta)
                .values((k.eq("schema"), v.eq(SCHEMA_VERSION.to_string())))
                .execute(c)?;
        }
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls() {
        assert!(matches!(
            parse_url("postgres://u@h/db").unwrap(),
            Kind::Postgres(_)
        ));
        assert_eq!(
            parse_url("sqlite:///var/lib/x.db").unwrap(),
            Kind::Sqlite("/var/lib/x.db".into())
        );
        assert_eq!(
            parse_url("sqlite:x.db").unwrap(),
            Kind::Sqlite("x.db".into())
        );
        assert_eq!(
            parse_url("C:\\data\\x.db").unwrap(),
            Kind::Sqlite("C:\\data\\x.db".into())
        );
        assert!(parse_url("mysql://h/db").is_err());
        assert!(parse_url("").is_err());
    }
}
