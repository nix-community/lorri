use crate::direnvtestcase::{DirenvTestCase, DirenvValue};
use std::env;

#[tokio::test]
async fn bug23_shell_hook() {
    env::set_var("EXAMPLE", "my-neat-path");
    let (testcase, _build) = DirenvTestCase::with_shell_eval("bug23_setuphook").await;

    let env = testcase.get_direnv_variables().await;
    println!("{:?}", env);
    assert_eq!(
        env.get_env("EXAMPLE"),
        DirenvValue::Value("my-neat-path:/tmp/foo/bar")
    );
}
