use lorri::project;
use lorri::{
    builder, cas::ContentAddressable, nix::options::NixOptions, ops, project::Project, AbsPathBuf,
    NixFile,
};
use std::env;
use std::fs;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::process::Command;

fn cargo_bin(name: &str) -> PathBuf {
    env::current_exe()
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

#[test]
fn loads_env() {
    let tempdir = tempfile::tempdir().expect("tempfile::tempdir() failed us!");
    let project = project(
        "loads_env",
        &lorri::AbsPathBuf::new(tempdir.path().to_owned()).unwrap(),
    );

    // Launch as a real user
    let res = Command::new(cargo_bin("lorri"))
        .args([
            "shell",
            "--shell-file",
            project
                .nix_file
                .as_absolute_path()
                .as_os_str()
                .to_str()
                .unwrap(),
        ])
        .current_dir(&tempdir)
        .output()
        .expect("fail to run lorri shell");
    assert!(res.status.success(), "lorri shell command failed");

    let logger = lorri::logging::test_logger();

    let output = ops::bash_cmd(build(&project, &logger), &project.cas, &logger)
        .unwrap()
        .args(["-c", "echo $MY_ENV_VAR"])
        .output()
        .expect("failed to run shell");

    assert_eq!(
        // The string conversion means we get a nice assertion failure message in case stdout does
        // not match what we expected.
        String::from_utf8(output.stdout).expect("stdout not UTF-8 clean"),
        "my_env_value\n"
    );
}

fn project(name: &str, cache_dir: &AbsPathBuf) -> Project {
    let test_root = AbsPathBuf::new(PathBuf::from_iter(&[
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "shell",
        name,
    ]))
    .expect("CARGO_MANIFEST_DIR was not absolute");
    let cas_dir = cache_dir.join("cas").to_owned();
    fs::create_dir_all(&cas_dir).expect("failed to create CAS directory");
    Project::new(
        NixFile::from(test_root.join("shell.nix")),
        &cache_dir.join("gc_roots"),
        ContentAddressable::new(cas_dir).unwrap(),
    )
    .unwrap()
}

fn build(project: &Project, logger: &slog::Logger) -> PathBuf {
    project
        .create_roots(
            builder::run(
                &project.nix_file,
                &project.cas,
                &NixOptions::empty(),
                logger,
            )
            .unwrap()
            .result,
            project::NixGcRootUserDir::get_or_create(&project::Username::from_env_var().unwrap())
                .unwrap(),
            logger,
        )
        .unwrap()
        .shell_gc_root
        .0
        .as_path()
        .to_owned()
}
