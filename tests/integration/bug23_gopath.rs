use crate::direnvtestcase::{DirenvTestCase, DirenvValue};
use std::env;

#[tokio::test]
async fn bug23_gopath() {
    env::set_var("GOPATH", "my-neat-go-path");
    let mut testcase = DirenvTestCase::with_shell("bug23_gopath");
    testcase
        .evaluate()
        .await
        .expect("Failed to build the first time");

    let env = testcase.get_direnv_variables();
    assert_eq!(
        env.await.get_env("GOPATH"),
        DirenvValue::Value("my-neat-go-path:/tmp/foo/bar")
    );
}
