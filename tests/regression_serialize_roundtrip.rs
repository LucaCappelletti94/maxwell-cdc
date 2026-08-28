//! Regression tests for serialization roundtrip behavior.

use maxwell_cdc::{Message, parse};

#[test]
fn database_create_serializes_and_reparses_identically() {
    let original_json = r#"{"type":"database-create","database":"testdb","charset":"utf8mb4","ts":1477053308000,"sql":"CREATE DATABASE testdb CHARACTER SET utf8mb4","position":"mysql-bin.000001:1234","gtid":"server:1-2","schema_id":5}"#;

    let message: Message = parse(original_json).expect("initial parse");

    let serialized = serde_json::to_string(&message).expect("serialize");
    let reparsed_message: Message = serde_json::from_str(&serialized).expect("reparse");

    let Message::DatabaseCreate(original) = &message else {
        panic!("expected database create in original");
    };

    let Message::DatabaseCreate(reparsed) = &reparsed_message else {
        panic!("expected database create after reparse");
    };

    assert_eq!(
        original.definition.database, reparsed.definition.database,
        "database should match"
    );
    assert_eq!(
        original.definition.charset, reparsed.definition.charset,
        "charset should match"
    );
    assert_eq!(
        original.metadata.ts, reparsed.metadata.ts,
        "ts should match"
    );
    assert_eq!(
        original.metadata.sql, reparsed.metadata.sql,
        "sql should match"
    );
    assert_eq!(
        original.metadata.position, reparsed.metadata.position,
        "position should match"
    );
    assert_eq!(
        original.metadata.gtid, reparsed.metadata.gtid,
        "gtid should match"
    );
    assert_eq!(
        original.metadata.schema_id, reparsed.metadata.schema_id,
        "schema_id should match"
    );

    let reserialized = serde_json::to_string(&reparsed_message).expect("reserialize");
    assert_eq!(
        serialized, reserialized,
        "serialization should be idempotent"
    );
}
