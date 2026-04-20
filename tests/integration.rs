use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::io::Write;

/// Command pointing at a dead port with no password env var.
/// Tests using this helper verify clap flag parsing without a live Neo4j.
fn cmd_no_neo4j() -> Command {
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env_remove("NEO4J_PASSWORD");
    c.env("NEO4J_URI", "http://localhost:19999");
    c
}

fn neo4j_available() -> bool {
    std::env::var("NEO4J_TEST_URI").is_ok()
        || std::net::TcpStream::connect("127.0.0.1:7474").is_ok()
}

fn ollama_available() -> bool {
    std::env::var("OLLAMA_TEST_URL").is_ok()
        || std::net::TcpStream::connect("127.0.0.1:11434").is_ok()
}

/// Base command wired for embed subcommand tests (no Neo4j needed).
/// Uses Ollama on default localhost port unless OLLAMA_TEST_URL overrides.
fn embed_cmd() -> Command {
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env_remove("NEO4J_PASSWORD");
    c.env("NEO4J_EMBED_PROVIDER", "ollama");
    c.env("NEO4J_EMBED_MODEL", "all-minilm");
    c.env(
        "NEO4J_EMBED_BASE_URL",
        std::env::var("OLLAMA_TEST_URL").unwrap_or_else(|_| "http://localhost:11434".into()),
    );
    c
}

/// Base command wired for query-mode tests that also need embeddings.
fn cmd_with_embed() -> Command {
    let mut c = cmd();
    c.env("NEO4J_EMBED_PROVIDER", "ollama");
    c.env("NEO4J_EMBED_MODEL", "all-minilm");
    c.env(
        "NEO4J_EMBED_BASE_URL",
        std::env::var("OLLAMA_TEST_URL").unwrap_or_else(|_| "http://localhost:11434".into()),
    );
    c
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

    // seed constraint + index
    cmd()
        .arg("CREATE CONSTRAINT schema_test_unique IF NOT EXISTS FOR (n:SchemaTest) REQUIRE n.x IS UNIQUE")
        .assert()
        .success();
    cmd()
        .arg("CREATE INDEX schema_test_z_idx IF NOT EXISTS FOR (n:SchemaTarget) ON (n.z)")
        .assert()
        .success();

    cmd().arg("schema").assert().success().stdout(
        predicate::str::contains("SchemaTest")
            .and(predicate::str::contains("SchemaTarget"))
            .and(predicate::str::contains("SCHEMA_REL"))
            .and(predicate::str::contains("nodes"))
            .and(predicate::str::contains("relationships"))
            .and(predicate::str::contains("properties"))
            .and(predicate::str::contains("indexes"))
            .and(predicate::str::contains("constraints"))
            .and(predicate::str::contains("schema_test_unique"))
            .and(predicate::str::contains("schema_test_z_idx"))
            .and(predicate::str::contains("RANGE"))
            .and(predicate::str::contains("UNIQUE"))
            .and(predicate::str::contains("database"))
            .and(predicate::str::contains("neo4jVersion"))
            .and(predicate::str::contains("edition"))
            .and(predicate::str::contains("defaultCypherVersion")),
    );

    // cleanup
    cmd()
        .arg("DROP CONSTRAINT schema_test_unique IF EXISTS")
        .assert()
        .success();
    cmd()
        .arg("DROP INDEX schema_test_z_idx IF EXISTS")
        .assert()
        .success();
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
fn subcommand_with_flags_before_it() {
    // Regression: flags before a subcommand should not cause clap to
    // treat the subcommand name as a positional query argument.
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "NEO4J_PASSWORD=x").unwrap();

    // schema subcommand should be recognized even with --env before it
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .env("NEO4J_URI", "http://localhost:19999")
        .args(["--env", tmp.path().to_str().unwrap(), "schema"])
        .assert()
        .failure()
        // Should fail with connection error, NOT a cypher syntax error
        .stderr(predicate::str::contains("SyntaxError").not());
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
        .args(["--format", "json", "RETURN 'a' as x, 'b' as y, 'c' as z"])
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
        .args(["--format", "json", "MATCH (n:DoesNotExist99999) RETURN n"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert!(parsed.is_empty());
}

// --- Schema flag-position tests (no Neo4j required) ---

#[test]
fn schema_short_password_before() {
    cmd_no_neo4j()
        .args(["-p", "secret", "schema"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_short_user_password_before() {
    cmd_no_neo4j()
        .args(["-u", "neo4j", "-p", "secret", "schema"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_short_password_after() {
    cmd_no_neo4j()
        .args(["schema", "-p", "secret"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_multiple_flags_after() {
    cmd_no_neo4j()
        .args(["schema", "-u", "neo4j", "-p", "secret"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_flags_split_around() {
    cmd_no_neo4j()
        .args(["-u", "neo4j", "schema", "-p", "secret"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_long_password_before() {
    cmd_no_neo4j()
        .args(["--password", "secret", "schema"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_long_password_after() {
    cmd_no_neo4j()
        .args(["schema", "--password", "secret"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_db_flag() {
    cmd_no_neo4j()
        .args(["-p", "secret", "--db", "mydb", "schema"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn schema_env_flag_after() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "NEO4J_PASSWORD=secret").unwrap();

    cmd_no_neo4j()
        .args(["schema", "--env", tmp.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

// --- Regression tests: query-mode and skill subcommand ---

#[test]
fn query_mode_password_flag_before_cypher() {
    cmd_no_neo4j()
        .args(["-p", "secret", "RETURN 1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn query_mode_format_and_password_mixed() {
    cmd_no_neo4j()
        .args(["--format", "json", "-p", "secret", "RETURN 1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password").not());
}

#[test]
fn skill_list_without_password() {
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .args(["skill", "list"])
        .assert()
        .success();
}

#[test]
#[ignore]
fn schema_wrong_password() {
    if !neo4j_available() {
        return;
    }
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env(
        "NEO4J_URI",
        std::env::var("NEO4J_TEST_URI").unwrap_or_else(|_| "http://localhost:7474".into()),
    );
    c.env("NEO4J_PASSWORD", "wrongpassword");
    c.arg("schema")
        .assert()
        .failure()
        .stderr(predicate::str::contains("401"));
}

#[test]
#[ignore]
fn schema_correct_password() {
    if !neo4j_available() {
        return;
    }
    cmd().arg("schema").assert().success();
}

#[test]
#[ignore]
fn query_correct_password() {
    if !neo4j_available() {
        return;
    }
    cmd()
        .arg("RETURN 1 AS n")
        .assert()
        .success()
        .stdout(predicate::str::contains("n"));
}

// --- Truncate large arrays integration tests ---

#[test]
#[ignore]
fn truncate_toon_large_array() {
    if !neo4j_available() {
        return;
    }
    // range(0, 150) produces 151 items, exceeding default threshold of 100
    cmd()
        .args([
            "--truncate-arrays-over",
            "50",
            "RETURN range(0, 150) AS arr",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[array truncated: 151 items]"));
}

#[test]
#[ignore]
fn truncate_json_large_array() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args([
            "--format",
            "json",
            "--truncate-arrays-over",
            "50",
            "RETURN range(0, 150) AS arr",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    // Truncated array should be empty array
    assert_eq!(parsed[0]["arr"], Value::Array(vec![]));
}

#[test]
#[ignore]
fn truncate_disabled_passes_full_array() {
    if !neo4j_available() {
        return;
    }
    let output = cmd()
        .args([
            "--format",
            "json",
            "--truncate-arrays-over",
            "0",
            "RETURN range(0, 150) AS arr",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    // With truncation disabled, full array should be present
    let arr = parsed[0]["arr"].as_array().expect("arr should be array");
    assert_eq!(arr.len(), 151);
}

#[test]
fn schema_without_password_errors() {
    Command::cargo_bin("neo4j-query")
        .unwrap()
        .env_remove("NEO4J_PASSWORD")
        .env("NEO4J_URI", "http://localhost:19999")
        .arg("schema")
        .assert()
        .failure()
        .stderr(predicate::str::contains("password"));
}

// --- Embedding integration tests (require Ollama with all-minilm pulled) ---

/// all-minilm (sentence-transformers/all-MiniLM-L6-v2) returns 384-dim vectors.
const ALL_MINILM_DIMS: usize = 384;

#[test]
#[ignore]
fn embed_subcommand_json_output() {
    if !ollama_available() {
        eprintln!("skipping: ollama not available");
        return;
    }
    let output = embed_cmd().args(["embed", "hello"]).output().unwrap();
    assert!(
        output.status.success(),
        "embed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<f32> = serde_json::from_str(stdout.trim()).expect("valid JSON array of floats");
    assert_eq!(parsed.len(), ALL_MINILM_DIMS);
}

#[test]
#[ignore]
fn embed_subcommand_stdin() {
    if !ollama_available() {
        eprintln!("skipping: ollama not available");
        return;
    }
    let output = embed_cmd()
        .arg("embed")
        .write_stdin("hello")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "embed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<f32> = serde_json::from_str(stdout.trim()).expect("valid JSON array of floats");
    assert_eq!(parsed.len(), ALL_MINILM_DIMS);
}

#[test]
#[ignore]
fn embed_subcommand_raw_format_line_count() {
    if !ollama_available() {
        eprintln!("skipping: ollama not available");
        return;
    }
    let output = embed_cmd()
        .args(["embed", "--format", "raw", "hello"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "embed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    // One float per line. Trim a potential trailing newline then count.
    let line_count = stdout.trim_end_matches('\n').lines().count();
    assert_eq!(line_count, ALL_MINILM_DIMS);
}

#[test]
#[ignore]
fn query_mode_embed_param_roundtrip() {
    if !neo4j_available() || !ollama_available() {
        eprintln!("skipping: neo4j or ollama not available");
        return;
    }
    // $v is the embedding vector; SIZE($v) returns its length (= 384 for all-minilm).
    let output = cmd_with_embed()
        .args([
            "--format",
            "json",
            "-P",
            "v:embed=hello",
            "RETURN size($v) AS n",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["n"], ALL_MINILM_DIMS);
}

#[test]
#[ignore]
fn query_mode_mixed_literal_and_embed_params() {
    if !neo4j_available() || !ollama_available() {
        eprintln!("skipping: neo4j or ollama not available");
        return;
    }
    // x stays an integer (literal path), v is an embedding vector (embed path).
    let output = cmd_with_embed()
        .args([
            "--format",
            "json",
            "-P",
            "x=42",
            "-P",
            "v:embed=hello",
            "RETURN $x AS x, size($v) AS n",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<Value> = serde_json::from_str(&stdout).expect("valid JSON array");
    assert_eq!(parsed.len(), 1);
    // Integer literal preserved as integer (not a string).
    assert_eq!(parsed[0]["x"], 42);
    assert!(parsed[0]["x"].is_i64());
    assert_eq!(parsed[0]["n"], ALL_MINILM_DIMS);
}

// --- Embedding error-path tests (no Neo4j, no Ollama required) ---
//
// Asserts the exact REQ-F-011 error strings. Each test strips embed-related
// env vars so a dev with NEO4J_EMBED_* already exported in their shell
// doesn't mask the failure mode under test.

fn embed_env_clean() -> Command {
    let mut c = Command::cargo_bin("neo4j-query").unwrap();
    c.env_remove("NEO4J_EMBED_PROVIDER");
    c.env_remove("NEO4J_EMBED_MODEL");
    c.env_remove("NEO4J_EMBED_DIMENSIONS");
    c.env_remove("NEO4J_EMBED_BASE_URL");
    c.env_remove("NEO4J_EMBED_API_KEY");
    c.env_remove("OPENAI_API_KEY");
    c.env_remove("HF_TOKEN");
    c
}

#[test]
fn embed_missing_provider_errors() {
    // Password present so we get past require_password and into param resolution.
    let mut c = embed_env_clean();
    c.env("NEO4J_PASSWORD", "x");
    c.env("NEO4J_URI", "http://localhost:19999");
    c.args(["-P", "v:embed=hello", "RETURN $v"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "embedding provider not configured: set NEO4J_EMBED_PROVIDER",
        ));
}

#[test]
fn embed_unknown_modifier_errors() {
    let mut c = embed_env_clean();
    c.env("NEO4J_PASSWORD", "x");
    c.env("NEO4J_URI", "http://localhost:19999");
    c.args(["-P", "v:foo=hello", "RETURN $v"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown param modifier: :foo"));
}

#[test]
fn embed_openai_missing_api_key_errors() {
    // Embed subcommand avoids the Neo4j password requirement entirely.
    let mut c = embed_env_clean();
    c.env("NEO4J_EMBED_PROVIDER", "openai");
    c.env("NEO4J_EMBED_MODEL", "text-embedding-3-small");
    c.args(["embed", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "missing API key for openai: set OPENAI_API_KEY",
        ));
}

#[test]
fn huggingface_missing_api_key_errors() {
    let mut c = embed_env_clean();
    c.env("NEO4J_EMBED_PROVIDER", "huggingface");
    c.env(
        "NEO4J_EMBED_MODEL",
        "sentence-transformers/clip-ViT-B-32-multilingual-v1",
    );
    c.args(["embed", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "missing API key for huggingface: set HF_TOKEN",
        ));
}

#[test]
fn embed_unknown_provider_errors() {
    let mut c = embed_env_clean();
    c.env("NEO4J_EMBED_PROVIDER", "bogus");
    c.env("NEO4J_EMBED_MODEL", "some-model");
    c.args(["embed", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown provider: bogus"));
}

// --- CLI-flag position tests for the `embed` subcommand ---
//
// Regression guard: `--embed-*` flags must reach the subcommand handler
// regardless of whether they're typed BEFORE or AFTER the `embed`
// subcommand name. Achieved via `global = true` on each EmbedCliArgs
// field — same pattern as ConnectionArgs.
//
// Probe strategy: point at an obviously unreachable base URL so Ollama
// fails fast with its own error string. If the flag didn't reach the
// handler we'd see "embedding provider not configured: set
// NEO4J_EMBED_PROVIDER" (NotConfigured) instead.

const UNREACHABLE_OLLAMA: &str = "http://127.0.0.1:1";

#[test]
fn embed_cli_flags_before_subcommand() {
    let mut c = embed_env_clean();
    c.args([
        "--embed-provider",
        "ollama",
        "--embed-model",
        "all-minilm",
        "--embed-base-url",
        UNREACHABLE_OLLAMA,
        "embed",
        "hello",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("ollama unreachable"))
    .stderr(predicate::str::contains(UNREACHABLE_OLLAMA));
}

#[test]
fn huggingface_cli_flags_before_subcommand() {
    // Regression guard per AGENTS.md "CLI Architecture" rule: global=true
    // flags must reach the subcommand handler when typed BEFORE `embed`.
    // Missing-key short-circuit proves the flags arrived (otherwise we'd
    // see "embedding provider not configured" instead).
    let mut c = embed_env_clean();
    c.args([
        "--embed-provider",
        "huggingface",
        "--embed-model",
        "anything",
        "embed",
        "hello",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "missing API key for huggingface: set HF_TOKEN",
    ));
}

#[test]
fn embed_cli_flags_after_subcommand() {
    let mut c = embed_env_clean();
    c.args([
        "embed",
        "--embed-provider",
        "ollama",
        "--embed-model",
        "all-minilm",
        "--embed-base-url",
        UNREACHABLE_OLLAMA,
        "hello",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("ollama unreachable"))
    .stderr(predicate::str::contains(UNREACHABLE_OLLAMA));
}

#[test]
fn embed_dimensions_flag_reaches_subcommand() {
    // --embed-dimensions is OpenAI-only; missing key short-circuits before
    // any HTTP call. Proves the flag is accepted at root position.
    let mut c = embed_env_clean();
    c.args([
        "--embed-provider",
        "openai",
        "--embed-model",
        "text-embedding-3-small",
        "--embed-dimensions",
        "512",
        "embed",
        "hello",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "missing API key for openai: set OPENAI_API_KEY",
    ));
}

#[test]
fn query_embed_cli_flags_before_query() {
    // `-P v:embed=...` path: flags live on QueryArgs and must work when
    // typed before the positional query. Unreachable base URL proves
    // the flag reached the embed resolver.
    let mut c = embed_env_clean();
    c.env("NEO4J_PASSWORD", "x");
    c.env("NEO4J_URI", "http://localhost:19999");
    c.args([
        "--embed-provider",
        "ollama",
        "--embed-model",
        "all-minilm",
        "--embed-base-url",
        UNREACHABLE_OLLAMA,
        "-P",
        "v:embed=hello",
        "RETURN $v",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("ollama unreachable"))
    .stderr(predicate::str::contains(UNREACHABLE_OLLAMA));
}

// --- Optional live HuggingFace serverless call (opt-in) ---
//
// Gated on HF_TEST_TOKEN env rather than plain HF_TOKEN so a dev with a
// shell-exported HF_TOKEN doesn't accidentally trigger a paid request.
// Mirrors the neo4j_available() / ollama_available() skip pattern.

/// clip-ViT-B-32-multilingual-v1 returns 512-dim vectors.
const CLIP_MULTILINGUAL_DIMS: usize = 512;

#[test]
#[ignore]
fn huggingface_serverless_real_call() {
    let token = match std::env::var("HF_TEST_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            eprintln!("skipping: HF_TEST_TOKEN not set");
            return;
        }
    };
    let mut c = embed_env_clean();
    c.env("HF_TOKEN", token);
    let output = c
        .args([
            "--embed-provider",
            "huggingface",
            "--embed-model",
            "sentence-transformers/clip-ViT-B-32-multilingual-v1",
            "embed",
            "hello",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "embed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<f32> = serde_json::from_str(stdout.trim()).expect("valid JSON array of floats");
    assert_eq!(parsed.len(), CLIP_MULTILINGUAL_DIMS);
}
