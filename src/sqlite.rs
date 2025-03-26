//! lorri data storage

use crate::constants::Paths;
use crate::ops::error::ExitError;
use crate::{project, AbsPathBuf};
use anyhow::Context;
use slog::{debug, info};
use std::fs;
use std::time::SystemTime;
use tokio_rusqlite::{named_params, Connection, Transaction};

/// Wrapper around our sqlite connection object.
#[derive(Clone, Debug)]
pub struct Sqlite {
    conn: tokio_rusqlite::Connection,
}

/// Whether we had to migrate from the old gcroots to the sqlite database
#[derive(Debug, PartialEq)]
pub enum MigrateGCState {
    /// No GCRoots exist, so we assume this is a new lorri setup
    LorriIsFresh,
    /// We had already migrated to sqlite
    AlreadyMigrated,
    /// We hadn’t migrated and the migration was finished
    MigrationFinished,
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
    pub async fn migrate_gc_roots_if_necessary(
        &self,
        logger: &slog::Logger,
        paths: &Paths,
        conn: Sqlite,
    ) -> Result<MigrateGCState, ExitError> {
        // Check that we already have set up something, otherwise the lorri setup is “fresh”
        // and we don’t have to migrate anything
        let has_gc_roots = std::fs::read_dir(paths.gc_root_dir())
            .map(|mut dir| dir.next().is_some())
            .unwrap_or(false);

        if !has_gc_roots {
            debug!(
                logger,
                "lorri hasn’t any registered GC roots yet, migration not necessary"
            );
            return Ok(MigrateGCState::LorriIsFresh);
        }

        let count = self
            .conn
            .call_unwrap(|conn| {
                let mut qry = conn
                    .prepare("SELECT count(*) as count FROM gc_roots")
                    .context("Cannot prepare count gc_roots")?;
                qry.query_row([], |r| r.get::<_, i64>("count"))
                    .context("Cannot count gc_roots")
            })
            .await
            .map_err(|e| ExitError::panic(e))?;

        if count != 0 {
            debug!(logger, "Migration to sqlite already done, skipping");
            return Ok(MigrateGCState::AlreadyMigrated);
        }

        info!(
            logger,
            "Migrating from the old lorri directories to sqlite database at {}",
            paths.sqlite_db.display()
        );
        let infos = project::list_roots_migration(&logger, &paths, conn)?;
        let infos2 = infos.clone();

        self.conn
            .call_unwrap(move |conn| {
                let mut stmt = conn
                    .prepare(
                        r"INSERT OR REPLACE INTO gc_roots (nix_file, last_updated, is_flake)
                          VALUES (:nix_file, :last_updated, :is_flake)",
                    )
                    .context("cannot prepare INSERT OR REPLACE gc roots")?;

                for info in infos2 {
                    let last_updated = info.timestamp.map(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .expect("expect file timestamp to be a unix timestamp")
                            .as_secs()
                    });

                    let _ = stmt
                        .execute(named_params! {
                        ":nix_file": info.nix_file.to_sql(),
                        ":last_updated": last_updated,
                        ":is_flake": info.is_flake})
                        .context(format!(
                            "cannot prepare INSERT OR REPLACE gc root for {}",
                            info.nix_file.display()
                        ))?;
                }
                Ok::<(), anyhow::Error>(())
            })
            .await
            .map_err(|e: anyhow::Error| {
                ExitError::panic(e.context("Unable to migrate to the new sqlite database format"))
            })?;

        // if the migration was run successfully, we can remove the nix file backlinks,
        // as we are successfully migrated to the sqlite database
        info!(logger, "Removing nix_file backlinks");
        for info in infos {
            if let Err(_) = fs::remove_file(info.nix_file_backlink.as_path()) {
                info!(
                    logger,
                    "Could not delete backlink for {}, ignoring (probably not an issue)",
                    info.nix_file_backlink.display()
                )
            }
        }

        info!(logger, "Finished migration to sqlite database");
        Ok(MigrateGCState::MigrationFinished)
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
    use crate::sqlite::{MigrateGCState, Sqlite};
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

            {
                // start by trying the migration without any gcroots
                let no_gc_roots = conn
                    .migrate_gc_roots_if_necessary(&test_logger("migrate db"), &paths, conn.clone())
                    .await
                    .expect("migration");

                assert_eq!(no_gc_roots, MigrateGCState::LorriIsFresh);
            }

            let flake_project_dir = AbsPathBuf::new(test_dir.path().join("flake_project")).unwrap();
            let flake_project_file = ProjectFile::flake(
                flake_project_dir.clone(),
                // NB: we have an installable here, but it’s ignored during the migration
                // because in the old storage system, the installable is not persisted between lorri invocations.
                ".my_installable".to_string(),
            );
            let flake_project = Project::new_internal_for_tests(
                flake_project_file.clone(),
                &paths.gc_root_dir(),
                conn.clone(),
            )
            .expect("project");
            let flake_project_file_backlink = flake_project.gc_root_path().join("nix_file");

            // create flake gc_dir setup to migrate
            {
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
                // create the (outdated) backlink to see whether the migration deletes it
                std::os::unix::fs::symlink(&flake_file, &flake_project_file_backlink)
                    .expect("backlink");
            }

            // create non-flake setup
            let nix_shell_project_dir =
                AbsPathBuf::new(test_dir.path().join("nix_shell_project")).unwrap();
            let nix_shell_file = nix_shell_project_dir.join("shell.nix");
            let nix_shell_project_file =
                ProjectFile::ShellNix(NixFile::from(nix_shell_file.clone()));
            let nix_shell_project = Project::new_internal_for_tests(
                nix_shell_project_file.clone(),
                &paths.gc_root_dir(),
                conn.clone(),
            )
            .expect("project");
            let nix_shell_project_file_backlink = nix_shell_project.gc_root_path().join("nix_file");
            {
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

                // create the (outdated) backlink to see whether the migration deletes it
                std::os::unix::fs::symlink(&nix_shell_file, &nix_shell_project_file_backlink)
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

            {
                // now do the migration
                let migrated = conn
                    .migrate_gc_roots_if_necessary(&test_logger("migrate db"), &paths, conn.clone())
                    .await
                    .expect("migration");

                assert_eq!(migrated, MigrateGCState::MigrationFinished)
            }

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

            // and the old backlink to the nix file was deleted during migration
            assert!(
                !flake_project_file_backlink.as_path().exists(),
                "backlink still there! {}",
                flake_project_file_backlink.display()
            );

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

            // and the old backlink to the nix file was deleted during migration
            assert!(
                !nix_shell_project_file_backlink.as_path().exists(),
                "backlink still there! {}",
                nix_shell_project_file_backlink.display()
            );
            {
                // Try to migrate again, this time it should not do anything
                let no_more = conn
                    .migrate_gc_roots_if_necessary(&test_logger("migrate db"), &paths, conn.clone())
                    .await
                    .expect("migration");

                assert_eq!(no_more, MigrateGCState::AlreadyMigrated);
            }

            drop(test_dir);
        })
    }
}
