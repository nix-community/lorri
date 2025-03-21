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

#[cfg(test)]
mod tests {
    use crate::constants::Paths;
    use crate::logging::test_logger;
    use crate::project::{ListRootsSort, Project, ProjectFile};
    use crate::sqlite::Sqlite;
    use crate::{lorri_runtime_block_on, project, AbsPathBuf, NixFile};
    use std::collections::HashSet;
    use std::fs;
    use std::fs::File;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    #[test]
    fn gc_roots_migration() {
        lorri_runtime_block_on(async {
            let test_dir = tempdir().expect("tmpdir");
            let project_dir = AbsPathBuf::new(test_dir.path().join("projectdir")).unwrap();
            fs::create_dir_all(project_dir.as_path()).unwrap();

            let paths = Paths::initialize_for_tests(&test_dir).await;
            let conn = Sqlite::new_connection(&paths.sqlite_db).await;

            let flake_project_dir = AbsPathBuf::new(test_dir.path().join("flake_project")).unwrap();
            let flake_project_file = ProjectFile::flake(
                flake_project_dir.clone(),
                // NB: we have an installable here, but it’s ignored during the migration
                // because in the old storage system, the installable is not persisted between lorri invocations.
                ".my_installable".to_string(),
            );

            // create flake gc_dir setup to migrate
            {
                let flake_project = Project::new_internal_for_tests(
                    flake_project_file.clone(),
                    &paths.gc_root_dir(),
                    conn.clone(),
                )
                .expect("project");
                let mut buf = [0u8; 100];
                {
                    let mut urandom = File::open("/dev/urandom").expect("urandom");
                    urandom.read_exact(&mut buf).expect("read urandom");
                }
                let flake_file = flake_project_dir.join("flake.nix");
                fs::create_dir_all(flake_project_dir.clone()).unwrap();
                File::create(&flake_file)
                    .expect("nix_file")
                    .write_all(&buf)
                    .expect("nix_file write");
                // now create the backlink
                std::os::unix::fs::symlink(
                    &flake_file,
                    flake_project.nix_file_backlink().as_path(),
                )
                .expect("backlink");
            }

            // create non-flake setup
            let nix_shell_project_dir =
                AbsPathBuf::new(test_dir.path().join("nix_shell_project")).unwrap();
            let nix_shell_file = nix_shell_project_dir.join("shell.nix");
            let nix_shell_project_file =
                ProjectFile::ShellNix(NixFile::from(nix_shell_file.clone()));
            {
                let nix_shell_project = Project::new_internal_for_tests(
                    nix_shell_project_file.clone(),
                    &paths.gc_root_dir(),
                    conn.clone(),
                )
                .expect("project");
                let mut buf = [0u8; 100];
                {
                    let mut urandom = File::open("/dev/urandom").expect("urandom");
                    urandom.read_exact(&mut buf).expect("read urandom");
                }
                fs::create_dir_all(nix_shell_project_dir.clone()).unwrap();
                File::create(&nix_shell_file)
                    .expect("nix_file")
                    .write_all(&buf)
                    .expect("nix_file write");
                // now create the backlink
                std::os::unix::fs::symlink(
                    &nix_shell_file,
                    nix_shell_project.nix_file_backlink().as_path(),
                )
                .expect("backlink");

                // we add a “build” symlink (pointing to nothing), so we can check that
                // migration correctly reads the timestamp from the symlink
                std::os::unix::fs::symlink("/dev/null", nix_shell_project.shell_gc_root())
                    .expect("shell_gc_root");
            }

            let before_migration = project::list_roots_migration(
                &test_logger("gc_roots_migration"),
                &paths,
                conn.clone(),
            )
            .expect("list_roots_migration")
            .into_iter()
            .map(|m| m.nix_file.as_absolute_path().to_owned())
            .collect::<HashSet<_>>();

            // check that before migration we have exactly these roots when listing the gc dir
            assert_eq!(
                before_migration,
                HashSet::from([
                    flake_project_file.as_absolute_path(),
                    nix_shell_project_file.as_absolute_path()
                ])
            );

            // now do the migration
            conn.migrate_gc_roots(&test_logger("migrate db"), &paths, conn.clone())
                .await
                .expect("migration");

            // listing after migration, from db
            let after = project::list_roots_gc(&paths, conn.clone(), ListRootsSort::NoSorting)
                .await
                .expect("list_roots_gc");
            let after_migration = after
                .iter()
                .map(|m| m.0.nix_file.as_path().to_owned())
                .collect::<HashSet<_>>();

            assert_eq!(before_migration, after_migration);

            // witness that the installable of the flake input was not preserved
            let x = after
                .iter()
                .find(|m| m.0.nix_file.as_path() == flake_project_file.as_absolute_path())
                .expect("has flake");

            assert_eq!(
                x.1.project_file,
                ProjectFile::flake(flake_project_dir, "#.".to_string())
            );

            // and it’s a flake
            match x.1.project_file {
                ProjectFile::FlakeNix(_) => {}
                ProjectFile::ShellNix(_) => {
                    panic!("is not a flake!")
                }
            }

            // make sure that the timestamp is read from shell_gc_root symlink and in the database
            let y = after
                .iter()
                .find(|m| m.0.nix_file.as_path() == nix_shell_project_file.as_absolute_path())
                .expect("has nix shell file");
            assert!(y.0.timestamp.is_some(), "no timestamp in shell file");

            // and it’s not a flake
            match y.1.project_file {
                ProjectFile::FlakeNix(_) => {
                    panic!("is a flake!")
                }
                ProjectFile::ShellNix(_) => {}
            }

            drop(test_dir);
        })
    }
}
