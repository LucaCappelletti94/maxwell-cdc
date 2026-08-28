//! Exact corpus test asserting full field preservation and variant correctness.

use maxwell_cdc::{Message, OpType, parse};
use serde_json::{Value, json};

#[test]
fn row_insert_fixture_parses_and_preserves_all_fields() {
    let fixture = include_str!("fixtures/row-insert.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    assert_eq!(message.op_type(), Some(OpType::Insert));

    let Message::Insert(row) = &message else {
        panic!("expected insert");
    };

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );

    assert_eq!(row.database, "testdb");
    assert_eq!(row.table, "capture_events");
    assert!(row.ts.is_some(), "ts must be present");
    assert!(row.xid.is_some(), "xid must be present");
    assert_eq!(row.commit, Some(true));
    assert!(row.position.is_some(), "position must be present");
}

#[test]
fn row_update_fixture_preserves_old_values() {
    let fixture = include_str!("fixtures/row-update.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    assert_eq!(message.op_type(), Some(OpType::Update));

    let Message::Update(row) = &message else {
        panic!("expected update");
    };

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );

    assert!(row.old.is_some(), "update must have old values");
    let old = row.old.as_ref().unwrap();
    assert_eq!(old.get("amount"), Some(&json!(1)));
    assert_eq!(old.get("nullable_text"), Some(&json!(null)));
    assert_eq!(row.data.get("amount"), Some(&json!(2)));
    assert_eq!(row.data.get("nullable_text"), Some(&json!("filled")));
}

#[test]
fn row_delete_fixture_parses() {
    let fixture = include_str!("fixtures/row-delete.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    assert_eq!(message.op_type(), Some(OpType::Delete));

    match &message {
        Message::Delete(_) => {}
        _ => panic!("expected delete"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn bootstrap_insert_fixture_is_row_operation_with_data() {
    let fixture = include_str!("fixtures/bootstrap-insert.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    assert_eq!(message.op_type(), Some(OpType::BootstrapInsert));

    let Message::BootstrapInsert(row) = &message else {
        panic!("expected bootstrap insert");
    };

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );

    assert_eq!(
        row.data.get("id"),
        Some(&json!(9_223_372_036_854_775_809_u64))
    );
    assert_eq!(row.data.get("nullable_text"), Some(&json!("bootstrap")));
    assert_eq!(row.data.get("status"), Some(&json!("closed")));
}

#[test]
fn bootstrap_start_fixture_parses() {
    let fixture = include_str!("fixtures/bootstrap-start.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    assert_eq!(message.op_type(), None);

    let Message::BootstrapStart(control) = &message else {
        panic!("expected bootstrap start");
    };

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );

    assert_eq!(control.database, "testdb");
    assert_eq!(control.table, "capture_events");
}

#[test]
fn bootstrap_complete_fixture_parses() {
    let fixture = include_str!("fixtures/bootstrap-complete.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    assert_eq!(message.op_type(), None);

    match &message {
        Message::BootstrapComplete(_) => {}
        _ => panic!("expected bootstrap complete"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn table_create_fixture_parses() {
    let fixture = include_str!("fixtures/table-create.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    match &message {
        Message::TableCreate(_) => {}
        _ => panic!("expected table create"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn table_alter_fixture_parses() {
    let fixture = include_str!("fixtures/table-alter.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    match &message {
        Message::TableAlter(_) => {}
        _ => panic!("expected table alter"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn table_drop_fixture_parses() {
    let fixture = include_str!("fixtures/table-drop.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    match &message {
        Message::TableDrop(_) => {}
        _ => panic!("expected table drop"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn database_create_fixture_parses() {
    let fixture = include_str!("fixtures/database-create.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    match &message {
        Message::DatabaseCreate(_) => {}
        _ => panic!("expected database create"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn database_alter_fixture_parses() {
    let fixture = include_str!("fixtures/database-alter.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    match &message {
        Message::DatabaseAlter(_) => {}
        _ => panic!("expected database alter"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}

#[test]
fn database_drop_fixture_parses() {
    let fixture = include_str!("fixtures/database-drop.json");
    let original: Value = serde_json::from_str(fixture).expect("parse fixture");

    let message: Message = parse(fixture).expect("parse message");
    match &message {
        Message::DatabaseDrop(_) => {}
        _ => panic!("expected database drop"),
    }

    let serialized = serde_json::to_value(&message).expect("serialize");
    assert_eq!(
        serialized, original,
        "serialized message must match fixture"
    );
}
