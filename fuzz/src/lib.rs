//! Properties the fuzz targets assert about `maxwell-cdc`, shared so every target checks each
//! message the same way.

use maxwell_cdc::{Message, ParseError, parse, parse_slice};
use serde_json::Value;

/// Every `type` tag Maxwell writes.
const TAGS: [&str; 12] = [
    "insert",
    "update",
    "delete",
    "bootstrap-insert",
    "bootstrap-start",
    "bootstrap-complete",
    "table-create",
    "table-alter",
    "table-drop",
    "database-create",
    "database-alter",
    "database-drop",
];

/// The tags of the messages that carry a row, and so an operation type.
const ROW_TAGS: [&str; 4] = ["insert", "update", "delete", "bootstrap-insert"];

/// Checks one input through both entry points.
pub fn check_bytes(bytes: &[u8]) {
    match core::str::from_utf8(bytes) {
        Ok(text) => {
            let _ = check(text);
        }
        Err(_) => match parse_slice(bytes) {
            Ok(message) => panic!("parse_slice accepted invalid UTF-8 {bytes:?} as {message:?}"),
            Err(error) => assert!(
                matches!(error, ParseError::Json(_)),
                "invalid UTF-8 {bytes:?} reported as {error}"
            ),
        },
    }
}

/// Checks one message and returns what [`parse`] made of it.
pub fn check(text: &str) -> Result<Message, ParseError> {
    let result = parse(text);
    assert_same(
        &result,
        &parse_slice(text.as_bytes()),
        "parse and parse_slice",
        text,
    );

    let value: Option<Value> = serde_json::from_str(text).ok();
    let declared = value.as_ref().and_then(declared_tag);
    let unknown = declared.filter(|tag| !TAGS.contains(tag));

    match &result {
        Ok(message) => {
            check_accepted(message, declared, text);
        }
        Err(ParseError::UnknownMessageType(tag)) => assert_eq!(
            unknown,
            Some(tag.as_str()),
            "UnknownMessageType({tag:?}) for {text:?}"
        ),
        Err(error @ ParseError::Json(_)) => assert!(
            unknown.is_none(),
            "unknown tag {unknown:?} in {text:?} reported as {error}"
        ),
    }
    result
}

/// The `type` field of an object. Only an object declares a tag, so an accepted message must
/// have been one.
fn declared_tag(value: &Value) -> Option<&str> {
    value.as_object()?.get("type")?.as_str()
}

fn check_accepted(message: &Message, declared: Option<&str>, text: &str) {
    let tag = message.tag();
    assert_eq!(declared, Some(tag), "{text:?} parsed as {tag}");

    match message.op_type() {
        Some(op_type) => assert_eq!(
            serde_json::to_value(op_type).ok(),
            Some(Value::from(tag)),
            "{tag} carries op_type {op_type:?}"
        ),
        None => assert!(!ROW_TAGS.contains(&tag), "{tag} carries no op_type"),
    }

    // Anything this crate parses, it must be able to write back and read again. Failures
    // carry the offending payload, since the corpus entry alone does not show which of the
    // two steps broke.
    let serialized = serde_json::to_string(message)
        .unwrap_or_else(|e| panic!("serialize failed for {text:?}: {e}"));
    let reparsed =
        parse(&serialized).unwrap_or_else(|e| panic!("reparse failed for {serialized:?}: {e}"));
    assert_eq!(
        message, &reparsed,
        "reparsed message differs, serialized as {serialized:?}"
    );
    let written: Value = serde_json::from_str(&serialized)
        .unwrap_or_else(|e| panic!("{serialized:?} is not JSON: {e}"));
    assert_eq!(
        written.get("type").and_then(Value::as_str),
        Some(tag),
        "{tag} written as {serialized:?}"
    );
}

/// Asserts two parses of the same input agree, errors included.
pub fn assert_same(
    left: &Result<Message, ParseError>,
    right: &Result<Message, ParseError>,
    what: &str,
    text: &str,
) {
    match (left, right) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "{what} disagree on {text:?}"),
        (Err(a), Err(b)) => {
            assert_eq!(
                core::mem::discriminant(a),
                core::mem::discriminant(b),
                "{what} disagree on {text:?}"
            );
            assert_eq!(a.to_string(), b.to_string(), "{what} disagree on {text:?}");
        }
        _ => panic!("{what} disagree on {text:?}: {left:?} and {right:?}"),
    }
}
