use crate::direnvtestcase::{DirenvTestCase, DirenvValue};

#[tokio::test]
async fn trivial_old_style() -> std::io::Result<()> {
    let (testcase, res) = DirenvTestCase::with_shell_eval("basic").await;

    assert!(
        res.all_exist(),
        "no build output (build-0) in {}.\nContents of {}\n{}",
        res.shell_gc_root.display(),
        testcase.cachedir.path().display(),
        std::str::from_utf8(
            &std::process::Command::new("ls")
                .args(["-la", "--recursive"])
                .args([testcase.cachedir.path().as_os_str()])
                .output()?
                .stdout
        )
        .unwrap()
    );

    let env = testcase.get_direnv_variables();
    assert_eq!(env.await.get_env("MARKER"), DirenvValue::Value("present"));
    Ok(())
}

// TODO: flaky on Ubuntu CI and failing on MacOS
//
// Ubuntu CI error:
// # Text("       … while fetching the input 'github:numtide/flake-utils/d465f4819400de7c8d874d50b982301f28a84605?narHash=sha256-q6EQdSeUZOG26WelxqkmR7kArjgWCdw5sfJVHPH/7j8%3D'")
// # Text("")
// # Text("       error: creating directory '/tmp/.tmptFAoyK/home/.cache/nix/nix-10902-0': No such file or directory")
// # Text("download thread shutting down")
//
// This needs some better setup before we can enable it again.
//
// #[tokio::test]
// async fn trivial_flake() -> std::io::Result<()> {
//     let (testcase, res) = DirenvTestCase::with_flake_eval("basic-flake").await;
//
//     assert!(
//         res.all_exist(),
//         "no build output (build-0) in {}.\nContents of {}\n{}",
//         res.shell_gc_root.display(),
//         testcase.cachedir.path().display(),
//         std::str::from_utf8(
//             &std::process::Command::new("ls")
//                 .args(["-Fla", "--recursive"])
//                 .args([testcase.cachedir.path().as_os_str()])
//                 .output()?
//                 .stdout
//         )
//         .unwrap()
//     );
//
//     let env = testcase.get_direnv_variables();
//     assert_eq!(env.await.get_env("MARKER"), DirenvValue::Value("present"));
//     Ok(())
// }
