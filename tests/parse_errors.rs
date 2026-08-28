//! Error-path contract tests: what `parse` must reject.

use maxwell_cdc::parse;

/// A tag this crate does not model must be rejected, not silently absorbed.
#[test]
fn unknown_type_tag_is_rejected() {
    let json = r#"{"type":"table-rename","database":"shop","table":"orders","ts":1477053217}"#;

    let error = parse(json).expect_err("unknown tag must not parse");
    assert!(
        error.to_string().contains("unknown variant"),
        "expected an unknown-variant error, got: {error}"
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
