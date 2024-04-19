use directories::ProjectDirs;

fn cargo_bin(name: &str) -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path.join(name)
        })
        .unwrap()
}

fn run_lorri<T: AsRef<std::ffi::OsStr>>(args: Vec<T>) -> std::io::Result<String> {
    let out = std::process::Command::new(cargo_bin("lorri"))
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()?;
    Ok(String::from_utf8(out.stdout).expect("non utf8 output"))
}

#[test]
fn gc() -> std::io::Result<()> {
    let testdir = tempfile::tempdir().expect("tempdirfailed");
    let project = testdir.path().join("project");
    std::fs::create_dir(&project).expect("mkdir project");
    let project = project.canonicalize().expect("canonicalize project");
    let nix_file = project.join("shell.nix");
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
        project.join("builder.sh"),
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
    std::env::set_current_dir(&project).expect("cd");
    // build the project
    run_lorri(vec!["shell", "--shell-file", "shell.nix"]).expect("running lorri shell");

    //look for the gc root
    let pd = ProjectDirs::from("com.github.nix-community.lorri", "lorri", "lorri")
        .expect("determining project directory");
    let gc_roots = pd
        .cache_dir()
        .canonicalize()
        .expect("canonicalize gc_root dir")
        .join("gc_roots");
    let mut subdirs = std::fs::read_dir(gc_roots)
        .expect("readdir")
        .collect::<Vec<_>>();
    assert_eq!(
        subdirs.len(),
        1,
        "{}!=1 gc roots were created",
        subdirs.len()
    );
    let subdir = subdirs.drain(..).next().unwrap().expect("direntry").path();
    let gc_root_dir = subdir.join("gc_root");
    let root = gc_root_dir.join("shell_gc_root");
    assert!(std::fs::read_link(root)
        .expect("readlink gc root")
        .starts_with("/nix/store"));

    let nix_file_symlink = gc_root_dir.join("nix_file");
    assert_eq!(
        std::fs::read_link(&nix_file_symlink).expect("readlink nix_file"),
        nix_file
    );

    // now run the gc, it should find the gc root
    let out = run_lorri(vec!["gc", "--json", "rm"]).unwrap();
    assert_eq!(out, "[]");
    // it should also not have removed it
    let out = run_lorri(vec!["gc", "info"]).unwrap();
    assert!(out.contains(&subdir.display().to_string()));
    // Now remove the project
    let backup_file = project.join("shell.nix.bak");
    std::fs::rename(&nix_file, &backup_file).expect("rename");
    assert!(std::fs::metadata(&nix_file_symlink).is_err());
    // it should be labeled as dead, but not removed by --print-roots
    let out = run_lorri(vec!["gc", "info"]).unwrap();
    assert!(&out.contains(&subdir.display().to_string()));
    assert!(out.contains("[dead]"));
    // now remove it
    let out = run_lorri(vec!["gc", "--json", "rm"]).unwrap();
    assert!(&out.contains(&subdir.display().to_string()));
    let out = run_lorri(vec!["gc", "--json", "rm"]).unwrap();
    assert_eq!(out, "[]");
    // rebuild the project
    std::fs::rename(&backup_file, &nix_file).expect("rename back");
    run_lorri(vec!["shell", "--shell-file", "shell.nix"]).expect("running lorri shell");
    // everything back to normal
    let out = run_lorri(vec!["gc", "info"]).unwrap();
    assert!(&out.contains(&subdir.display().to_string()));
    assert!(out.contains(&nix_file.display().to_string()));
    Ok(())
}
