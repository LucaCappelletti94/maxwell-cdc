//! Benchmark for parser performance.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn benchmark_parse(c: &mut Criterion) {
    let fixtures = [
        (
            "row-insert",
            include_str!("../tests/fixtures/row-insert.json"),
        ),
        (
            "row-update",
            include_str!("../tests/fixtures/row-update.json"),
        ),
        (
            "row-delete",
            include_str!("../tests/fixtures/row-delete.json"),
        ),
        (
            "bootstrap-insert",
            include_str!("../tests/fixtures/bootstrap-insert.json"),
        ),
        (
            "bootstrap-start",
            include_str!("../tests/fixtures/bootstrap-start.json"),
        ),
        (
            "bootstrap-complete",
            include_str!("../tests/fixtures/bootstrap-complete.json"),
        ),
        (
            "table-create",
            include_str!("../tests/fixtures/table-create.json"),
        ),
        (
            "table-alter",
            include_str!("../tests/fixtures/table-alter.json"),
        ),
        (
            "table-drop",
            include_str!("../tests/fixtures/table-drop.json"),
        ),
        (
            "database-create",
            include_str!("../tests/fixtures/database-create.json"),
        ),
        (
            "database-alter",
            include_str!("../tests/fixtures/database-alter.json"),
        ),
        (
            "database-drop",
            include_str!("../tests/fixtures/database-drop.json"),
        ),
    ];

    let mut group = c.benchmark_group("parse");
    group.sample_size(100);

    for (name, json) in &fixtures {
        group.bench_with_input(*name, name, |b, _| {
            b.iter(|| maxwell_cdc::parse(black_box(json)));
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_parse);
criterion_main!(benches);
