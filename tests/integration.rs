use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::io::Write;

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
    cmd().arg("MATCH (t:TestInteg) DELETE t").assert().success();
}

#[test]
#[ignore]
fn query_with_params() {
    if !neo4j_available() {
        eprintln!("skipping: neo4j not available");
        return;
    }
    cmd()
        .args(["-P", "x=42", "RETURN $x as val"])
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
    cmd().arg("CREATE (n:WriteTest {v: 1})").assert().success();
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
fn short_flag_u_for_username() {
    if !neo4j_available() {
        return;
    }
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env(
        "NEO4J_URI",
        std::env::var("NEO4J_TEST_URI").unwrap_or_else(|_| "http://localhost:7474".into()),
    );
    c.args(["-u", "neo4j", "-p", "testpassword", "RETURN 1 as n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
#[ignore]
fn short_flag_p_for_password() {
    if !neo4j_available() {
        return;
    }
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env(
        "NEO4J_URI",
        std::env::var("NEO4J_TEST_URI").unwrap_or_else(|_| "http://localhost:7474".into()),
    );
    c.args(["-p", "testpassword", "RETURN 1 as n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
#[ignore]
fn param_negative_number() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .args(["-P", "x=-5", "RETURN $x as val"])
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
        .args(["-P", "x=", "RETURN $x as val"])
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
        .stdout(
            predicate::str::contains("x")
                .and(predicate::str::contains("y"))
                .and(predicate::str::contains("z")),
        );
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

    cmd().arg(".schema").assert().success().stdout(
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
        .stdout(
            predicate::str::contains("1")
                .and(predicate::str::contains("2"))
                .and(predicate::str::contains("3")),
        );
}

#[test]
fn env_flag_nonexistent_file_errors() {
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .args(["--env", "/tmp/does_not_exist_neo4j.env", "RETURN 1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to load env file"));
}

#[test]
fn env_flag_loads_password() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "NEO4J_PASSWORD=fromenvfile").unwrap();

    // Should not fail with "password required" — will fail with connection error instead
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .env("NEO4J_URI", "http://localhost:19999")
        .args(["--env", tmp.path().to_str().unwrap(), "RETURN 1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn env_flag_equals_syntax() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "NEO4J_PASSWORD=fromenvfile").unwrap();

    let arg = format!("--env={}", tmp.path().to_str().unwrap());
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .env("NEO4J_URI", "http://localhost:19999")
        .args([&arg, "RETURN 1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
#[ignore]
fn env_flag_with_neo4j() {
    if !neo4j_available() {
        return;
    }
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "NEO4J_PASSWORD=testpassword").unwrap();
    writeln!(
        tmp,
        "NEO4J_URI={}",
        std::env::var("NEO4J_TEST_URI").unwrap_or_else(|_| "http://localhost:7474".into())
    )
    .unwrap();

    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .env_remove("NEO4J_URI")
        .args(["--env", tmp.path().to_str().unwrap(), "RETURN 1 as n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("n"));
}

#[test]
fn dotenv_auto_discovery() {
    // Create a .env in a temp dir, run the binary from that dir
    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join(".env");
    std::fs::write(&env_path, "NEO4J_PASSWORD=autodiscovered\n").unwrap();

    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .env("NEO4J_URI", "http://localhost:19999")
        .current_dir(dir.path())
        .arg("RETURN 1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

// --- JSON output format tests ---

#[test]
#[ignore]
fn json_return_literal() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args(["--format", "json", "RETURN 1 as n"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["n"], 1);
}

#[test]
#[ignore]
fn json_multi_column() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args([
            "--format",
            "json",
            "RETURN 'a' as x, 'b' as y, 'c' as z",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["x"], "a");
    assert_eq!(parsed[0]["y"], "b");
    assert_eq!(parsed[0]["z"], "c");
}

#[test]
#[ignore]
fn json_with_params() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args(["--format", "json", "-P", "x=42", "RETURN $x as val"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["val"], 42);
}

#[test]
#[ignore]
fn json_null_values() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args(["--format", "json", "RETURN null as x, 1 as y"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    assert!(parsed[0]["x"].is_null());
    assert_eq!(parsed[0]["y"], 1);
}

#[test]
#[ignore]
fn json_empty_result_set() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args([
            "--format",
            "json",
            "MATCH (n:DoesNotExist99999) RETURN n",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert!(parsed.is_empty());
}
