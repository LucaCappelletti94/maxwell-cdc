//! Error-path contract tests: what `parse` must reject, and how it reports why.

use maxwell_cdc::{ParseError, parse, parse_lines, parse_slice};

/// A tag this crate does not model must be named as such, so a caller tailing a stream can
/// skip a newer Maxwell's message types without also swallowing corrupt input.
#[test]
fn unknown_type_tag_is_reported_as_an_unknown_message_type() {
    let json = r#"{"type":"table-rename","database":"shop","table":"orders","ts":1477053217}"#;

    let error = parse(json).expect_err("unknown tag must not parse");

    match error {
        ParseError::UnknownMessageType(tag) => assert_eq!(tag, "table-rename"),
        ParseError::Json(e) => panic!("expected an unknown-type error, got a json error: {e}"),
    }
}

/// A tag this crate does model, with a bad payload, is a data error and not an unknown type.
#[test]
fn known_tag_with_a_bad_payload_is_a_json_error() {
    let json = r#"{"type":"insert","database":"shop","table":"orders"}"#;

    let error = parse(json).expect_err("missing data must not parse");

    assert!(
        matches!(error, ParseError::Json(_)),
        "a modelled tag with a bad payload must not be reported as an unknown type"
    );
}

/// An absent tag leaves the message unclassifiable.
#[test]
fn missing_type_tag_is_rejected() {
    let json = r#"{"database":"shop","table":"orders","ts":1477053217,"data":{}}"#;

    assert!(parse(json).is_err());
}

/// Structurally valid JSON that is not a Maxwell message must be rejected.
#[test]
fn non_message_json_is_rejected() {
    for json in ["{}", "null", "[]", "42", r#""insert""#] {
        assert!(parse(json).is_err(), "{json} must not parse as a message");
    }
}

/// Malformed JSON must surface as a syntax error.
#[test]
fn malformed_json_is_rejected() {
    for json in ["", "{", r#"{"type":"insert""#, r#"{"type":insert}"#] {
        assert!(parse(json).is_err(), "{json:?} must not parse");
    }
}

/// Row messages carry required identity and payload fields.
#[test]
fn row_message_requires_database_table_and_data() {
    let cases = [
        (
            "table",
            r#"{"type":"insert","database":"shop","ts":1,"data":{}}"#,
        ),
        (
            "database",
            r#"{"type":"insert","table":"orders","ts":1,"data":{}}"#,
        ),
        (
            "data",
            r#"{"type":"insert","database":"shop","table":"orders","ts":1}"#,
        ),
    ];

    for (missing, json) in cases {
        assert!(
            parse(json).is_err(),
            "a row message without {missing} must not parse"
        );
    }
}

/// DDL messages carry a required timestamp and statement.
#[test]
fn ddl_message_requires_ts_and_sql() {
    let missing_ts =
        r#"{"type":"database-drop","database":"archive","sql":"DROP DATABASE archive"}"#;
    let missing_sql = r#"{"type":"database-drop","database":"archive","ts":1477053308000}"#;

    assert!(parse(missing_ts).is_err(), "missing ts must not parse");
    assert!(parse(missing_sql).is_err(), "missing sql must not parse");
}

/// A duplicated tag is ambiguous and must be rejected rather than resolved by position.
#[test]
fn duplicate_type_tag_is_rejected() {
    let json = r#"{"type":"insert","type":"delete","database":"shop","table":"orders","data":{}}"#;

    assert!(parse(json).is_err());
}

/// `parse_slice` must agree with `parse`, including on why it failed.
#[test]
fn parse_slice_reports_the_same_failures() {
    let unknown = br#"{"type":"table-rename","database":"shop","table":"orders"}"#;
    assert!(matches!(
        parse_slice(unknown),
        Err(ParseError::UnknownMessageType(_))
    ));

    let malformed = br#"{"type":"insert""#;
    assert!(matches!(parse_slice(malformed), Err(ParseError::Json(_))));

    // Invalid UTF-8 is a data error, not a panic.
    assert!(parse_slice(&[0xff, 0xfe]).is_err());
}

/// A bad line must name its line number and must not end the stream.
#[test]
fn parse_lines_isolates_a_bad_line() {
    let stream = concat!(
        r#"{"type":"insert","database":"d","table":"t","data":{"id":1}}"#,
        "\n",
        "not json\n",
        "\n",
        r#"{"type":"delete","database":"d","table":"t","data":{"id":1}}"#,
        "\n"
    );

    let results: Vec<_> = parse_lines(stream).collect();

    assert_eq!(results.len(), 3, "the blank line must be skipped");
    assert!(results[0].is_ok());
    assert!(results[2].is_ok(), "a bad line must not end the stream");

    let error = results[1].as_ref().expect_err("line 2 must fail");
    assert_eq!(error.line, 2);
    assert!(
        error.to_string().starts_with("line 2:"),
        "error must name its line, got: {error}"
    );
}

/// An empty stream yields nothing rather than one empty-line error.
#[test]
fn parse_lines_on_blank_input_yields_nothing() {
    for stream in ["", "\n", "  \n\n"] {
        assert_eq!(parse_lines(stream).count(), 0, "{stream:?} must yield none");
    }
}
