//! Benchmark for parser performance across the whole fixture corpus.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

#[path = "../tests/support/mod.rs"]
mod support;

fn benchmark_parse(c: &mut Criterion) {
    let fixtures = support::all();

    let mut group = c.benchmark_group("parse");
    group.sample_size(100);

    for (name, json) in &fixtures {
        group.bench_with_input(name, json.as_str(), |b, json| {
            b.iter(|| maxwell_cdc::parse(black_box(json)));
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_parse);
criterion_main!(benches);
