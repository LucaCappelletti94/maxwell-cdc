//! The message enum and its `type` tag.

use crate::row::{ControlMessage, RowChange};
use crate::schema::{
    DatabaseChange, DatabaseDropChange, TableAlterChange, TableCreateChange, TableDropChange,
};
use serde::{Deserialize, Serialize};

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
    /// The operation type if this is a row change, else `None`. Exhaustive, so a new
    /// row-carrying variant fails to compile here instead of silently reporting nothing.
    #[must_use]
    pub const fn op_type(&self) -> Option<OpType> {
        match self {
            Self::Insert(_) => Some(OpType::Insert),
            Self::Update(_) => Some(OpType::Update),
            Self::Delete(_) => Some(OpType::Delete),
            Self::BootstrapInsert(_) => Some(OpType::BootstrapInsert),
            Self::BootstrapStart(_)
            | Self::BootstrapComplete(_)
            | Self::TableCreate(_)
            | Self::TableAlter(_)
            | Self::TableDrop(_)
            | Self::DatabaseCreate(_)
            | Self::DatabaseAlter(_)
            | Self::DatabaseDrop(_) => None,
        }
    }

    /// The `type` tag Maxwell writes for this variant. Exhaustive, so a new variant cannot
    /// be added without deciding its tag.
    #[must_use]
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::Insert(_) => "insert",
            Self::Update(_) => "update",
            Self::Delete(_) => "delete",
            Self::BootstrapInsert(_) => "bootstrap-insert",
            Self::BootstrapStart(_) => "bootstrap-start",
            Self::BootstrapComplete(_) => "bootstrap-complete",
            Self::TableCreate(_) => "table-create",
            Self::TableAlter(_) => "table-alter",
            Self::TableDrop(_) => "table-drop",
            Self::DatabaseCreate(_) => "database-create",
            Self::DatabaseAlter(_) => "database-alter",
            Self::DatabaseDrop(_) => "database-drop",
        }
    }
}
