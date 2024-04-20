use directories::ProjectDirs;
use lorri::cas::ContentAddressable;
use lorri::ops::{
    gc_find_roots_to_remove, gc_remove_roots, get_paths, main_run_once,
    write_gc_info_human_readable, write_gc_info_json, write_gc_rm_json,
};
use lorri::project::{list_roots_gc, ListRootsSort, Project, ProjectFile};
use lorri::sqlite::Sqlite;
use lorri::{lorri_runtime_block_on, AbsPathBuf, NixFile};
use std::collections::HashSet;
use std::io::Cursor;

#[test]
fn gc() -> std::io::Result<()> {
    lorri_runtime_block_on(async {
        let testdir = tempfile::tempdir().expect("tempdirfailed");
        let project_dir = testdir.path().join("project");
        std::fs::create_dir(&project_dir).expect("mkdir project");
        let nix_file = project_dir.join("shell.nix");
        std::fs::write(
            &nix_file,
            r#"
derivation {
  name = "bogus";
  builder = ./builder.sh;
  system = builtins.currentSystem;
  MARKER = "foo";
}
    "#,
        )
        .expect("writing shell.nix");
        std::fs::write(
            project_dir.join("builder.sh"),
            r#"
#!/bin/sh
    "#,
        )
        .expect("writing builder.sh");

        let home = testdir.path().join("home");
        std::fs::create_dir(&home).expect("mkdir home");
        let home = home.canonicalize().expect("canonicalize home");
        std::env::set_var("HOME", home);
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::set_current_dir(&project_dir).expect("cd");
        let project_dirs = ProjectDirs::from("com.github.nix-community.lorri", "lorri", "lorri")
            .expect("determining project directory");
        let cache_dir = project_dirs.cache_dir();
        std::fs::create_dir_all(cache_dir).unwrap();
        let conn =
            Sqlite::new_connection(&AbsPathBuf::new(cache_dir.join("db.sqlite")).unwrap()).await;
        let gc_roots = AbsPathBuf::new(cache_dir.join("gc_roots")).unwrap();

        let cas =
            ContentAddressable::new(AbsPathBuf::new(cache_dir.join("cas")).unwrap()).expect("cas");
        let logger = lorri::logging::test_logger("gc");

        // we have to re-create the Project because it sets up the GC dir correctly
        {
            let project = Project::new_and_gc_nix_files(
                conn.clone(),
                ProjectFile::ShellNix(NixFile::from(AbsPathBuf::new(nix_file.clone()).unwrap())),
                &gc_roots,
            )
            .await
            .expect("project");
            // build the project
            main_run_once(project, &cas, &logger)
                .await
                .expect("failed running gc build");
        }
        let mut subdirs = std::fs::read_dir(gc_roots.clone())
            .expect("readdir")
            .collect::<Vec<_>>();
        assert_eq!(
            subdirs.len(),
            1,
            "{}!=1 gc roots were created",
            subdirs.len()
        );
        let subdir = subdirs.pop().expect("direntry").expect("direntry").path();
        drop(subdirs);
        let gc_root_dir = subdir.join("gc_root");

        {
            let root = gc_root_dir.join("shell_gc_root");
            assert!(std::fs::read_link(root)
                .expect("readlink gc root")
                .starts_with("/nix/store"));
        }

        let nix_file_symlink = gc_root_dir.join("nix_file");
        assert_eq!(
            std::fs::read_link(&nix_file_symlink).expect("readlink nix_file"),
            nix_file
        );
        let paths = get_paths().unwrap();

        {
            // The default GC without any options should not have anything to remove,
            // because the original nix file still exists at this point.
            let roots = list_roots_gc(&logger, &paths, conn.clone(), ListRootsSort::NoSorting)
                .expect("cannot list roots");
            let to_remove = gc_find_roots_to_remove(false, None, HashSet::new(), roots);
            assert!(to_remove.len() < 1, "to_remove longer than 1 {to_remove:?}");
        }

        {
            // Meaning the root should still be listed in the call to info
            let roots = list_roots_gc(&logger, &paths, conn.clone(), ListRootsSort::NoSorting)
                .expect("cannot list roots");
            let mut buf = Cursor::new(vec![]);
            write_gc_info_json(&roots, &mut buf).expect("gc info");
            let out = String::from_utf8_lossy(&buf.into_inner()).into_owned();
            assert!(out.contains(&subdir.display().to_string()));
        }

        // Now remove the project nix file by renaming it
        let backup_file = project_dir.join("shell.nix.bak");
        {
            std::fs::rename(&nix_file, &backup_file).expect("rename");
            assert!(std::fs::metadata(&nix_file_symlink).is_err());
        }

        {
            // it should be labeled as gone, but not removed by --print-roots
            let roots = list_roots_gc(&logger, &paths, conn.clone(), ListRootsSort::NoSorting)
                .expect("cannot list roots");
            let mut buf = Cursor::new(vec![]);
            write_gc_info_human_readable(&roots, &mut buf);
            let out = String::from_utf8_lossy(&buf.into_inner()).into_owned();
            assert!(out.contains(&subdir.display().to_string()));
            assert!(out.contains("[gone]"));
        }

        {
            // Now run the GC to remove the root
            let roots = list_roots_gc(&logger, &paths, conn.clone(), ListRootsSort::NoSorting)
                .expect("cannot list roots");
            let to_remove = gc_find_roots_to_remove(false, None, HashSet::new(), roots);
            assert_eq!(to_remove.len(), 1, "to_remove not 1 {to_remove:?}");
            let mut buf = Cursor::new(vec![]);
            let res = gc_remove_roots(to_remove).await;
            write_gc_rm_json(&res, &mut buf);
            let out = String::from_utf8_lossy(&buf.into_inner()).into_owned();
            assert!(&out.contains(&subdir.display().to_string()));
        }

        {
            // run it again and make sure there is no more root to remove now
            let roots = list_roots_gc(&logger, &paths, conn.clone(), ListRootsSort::NoSorting)
                .expect("cannot list roots");
            let to_remove = gc_find_roots_to_remove(false, None, HashSet::new(), roots);
            assert_eq!(to_remove.len(), 0, "to_remove not 0: {to_remove:?}");
        }

        {
            // rename file back and rebuild the project
            std::fs::rename(&backup_file, &nix_file).expect("rename back");
            // build the project
            let project = Project::new_and_gc_nix_files(
                conn.clone(),
                ProjectFile::ShellNix(NixFile::from(AbsPathBuf::new(nix_file.clone()).unwrap())),
                &gc_roots,
            )
            .await
            .expect("project");
            main_run_once(project, &cas, &logger)
                .await
                .expect("failed running gc build");
        }

        {
            // everything back to normal
            let roots = list_roots_gc(&logger, &paths, conn.clone(), ListRootsSort::NoSorting)
                .expect("cannot list roots");
            let mut buf = Cursor::new(vec![]);
            write_gc_info_json(&roots, &mut buf).expect("gc info");
            let out = String::from_utf8_lossy(&buf.into_inner()).into_owned();
            assert!(out.contains(&subdir.display().to_string()));
            assert!(out.contains(&nix_file.display().to_string()));
        }

        Ok(())
    })
}
