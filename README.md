# maxwell-cdc

[![Crates.io](https://img.shields.io/crates/v/maxwell-cdc.svg)](https://crates.io/crates/maxwell-cdc)
[![Documentation](https://docs.rs/maxwell-cdc/badge.svg)](https://docs.rs/maxwell-cdc)
[![CI](https://github.com/LucaCappelletti94/maxwell-cdc/actions/workflows/ci.yml/badge.svg)](https://github.com/LucaCappelletti94/maxwell-cdc/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/LucaCappelletti94/maxwell-cdc/branch/main/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/maxwell-cdc)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/LucaCappelletti94/maxwell-cdc/blob/main/LICENSE)

`maxwell-cdc` turns the JSON that [Maxwell's Daemon](https://maxwells-daemon.io/) writes into typed
Rust messages. It parses bytes you already have, so it never touches MySQL. `no_std`, needs an
allocator.

Maxwell emits one JSON object per change, in twelve kinds: four carry a row, two bracket a bootstrap
snapshot, six describe a schema change. Each kind is its own variant, so a field is optional here
only when Maxwell may omit it.

```rust
use maxwell_cdc::{Message, OpType, parse};

let json = r#"{"database":"shop","table":"orders","type":"insert","ts":1477053217,"data":{"id":1,"total":"9.99"}}"#;
let message = parse(json)?;

assert_eq!(message.op_type(), Some(OpType::Insert));
assert_eq!(message.tag(), "insert");

let Message::Insert(row) = &message else { panic!("expected an insert") };
assert_eq!(row.table, "orders");
assert_eq!(row.data["total"], "9.99");
# Ok::<(), maxwell_cdc::ParseError>(())
```

## Updates carry the previous values

Maxwell puts only the changed columns in `old`, so a diff is the intersection of the two maps.

```rust
use maxwell_cdc::{Message, parse};

let json = r#"{"database":"shop","table":"orders","type":"update","ts":1477053217,
               "data":{"id":1,"status":"paid"},"old":{"status":"open"}}"#;

let Message::Update(row) = parse(json)? else { panic!("expected an update") };

let old = row.old.as_ref().expect("an update carries old values");
assert_eq!(old["status"], "open");
assert_eq!(row.data["status"], "paid");
# Ok::<(), maxwell_cdc::ParseError>(())
```

## Reading a stream

Maxwell's file and stdout producers write JSON Lines. `parse_lines` yields one result per line that
carries content, so one bad line does not end the stream, and the error names the line it came from.

```rust
use maxwell_cdc::parse_lines;

let stream = concat!(
    r#"{"database":"shop","table":"orders","type":"insert","data":{"id":1}}"#, "\n",
    "\n",
    "{ this line is not json\n",
    r#"{"database":"shop","table":"orders","type":"delete","data":{"id":1}}"#, "\n",
);

let (good, bad): (Vec<_>, Vec<_>) = parse_lines(stream).partition(Result::is_ok);
assert_eq!(good.len(), 2);

let error = bad[0].as_ref().unwrap_err();
assert_eq!(error.line, 3, "blank lines still count towards the line number");
```

## Telling a new Maxwell apart from a broken payload

Both arrive as a failure to deserialize, and `serde_json` reports them identically. `ParseError`
separates them, so a consumer can skip a message type it does not know while still failing loudly on
corrupt input.

```rust
use maxwell_cdc::{ParseError, parse};

// A type this crate does not model, most likely from a newer Maxwell.
let unknown = parse(r#"{"type":"table-rename","database":"shop","table":"orders"}"#);
assert!(matches!(unknown, Err(ParseError::UnknownMessageType(tag)) if tag == "table-rename"));

// A type it does model, with a payload that is missing required fields.
let broken = parse(r#"{"type":"insert","database":"shop"}"#);
assert!(matches!(broken, Err(ParseError::Json(_))));
```

## Nothing is silently dropped

Maxwell has many output flags, and its scripting hooks can inject arbitrary keys. Any field the model
does not name is kept verbatim in `extra`, so a message survives a round trip even when this crate is
older than the Maxwell that produced it.

```rust
use maxwell_cdc::{Message, parse};

let json = r#"{"database":"shop","table":"orders","type":"insert","data":{"id":1},
               "injected_by_a_script":"keep me"}"#;

let Message::Insert(row) = parse(json)? else { panic!("expected an insert") };
assert_eq!(row.extra["injected_by_a_script"], "keep me");

let written = serde_json::to_value(parse(json)?)?;
assert_eq!(written["injected_by_a_script"], "keep me");
# Ok::<(), maxwell_cdc::ParseError>(())
```

## Building a message

Every payload struct derives `Default`, so a test or a translation layer names the fields it cares
about and lets the rest default. This also means a field added in a later release does not break
construction.

```rust
use maxwell_cdc::{Message, RowChange};

let row = RowChange {
    database: "shop".to_owned(),
    table: "orders".to_owned(),
    ts: Some(1_477_053_217),
    ..Default::default()
};

let json = serde_json::to_string(&Message::Insert(row))?;
assert_eq!(
    json,
    r#"{"type":"insert","database":"shop","table":"orders","ts":1477053217,"data":{}}"#
);
# Ok::<(), serde_json::Error>(())
```

A defaulted value is a starting point, not a valid Maxwell message: the required fields come back
empty.

## Column order

`data` and `old` are a `serde_json::Map`, which by default sorts keys and therefore loses the column
order Maxwell emitted. Enable `serde_json`'s `preserve_order` feature in your own build to keep it.

## Types

`Message` and `OpType` are `#[non_exhaustive]`, because Maxwell has added message types before and
will again. Match with a catch-all arm. Every payload struct derives `Eq` and `Hash`, so messages can
be deduplicated through a set.

Row timestamps (`ts`) are in seconds, schema-change timestamps in milliseconds. That is Maxwell's
choice. A MySQL `DECIMAL` wider than an `f64` loses digits, because `serde_json` holds fractional
numbers as `f64`.
