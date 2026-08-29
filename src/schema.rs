//! Schema-change payloads.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
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
    /// Fields Maxwell emitted that this crate does not model, kept verbatim. Keys must not
    /// collide with the named fields above, or serializing writes them twice.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
