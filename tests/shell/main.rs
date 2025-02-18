use lorri::project::ProjectFile;
use lorri::{
    builder, cas::ContentAddressable, nix::options::NixOptions, ops, project::Project, AbsPathBuf,
    NixFile,
};
use std::env;
use std::iter::FromIterator;
use std::path::PathBuf;
use tokio::process::Command;

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

// This test fails because Command does not provide an interactive TTY, so `lorri shell`
// can’t be run. I’d argue it’s because this test is crap.
// #[tokio::test]
// async fn loads_env() {
//     let tempdir = tempfile::tempdir().expect("tempfile::tempdir() failed us!");
//     let project = project(
//         "loads_env",
//         &AbsPathBuf::new(tempdir.path().to_owned()).unwrap(),
//     )
//     .await;
//
//     // Launch as a real user
//     let res = Command::new(cargo_bin("lorri"))
//         .args([
//             "shell",
//             "--shell-file",
//             project
//                 .file
//                 .as_absolute_path()
//                 .as_os_str()
//                 .to_str()
//                 .unwrap(),
//         ])
//         .current_dir(&tempdir)
//         .output()
//         .await
//         .expect("fail to run lorri shell");
//     assert!(res.status.success(), "lorri shell command failed: {:?}", String::from_utf8_lossy(&res.stderr));
//
//     let logger = lorri::logging::test_logger("loads_env");
//
//     let output = ops::bash_cmd(build(&project, &logger).await, &project.cas, &logger)
//         .await
//         .unwrap()
//         .args(["-c", "echo $MY_ENV_VAR"])
//         .output()
//         .expect("failed to run shell");
//
//     assert_eq!(
//         // The string conversion means we get a nice assertion failure message in case stdout does
//         // not match what we expected.
//         String::from_utf8(output.stdout).expect("stdout not UTF-8 clean"),
//         "my_env_value\n"
//     );
// }

async fn project(name: &str, cache_dir: &AbsPathBuf) -> Project {
    let test_root = AbsPathBuf::new(PathBuf::from_iter(&[
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "shell",
        name,
    ]))
    .expect("CARGO_MANIFEST_DIR was not absolute");
    let cas_dir = cache_dir.join("cas").to_owned();
    tokio::fs::create_dir_all(&cas_dir)
        .await
        .expect("failed to create CAS directory");
    let nixfile = NixFile::from(test_root.join("shell.nix"));
    let project_file = ProjectFile::ShellNix(nixfile);
    Project::new(
        project_file,
        &cache_dir.join("gc_roots"),
        ContentAddressable::new(cas_dir).unwrap(),
    )
    .unwrap()
}

async fn build(project: &Project, logger: &slog::Logger) -> PathBuf {
    project
        .create_roots(
            builder::run(
                &project.file.as_nix_file(),
                &project.cas,
                &NixOptions::empty(),
                logger,
            )
            .await
            .unwrap()
            .result,
        )
        .unwrap()
        .shell_gc_root
        .0
        .as_path()
        .to_owned()
}
