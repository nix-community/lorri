//! Handling of nix GC roots
//!
//! TODO: inline this module into `::project`
use crate::builder::{OutputPath, RootedPath};
use crate::project::Project;
use crate::AbsPathBuf;
use slog_scope::debug;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

/// Roots manipulation
#[derive(Clone)]
pub struct Roots {
    /// The GC root directory in the lorri user cache dir
    gc_root_path: AbsPathBuf,
    id: String,
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

impl OutputPath<RootPath> {
    /// Check whether all all GC roots exist.
    pub fn all_exist(&self) -> bool {
        let crate::builder::OutputPath { shell_gc_root } = self;

        shell_gc_root.0.as_absolute_path().exists()
    }
}

impl Roots {
    // TODO: all use-cases are from_project; just save a reference to a project?
    /// Construct a Roots struct based on a project's GC root directory
    /// and ID.
    pub fn from_project(project: &Project) -> Roots {
        Roots {
            gc_root_path: project.gc_root_path.clone(),
            id: project.hash().to_string(),
        }
    }

    /// Return the filesystem paths for these roots.
    pub fn paths(&self) -> OutputPath<RootPath> {
        OutputPath {
            shell_gc_root: RootPath(self.gc_root_path.join("shell_gc_root")),
        }
    }

    /// Create roots to store paths.
    pub fn create_roots(
        &self,
        // Important: this intentionally only allows creating
        // roots to `StorePath`, not to `DrvFile`, because we have
        // no use case for creating GC roots for drv files.
        path: RootedPath,
    ) -> Result<OutputPath<RootPath>, AddRootError>
where {
        let root_name = "shell_gc_root";
        let store_path = &path.path;

        // final path in the `self.gc_root_path` directory
        let path = self.gc_root_path.join(root_name);

        debug!("adding root"; "from" => store_path.as_path().to_str(), "to" => path.display());
        std::fs::remove_file(&path)
            .or_else(|e| AddRootError::remove(e, &path.as_absolute_path()))?;

        // the forward GC root that points from the store path to our cache gc_roots dir
        std::os::unix::fs::symlink(store_path.as_path(), &path)
            .map_err(|e| AddRootError::symlink(e, store_path.as_path(), path.as_absolute_path()))?;

        // the reverse GC root that points from nix to our cache gc_roots dir
        let mut root = if let Ok(path) = env::var("NIX_STATE_DIR") {
            PathBuf::from(path)
        } else {
            PathBuf::from("/nix/var/nix/")
        };
        root.push("gcroots");
        root.push("per-user");

        // TODO: check on start of lorri
        root.push(env::var("USER").expect("env var 'USER' must be set"));

        // The user directory sometimes doesn’t exist,
        // but we can create it (it’s root but `rwxrwxrwx`)
        if !root.is_dir() {
            std::fs::create_dir_all(&root).map_err(|e| AddRootError::create_dir_all(e, &root))?;
        }

        root.push(format!("{}-{}", self.id, root_name));

        debug!("connecting root"; "from" => path.display(), "to" => root.to_str());
        std::fs::remove_file(&root).or_else(|e| AddRootError::remove(e, &root))?;

        std::os::unix::fs::symlink(&path, &root)
            .map_err(|e| AddRootError::symlink(e, path.as_absolute_path(), &root))?;

        // TODO: don’t return the RootPath here
        Ok(OutputPath {
            shell_gc_root: RootPath(path),
        })
    }
}

/// Error conditions encountered when adding roots
#[derive(Debug)]
pub enum AddRootError {
    /// IO-related errors
    Io(std::io::Error, String),
}

impl AddRootError {
    /// Create a contextualized error around failing to create a directory
    fn create_dir_all(err: std::io::Error, path: &Path) -> AddRootError {
        AddRootError::Io(
            err,
            format!("Failed to recursively create directory {}", path.display()),
        )
    }

    /// Ignore NotFound errors (it is after all a remove), and otherwise
    /// return an error explaining a delete on path failed.
    fn remove(err: std::io::Error, path: &Path) -> Result<(), AddRootError> {
        if err.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(AddRootError::Io(
                err,
                format!("Failed to delete {}", path.display()),
            ))
        }
    }

    /// Return an error explaining what symlink failed
    fn symlink(err: std::io::Error, src: &Path, dest: &Path) -> AddRootError {
        AddRootError::Io(
            err,
            format!("Failed to symlink {} to {}", src.display(), dest.display()),
        )
    }
}

impl fmt::Display for AddRootError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddRootError::Io(e, msg) => write!(f, "{}: {}", msg, e),
        }
    }
}
