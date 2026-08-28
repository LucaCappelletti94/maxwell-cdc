#![no_main]

use libfuzzer_sys::fuzz_target;
use maxwell_cdc::parse;
use serde_json::to_string;

fuzz_target!(|data: &[u8]| {
    let Ok(json_str) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(message) = parse(json_str) else {
        return;
    };

    // Anything this crate parses, it must be able to write back and read again. Failures
    // carry the offending payload, since the corpus entry alone does not show which of the
    // two steps broke.
    let serialized = to_string(&message)
        .unwrap_or_else(|e| panic!("serialize failed for {json_str:?}: {e}"));

    let reparsed = parse(&serialized)
        .unwrap_or_else(|e| panic!("reparse failed for {serialized:?}: {e}"));

    assert_eq!(
        message, reparsed,
        "reparsed message differs, serialized as {serialized:?}"
    );
});
