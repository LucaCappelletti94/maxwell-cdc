//! Regenerate Maxwell CDC fixtures by executing the exact captured operation matrix.
//!
//! Fixtures are written as the verbatim lines Maxwell emitted. Reserializing them through
//! `serde_json::Value` would sort the keys and hide the wire order, which would make the
//! corpus a mirror of this crate's own output instead of an independent record.

use chrono::NaiveDateTime;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::thread;
use std::time::Duration;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::{IntoContainerPort, Mount, WaitFor};
use testcontainers::runners::SyncRunner;

mod tables {
    diesel::table! {
        capture_events (id) {
            id -> Unsigned<BigInt>,
            nullable_text -> Nullable<Varchar>,
            happened_at -> Datetime,
            amount -> Integer,
            status -> Text,
        }
    }

    pub mod maxwell {
        diesel::table! {
            maxwell.bootstrap (id) {
                id -> BigInt,
                database_name -> Varchar,
                table_name -> Varchar,
                comment -> Nullable<Varchar>,
            }
        }
    }
}

const FIRST_ID: u64 = 9_223_372_036_854_775_808;
const SECOND_ID: u64 = 9_223_372_036_854_775_809;
/// Non-final row of a multi-statement transaction, which is the only row that carries
/// `xoffset` and omits `commit`.
const THIRD_ID: u64 = 9_223_372_036_854_775_810;
/// Final row of that transaction, which carries `commit` instead.
const FOURTH_ID: u64 = 9_223_372_036_854_775_811;

const BOOTSTRAP_COMMENT: &str = "captured for the maxwell-cdc fixture corpus";

/// Container startup budget. Generous because this machine often runs other container
/// suites concurrently, and a contended daemon takes minutes to bring MySQL up.
const CONTAINER_STARTUP: Duration = Duration::from_secs(300);

/// Budget for a set of expected events to appear in the output file.
const EVENT_DEADLINE: Duration = Duration::from_secs(120);

/// Fixture names the corpus must contain, and the message shape each one records.
const EXPECTED: &[&str] = &[
    "row-insert",
    "row-update",
    "row-delete",
    "row-insert-uncommitted",
    "bootstrap-start",
    "bootstrap-insert",
    "bootstrap-complete",
    "table-create",
    "table-alter",
    "table-drop",
    "database-create",
    "database-alter",
    "database-drop",
];

fn execute_ddl(conn: &mut MysqlConnection, sql: &str) {
    // Diesel has no DDL query builder, and MySQL rejects prepared DDL.
    SimpleConnection::batch_execute(conn, sql).unwrap_or_else(|e| panic!("DDL failed: {e}"));
}

fn wait_for_bootstrap(path: &Path) {
    let deadline = std::time::Instant::now() + EVENT_DEADLINE;
    loop {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        if let (Some("bootstrap-complete"), Some("capture_events")) = (
                            val.get("type").and_then(|v| v.as_str()),
                            val.get("table").and_then(|v| v.as_str()),
                        ) {
                            return;
                        }
                    }
                }
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for bootstrap-complete at {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(200));
    }
}

/// Classify one emitted line into a fixture name, or `None` to ignore it.
fn classify(val: &serde_json::Value) -> Option<&'static str> {
    let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let database = val.get("database").and_then(|v| v.as_str()).unwrap_or("");
    let table = val.get("table").and_then(|v| v.as_str()).unwrap_or("");
    let id = val
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(serde_json::Value::as_u64);

    match (msg_type, database, table, id) {
        ("insert", "testdb", "capture_events", Some(FIRST_ID)) => Some("row-insert"),
        ("update", "testdb", "capture_events", Some(FIRST_ID)) => Some("row-update"),
        ("delete", "testdb", "capture_events", Some(FIRST_ID)) => Some("row-delete"),
        ("insert", "testdb", "capture_events", Some(THIRD_ID)) => Some("row-insert-uncommitted"),
        ("bootstrap-start", "testdb", "capture_events", None) => Some("bootstrap-start"),
        ("bootstrap-insert", "testdb", "capture_events", Some(SECOND_ID)) => {
            Some("bootstrap-insert")
        }
        ("bootstrap-complete", "testdb", "capture_events", None) => Some("bootstrap-complete"),
        ("table-create", "testdb", "capture_events", None) => Some("table-create"),
        ("table-alter", "testdb", "capture_events", None) => Some("table-alter"),
        ("table-drop", "testdb", "capture_events", None) => Some("table-drop"),
        ("database-create", "archive_capture", "", None) => Some("database-create"),
        ("database-alter", "archive_capture", "", None) => Some("database-alter"),
        ("database-drop", "archive_capture", "", None) => Some("database-drop"),
        _ => None,
    }
}

/// Poll the output file until every expected message type has appeared, then return the
/// verbatim line for each.
fn collect(path: &Path) -> BTreeMap<&'static str, String> {
    let mut results: BTreeMap<&'static str, String> = BTreeMap::new();
    let deadline = std::time::Instant::now() + EVENT_DEADLINE;

    loop {
        results.clear();

        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(name) = classify(&val) {
                            results.insert(name, line.to_owned());
                        }
                    }
                }
            }
        }

        if EXPECTED.iter().all(|name| results.contains_key(name)) {
            return results;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {} message types at {}, have {:?}",
            EXPECTED.len(),
            path.display(),
            results.keys().collect::<Vec<_>>()
        );
        thread::sleep(Duration::from_millis(200));
    }
}

fn setup_containers(
    ts: u128,
) -> (
    String,
    String,
    testcontainers::Container<GenericImage>,
    testcontainers::Container<GenericImage>,
    tempfile::TempDir,
) {
    let network = format!("maxwell-test-{ts}");
    let mysql_name = format!("mysql-{ts}");
    let output_dir = tempfile::tempdir().expect("temp dir");
    fs::set_permissions(&output_dir, fs::Permissions::from_mode(0o777)).expect("chmod output dir");
    let output_path = output_dir.path().to_string_lossy().to_string();

    let mysql = GenericImage::new("mysql", "8.0")
        .with_wait_for(WaitFor::message_on_stderr("port: 3306"))
        .with_exposed_port(3306.tcp())
        .with_env_var("MYSQL_ROOT_PASSWORD", "subql_test")
        .with_env_var("MYSQL_DATABASE", "testdb")
        .with_cmd([
            "--server-id=1",
            "--log-bin=mysql-bin",
            "--binlog-format=ROW",
            "--binlog-row-image=FULL",
            // Populates the `gtid` field.
            "--gtid-mode=ON",
            "--enforce-gtid-consistency=ON",
            // Populates the `query` field, which output_row_query depends on.
            "--binlog-rows-query-log-events=ON",
        ])
        .with_network(&network)
        .with_container_name(&mysql_name)
        .with_startup_timeout(CONTAINER_STARTUP)
        .start()
        .unwrap_or_else(|e| panic!("start mysql: {e}"));

    let mysql_port = mysql.get_host_port_ipv4(3306.tcp()).expect("mysql port");
    let mysql_url = format!("mysql://root:subql_test@127.0.0.1:{mysql_port}/testdb");

    let host_flag = format!("--host={mysql_name}");
    let maxwell = GenericImage::new("zendesk/maxwell", "v1.44.0")
        .with_wait_for(WaitFor::message_on_stderr("Binlog connected"))
        .with_network(&network)
        .with_mount(Mount::bind_mount(&output_path, "/output"))
        .with_cmd([
            "bin/maxwell",
            "--producer=file",
            "--output_file=/output/maxwell.jsonl",
            "--output_ddl=true",
            "--output_binlog_position=true",
            "--output_primary_keys=true",
            "--output_primary_key_columns=true",
            "--output_server_id=true",
            "--output_thread_id=true",
            // Maxwell rejects output_gtid_position unless it is itself in GTID mode.
            "--gtid_mode=true",
            // Every remaining optional field, so the corpus covers the whole row shape.
            "--output_gtid_position=true",
            "--output_xoffset=true",
            "--output_schema_id=true",
            "--output_row_query=true",
            "--output_push_timestamp=true",
            &host_flag,
            "--port=3306",
            "--user=root",
            "--password=subql_test",
            "--bootstrapper=sync",
        ])
        .with_startup_timeout(CONTAINER_STARTUP)
        .start()
        .unwrap_or_else(|e| panic!("start maxwell: {e}"));

    (output_path, mysql_url, mysql, maxwell, output_dir)
}

fn captured_datetime() -> NaiveDateTime {
    NaiveDateTime::parse_from_str("2026-08-28 14:03:02.123456", "%Y-%m-%d %H:%M:%S%.f")
        .expect("datetime")
}

fn insert_row(conn: &mut MysqlConnection, id: u64, text: Option<&str>, amount: i32, status: &str) {
    diesel::insert_into(tables::capture_events::table)
        .values((
            tables::capture_events::id.eq(id),
            tables::capture_events::nullable_text.eq(text.map(str::to_owned)),
            tables::capture_events::happened_at.eq(captured_datetime()),
            tables::capture_events::amount.eq(amount),
            tables::capture_events::status.eq(status),
        ))
        .execute(conn)
        .unwrap_or_else(|e| panic!("insert {id}: {e}"));
}

fn run_operations(conn: &mut MysqlConnection, output_path: &Path) {
    execute_ddl(conn, "CREATE DATABASE archive_capture");
    execute_ddl(conn, "ALTER DATABASE archive_capture CHARACTER SET latin1");
    execute_ddl(conn, "DROP DATABASE archive_capture");

    execute_ddl(
        conn,
        "CREATE TABLE capture_events (
            id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
            nullable_text VARCHAR(255),
            happened_at DATETIME(6),
            amount INT,
            status ENUM('open', 'closed')
        )",
    );

    insert_row(conn, FIRST_ID, None, 1, "open");

    diesel::update(tables::capture_events::table.filter(tables::capture_events::id.eq(FIRST_ID)))
        .set((
            tables::capture_events::nullable_text.eq("filled"),
            tables::capture_events::amount.eq(2),
        ))
        .execute(conn)
        .expect("update first");

    diesel::delete(tables::capture_events::table.filter(tables::capture_events::id.eq(FIRST_ID)))
        .execute(conn)
        .expect("delete first");

    insert_row(conn, SECOND_ID, Some("bootstrap"), 3, "closed");

    diesel::insert_into(tables::maxwell::bootstrap::table)
        .values((
            tables::maxwell::bootstrap::database_name.eq("testdb"),
            tables::maxwell::bootstrap::table_name.eq("capture_events"),
            tables::maxwell::bootstrap::comment.eq(BOOTSTRAP_COMMENT),
        ))
        .execute(conn)
        .expect("insert bootstrap");

    wait_for_bootstrap(Path::new(output_path).join("maxwell.jsonl").as_path());

    // Two rows in one transaction: the first is uncommitted and carries `xoffset`, the
    // second carries `commit`. A single-statement autocommit insert produces neither.
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        insert_row(conn, THIRD_ID, Some("uncommitted"), 4, "open");
        insert_row(conn, FOURTH_ID, Some("committed"), 5, "closed");
        Ok(())
    })
    .expect("multi row transaction");

    execute_ddl(
        conn,
        "ALTER TABLE capture_events ADD COLUMN tags SET('one','two')",
    );

    execute_ddl(conn, "DROP TABLE capture_events");
}

/// Write each captured line verbatim, and prove the crate reaches a fixed point on it.
fn save_fixtures(results: &BTreeMap<&'static str, String>) {
    for (name, raw) in results {
        let filename = format!("tests/fixtures/{name}.json");
        fs::write(&filename, format!("{raw}\n"))
            .unwrap_or_else(|e| panic!("write fixture {filename}: {e}"));

        let parsed = maxwell_cdc::parse(raw).unwrap_or_else(|e| panic!("parse {name}: {e}"));
        let serialized = serde_json::to_string(&parsed).expect("serialize message");
        let reparsed: maxwell_cdc::Message =
            maxwell_cdc::parse(&serialized).unwrap_or_else(|e| panic!("reparse {name}: {e}"));
        assert_eq!(parsed, reparsed, "message roundtrip mismatch for {name}");

        let original: serde_json::Value = serde_json::from_str(raw).expect("parse raw");
        let round_tripped: serde_json::Value =
            serde_json::from_str(&serialized).expect("parse serialized");
        assert_eq!(
            round_tripped, original,
            "{name}: this crate dropped or invented a field Maxwell emitted"
        );
    }
}

/// Remove fixtures no longer produced, so a stale file cannot outlive the matrix.
fn prune_stale_fixtures(results: &BTreeMap<&'static str, String>) {
    for entry in fs::read_dir("tests/fixtures").expect("read fixture dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("fixture stem")
            .to_owned();
        if !results.contains_key(stem.as_str()) {
            fs::remove_file(&path).unwrap_or_else(|e| panic!("remove {}: {e}", path.display()));
        }
    }
}

#[test]
#[ignore = "expensive test: requires docker, mysql, maxwell"]
fn regenerate_fixtures() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let (output_path, mysql_url, _mysql, _maxwell, _output_dir) = setup_containers(ts);

    let mut conn = MysqlConnection::establish(&mysql_url).expect("mysql connection");

    run_operations(&mut conn, Path::new(&output_path));

    let results = collect(Path::new(&output_path).join("maxwell.jsonl").as_path());

    save_fixtures(&results);
    prune_stale_fixtures(&results);
}
