//! Regenerate Maxwell CDC fixtures by executing the exact captured operation matrix.

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
            }
        }
    }
}

const FIRST_ID: u64 = 9_223_372_036_854_775_808;
const SECOND_ID: u64 = 9_223_372_036_854_775_809;

fn execute_ddl(conn: &mut MysqlConnection, sql: &str) {
    // Diesel has no DDL query builder, and MySQL rejects prepared DDL.
    SimpleConnection::batch_execute(conn, sql).unwrap_or_else(|e| panic!("DDL failed: {e}"));
}

fn wait_for_bootstrap(path: &Path) {
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
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

fn collect(path: &Path) -> BTreeMap<String, serde_json::Value> {
    let mut results: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(60);

    loop {
        results.clear();

        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                for line in content.lines() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let database = val.get("database").and_then(|v| v.as_str()).unwrap_or("");
                        let table = val.get("table").and_then(|v| v.as_str()).unwrap_or("");
                        let id = val.get("data").and_then(|d| d.get("id"));

                        let fixture_name = match (msg_type, database, table, id) {
                            ("insert", "testdb", "capture_events", Some(id_val))
                                if id_val.as_u64() == Some(FIRST_ID) =>
                            {
                                "row-insert"
                            }
                            ("update", "testdb", "capture_events", Some(id_val))
                                if id_val.as_u64() == Some(FIRST_ID) =>
                            {
                                "row-update"
                            }
                            ("delete", "testdb", "capture_events", Some(id_val))
                                if id_val.as_u64() == Some(FIRST_ID) =>
                            {
                                "row-delete"
                            }
                            ("bootstrap-start", "testdb", "capture_events", None) => {
                                "bootstrap-start"
                            }
                            ("bootstrap-insert", "testdb", "capture_events", Some(id_val))
                                if id_val.as_u64() == Some(SECOND_ID) =>
                            {
                                "bootstrap-insert"
                            }
                            ("bootstrap-complete", "testdb", "capture_events", None) => {
                                "bootstrap-complete"
                            }
                            ("table-create", "testdb", "capture_events", None) => "table-create",
                            ("table-alter", "testdb", "capture_events", None) => "table-alter",
                            ("table-drop", "testdb", "capture_events", None) => "table-drop",
                            ("database-create", "archive_capture", "", None) => "database-create",
                            ("database-alter", "archive_capture", "", None) => "database-alter",
                            ("database-drop", "archive_capture", "", None) => "database-drop",
                            _ => continue,
                        };

                        results.insert(fixture_name.to_string(), val);
                    }
                }
            }
        }

        let expected = [
            "row-insert",
            "row-update",
            "row-delete",
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

        if expected.iter().all(|name| results.contains_key(*name)) {
            return results;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for all 12 event types at {}",
            path.display()
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
        ])
        .with_network(&network)
        .with_container_name(&mysql_name)
        .with_startup_timeout(Duration::from_secs(120))
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
            &host_flag,
            "--port=3306",
            "--user=root",
            "--password=subql_test",
            "--bootstrapper=sync",
        ])
        .with_startup_timeout(Duration::from_secs(90))
        .start()
        .unwrap_or_else(|e| panic!("start maxwell: {e}"));

    (output_path, mysql_url, mysql, maxwell, output_dir)
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

    {
        let now =
            NaiveDateTime::parse_from_str("2026-08-28 14:03:02.123456", "%Y-%m-%d %H:%M:%S%.f")
                .expect("datetime");
        diesel::insert_into(tables::capture_events::table)
            .values((
                tables::capture_events::id.eq(FIRST_ID),
                tables::capture_events::nullable_text.eq::<Option<String>>(None),
                tables::capture_events::happened_at.eq(now),
                tables::capture_events::amount.eq(1),
                tables::capture_events::status.eq("open"),
            ))
            .execute(conn)
            .expect("insert first");
    }

    {
        diesel::update(
            tables::capture_events::table.filter(tables::capture_events::id.eq(FIRST_ID)),
        )
        .set((
            tables::capture_events::nullable_text.eq("filled"),
            tables::capture_events::amount.eq(2),
        ))
        .execute(conn)
        .expect("update first");
    }

    {
        diesel::delete(
            tables::capture_events::table.filter(tables::capture_events::id.eq(FIRST_ID)),
        )
        .execute(conn)
        .expect("delete first");
    }

    {
        let now =
            NaiveDateTime::parse_from_str("2026-08-28 14:03:02.123456", "%Y-%m-%d %H:%M:%S%.f")
                .expect("datetime");
        diesel::insert_into(tables::capture_events::table)
            .values((
                tables::capture_events::id.eq(SECOND_ID),
                tables::capture_events::nullable_text.eq("bootstrap"),
                tables::capture_events::happened_at.eq(now),
                tables::capture_events::amount.eq(3),
                tables::capture_events::status.eq("closed"),
            ))
            .execute(conn)
            .expect("insert second");
    }

    diesel::insert_into(tables::maxwell::bootstrap::table)
        .values((
            tables::maxwell::bootstrap::database_name.eq("testdb"),
            tables::maxwell::bootstrap::table_name.eq("capture_events"),
        ))
        .execute(conn)
        .expect("insert bootstrap");

    wait_for_bootstrap(Path::new(output_path).join("maxwell.jsonl").as_path());

    execute_ddl(
        conn,
        "ALTER TABLE capture_events ADD COLUMN tags SET('one','two')",
    );

    execute_ddl(conn, "DROP TABLE capture_events");
}

fn save_fixtures(results: &BTreeMap<String, serde_json::Value>) {
    for (name, val) in results {
        let filename = format!("tests/fixtures/{name}.json");
        let json_str = serde_json::to_string_pretty(val).expect("serialize json");

        fs::write(&filename, format!("{json_str}\n"))
            .unwrap_or_else(|e| panic!("write fixture {filename}: {e}"));

        let serialized_msg = serde_json::to_string(val).expect("serialize json");
        let parsed = maxwell_cdc::parse(&serialized_msg).expect("parse json");
        let serialized = serde_json::to_string(&parsed).expect("serialize message");
        let reparsed: maxwell_cdc::Message =
            maxwell_cdc::parse(&serialized).expect("reparse message");
        assert_eq!(parsed, reparsed, "message roundtrip mismatch for {name}");
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

    assert_eq!(
        results.len(),
        12,
        "expected 12 event types, got {}",
        results.len()
    );
}
