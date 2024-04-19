//! Wrap a nix file and manage corresponding state.

use thiserror::Error;

use crate::builder::{OutputPath, RootedPath};
use crate::nix::StorePath;
use crate::{AbsPathBuf, Installable, NixFile};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A “project” knows how to handle the lorri state
/// for a given nix file.
#[derive(Clone)]
pub struct Project {
    /// Absolute path to this project’s nix file.
    pub project_file: ProjectFile,

    /// Directory in which this project’s
    /// garbage collection roots are stored.
    gc_root_path: AbsPathBuf,

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
        let nix_file_symlink = project.gc_root_path.join("nix_file");

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
        let project_gc_root = gc_root_dir.join(&hash).join("gc_root");

        std::fs::create_dir_all(&project_gc_root)?;

        Ok(Project {
            project_file,
            gc_root_path: project_gc_root,
            hash,
        })
    }

    /// Generate a "unique" ID for this project based on its absolute path.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    // final path in the `self.gc_root_path` directory,
    // the symlink which points to the lorri-keep-env-hack-nix-shell drv (see ./logged-evaluation.nix)
    fn gc_root(&self, base: &PathBuf) -> AbsPathBuf {
        self.gc_root_path.join(base)
    }

    /// Return the filesystem paths for these roots.
    pub fn root_paths(&self) -> OutputPath {
        OutputPath {
            shell_gc_root: RootPath(self.gc_root(&Self::ENV_CONTEXT.into())),
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

        Ok(OutputPath {
            shell_gc_root: RootPath(self.gc_root(&base_name)),
        })
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

impl OutputPath {
    /// Check whether all all GC roots exist.
    pub fn all_exist(&self) -> bool {
        let crate::builder::OutputPath { shell_gc_root } = self;

        shell_gc_root.0.as_path().exists()
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
