//! Handling of nix GC roots
//!
//! TODO: inline this module into `::project`
use crate::builder::{OutputPath, RootedPath};
use crate::project::Project;
use crate::AbsPathBuf;
use slog_scope::debug;
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;

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

    /// Return the filesystem path for this root.
    pub fn paths(&self) -> OutputPath<RootPath> {
        OutputPath {
            shell_gc_root: RootPath(self.gc_root_path.join("shell_gc_root")),
        }
    }

    /// Create roots to store paths.
    ///
    /// A lorri root is an *indirect gc root*.
    /// An indirect gc root in nix terms is a gc root controlled by the user,
    /// with a backlink from the nix store pointing to the user controlled link.
    ///
    /// This is the same mechanism that ensures `./result` symlinks created
    /// by `nix-build` are not deleted by a GC run until the symlink is deleted.
    ///
    /// In practice that means that we need a two-step setup,
    /// once we have the store path to gc_root:
    ///
    /// 1) We create a symlink from `~/.cache/lorri/<foo>` pointing to `/nix/store/…`
    /// 2) We create a backlink from `/nix/var/nix/gcroots/per-user/<user>/<bar>`
    ///    pointing to `~/.cache/lorri/<foo>`.
    ///
    /// The directory in `/nix/var/nix/gcroots/per-user/<user>` is set up
    /// user-writable by nix, so we can create new backlinks there.
    ///
    /// That way lorri keeps all its information in `~/.cache/lorri`
    /// and can control how long a gc root should exist, and if we overwrite
    /// the store path symlink with a new one, the old backlink is freed
    /// to be GCed by nix.
    ///
    /// TODO: passing the store path to this function means there is a
    /// race condition between the nix build finishing and us creating the gc root,
    /// during which the store path might be GCed.
    /// Can we use nix’s `--indirect` instead?
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
        //
        // example:
        //
        // ```
        // < home dir > <  cache   > <   gc roots for this project/nix file          > < root symlink >
        // /home/myuser/.cache/lorri/gc_roots/1862b3eddd2ef1a0958d7630dd7a37c5/gc_root/shell_gc_root
        // ```
        let path = self.gc_root_path.join(root_name);

        debug!("adding root"; "from" => store_path.as_path().to_str(), "to" => path.display());

        // TODO: this leads to a short period where the gc root does not exist,
        // which might be responsible for some of the crashes we’ve been seeing.
        // Use a lockfile in the directory and atomic rename via rust-atomicwrites.
        std::fs::remove_file(&path)
            .or_else(|e| AddRootError::remove(e, &path.as_absolute_path()))?;

        // the forward GC root that points from our cache gc_roots dir to the store path
        std::os::unix::fs::symlink(store_path.as_path(), &path)
            .map_err(|e| AddRootError::symlink(e, store_path.as_path(), path.as_absolute_path()))?;

        // the backlink GC root that points from nix to our cache gc_roots dir
        //
        // Example:
        // /nix/var/nix/gcroots/per-user/<user>/e68fc0e54c6a8d9b92814b371da289ed-shell_gc_root
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
        // TODO: do at the start of lorri
        if !root.is_dir() {
            std::fs::create_dir_all(&root).map_err(|source| AddRootError {
                source,
                msg: format!("Failed to recursively create directory {}", root.display()),
            })?
        }

        root.push(format!("{}-{}", self.id, root_name));

        // TODO: Same as above, there is a short while when this file is missing
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
#[derive(Error, Debug)]
#[error("{msg}: {source}")]
pub struct AddRootError {
    #[source]
    source: std::io::Error,
    msg: String,
}

impl AddRootError {
    /// Ignore NotFound errors (it is after all a remove), and otherwise
    /// return an error explaining a delete on path failed.
    fn remove(source: std::io::Error, path: &Path) -> Result<(), AddRootError> {
        if source.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(AddRootError {
                source,
                msg: format!("Failed to delete {}", path.display()),
            })
        }
    }

    /// Return an error explaining what symlink failed
    fn symlink(source: std::io::Error, src: &Path, dest: &Path) -> AddRootError {
        AddRootError {
            source,
            msg: format!("Failed to symlink {} to {}", src.display(), dest.display()),
        }
    }
}
