use crate::direnvtestcase::{DirenvTestCase, DirenvValue};
use std::time::{Duration, Instant};

#[tokio::test]
async fn not_so_slow() {
    let (testcase, _build) = DirenvTestCase::with_shell_eval("bug110_duplicate_appends").await;

    let start = Instant::now();
    let env = testcase.get_direnv_variables().await;
    println!("direnv time: {:?}", start.elapsed());
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "direnv export should be under 2 seconds (even on CI)"
    );
    let itworked = env.get_env("ITWORKED");
    assert!(
        match itworked {
            DirenvValue::Value(v) => v.ends_with("foo/bar"),
            _ => false,
        },
        "ITWORKED shoud end with 'foo/bar', but is {:?}",
        itworked
    )
}
