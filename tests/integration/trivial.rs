use crate::direnvtestcase::{DirenvTestCase, DirenvValue};

#[test]
fn trivial() -> std::io::Result<()> {
    let mut testcase = DirenvTestCase::with_shell("basic");
    let res = testcase.evaluate().expect("Failed to build the first time");

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
    assert_eq!(env.get_env("MARKER"), DirenvValue::Value("present"));
    Ok(())
}

#[test]
fn flake() -> std::io::Result<()> {
    let mut testcase = DirenvTestCase::with_flake("basic-flake");
    let res = testcase.evaluate().expect("Failed to build the first time");

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
    assert_eq!(env.get_env("MARKER"), DirenvValue::Value("present"));
    Ok(())
}
