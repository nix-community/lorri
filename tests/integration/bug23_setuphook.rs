use crate::direnvtestcase::{DirenvTestCase, DirenvValue};
use std::env;

#[tokio::test]
async fn bug23_shell_hook() {
    env::set_var("EXAMPLE", "my-neat-path");
    let mut testcase = DirenvTestCase::with_shell("bug23_setuphook");
    testcase
        .evaluate()
        .await
        .expect("Failed to build the first time");

    let env = testcase.get_direnv_variables().await;
    println!("{:?}", env);
    assert_eq!(
        env.get_env("EXAMPLE"),
        DirenvValue::Value("my-neat-path:/tmp/foo/bar")
    );
}
