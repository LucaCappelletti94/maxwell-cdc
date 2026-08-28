//! Schema DDL message parsing contract tests.

use maxwell_cdc::{DdlMetadata, Message, parse};

fn assert_metadata(metadata: &DdlMetadata, sql: &str) {
    assert_eq!(metadata.ts, 1_477_053_308_000);
    assert_eq!(metadata.sql, sql);
    assert_eq!(metadata.position.as_deref(), Some("master.000006:804398"));
    assert_eq!(metadata.gtid.as_deref(), Some("server:1-2"));
    assert_eq!(metadata.schema_id, Some(17));
}

#[test]
fn table_create_has_typed_definition() {
    let sql = "create table test.events (id bigint unsigned primary key, label varchar(255))";
    let json = format!(
        r#"{{
            "type":"table-create",
            "database":"test",
            "table":"events",
            "def":{{
                "database":"test",
                "table":"events",
                "charset":"utf8mb4",
                "columns":[{{"type":"bigint","name":"id","signed":false}},
                           {{"type":"varchar","name":"label","charset":"utf8mb4"}}],
                "primary-key":["id"]
            }},
            "ts":1477053308000,
            "sql":"{sql}",
            "position":"master.000006:804398",
            "gtid":"server:1-2",
            "schema_id":17
        }}"#
    );

    let message = parse(&json).expect("table create should parse");
    let Message::TableCreate(change) = message else {
        panic!("expected table create");
    };
    assert_eq!(change.database, "test");
    assert_eq!(change.table, "events");
    assert_eq!(change.definition.database, "test");
    assert_eq!(change.definition.table, "events");
    assert_eq!(change.definition.charset.as_deref(), Some("utf8mb4"));
    assert_eq!(change.definition.primary_key, ["id"]);
    assert_eq!(change.definition.columns.len(), 2);
    let id = &change.definition.columns[0];
    assert_eq!(id.column_type, "bigint");
    assert_eq!(id.name, "id");
    assert_eq!(id.signed, Some(false));
    assert_eq!(id.charset, None);
    let label = &change.definition.columns[1];
    assert_eq!(label.column_type, "varchar");
    assert_eq!(label.charset.as_deref(), Some("utf8mb4"));
    assert_metadata(&change.metadata, sql);
}

#[test]
fn table_alter_has_typed_old_and_new_definitions() {
    let sql = "alter table test.events add column created_at timestamp(6)";
    let json = format!(
        r#"{{
            "type":"table-alter",
            "database":"test",
            "table":"events",
            "old":{{
                "database":"test",
                "table":"events",
                "charset":"utf8mb4",
                "columns":[{{"type":"bigint","name":"id","signed":false}}],
                "primary-key":["id"]
            }},
            "def":{{
                "database":"test",
                "table":"events",
                "charset":"utf8mb4",
                "columns":[
                    {{"type":"bigint","name":"id","signed":false}},
                    {{"type":"timestamp","name":"created_at","column-length":6}}
                ],
                "primary-key":["id"]
            }},
            "ts":1477053308000,
            "sql":"{sql}",
            "position":"master.000006:804398",
            "gtid":"server:1-2",
            "schema_id":17
        }}"#
    );

    let message = parse(&json).expect("table alter should parse");
    let Message::TableAlter(change) = message else {
        panic!("expected table alter");
    };
    assert_eq!(change.old_definition.columns.len(), 1);
    assert_eq!(change.definition.columns.len(), 2);
    let created_at = &change.definition.columns[1];
    assert_eq!(created_at.column_type, "timestamp");
    assert_eq!(created_at.column_length, Some(6));
    assert_metadata(&change.metadata, sql);
}

/// Maxwell writes `column-length` as a Java `Long`, so a value past `u32::MAX` must parse.
#[test]
fn column_length_accepts_the_full_upstream_range() {
    let json = r#"{
        "type":"table-create",
        "database":"test",
        "table":"events",
        "def":{
            "database":"test",
            "table":"events",
            "columns":[{"type":"datetime","name":"at","column-length":4294967296}],
            "primary-key":[]
        },
        "ts":1477053308000,
        "sql":"create table test.events (at datetime(6))"
    }"#;

    let message = parse(json).expect("a Long column-length must parse");
    let Message::TableCreate(change) = message else {
        panic!("expected table create");
    };

    assert_eq!(
        change.definition.columns[0].column_length,
        Some(4_294_967_296)
    );
}

#[test]
fn table_drop_has_no_definition() {
    let sql = "drop table test.events";
    let json = format!(
        r#"{{
            "type":"table-drop",
            "database":"test",
            "table":"events",
            "ts":1477053308000,
            "sql":"{sql}",
            "position":"master.000006:804398",
            "gtid":"server:1-2",
            "schema_id":17
        }}"#
    );

    let message = parse(&json).expect("table drop should parse");
    let Message::TableDrop(change) = message else {
        panic!("expected table drop");
    };
    assert_eq!(change.database, "test");
    assert_eq!(change.table, "events");
    assert_metadata(&change.metadata, sql);
}

#[test]
fn database_create_has_typed_definition() {
    let sql = "create database archive character set utf8mb4";
    let json = format!(
        r#"{{
            "type":"database-create",
            "database":"archive",
            "charset":"utf8mb4",
            "ts":1477053308000,
            "sql":"{sql}",
            "position":"master.000006:804398",
            "gtid":"server:1-2",
            "schema_id":17
        }}"#
    );

    let message = parse(&json).expect("database create should parse");
    let Message::DatabaseCreate(change) = message else {
        panic!("expected database create");
    };
    assert_eq!(change.definition.database, "archive");
    assert_eq!(change.definition.charset.as_deref(), Some("utf8mb4"));
    assert_metadata(&change.metadata, sql);
}

#[test]
fn database_alter_has_typed_definition() {
    let sql = "alter database archive character set latin1";
    let json = format!(
        r#"{{
            "type":"database-alter",
            "database":"archive",
            "charset":"latin1",
            "ts":1477053308000,
            "sql":"{sql}",
            "position":"master.000006:804398",
            "gtid":"server:1-2",
            "schema_id":17
        }}"#
    );

    let message = parse(&json).expect("database alter should parse");
    let Message::DatabaseAlter(change) = message else {
        panic!("expected database alter");
    };
    assert_eq!(change.definition.database, "archive");
    assert_eq!(change.definition.charset.as_deref(), Some("latin1"));
    assert_metadata(&change.metadata, sql);
}

#[test]
fn database_drop_has_no_definition() {
    let sql = "drop database archive";
    let json = format!(
        r#"{{
            "type":"database-drop",
            "database":"archive",
            "ts":1477053308000,
            "sql":"{sql}",
            "position":"master.000006:804398",
            "gtid":"server:1-2",
            "schema_id":17
        }}"#
    );

    let message = parse(&json).expect("database drop should parse");
    let Message::DatabaseDrop(change) = message else {
        panic!("expected database drop");
    };
    assert_eq!(change.database, "archive");
    assert_metadata(&change.metadata, sql);
}
