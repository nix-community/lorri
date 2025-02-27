use crate::direnvtestcase::{DirenvTestCase, DirenvValue};
use lorri::lorri_runtime_block_on;
use std::env;

#[test]
fn bug23_gopath() {
    lorri_runtime_block_on(async {
        env::set_var("GOPATH", "my-neat-go-path");
        let (testcase, _build) = DirenvTestCase::with_shell_eval("bug23_gopath").await;

        let env = testcase.get_direnv_variables();
        assert_eq!(
            env.await.get_env("GOPATH"),
            DirenvValue::Value("my-neat-go-path:/tmp/foo/bar")
        );
    })
}
