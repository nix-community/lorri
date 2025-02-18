use crate::direnvtestcase::{DirenvTestCase, DirenvValue};

#[tokio::test]
async fn trivial() -> std::io::Result<()> {
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

#[tokio::test]
async fn flake() -> std::io::Result<()> {
    let (testcase, res) = DirenvTestCase::with_flake_eval("basic-flake").await;

    assert!(
        res.all_exist(),
        "no build output (build-0) in {}.\nContents of {}\n{}",
        res.shell_gc_root.display(),
        testcase.cachedir.path().display(),
        std::str::from_utf8(
            &std::process::Command::new("ls")
                .args(["-Fla", "--recursive"])
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
