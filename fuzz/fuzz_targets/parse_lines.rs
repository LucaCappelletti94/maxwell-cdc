#![no_main]

use libfuzzer_sys::fuzz_target;
use maxwell_cdc::{parse, parse_lines};
use maxwell_cdc_fuzz::assert_same;

fuzz_target!(|data: &[u8]| {
    let Ok(stream) = std::str::from_utf8(data) else {
        return;
    };

    // The documented contract, spelled out independently of `str::lines`: a line ends at
    // `\n`, a `\r` right before it belongs to the terminator, numbering counts every line,
    // and lines that are empty after trimming yield nothing.
    let mut expected = Vec::new();
    let mut rest = stream;
    let mut number = 0;
    while !rest.is_empty() {
        number += 1;
        let line = match rest.split_once('\n') {
            Some((line, tail)) => {
                rest = tail;
                line.strip_suffix('\r').unwrap_or(line)
            }
            None => core::mem::take(&mut rest),
        };
        if !line.trim().is_empty() {
            expected.push((number, line));
        }
    }

    let yielded: Vec<_> = parse_lines(stream).collect();
    assert_eq!(
        yielded.len(),
        expected.len(),
        "parse_lines yielded {} results for {} content lines in {stream:?}",
        yielded.len(),
        expected.len()
    );

    // Each line must parse as it would alone. What a message must satisfy is the `parse`
    // target's job, so it is not repeated here.
    for (result, (number, line)) in yielded.into_iter().zip(expected) {
        let alone = parse(line);
        match result {
            Ok(message) => assert_same(&Ok(message), &alone, "parse_lines and parse", line),
            Err(error) => {
                assert_eq!(error.line, number, "line number for {line:?} in {stream:?}");
                assert_eq!(
                    error.to_string(),
                    format!("line {number}: {}", error.source),
                    "LineError display for {line:?}"
                );
                assert_same(&Err(error.source), &alone, "parse_lines and parse", line);
            }
        }
    }
});
