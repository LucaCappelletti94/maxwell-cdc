#![no_std]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod error;
mod message;
mod parse;
mod row;
mod schema;

pub use error::{LineError, ParseError};
pub use message::{Message, OpType};
pub use parse::{parse, parse_lines, parse_slice};
pub use row::{ControlMessage, RowChange};
pub use schema::{
    ColumnDefinition, DatabaseChange, DatabaseDefinition, DatabaseDropChange, DdlMetadata,
    TableAlterChange, TableCreateChange, TableDefinition, TableDropChange,
};
