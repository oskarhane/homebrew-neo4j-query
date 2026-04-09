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

#[test]
#[ignore]
fn empty_result_set() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("MATCH (n:DoesNotExist99999) RETURN n")
        .assert()
        .success()
        .stdout(predicate::str::contains("[0]"));
}

#[test]
#[ignore]
fn write_only_no_return() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("CREATE (n:WriteTest {v: 1})")
        .assert()
        .success();
    // cleanup
    cmd().arg("MATCH (n:WriteTest) DELETE n").assert().success();
}

#[test]
#[ignore]
fn return_null_values() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("RETURN null as x, 1 as y")
        .assert()
        .success()
        .stdout(predicate::str::contains("null"));
}

#[test]
#[ignore]
fn return_node_object() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("CREATE (n:NodeTest {name: 'test'}) RETURN n")
        .assert()
        .success()
        .stdout(predicate::str::contains("NodeTest"));
    cmd().arg("MATCH (n:NodeTest) DELETE n").assert().success();
}

#[test]
#[ignore]
fn wrong_password() {
    if !neo4j_available() {
        return;
    }
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env("NEO4J_URI", "http://localhost:7474");
    c.env("NEO4J_PASSWORD", "wrongpassword");
    c.arg("RETURN 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("401"));
}

#[test]
fn connection_refused() {
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env("NEO4J_URI", "http://localhost:19999")
        .env("NEO4J_PASSWORD", "x")
        .arg("RETURN 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
#[ignore]
fn param_negative_number() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .args(["-p", "x=-5", "RETURN $x as val"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-5"));
}

#[test]
#[ignore]
fn param_empty_value() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .args(["-p", "x=", "RETURN $x as val"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
#[ignore]
fn multiple_columns() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("RETURN 'a' as x, 'b' as y, 'c' as z")
        .assert()
        .success()
        .stdout(predicate::str::contains("x").and(predicate::str::contains("y")).and(predicate::str::contains("z")));
}

#[test]
#[ignore]
fn schema_command() {
    if !neo4j_available() {
        return;
    }
    // seed data
    cmd()
        .arg("CREATE (a:SchemaTest {x: 1})-[:SCHEMA_REL {y: 'hi'}]->(b:SchemaTarget {z: true})")
        .assert()
        .success();

    cmd()
        .arg(".schema")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("SchemaTest")
                .and(predicate::str::contains("SchemaTarget"))
                .and(predicate::str::contains("SCHEMA_REL"))
                .and(predicate::str::contains("nodes"))
                .and(predicate::str::contains("relationships"))
                .and(predicate::str::contains("properties")),
        );

    // cleanup
    cmd()
        .arg("MATCH (n:SchemaTest) DETACH DELETE n")
        .assert()
        .success();
    cmd()
        .arg("MATCH (n:SchemaTarget) DELETE n")
        .assert()
        .success();
}

#[test]
#[ignore]
fn multiple_rows() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("UNWIND range(1,3) AS i RETURN i")
        .assert()
        .success()
        .stdout(predicate::str::contains("1").and(predicate::str::contains("2")).and(predicate::str::contains("3")));
}
