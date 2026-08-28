//! Single test measuring exact stable allocation count for parsing row-insert.json.

use maxwell_cdc::parse;
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn row_insert_parsing_stable_allocation_count() {
    let fixture = include_str!("fixtures/row-insert.json");

    let _ = parse(fixture).expect("warmup parse");

    let stats_before = INSTRUMENTED_SYSTEM.stats();
    let allocs_before = stats_before.allocations;

    let _message = parse(fixture).expect("measured parse");

    let stats_after = INSTRUMENTED_SYSTEM.stats();
    let allocs_after = stats_after.allocations;

    let allocation_count = allocs_after - allocs_before;

    assert_eq!(
        allocation_count, 18,
        "row-insert.json parse should allocate exactly 18 times, this test catches allocation pattern changes"
    );
}
