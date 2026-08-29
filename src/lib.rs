#![no_std]
#![doc = include_str!("../README.md")]
//!
//! Every payload struct derives [`Default`] so a caller can name the fields it cares about
//! and finish with `..Default::default()`, which also keeps a later field addition from
//! breaking construction. A defaulted value is a starting point for building one, not a
//! valid Maxwell message: the required fields come back empty.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use thiserror::Error;

/// Every `type` tag Maxwell emits, in [`Message`] variant order.
///
/// Used only to tell an unrecognised message type apart from malformed input, which
/// `serde_json` reports as the same error category. Kept in step with the enum by
/// [`Message::tag`] and the `message_tags_match_the_variants` test below.
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

/// Why a Maxwell message failed to parse.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The message carries a `type` this crate does not model, most likely because it was
    /// added by a newer Maxwell. Callers tailing a stream may choose to skip these.
    #[error("unrecognised Maxwell message type `{0}`")]
    UnknownMessageType(String),
    /// The input is not valid JSON, or is not a valid message of its declared type.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// A parse failure at a known line of a JSON Lines stream.
#[derive(Debug, Error)]
#[error("line {line}: {source}")]
pub struct LineError {
    /// Line number, counting from one.
    pub line: usize,
    /// The underlying failure.
    #[source]
    pub source: ParseError,
}

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

/// Parse a Maxwell CDC JSON message from bytes.
///
/// Prefer this over [`parse`] when the input is already a byte buffer, since it skips the
/// UTF-8 validation a `&str` would require.
///
/// # Errors
///
/// As [`parse`], plus a [`ParseError::Json`] when the bytes are not valid UTF-8.
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

/// The operation type for row change messages.
///
/// Non-exhaustive for the same reason as [`Message`]: a new row-carrying message type would
/// otherwise make [`Message::op_type`] return a variant that breaks existing matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum OpType {
    /// Row insertion.
    Insert,
    /// Row update.
    Update,
    /// Row deletion.
    Delete,
    /// Bootstrap row insert (initial snapshot).
    BootstrapInsert,
}

/// A Maxwell CDC message parsed from JSON.
///
/// Non-exhaustive: Maxwell has added message types before, and a caller matching on this
/// enum must carry a catch-all arm so a new one is not a breaking change here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Message {
    /// Row insertion into a table.
    Insert(RowChange),
    /// Row update in a table.
    Update(RowChange),
    /// Row deletion from a table.
    Delete(RowChange),
    /// Row inserted during bootstrap (initial snapshot).
    BootstrapInsert(RowChange),
    /// Start of bootstrap period for a table.
    BootstrapStart(ControlMessage),
    /// End of bootstrap period for a table.
    BootstrapComplete(ControlMessage),
    /// Table created.
    TableCreate(TableCreateChange),
    /// Table altered.
    TableAlter(TableAlterChange),
    /// Table dropped.
    TableDrop(TableDropChange),
    /// Database created.
    DatabaseCreate(DatabaseChange),
    /// Database altered.
    DatabaseAlter(DatabaseChange),
    /// Database dropped.
    DatabaseDrop(DatabaseDropChange),
}

impl Message {
    /// The operation type if this is a row change, else `None`.
    ///
    /// Matched exhaustively on purpose: a new row-carrying variant must fail to compile
    /// here rather than silently report no operation.
    #[must_use]
    pub fn op_type(&self) -> Option<OpType> {
        match self {
            Message::Insert(_) => Some(OpType::Insert),
            Message::Update(_) => Some(OpType::Update),
            Message::Delete(_) => Some(OpType::Delete),
            Message::BootstrapInsert(_) => Some(OpType::BootstrapInsert),
            Message::BootstrapStart(_)
            | Message::BootstrapComplete(_)
            | Message::TableCreate(_)
            | Message::TableAlter(_)
            | Message::TableDrop(_)
            | Message::DatabaseCreate(_)
            | Message::DatabaseAlter(_)
            | Message::DatabaseDrop(_) => None,
        }
    }

    /// The `type` tag Maxwell writes for this variant.
    ///
    /// Matched exhaustively so a new variant cannot be added without deciding its tag, which
    /// is what keeps the crate's internal tag list from drifting away from the enum.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Message::Insert(_) => "insert",
            Message::Update(_) => "update",
            Message::Delete(_) => "delete",
            Message::BootstrapInsert(_) => "bootstrap-insert",
            Message::BootstrapStart(_) => "bootstrap-start",
            Message::BootstrapComplete(_) => "bootstrap-complete",
            Message::TableCreate(_) => "table-create",
            Message::TableAlter(_) => "table-alter",
            Message::TableDrop(_) => "table-drop",
            Message::DatabaseCreate(_) => "database-create",
            Message::DatabaseAlter(_) => "database-alter",
            Message::DatabaseDrop(_) => "database-drop",
        }
    }
}

/// A row change message (insert, update, delete, or bootstrap insert).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RowChange {
    /// Database name.
    pub database: String,
    /// Table name.
    pub table: String,
    /// SQL statement that produced this row, present when `output_row_query` is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Transaction timestamp in seconds since epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    /// Transaction ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xid: Option<i64>,
    /// Offset of this row within its transaction, present on uncommitted rows when
    /// `output_xoffset` is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xoffset: Option<i64>,
    /// Whether the transaction was committed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<bool>,
    /// Binlog position (filename:offset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// Global transaction ID, present when `output_gtid_position` is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gtid: Option<String>,
    /// MySQL server ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    /// Thread ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<i64>,
    /// Schema ID, present when `output_schema_id` is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<i64>,
    /// When Maxwell produced the message, in seconds with a fractional part. A [`Number`]
    /// because Maxwell writes it as an arbitrary-precision decimal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_ts: Option<Number>,
    /// Comment attached to the originating bootstrap task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Primary key values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<Vec<Value>>,
    /// Primary key column names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key_columns: Option<Vec<String>>,
    /// Current row data as JSON object.
    ///
    /// A MySQL `DECIMAL` wider than `f64` loses digits here, because [`Value`] holds
    /// fractional numbers as `f64`. Column order is not preserved either.
    pub data: Map<String, Value>,
    /// Previous row data (for updates only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old: Option<Map<String, Value>>,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A control message (bootstrap start/complete).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlMessage {
    /// Database name.
    pub database: String,
    /// Table name.
    pub table: String,
    /// Event timestamp in seconds since epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    /// When Maxwell produced the message, in seconds with a fractional part. A [`Number`]
    /// because Maxwell writes it as an arbitrary-precision decimal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_ts: Option<Number>,
    /// Comment attached to the originating bootstrap task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Event data (typically empty for control messages).
    pub data: Map<String, Value>,
    /// Primary key values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<Vec<Value>>,
    /// Primary key column names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key_columns: Option<Vec<String>>,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Metadata common to DDL messages.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DdlMetadata {
    /// Event timestamp in milliseconds since epoch.
    pub ts: i64,
    /// The DDL SQL statement.
    pub sql: String,
    /// Binlog position (filename:offset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    /// Global transaction ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gtid: Option<String>,
    /// Schema ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<i64>,
}

/// A column definition in a table.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnDefinition {
    /// Column name.
    pub name: String,
    /// Column data type.
    #[serde(rename = "type")]
    pub column_type: String,
    /// Character set for text columns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
    /// Whether the column is signed (for numeric types).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed: Option<bool>,
    /// Enum values for ENUM columns.
    #[serde(rename = "enum-values", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// Fractional-second precision, on the types that carry one (`DATETIME`, `TIMESTAMP`,
    /// `TIME`). An `i64` because Maxwell writes a Java `Long`, though real values are 0 to 6.
    #[serde(rename = "column-length", skip_serializing_if = "Option::is_none")]
    pub column_length: Option<i64>,
}

/// A table definition in a database.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableDefinition {
    /// Database name.
    pub database: String,
    /// Table name.
    pub table: String,
    /// Table character set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
    /// Primary key column names.
    #[serde(rename = "primary-key")]
    pub primary_key: Vec<String>,
    /// Column definitions.
    pub columns: Vec<ColumnDefinition>,
}

/// A database definition.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseDefinition {
    /// Database name.
    pub database: String,
    /// Database character set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
}

/// A table creation message.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableCreateChange {
    /// Database name.
    pub database: String,
    /// Table name.
    pub table: String,
    /// New table definition.
    #[serde(rename = "def")]
    pub definition: TableDefinition,
    /// DDL metadata.
    #[serde(flatten)]
    pub metadata: DdlMetadata,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A table alteration message.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableAlterChange {
    /// Database name.
    pub database: String,
    /// Table name.
    pub table: String,
    /// Old table definition before alteration.
    #[serde(rename = "old")]
    pub old_definition: TableDefinition,
    /// New table definition after alteration.
    #[serde(rename = "def")]
    pub definition: TableDefinition,
    /// DDL metadata.
    #[serde(flatten)]
    pub metadata: DdlMetadata,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A table drop message.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableDropChange {
    /// Database name.
    pub database: String,
    /// Table name.
    pub table: String,
    /// DDL metadata.
    #[serde(flatten)]
    pub metadata: DdlMetadata,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A database creation or alteration message.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseChange {
    /// Database definition.
    #[serde(flatten)]
    pub definition: DatabaseDefinition,
    /// DDL metadata.
    #[serde(flatten)]
    pub metadata: DdlMetadata,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A database drop message.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseDropChange {
    /// Database name.
    pub database: String,
    /// DDL metadata.
    #[serde(flatten)]
    pub metadata: DdlMetadata,
    /// Any field Maxwell emitted that this crate does not model, kept verbatim so nothing
    /// is lost on the way through. Populated by Maxwell scripting hooks and by fields
    /// added after this crate's last release.
    ///
    /// Keys here must not collide with the named fields above. Serializing a colliding key
    /// writes the JSON object twice over, and the result no longer parses.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
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
