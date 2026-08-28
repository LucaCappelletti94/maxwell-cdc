//! Property test proving Message serialization and parsing are inverse operations.

use maxwell_cdc::{Message, parse};
use proptest::prelude::*;

fn arb_fixture() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just(include_str!("fixtures/row-insert.json")),
        Just(include_str!("fixtures/row-update.json")),
        Just(include_str!("fixtures/row-delete.json")),
        Just(include_str!("fixtures/bootstrap-insert.json")),
        Just(include_str!("fixtures/bootstrap-start.json")),
        Just(include_str!("fixtures/bootstrap-complete.json")),
        Just(include_str!("fixtures/table-create.json")),
        Just(include_str!("fixtures/table-alter.json")),
        Just(include_str!("fixtures/table-drop.json")),
        Just(include_str!("fixtures/database-create.json")),
        Just(include_str!("fixtures/database-alter.json")),
        Just(include_str!("fixtures/database-drop.json")),
    ]
}

proptest! {
    #[test]
    fn fixture_message_serialize_then_parse_is_identity(fixture in arb_fixture()) {
        let parsed_message: Message = parse(fixture)
            .expect("parse fixture");

        let serialized_json = serde_json::to_string(&parsed_message)
            .expect("serialize message");

        let reparsed_message: Message = parse(&serialized_json)
            .expect("reparse serialized");
        let re_serialized = serde_json::to_string(&reparsed_message)
            .expect("reserialize reparsed");

        prop_assert_eq!(
            parsed_message, reparsed_message,
            "messages must be equal after roundtrip"
        );
        prop_assert_eq!(
            serialized_json, re_serialized,
            "serialized messages must be bitwise identical after roundtrip"
        );
    }
}
