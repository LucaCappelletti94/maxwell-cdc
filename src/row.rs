//! Row and bootstrap-control payloads.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
