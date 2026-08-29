//! Allocation ceiling for parsing a row message. A ceiling rather than an exact count,
//! which would fail on any `serde_json` retuning.

mod support;

use maxwell_cdc::parse;
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Parsing `row-insert` must stay within a small, fixed number of allocations.
///
/// Observed is 20. The ceiling sits close because `RowChange` has 15 named fields, so a
/// per-field allocation bug would land near 35 and a looser bound would miss it.
#[test]
fn row_insert_parsing_stays_within_its_allocation_ceiling() {
    const CEILING: usize = 24;

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
