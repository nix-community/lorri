use crate::direnvtestcase::{DirenvTestCase, DirenvValue};
use lorri::lorri_runtime_block_on;
use std::iter::FromIterator;
use std::path::PathBuf;

#[test]
fn in_lorri_shell() {
    lorri_runtime_block_on(async {
        let (testcase, _build) = DirenvTestCase::with_shell_eval("basic").await;

        let env = testcase.get_direnv_variables();
        let shell =
            PathBuf::from_iter(&[env!("CARGO_MANIFEST_DIR"), "tests", "integration", "basic"])
                .join("shell.nix");

        assert_eq!(
            env.await.get_env("IN_LORRI_SHELL"),
            DirenvValue::Value(shell.to_str().unwrap())
        );
    })
}
