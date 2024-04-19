//! lorri data storage
use crate::constants::Paths;
use crate::ops::error::ExitError;
use crate::{project, AbsPathBuf};
use anyhow::Context;
use slog::info;
use std::os::unix::ffi::OsStrExt;
use std::{
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    time::{Duration, SystemTime},
};
use tokio_rusqlite::{named_params, Connection, Transaction};

/// TODO
pub struct Sqlite {
    conn: tokio_rusqlite::Connection,
}

impl Sqlite {
    /// Connect to sqlite
    pub async fn new_connection(sqlite_path: &AbsPathBuf) -> Self {
        let conn = Connection::open(sqlite_path.as_path())
            .await
            .expect("cannot open sqlite db");
        conn.call_unwrap(|conn| {
            conn.execute_batch(
                r#"CREATE TABLE IF NOT EXISTS gc_roots (
                        id INTEGER PRIMARY KEY,
                        nix_file PATH NOT NULL,
                        last_updated EPOCH_TIME
                );
                "#,
            )
            .unwrap()
        })
        .await;

        Self { conn }
    }

    /// Migrate the GC roots into our sqlite
    pub async fn migrate_gc_roots(
        &self,
        logger: &slog::Logger,
        paths: &Paths,
    ) -> Result<(), ExitError> {
        let infos = project::list_roots(&logger, &paths)?;

        let logger2 = logger.clone();

        let mut res = self
            .conn
            .call_unwrap(move |conn| {
                let mut stmt = conn.prepare(
                "INSERT INTO gc_roots (nix_file, last_updated) VALUES (:nix_file, :last_updated);",
            )
                .unwrap();
                for (info, _project) in infos {
                    let ts = info.timestamp.map(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .expect("expect file timestamp to be a unix timestamp")
                            .as_secs()
                    });
                    stmt.execute(named_params! {
                        ":nix_file": info.nix_file.as_path().as_os_str().as_bytes().to_owned(),
                        ":last_updated": ts
                    })
                    .expect("cannot insert");
                }

                let mut stmt = conn
                    .prepare("SELECT nix_file, last_updated from gc_roots")
                    .unwrap();
                let res = stmt
                    .query_map((), |row| {
                        let nix_file =
                            OsString::from_vec(row.get::<_, Vec<u8>>("nix_file").unwrap());
                        let t = row.get::<_, Option<u64>>("last_updated").unwrap().map(|u| {
                            SystemTime::elapsed(&(SystemTime::UNIX_EPOCH + Duration::from_secs(u)))
                                .unwrap()
                        });
                        Ok((nix_file, t, t.map(ago)))
                    })
                    .unwrap()
                    .filter_map(|r| match r {
                        Err(_) => None,
                        Ok(r) => Some((r.0, r.1, r.2)),
                    })
                    .collect::<Vec<_>>();
                Ok::<_, ExitError>(res)
            })
            .await?;

        res.sort_by_key(|r| r.1);
        info!(logger2, "We have these nix files: {:#?}", res);

        Ok(())
    }

    /// Run the given code in the context of a transaction, automatically aborting the transaction if the function returns `Err`, comitting if it returns `Ok`.
    pub async fn in_transaction<F, R, E>(self, f: F) -> anyhow::Result<Result<R, E>>
    where
        F: FnOnce(&mut Transaction) -> Result<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        self.conn
            .call_unwrap(|conn| {
                let mut t = conn.transaction()?;
                match f(&mut t) {
                    Err(e) => {
                        t.rollback()?;
                        Ok(Err(e))
                    }
                    Ok(o) => {
                        t.commit()?;
                        Ok(Ok(o))
                    }
                }
            })
            .await
            .map_err(|e: tokio_rusqlite::Error| anyhow::Error::new(e))
            .context("executing sqlite transaction failed")
    }
}

fn ago(dur: Duration) -> String {
    let secs = dur.as_secs();
    let mins = dur.as_secs() / 60;
    let hours = dur.as_secs() / (60 * 60);
    let days = dur.as_secs() / (60 * 60 * 24);

    if days > 0 {
        return format!("{} days ago", days);
    }
    if hours > 0 {
        return format!("{} hours ago", hours);
    }
    if mins > 0 {
        return format!("{} minutes ago", mins);
    }

    format!("{} seconds ago", secs)
}
