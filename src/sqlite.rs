/// lorri data storage
use rusqlite as sqlite;


#[test]
fn migrate_db() {
    let conn = sqlite::Connection::open_in_memory().expect("cannot open sqlite db");
    conn.execute_batch(
        r#"CREATE TABLE foo(id INTEGER PRIMARY KEY, bla TEXT);
                    INSERT INTO foo (id, bla) VALUES (1, 'blabla');
                    INSERT INTO foo (id, bla) VALUES (2, 'bubatz');
"#,
    )
    .unwrap();
    let mut stmt = conn.prepare("SELECT * from foo;").unwrap();
    let res = stmt
        .query_map((), |row| row.get::<_, String>("bla"))
        .unwrap()
        .map(|x| x.unwrap())
        .collect::<Vec<String>>();

    assert_eq!(res, vec!["blabla", "bubatz"]);
}
