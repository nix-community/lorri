//! Wrap a nix file and manage corresponding state.

use anyhow::Context;
use thiserror::Error;

use crate::builder::{OutputPath, RootedPath};
use crate::constants::Paths;
use crate::nix::StorePath;
use crate::ops::error::ExitError;
use crate::sqlite::Sqlite;
use crate::{pretty_time_ago, AbsPathBuf, NixFile, TimeAgo};
use slog::warn;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;
use tokio_rusqlite::named_params;

/// A “project” knows how to handle the lorri state
/// for a given nix file.
#[derive(Clone, Debug)]
pub struct Project {
    /// Absolute path to this project’s nix file.
    pub project_file: ProjectFile,

    // Directory in which this project’s info is stored.
    project_root_dir: AbsPathBuf,

    conn: Sqlite,
}

/// ProjectFile describes the build source Nix file for a watched project
/// Could be a shell.nix (or similar) or a Flake description
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProjectFile {
    /// A shell.nix (or default.nix etc)
    ShellNix(NixFile),
    /// A Flake installable - captures the flake target itself (e.g. .#)
    /// and the context directory to resolve it from
    FlakeNix(FlakeOutput),
}

/// A shell for a flake does not just call the `flake.nix`, but each flake can have multiple
/// outputs which are referred to by `installable”.
///
/// TODO: this does not really correspond to all the things that can point to flakes,
/// The nix installable syntax can do way more things than just combine a flake.nix and directory.
/// For more information:
/// https://nix.dev/manual/nix/2.26/command-ref/new-cli/nix.html?highlight=installable#installables
/// If we implement it in a more generic fashion, how should we do the mapping from installable
/// to build output? Should we try and normalize the installable?
#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct FlakeOutput {
    /// The directory used to resolve a flakeref in the the Nix installable, if present
    pub context: AbsPathBuf, // XXX Would like an AbsDir
    /// A nix "installable" c.f. https://nixos.org/manual/nix/stable/command-ref/new-cli/nix#installables
    /// A flakeref in the installable is resolved relative to the `context` directory
    pub installable: String,
}

impl ProjectFile {
    /// Creates a ProjectFile::FlakeNix from a context and installable
    /// c.f. https://nix.dev/manual/nix/2.18/command-ref/new-cli/nix3-flake#description
    pub fn flake(context: AbsPathBuf, installable: String) -> Self {
        ProjectFile::FlakeNix(FlakeOutput {
            context,
            installable,
        })
    }

    /// We have lost the info which installable was used to evaluate this flake.
    /// So we just assume #. lol
    /// Obviously TODO remove
    #[deprecated]
    pub fn flake_unknown_installable(context: AbsPathBuf) -> Self {
        ProjectFile::FlakeNix(FlakeOutput {
            context,
            installable: "#.".to_string(),
        })
    }

    /// Proxy through the `Display` class for `PathBuf`.
    //    pub fn display(&self) -> std::path::Display {
    //        self.as_absolute_path().display()
    //    }

    /// Convert into a `std::path::PathBuf`
    pub fn as_absolute_path(&self) -> PathBuf {
        self.as_nix_file().as_absolute_path().to_path_buf()
    }

    /// XXX temporary compatibility
    pub fn as_nix_file(&self) -> NixFile {
        match self {
            ProjectFile::ShellNix(f) => f.clone(),
            ProjectFile::FlakeNix(i) => i.context.join("flake.nix").into(),
        }
    }
}

impl slog::Value for ProjectFile {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_arguments(key, &format_args!("{}", self.as_nix_file().display()))
    }
}

impl Project {
    /// Construct a `Project` from nix file path
    /// and the base GC root directory
    /// (as returned by `Paths.gc_root_dir()`),
    pub async fn new_and_gc_nix_files(
        mut conn: Sqlite,
        project_file: ProjectFile,
        gc_root_dir: &AbsPathBuf,
    ) -> std::io::Result<Project> {
        let project = Self::new_internal(project_file.clone(), gc_root_dir, conn.clone())?;

        // Adjust the nix_file symlink to point to this project’s nix file
        conn.in_transaction(move |t| {
            dbg!("inserting new project into db for {:?}", &project_file);
            t.execute(
                r#"
              INSERT INTO gc_roots (nix_file, is_flake, flake_installable)
              VALUES (:nix_file, :is_flake, :flake_installable)
              ON CONFLICT (nix_file) DO NOTHING
            "#,
                named_params!(
                    ":nix_file": project.project_file.as_nix_file().to_sql(),
                    ":is_flake": match project.project_file {
                        ProjectFile::ShellNix(_) => false,
                        ProjectFile::FlakeNix(_) => true
                    },
                    ":flake_installable":  match &project.project_file {
                        ProjectFile::ShellNix(_) => None,
                        // The f.context was already used by `as_nix_file()` above
                        ProjectFile::FlakeNix(ref f) => Some(f.installable.clone())
                    },
                ),
            )
            .expect("cannot insert new gc roots");
            // Adjust the nix_file symlink to point to this project’s nix file

            // A symlink from our gc_root_path directory back to the nix file which created this project.
            // Used to implement garbage collection.
            let nix_file_symlink = project.nix_file_backlink();

            let (remove, create) = match std::fs::read_link(&nix_file_symlink) {
                Ok(path) if path == project_file.as_absolute_path() => (false, false),
                Ok(_) => (true, true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (false, true),
                Err(_) => (true, true),
            };
            if remove {
                std::fs::remove_file(&nix_file_symlink)?;
            }
            if create {
                std::os::unix::fs::symlink(project_file.as_absolute_path(), nix_file_symlink)?;
            }
            Ok(project)
        })
        .await
    }

    fn new_internal(
        project_file: ProjectFile,
        gc_root_dir: &AbsPathBuf,
        conn: Sqlite,
    ) -> std::io::Result<Project> {
        let hash = format!(
            "{:x}",
            md5::compute(project_file.as_absolute_path().as_os_str().as_bytes())
        );
        let project_root_dir = gc_root_dir.join(&hash);

        std::fs::create_dir_all(&project_root_dir.join("gc_root"))?;

        Ok(Project {
            project_root_dir,
            project_file,
            conn,
        })
    }

    /// If the hash for our gc directory is already known, create a project by resolving the nix file via its symlink.
    fn new_internal_from_existing_gc_dir(
        hash: String,
        gc_root_dir: &AbsPathBuf,
        conn: Sqlite,
    ) -> Result<Project, anyhow::Error> {
        let project_root_dir = gc_root_dir.join(&hash);

        let nix_file_symlink = project_root_dir.join("gc_root").join("nix_file");
        let link = std::fs::read_link(&nix_file_symlink).map_err(|e| {
            anyhow::Error::new(e).context(format!("Cannot fs::read_link {nix_file_symlink:?}"))
        })?;
        let original_file = AbsPathBuf::new(link.clone()).expect(&format!(
            "nix_file symlink is a relative path, this should not happen: {link:?}"
        ));
        let project_file = match original_file.as_path().file_name().map(OsStr::to_str) {
            Some(Some("flake.nix")) => {
                let p = ProjectFile::flake_unknown_installable(
                    AbsPathBuf::new(
                        original_file
                            .as_path()
                            .parent()
                            .expect(&format!("flake.nix not in directory {original_file:?}"))
                            .to_owned(),
                    )
                    .unwrap(),
                );
                p
            }
            Some(_) => ProjectFile::ShellNix(NixFile(original_file)),
            None => {
                panic!(
                    "nix file does not have a file_name(), should not happen: {original_file:?}"
                );
            }
        };

        Ok(Project {
            project_root_dir,
            project_file,
            conn,
        })
    }

    /// Directory in which this project’s
    /// garbage collection roots are stored.
    fn gc_root_path(&self) -> AbsPathBuf {
        self.project_root_dir.join("gc_root")
    }

    /// final path in the `self.gc_root_path` directory,
    /// the symlink which points to the lorri-keep-env-hack-nix-shell drv (see ./logged-evaluation.nix)
    fn shell_gc_root(&self) -> AbsPathBuf {
        self.gc_root_path().join("shell_gc_root")
    }

    /// The symlink that points to the flake shell profile generated by `nix develop`.
    fn flake_profile_gc_root(&self) -> AbsPathBuf {
        self.gc_root_path().join("flake_profile")
    }

    /// A symlink from our gc_root_path directory back to the nix file which created this project.
    /// Used to implement garbage collection.
    fn nix_file_backlink(&self) -> AbsPathBuf {
        self.gc_root_path().join("nix_file")
    }

    /// Return the filesystem paths for these roots.
    pub fn root_path(&self) -> OutputPath {
        OutputPath::new(RootPath(self.shell_gc_root()))
    }

    /// Get the timestamp for when this project was last built, if it was.
    pub fn last_built_timestamp(&self) -> Option<SystemTime> {
        match std::fs::symlink_metadata(self.shell_gc_root()) {
            Err(_) => {
                // no gc root, so nothing to report
                None
            }
            Ok(m) => m.modified().map_or(None, Some),
        }
    }

    /// Create roots to store paths.
    /// Consumes a temporary [RootedPath] and creates a root for each path it points to.
    pub fn create_roots(&self, rooted_path: RootedPath) -> Result<OutputPath, AddRootError> {
        if let Some(store_path) = rooted_path.flake_profile_path {
            self.create_root(store_path, self.flake_profile_gc_root())?;
        }
        self.create_root(rooted_path.path, self.shell_gc_root())
    }

    /// Takes the given [StorePath] and creates a nix GC root in our gc_roots cache directory,
    /// under the project hash, e.g. `~/.cache/lorri/gc_roots/<project.hash>/<gc_root_name>`
    fn create_root(
        &self,
        store_path: StorePath,
        lorri_gc_root: AbsPathBuf,
    ) -> Result<OutputPath, AddRootError> {
        // nix-store --add-root /tmp/test-root --realise
        let mut cmd = Command::new("nix-store");
        cmd.args([
            OsStr::new("--realise"),
            OsStr::new("--add-root"),
            lorri_gc_root.as_path().as_os_str(),
            store_path.as_path().as_os_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

        if !cmd
            .status()
            .map_err(|e| AddRootError::nix_run_error(e, store_path.as_path()))?
            .success()
        {
            return Err(AddRootError::nix_failed(store_path.as_path()));
        }

        Ok(OutputPath::new(RootPath(lorri_gc_root)))
    }

    /// Removes this project from lorri. Removes the GC root and consumes the project.
    pub async fn remove_project(mut self) -> anyhow::Result<()> {
        self.conn
            .in_transaction(move |t| {
                std::fs::remove_dir_all(&self.project_root_dir).context(format!(
                    "Unable to remove the project directory from {}",
                    self.project_root_dir.display()
                ))?;

                t.execute(
                    "DELETE FROM gc_roots WHERE nix_file = :nix_file",
                    named_params!(":nix_file": self.project_file.as_nix_file().to_sql()),
                )?;
                Ok(())
            })
            .await
    }
}

/// A path to a gc root.
#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct RootPath(pub AbsPathBuf);

impl RootPath {
    /// `display` the path.
    pub fn display(&self) -> std::path::Display {
        self.0.display()
    }
}

/// Error conditions encountered when adding roots
#[derive(Error, Debug)]
#[error("{msg}: {source}")]
pub struct AddRootError {
    #[source]
    source: std::io::Error,
    msg: String,
}

impl AddRootError {
    fn nix_run_error(source: std::io::Error, path: &Path) -> AddRootError {
        AddRootError {
            source,
            msg: format!("error running nix command for {}", path.display()),
        }
    }

    fn nix_failed(path: &Path) -> AddRootError {
        AddRootError {
            source: std::io::Error::new(std::io::ErrorKind::Other, "nix failed"),
            msg: format!("nix build returned non-zero status for {}", path.display()),
        }
    }
}

/// How the list_roots result should be sorted
#[derive(Debug, Clone, Copy)]
pub enum ListRootsSort {
    /// Return in arbitrary order
    NoSorting,
    /// Return most recent items last, sort secondary by project_file path name
    MoreRecentLast,
}

/// Returns a list of existing gc roots along with some metadata
fn list_roots_impl(
    logger: &slog::Logger,
    paths: &Paths,
    conn: Sqlite,
    list_roots_sort: ListRootsSort,
) -> Result<Vec<ListRoots>, ExitError> {
    let mut res = Vec::new();
    let gc_root_dir_iter = std::fs::read_dir(paths.gc_root_dir()).map_err(|e| {
        ExitError::environment_problem(
            anyhow::anyhow!(e).context("Cannot read lorri gc root directory"),
        )
    })?;
    let project_gc_root_dirs = {
        let mut res = vec![];
        for entry in gc_root_dir_iter {
            match entry {
                Err(e) => {
                    warn!(logger, "Cannot read gc project directory: {}", e)
                }
                Ok(entry) => {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            res.push(entry);
                            continue;
                        }
                    }
                    warn!(
                        logger,
                        "Skipping {} which should be a directory",
                        entry.path().display()
                    );
                }
            }
        }
        res
    };
    for project_gc_root_dir in project_gc_root_dirs {
        let hash = project_gc_root_dir
            .file_name()
            .to_string_lossy()
            .into_owned();
        let project = match Project::new_internal_from_existing_gc_dir(
            hash.clone(),
            paths.gc_root_dir(),
            conn.clone(),
        ) {
            Err(e) => {
                warn!(
                    logger,
                    "Could not create project for hash {} in root dir {}, skipping: {}",
                    &hash,
                    paths.gc_root_dir().display(),
                    e
                );
                continue;
            }
            Ok(p) => p,
        };
        let timestamp = project.last_built_timestamp();

        let project_file_exists = project.project_file.as_absolute_path().is_file();
        res.push(ListRoots {
            project,
            timestamp,
            project_file_exists,
        });
    }
    match list_roots_sort {
        ListRootsSort::NoSorting => {}
        ListRootsSort::MoreRecentLast => {
            let now = SystemTime::now();
            res.sort_by_key(|r| {
                (
                    r.timestamp.map(|t| TimeAgo::from_system_time(now, t)),
                    r.project
                        .project_file
                        .as_nix_file()
                        .as_absolute_path()
                        .to_owned(),
                )
            })
        }
    }
    Ok(res)
}

struct ListRoots {
    project: Project,
    timestamp: Option<SystemTime>,
    project_file_exists: bool,
}

/// List roots for doing the sqlite migration
pub fn list_roots_migration(
    logger: &slog::Logger,
    paths: &Paths,
    conn: Sqlite,
    list_roots_sort: ListRootsSort,
) -> Result<Vec<ListRootsMigration>, ExitError> {
    Ok(list_roots_impl(logger, paths, conn, list_roots_sort)?
        .into_iter()
        .map(
            |ListRoots {
                 project, timestamp, ..
             }| {
                ListRootsMigration {
                    nix_file: project.project_file.as_nix_file(),
                    is_flake: match project.project_file {
                        ProjectFile::ShellNix(_) => false,
                        ProjectFile::FlakeNix(_) => true,
                    },
                    timestamp,
                }
            },
        )
        .collect())
}

/// Result of [list_roots_migration]
pub struct ListRootsMigration {
    /// absolute path to the nix file
    pub nix_file: NixFile,
    /// Whether the nix file is a flake.nix
    /// (we don’t know the flake installable when migrating from the old directory structure)
    pub is_flake: bool,
    /// timestamp the GC was last built
    pub timestamp: Option<SystemTime>,
}

/// Returns a list of existing gc roots along with some metadata
pub fn list_roots_gc(
    logger: &slog::Logger,
    paths: &Paths,
    conn: Sqlite,
    list_roots_sort: ListRootsSort,
) -> Result<Vec<(GcRootInfo, Project)>, ExitError> {
    Ok(list_roots_impl(logger, paths, conn, list_roots_sort)?
        .into_iter()
        .map(
            |ListRoots {
                 project,
                 timestamp,
                 project_file_exists,
             }| {
                (
                    GcRootInfo {
                        gc_dir: project.project_root_dir.clone(),
                        nix_file: project.project_file.as_nix_file().0,
                        timestamp,
                        project_file_exists,
                    },
                    project,
                )
            },
        )
        .collect())
}

/// Represents a gc root along with some metadata, used for json output of lorri gc info
#[derive(Debug)]
pub struct GcRootInfo {
    /// directory where root is stored
    pub gc_dir: AbsPathBuf,
    /// nix file from which the root originates.
    pub nix_file: AbsPathBuf,
    /// timestamp of the last build
    pub timestamp: Option<SystemTime>,
    /// whether `nix_file` still exists
    pub project_file_exists: bool,
}

impl GcRootInfo {
    /// Format for printing to stdout
    pub fn format_pretty_oneline(&self) -> String {
        let target = self.nix_file.display().to_string();
        let age = match self.timestamp.map(|t| t.elapsed()) {
            None => "sometime in the past".to_string(),
            Some(Err(_)) => "future".to_string(),
            Some(Ok(d)) => pretty_time_ago(d),
        };
        let alive = if self.project_file_exists {
            ""
        } else {
            "[gone]"
        };
        format!(
            "{} -> {} {} ({})",
            self.gc_dir.display(),
            target,
            alive,
            age
        )
    }
}
