//! The parsing entry points.

use crate::error::{LineError, ParseError};
use crate::message::Message;
use alloc::string::ToString;
use serde_json::Value;

/// Every `type` tag Maxwell emits. Used only to tell an unrecognised type apart from
/// malformed input, which `serde_json` reports as the same error category.
const MESSAGE_TAGS: [&str; 12] = [
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

/// Distinguish an unrecognised `type` tag from every other data error.
fn classify(json_error: serde_json::Error, tag: Option<&str>) -> ParseError {
    match tag {
        Some(tag) if !MESSAGE_TAGS.contains(&tag) => {
            ParseError::UnknownMessageType(tag.to_string())
        }
        _ => ParseError::Json(json_error),
    }
}

/// Read the `type` tag without committing to a message shape.
fn peek_tag(value: &Value) -> Option<&str> {
    value.get("type")?.as_str()
}

/// Parse a Maxwell CDC JSON string into a typed message.
///
/// # Errors
///
/// [`ParseError::UnknownMessageType`] when the `type` tag is not one this crate models, and
/// [`ParseError::Json`] when the input is not valid JSON or not a valid message of its type.
pub fn parse(json: &str) -> Result<Message, ParseError> {
    match serde_json::from_str(json) {
        Ok(message) => Ok(message),
        Err(e) => Err(match serde_json::from_str::<Value>(json) {
            Ok(value) => classify(e, peek_tag(&value)),
            Err(_) => ParseError::Json(e),
        }),
    }
}

/// Parse a Maxwell CDC JSON message from bytes, skipping the UTF-8 validation a `&str`
/// would need.
///
/// # Errors
///
/// As [`parse`], plus [`ParseError::Json`] when the bytes are not valid UTF-8.
pub fn parse_slice(json: &[u8]) -> Result<Message, ParseError> {
    match serde_json::from_slice(json) {
        Ok(message) => Ok(message),
        Err(e) => Err(match serde_json::from_slice::<Value>(json) {
            Ok(value) => classify(e, peek_tag(&value)),
            Err(_) => ParseError::Json(e),
        }),
    }
}

/// Parse a JSON Lines stream, the shape Maxwell's file and stdout producers write.
///
/// Yields one result per line that carries content, so a single bad line does not end the
/// stream and the caller decides whether to skip it. Empty and whitespace-only lines are
/// ignored, and line numbers count every line including those.
///
/// ```
/// let stream = concat!(
///     r#"{"type":"insert","database":"d","table":"t","data":{"id":1}}"#,
///     "\n",
///     r#"{"type":"delete","database":"d","table":"t","data":{"id":1}}"#,
/// );
///
/// let messages: Result<Vec<_>, _> = maxwell_cdc::parse_lines(stream).collect();
/// assert_eq!(messages.unwrap().len(), 2);
/// ```
pub fn parse_lines(json: &str) -> impl Iterator<Item = Result<Message, LineError>> + '_ {
    json.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            parse(line).map_err(|source| LineError {
                line: index + 1,
                source,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::{MESSAGE_TAGS, ParseError, parse};
    use alloc::format;

    /// Every tag in `MESSAGE_TAGS` must name a real variant.
    ///
    /// A tag-only document is missing every required field, so a modelled tag fails with
    /// [`ParseError::Json`]. Getting [`ParseError::UnknownMessageType`] instead means the
    /// array lists a tag the enum does not have.
    #[test]
    fn message_tags_match_the_variants() {
        for tag in MESSAGE_TAGS {
            let json = format!(r#"{{"type":"{tag}"}}"#);
            match parse(&json) {
                Err(ParseError::Json(_)) => {}
                Err(ParseError::UnknownMessageType(unknown)) => {
                    panic!("`{unknown}` is in MESSAGE_TAGS but is not a Message variant")
                }
                Ok(message) => panic!("`{tag}` parsed with no fields at all: {message:?}"),
            }
        }
    }

    /// A tag outside the array must be reported as unknown, not as malformed input.
    #[test]
    fn a_tag_outside_the_array_is_unknown() {
        let error = parse(r#"{"type":"table-rename"}"#).expect_err("must not parse");
        assert!(matches!(error, ParseError::UnknownMessageType(_)));
    }
}
