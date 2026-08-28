//! Bootstrap message parsing contract tests.

use maxwell_cdc::{Message, OpType, parse};
use serde_json::json;

#[test]
fn bootstrap_start_preserves_control_fields() {
    let json = r#"{
        "database":"shop",
        "table":"orders",
        "type":"bootstrap-start",
        "ts":1450557744,
        "push_ts":1450557744.987654,
        "comment":"nightly resync",
        "data":{}
    }"#;

    let message = parse(json).expect("bootstrap start should parse");
    assert_eq!(message.op_type(), None);
    let Message::BootstrapStart(control) = message else {
        panic!("expected bootstrap start");
    };
    assert_eq!(control.database, "shop");
    assert_eq!(control.table, "orders");
    assert_eq!(control.ts, Some(1_450_557_744));
    assert_eq!(
        control
            .push_ts
            .as_ref()
            .and_then(serde_json::Number::as_f64),
        Some(1_450_557_744.987_654)
    );
    assert_eq!(control.comment.as_deref(), Some("nightly resync"));
    assert!(control.data.is_empty());
}

#[test]
fn bootstrap_insert_is_a_row_operation() {
    let json = r#"{
        "database":"shop",
        "table":"orders",
        "type":"bootstrap-insert",
        "ts":1450557744,
        "data":{"id":42,"status":"open"}
    }"#;

    let message = parse(json).expect("bootstrap insert should parse");
    assert_eq!(message.op_type(), Some(OpType::BootstrapInsert));
    let Message::BootstrapInsert(row) = message else {
        panic!("expected bootstrap insert");
    };
    assert_eq!(row.data.get("id"), Some(&json!(42)));
}

#[test]
fn bootstrap_complete_preserves_control_fields() {
    let json = r#"{
        "database":"shop",
        "table":"orders",
        "type":"bootstrap-complete",
        "ts":1450557744,
        "data":{}
    }"#;

    let message = parse(json).expect("bootstrap complete should parse");
    assert_eq!(message.op_type(), None);
    let Message::BootstrapComplete(control) = message else {
        panic!("expected bootstrap complete");
    };
    assert_eq!(control.database, "shop");
    assert_eq!(control.table, "orders");
    assert_eq!(control.ts, Some(1_450_557_744));
    assert!(control.data.is_empty());
}
