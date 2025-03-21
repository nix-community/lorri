//! lorri data storage
use crate::constants::Paths;
use crate::ops::error::ExitError;
use crate::{project, AbsPathBuf};
use std::time::SystemTime;
use tokio_rusqlite::{named_params, Connection, Transaction};

/// Wrapper around our sqlite connection object.
#[derive(Clone, Debug)]
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
                        -- absolute file path to the nix file
                        nix_file TEXT UNIQUE NOT NULL,
                        -- last time the gc_root was built and linked
                        last_updated EPOCH_TIME,
                        -- if is_flake is set, this gc root refers to a nix flake.nix file
                        is_flake BOOLEAN NOT NULL default FALSE,
                        -- only if is_flake, might still be NULL if no INSTALLABLE was ever specified.
                        flake_installable TEXT default NULL
                );
                "#,
            )
            .unwrap()
        })
        .await;

        Sqlite { conn }
    }

    /// Migrate the GC roots into our sqlite
    pub async fn migrate_gc_roots(
        &self,
        logger: &slog::Logger,
        paths: &Paths,
        conn: Sqlite,
    ) -> Result<(), ExitError> {
        let infos = project::list_roots_migration(&logger, &paths, conn)?;

        self.conn
            .call_unwrap(move |conn| {
                conn.execute("DELETE FROM gc_roots", ()).unwrap();

                let mut stmt = conn
                    .prepare(
                        r"INSERT OR REPLACE INTO gc_roots (nix_file, last_updated, is_flake)
                          VALUES (:nix_file, :last_updated, :is_flake)",
                    )
                    .unwrap();

                for info in infos {
                    let last_updated = info.timestamp.map(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .expect("expect file timestamp to be a unix timestamp")
                            .as_secs()
                    });

                    stmt.execute(named_params! {
                        ":nix_file": info.nix_file.to_sql(),
                        ":last_updated": last_updated,
                        ":is_flake": info.is_flake
                    })
                    .expect("cannot insert");
                }
            })
            .await;

        //         let mut stmt = conn
        //             .prepare("SELECT nix_file, last_updated from gc_roots")
        //             .unwrap();
        //         let res = stmt
        //             .query_map((), |row| {
        //                 let nix_file =
        //                     OsString::from_vec(row.get::<_, Vec<u8>>("nix_file").unwrap());
        //                 let t = row.get::<_, Option<u64>>("last_updated").unwrap().map(|u| {
        //                     SystemTime::elapsed(&(SystemTime::UNIX_EPOCH + Duration::from_secs(u)))
        //                         .unwrap()
        //                 });
        //                 Ok((nix_file, t, t.map(ago)))
        //             })
        //             .unwrap()
        //             .filter_map(|r| match r {
        //                 Err(_) => None,
        //                 Ok(r) => Some((r.0, r.1, r.2)),
        //             })
        //             .collect::<Vec<_>>();
        //         Ok::<_, ExitError>(res)
        //     })
        //     .await?;
        //
        // res.sort_by_key(|r| r.1);
        // info!(logger2, "We have these nix files: {:#?}", res);

        Ok(())
    }

    /// Run the given code in the context of a transaction, automatically aborting the transaction if the function returns `Err`, comitting if it returns `Ok`.
    pub async fn in_transaction<F, R, E>(&mut self, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut Transaction) -> Result<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        self.conn
            .call_unwrap(|conn| {
                let mut t = conn.transaction()?;
                // We do not have to abort manually on panic in f, because commit() is never
                // called in that case and so sqlite will never complete the transaction.
                Ok::<_, rusqlite::Error>(match f(&mut t) {
                    Err(e) => {
                        t.rollback()?;
                        Err(e)
                    }
                    Ok(o) => {
                        t.commit()?;
                        Ok(o)
                    }
                })
            })
            .await
            // We assume the transaction meta-command will succeed, otherwise panic.
            // We only use a single connection thread via tokio_rusqlite, so we should
            // never run into the SQLITE_BUSY case and I think it’s ok to panic on resource shortages
            // in lorri. See https://www.sqlite.org/lang_transaction.html for all error cases.
            .expect("executing sqlite transaction failed")
    }
}
