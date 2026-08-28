//! Row message parsing contract tests.

use maxwell_cdc::{Message, OpType, parse};
use serde_json::json;

#[test]
fn insert_exposes_every_row_field() {
    let json = r#"{
        "database":"shop",
        "table":"orders",
        "type":"insert",
        "ts":1477053217,
        "xid":23396,
        "commit":true,
        "position":"master.000006:800911",
        "server_id":23042,
        "thread_id":108,
        "primary_key":[1],
        "primary_key_columns":["id"],
        "data":{"id":1,"note":"new"},
        "columns_types":{"id":"int","note":"varchar"}
    }"#;

    let message = parse(json).expect("insert should parse");
    assert_eq!(message.op_type(), Some(OpType::Insert));
    let Message::Insert(row) = message else {
        panic!("expected insert");
    };
    assert_eq!(row.database, "shop");
    assert_eq!(row.table, "orders");
    assert_eq!(row.ts, Some(1_477_053_217));
    assert_eq!(row.xid, Some(23396));
    assert_eq!(row.commit, Some(true));
    assert_eq!(row.position.as_deref(), Some("master.000006:800911"));
    assert_eq!(row.server_id, Some(23042));
    assert_eq!(row.thread_id, Some(108));
    assert_eq!(row.primary_key, Some(vec![json!(1)]));
    assert_eq!(row.primary_key_columns, Some(vec!["id".to_owned()]));
    assert_eq!(row.data.get("note"), Some(&json!("new")));
    assert_eq!(
        row.columns_types
            .as_ref()
            .and_then(|types| types.get("note"))
            .map(String::as_str),
        Some("varchar")
    );
}

#[test]
fn update_retains_old_values() {
    let json = r#"{
        "database":"shop",
        "table":"orders",
        "type":"update",
        "ts":1477053217,
        "data":{"status":"paid"},
        "old":{"status":"open"}
    }"#;

    let message = parse(json).expect("update should parse");
    assert_eq!(message.op_type(), Some(OpType::Update));
    let Message::Update(row) = message else {
        panic!("expected update");
    };
    assert_eq!(row.data.get("status"), Some(&json!("paid")));
    assert_eq!(
        row.old.as_ref().and_then(|o| o.get("status")),
        Some(&json!("open"))
    );
}

#[test]
fn delete_retains_null_datetime_and_unsigned_bigint() {
    let json = r#"{
        "database":"shop",
        "table":"orders",
        "type":"delete",
        "ts":1477053217,
        "data":{"id":9223372036854775808,"created_at":null}
    }"#;

    let message = parse(json).expect("delete should parse");
    assert_eq!(message.op_type(), Some(OpType::Delete));
    let Message::Delete(row) = message else {
        panic!("expected delete");
    };
    assert_eq!(
        row.data.get("id"),
        Some(&json!(9_223_372_036_854_775_808_u64))
    );
    assert_eq!(row.data.get("created_at"), Some(&json!(null)));
}

#[test]
fn row_message_requires_table() {
    let json = r#"{
        "database":"shop",
        "type":"insert",
        "ts":1477053217,
        "data":{}
    }"#;

    assert!(parse(json).is_err());
}
