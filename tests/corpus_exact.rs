//! Corpus tests asserting full field preservation against Maxwell's own wire bytes.

mod support;

use maxwell_cdc::{Message, OpType, ParseError, parse};
use serde_json::{Value, json};

/// The operation type a message with this `type` tag must report.
fn expected_op_type(tag: &str) -> Option<OpType> {
    match tag {
        "insert" => Some(OpType::Insert),
        "update" => Some(OpType::Update),
        "delete" => Some(OpType::Delete),
        "bootstrap-insert" => Some(OpType::BootstrapInsert),
        _ => None,
    }
}

/// `Message` is `Eq + Hash`, so consumers can deduplicate a stream through a set.
#[test]
fn messages_deduplicate_through_a_hash_set() {
    let fixtures = support::all();

    let mut seen = std::collections::HashSet::new();
    for (name, raw) in &fixtures {
        let message = parse(raw).expect("parse message");
        assert!(
            seen.insert(message.clone()),
            "{name}: distinct message collided"
        );
        assert!(!seen.insert(message), "{name}: duplicate was not detected");
    }

    assert_eq!(seen.len(), fixtures.len());
}

/// The catch-all must stay empty for messages this crate fully models.
///
/// The real guard against a dropped field, which the reserialization test cannot see. If it
/// fills up, either Maxwell added a field worth modelling or the catch-all is swallowing one
/// the crate already has. Blind to a variant with no fixture.
#[test]
fn nothing_in_the_corpus_lands_in_the_catch_all() {
    for (name, raw) in support::all() {
        let message = parse(&raw).expect("parse message");

        let extra = match &message {
            Message::Insert(row)
            | Message::Update(row)
            | Message::Delete(row)
            | Message::BootstrapInsert(row) => &row.extra,
            Message::BootstrapStart(control) | Message::BootstrapComplete(control) => {
                &control.extra
            }
            Message::TableCreate(change) => &change.extra,
            Message::TableAlter(change) => &change.extra,
            Message::TableDrop(change) => &change.extra,
            Message::DatabaseCreate(change) | Message::DatabaseAlter(change) => &change.extra,
            Message::DatabaseDrop(change) => &change.extra,
            _ => unreachable!("{name}: unhandled variant"),
        };

        assert!(
            extra.is_empty(),
            "{name}: unmodelled fields landed in the catch-all: {extra:?}"
        );
    }
}

/// Every tag in the corpus must be recognised by the error classifier.
///
/// A modelled tag missing from the internal list still parses when the payload is valid, so
/// only a broken payload reveals it, misblamed on the message type. Stripping each fixture
/// to its tag is the cheapest way to reach that path.
#[test]
fn a_broken_payload_is_never_blamed_on_its_message_type() {
    for (name, raw) in support::all() {
        let original: Value = serde_json::from_str(&raw).expect("parse fixture");
        let tag = original["type"].as_str().expect("type tag");

        let tag_only = json!({ "type": tag }).to_string();

        match parse(&tag_only) {
            Err(ParseError::Json(_)) => {}
            Err(ParseError::UnknownMessageType(reported)) => panic!(
                "{name}: `{reported}` is a modelled type but the classifier does not know it"
            ),
            Ok(message) => panic!("{name}: parsed with no fields at all: {message:?}"),
        }
    }
}

/// A field this crate does not know must survive parsing and reserialization.
#[test]
fn unmodelled_fields_survive_a_roundtrip() {
    let json = concat!(
        r#"{"type":"insert","database":"d","table":"t","data":{"id":1},"#,
        r#""injected_by_a_script":"keep me","future_field":42}"#
    );

    let message = parse(json).expect("unknown fields must not block parsing");

    let Message::Insert(row) = &message else {
        panic!("expected insert");
    };
    assert_eq!(
        row.extra.get("injected_by_a_script"),
        Some(&json!("keep me"))
    );
    assert_eq!(row.extra.get("future_field"), Some(&json!(42)));

    let reserialized: Value = serde_json::from_str(&serde_json::to_string(&message).unwrap())
        .expect("reserialized must be valid json");
    let original: Value = serde_json::from_str(json).expect("original must be valid json");

    assert_eq!(
        reserialized, original,
        "an unmodelled field must not be dropped"
    );
}

/// Reserializing a parsed message must reproduce every field Maxwell emitted.
///
/// Narrow on its own: the `extra` catch-all puts a dropped field back, so this cannot see
/// one. It catches an invented field and a mangled value.
/// `nothing_in_the_corpus_lands_in_the_catch_all` catches the dropped ones, so the pair is
/// what proves the model complete.
#[test]
fn every_fixture_reserializes_to_the_bytes_maxwell_emitted() {
    for (name, raw) in support::all() {
        let original: Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse fixture {name}: {e}"));

        let message = parse(&raw).unwrap_or_else(|e| panic!("parse message {name}: {e}"));

        let reserialized =
            serde_json::to_value(&message).unwrap_or_else(|e| panic!("serialize {name}: {e}"));

        assert_eq!(
            reserialized, original,
            "{name}: reserialized message dropped or invented fields"
        );
    }
}

/// `op_type` must report an operation exactly for the four row-carrying tags.
#[test]
fn every_fixture_reports_the_op_type_its_tag_implies() {
    for (name, raw) in support::all() {
        let original: Value = serde_json::from_str(&raw).expect("parse fixture");
        let tag = original["type"].as_str().expect("type tag");

        let message = parse(&raw).expect("parse message");

        assert_eq!(
            message.op_type(),
            expected_op_type(tag),
            "{name}: op_type disagrees with the {tag} tag"
        );
    }
}

/// Parsing, serializing and reparsing must reach a fixed point.
#[test]
fn every_fixture_survives_a_serialize_reparse_cycle() {
    for (name, raw) in support::all() {
        let message = parse(&raw).expect("parse message");

        let serialized = serde_json::to_string(&message).expect("serialize");
        let reparsed = parse(&serialized).unwrap_or_else(|e| panic!("reparse {name}: {e}"));
        let reserialized = serde_json::to_string(&reparsed).expect("reserialize");

        assert_eq!(message, reparsed, "{name}: reparsed message differs");
        assert_eq!(
            serialized, reserialized,
            "{name}: reserialization not stable"
        );
    }
}

/// Every fixture must land in the variant its tag names.
#[test]
fn every_fixture_lands_in_the_variant_its_tag_names() {
    for (name, raw) in support::all() {
        let original: Value = serde_json::from_str(&raw).expect("parse fixture");
        let tag = original["type"].as_str().expect("type tag");

        let message = parse(&raw).expect("parse message");

        let matched = matches!(
            (tag, &message),
            ("insert", Message::Insert(_))
                | ("update", Message::Update(_))
                | ("delete", Message::Delete(_))
                | ("bootstrap-insert", Message::BootstrapInsert(_))
                | ("bootstrap-start", Message::BootstrapStart(_))
                | ("bootstrap-complete", Message::BootstrapComplete(_))
                | ("table-create", Message::TableCreate(_))
                | ("table-alter", Message::TableAlter(_))
                | ("table-drop", Message::TableDrop(_))
                | ("database-create", Message::DatabaseCreate(_))
                | ("database-alter", Message::DatabaseAlter(_))
                | ("database-drop", Message::DatabaseDrop(_))
        );

        assert!(matched, "{name}: tag {tag} landed in {message:?}");
    }
}

/// An update must keep the previous values Maxwell put in `old`.
#[test]
fn row_update_fixture_preserves_old_values() {
    let message = parse(&support::get("row-update")).expect("parse message");

    let Message::Update(row) = &message else {
        panic!("expected update");
    };

    let old = row.old.as_ref().expect("update must carry old values");
    assert_eq!(old.get("amount"), Some(&json!(1)));
    assert_eq!(old.get("nullable_text"), Some(&json!(null)));
    assert_eq!(row.data.get("amount"), Some(&json!(2)));
    assert_eq!(row.data.get("nullable_text"), Some(&json!("filled")));
}

/// A `BIGINT UNSIGNED` above `i64::MAX` must survive as an unsigned integer.
#[test]
fn bootstrap_insert_fixture_keeps_unsigned_bigint_precision() {
    let message = parse(&support::get("bootstrap-insert")).expect("parse message");

    let Message::BootstrapInsert(row) = &message else {
        panic!("expected bootstrap insert");
    };

    assert_eq!(
        row.data.get("id"),
        Some(&json!(9_223_372_036_854_775_809_u64))
    );
    assert_eq!(row.data.get("status"), Some(&json!("closed")));
}

/// Binlog metadata must reach the typed fields rather than being dropped.
#[test]
fn row_insert_fixture_exposes_binlog_metadata() {
    let message = parse(&support::get("row-insert")).expect("parse message");

    let Message::Insert(row) = &message else {
        panic!("expected insert");
    };

    assert_eq!(row.database, "testdb");
    assert_eq!(row.table, "capture_events");
    assert!(row.ts.is_some(), "ts must be present");
    assert!(row.xid.is_some(), "xid must be present");
    assert_eq!(row.commit, Some(true));
    assert!(row.position.is_some(), "position must be present");
    assert!(row.gtid.is_some(), "gtid must be present");
    assert!(row.schema_id.is_some(), "schema_id must be present");
    assert!(row.push_ts.is_some(), "push_ts must be present");
    assert!(row.query.is_some(), "query must be present");
}
