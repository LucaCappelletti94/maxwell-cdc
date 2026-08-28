#![no_main]

use libfuzzer_sys::fuzz_target;
use maxwell_cdc::{parse, Message};
use serde_json::to_string;

fuzz_target!(|data: &[u8]| {
    let Ok(json_str) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(message) = parse(json_str) else {
        return;
    };

    let serialized = to_string(&message).expect("serialize parsed message");
    let reparsed = parse(&serialized).expect("reparse serialized message");

    assert_eq!(message, reparsed, "reparsed message differs from original");
});
