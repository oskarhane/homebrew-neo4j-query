use assert_cmd::Command;
use predicates::prelude::*;

fn neo4j_available() -> bool {
    std::env::var("NEO4J_TEST_URI").is_ok()
        || std::net::TcpStream::connect("127.0.0.1:7474").is_ok()
}

fn cmd() -> Command {
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env(
        "NEO4J_URI",
        std::env::var("NEO4J_TEST_URI").unwrap_or_else(|_| "http://localhost:7474".into()),
    );
    c.env("NEO4J_PASSWORD", "testpassword");
    c
}

#[test]
#[ignore] // requires running Neo4j
fn return_literal() {
    if !neo4j_available() {
        eprintln!("skipping: neo4j not available");
        return;
    }
    cmd()
        .arg("RETURN 1 as n")
        .assert()
        .success()
        .stdout(predicate::str::contains("n"));
}

#[test]
#[ignore]
fn create_and_match_node() {
    if !neo4j_available() {
        eprintln!("skipping: neo4j not available");
        return;
    }
    // create
    cmd()
        .arg("CREATE (t:TestInteg {val: 'hello'}) RETURN t.val as v")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));

    // match
    cmd()
        .arg("MATCH (t:TestInteg) RETURN t.val as v LIMIT 1")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));

    // cleanup
    cmd()
        .arg("MATCH (t:TestInteg) DELETE t")
        .assert()
        .success();
}

#[test]
#[ignore]
fn query_with_params() {
    if !neo4j_available() {
        eprintln!("skipping: neo4j not available");
        return;
    }
    cmd()
        .args(["-p", "x=42", "RETURN $x as val"])
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
#[ignore]
fn stdin_query() {
    if !neo4j_available() {
        eprintln!("skipping: neo4j not available");
        return;
    }
    cmd()
        .write_stdin("RETURN 'from_stdin' as s")
        .assert()
        .success()
        .stdout(predicate::str::contains("from_stdin"));
}

#[test]
fn missing_password_errors() {
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .arg("RETURN 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("password"));
}

#[test]
#[ignore]
fn bad_query_errors() {
    if !neo4j_available() {
        eprintln!("skipping: neo4j not available");
        return;
    }
    cmd()
        .arg("THIS IS NOT CYPHER")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}
