extern crate criterion;
use slidestream::generator::DeepZoomGenerator;

use criterion::{criterion_group, criterion_main, Criterion};
use std::path::Path;

fn bench_get_tile(c: &mut Criterion) {
    let filename = Path::new("assets/CMU-1-Small-Region.svs");
    let g = DeepZoomGenerator::new(filename).unwrap();

    c.bench_function("get_tile", move |b| b.iter(|| g.get_tile(9, 1, 1).unwrap()));
}

criterion_group!(benches, bench_get_tile);
criterion_main!(benches);
