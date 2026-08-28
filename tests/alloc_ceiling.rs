//! Allocation ceiling for parsing a row message.
//!
//! The point is to catch a change that makes the parser allocate per field or per byte, not
//! to pin an exact figure. An exact count would fail on any `serde_json` retuning, so this
//! asserts a ceiling with headroom and reports the observed figure on breach.

mod support;

use maxwell_cdc::parse;
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Parsing `row-insert` must stay within a small, fixed number of allocations.
#[test]
fn row_insert_parsing_stays_within_its_allocation_ceiling() {
    const CEILING: usize = 40;

    let fixture = support::get("row-insert");

    let _ = parse(&fixture).expect("warmup parse");

    let before = INSTRUMENTED_SYSTEM.stats().allocations;
    let _message = parse(&fixture).expect("measured parse");
    let after = INSTRUMENTED_SYSTEM.stats().allocations;

    let allocations = after - before;

    assert!(
        allocations <= CEILING,
        "parsing row-insert.json made {allocations} allocations, ceiling is {CEILING}"
    );
}
