//! Wrap a nix file and manage corresponding state.

use slog::warn;
use thiserror::Error;

use crate::builder::{OutputPath, RootedPath};
use crate::constants::Paths;
use crate::nix::StorePath;
use crate::ops::error::ExitError;
use crate::{pretty_time_ago, AbsPathBuf, Installable, NixFile, TimeAgo};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// A “project” knows how to handle the lorri state
/// for a given nix file.
#[derive(Clone, Debug)]
pub struct Project {
    /// Absolute path to this project’s nix file.
    pub project_file: ProjectFile,

    // Directory in which this project’s info is stored.
    project_root_dir: AbsPathBuf,

    /// Hash of the nix file’s absolute path.
    hash: String,
}

/// ProjectFile describes the build source Nix file for a watched project
/// Could be a shell.nix (or similar) or a Flake description
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProjectFile {
    /// A shell.nix (or default.nix etc)
    ShellNix(NixFile),
    /// A Flake installable - captures the flake target itself (e.g. .#)
    /// and the context directory to resolve it from
    FlakeNix(Installable),
}

impl ProjectFile {
    /// Creates a ProjectFile::FlakeNix from a context and installable
    /// c.f. https://nix.dev/manual/nix/2.18/command-ref/new-cli/nix3-flake#description
    pub fn flake(context: AbsPathBuf, installable: String) -> Self {
        ProjectFile::FlakeNix(Installable {
            context,
            installable,
        })
    }

    /// We have lost the info which installable was used to evaluate this flake.
    /// So we just assume #. lol
    /// Obviously TODO remove
    #[deprecated]
    pub fn flake_unknown_installable(context: AbsPathBuf) -> Self {
        ProjectFile::FlakeNix(Installable {
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
    /// The name for the build output that's sourced in direnv to produce environment variables
    pub const ENV_CONTEXT: &'static str = "shell_gc_root";

    /// Construct a `Project` from nix file path
    /// and the base GC root directory
    /// (as returned by `Paths.gc_root_dir()`),
    pub fn new_and_gc_nix_files(
        project_file: ProjectFile,
        gc_root_dir: &AbsPathBuf,
    ) -> std::io::Result<Project> {
        let project = Self::new_internal(project_file.clone(), gc_root_dir)?;

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
    }

    fn new_internal(
        project_file: ProjectFile,
        gc_root_dir: &AbsPathBuf,
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
            hash,
        })
    }

    /// If the hash for our gc directory is already known, create a project by resolving the nix file via its symlink.
    fn new_internal_from_existing_gc_dir(
        hash: String,
        gc_root_dir: &AbsPathBuf,
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
            Some(Some("flake.nix")) => ProjectFile::flake_unknown_installable(
                AbsPathBuf::new(
                    original_file
                        .as_path()
                        .parent()
                        .expect(&format!("flake.nix not in directory {original_file:?}"))
                        .to_owned(),
                )
                .unwrap(),
            ),
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
            hash,
        })
    }

    /// Generate a "unique" ID for this project based on its absolute path.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    // final path in the `self.gc_root_path` directory,
    // the symlink which points to the lorri-keep-env-hack-nix-shell drv (see ./logged-evaluation.nix)
    // TODO: why is this variable?
    fn gc_root(&self, base: &PathBuf) -> AbsPathBuf {
        self.gc_root_path().join(base)
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

    /// A symlink from our gc_root_path directory back to the nix file which created this project.
    /// Used to implement garbage collection.
    fn nix_file_backlink(&self) -> AbsPathBuf {
        self.gc_root_path().join("nix_file")
    }

    /// Return the filesystem paths for these roots.
    pub fn root_path(&self) -> OutputPath {
        OutputPath::new(RootPath(self.gc_root(&Self::ENV_CONTEXT.into())))
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
    pub fn create_roots(&self, rooted_path: RootedPath) -> Result<OutputPath, AddRootError> {
        for path in rooted_path.extra_paths {
            let base = path
                .as_path()
                .file_name()
                .ok_or_else(|| AddRootError::naming(path.as_path()))?
                .into();
            self.create_root(base, path)?;
        }
        self.create_root(Self::ENV_CONTEXT.into(), rooted_path.path)
    }

    fn create_root(
        &self,
        base_name: PathBuf,
        store_path: StorePath,
    ) -> Result<OutputPath, AddRootError> {
        // nix-store --add-root /tmp/test-root --realise
        let mut cmd = Command::new("nix-store");
        cmd.args([
            OsStr::new("--realise"),
            OsStr::new("--add-root"),
            self.gc_root(&base_name).as_path().as_os_str(),
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

        Ok(OutputPath::new(RootPath(self.gc_root(&base_name))))
    }

    /// Removes this project from lorri. Removes the GC root and consumes the project.
    pub fn remove_project(self) -> std::io::Result<()> {
        std::fs::remove_dir_all(self.project_root_dir)
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

    /// A root path can't be properly named - because it doesn't have a filename
    /// c.f. Path.filename()
    fn naming(path: &Path) -> AddRootError {
        AddRootError {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "no filename"),
            msg: format!("Could not determine a filename for {}", path.display()),
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
pub fn list_roots(
    logger: &slog::Logger,
    paths: &Paths,
    list_roots_sort: ListRootsSort,
) -> Result<Vec<(GcRootInfo, Project)>, ExitError> {
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
        let project =
            match Project::new_internal_from_existing_gc_dir(hash.clone(), paths.gc_root_dir()) {
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

        let alive = project.project_file.as_absolute_path().is_file();
        res.push((
            GcRootInfo {
                gc_dir: project.project_root_dir.clone(),
                nix_file: project.project_file.as_nix_file().0,
                timestamp,
                project_file_exists: alive,
            },
            project,
        ));
    }
    match list_roots_sort {
        ListRootsSort::NoSorting => {}
        ListRootsSort::MoreRecentLast => {
            let now = SystemTime::now();
            res.sort_by_key(|r| {
                (
                    r.0.timestamp.map(|t| TimeAgo::from_system_time(now, t)),
                    r.1.project_file.as_nix_file().as_absolute_path().to_owned(),
                )
            })
        }
    }
    Ok(res)
}

/// Represents a gc root along with some metadata, used for json output of lorri gc info
#[derive(Debug)]
pub struct GcRootInfo {
    /// directory where root is stored
    pub gc_dir: AbsPathBuf,
    /// nix file from which the root originates. If None, then the root is considered dead.
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
