#![no_main]

use libfuzzer_sys::fuzz_target;
use maxwell_cdc::{parse, parse_lines};
use serde_json::to_string;

fuzz_target!(|data: &[u8]| {
    let Ok(json_str) = std::str::from_utf8(data) else {
        return;
    };

    for result in parse_lines(json_str) {
        let Ok(message) = result else {
            // parse_lines yielded a LineError; that is expected for bad lines.
            continue;
        };

        // Whatever parse_lines accepts, round-trip through serialize + parse must
        // produce the identical value. The payload is included in the panic so the
        // corpus entry alone is enough to reproduce the failure.
        let serialized = to_string(&message)
            .unwrap_or_else(|e| panic!("serialize failed for {json_str:?}: {e}"));

        let reparsed = parse(&serialized)
            .unwrap_or_else(|e| panic!("reparse failed for {serialized:?}: {e}"));

        assert_eq!(
            message, reparsed,
            "reparsed message differs, serialized as {serialized:?}"
        );
    }
});
