#![allow(missing_docs)]

mod helpers;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use crustywad::{Wad, WadBuilder, WadKind, WriteOptions};

fn bench_build_strict(c: &mut Criterion) {
    let cases = [
        ("small", helpers::small_wad()),
        ("medium", helpers::medium_wad()),
        ("large", helpers::large_wad()),
    ];

    let mut group = c.benchmark_group("write/build_strict");
    for (label, src_bytes) in &cases {
        let wad = Wad::from_bytes(src_bytes.clone()).unwrap();
        let builder = wad.to_builder();
        // Measure the output size for throughput reporting.
        let output_len = builder.build().unwrap().len() as u64;
        group.throughput(Throughput::Bytes(output_len));
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &builder,
            |b, builder| {
                b.iter(|| builder.build().unwrap());
            },
        );
    }
    group.finish();
}

fn bench_build_lenient(c: &mut Criterion) {
    let cases = [
        ("small", helpers::small_wad()),
        ("medium", helpers::medium_wad()),
        ("large", helpers::large_wad()),
    ];

    let mut group = c.benchmark_group("write/build_lenient");
    for (label, src_bytes) in &cases {
        let wad = Wad::from_bytes(src_bytes.clone()).unwrap();
        let builder = wad.to_builder();
        let (output, _) = builder
            .build_with_options(&WriteOptions::lenient())
            .unwrap();
        group.throughput(Throughput::Bytes(output.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &builder,
            |b, builder| {
                b.iter(|| {
                    builder
                        .build_with_options(&WriteOptions::lenient())
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn bench_build_from_scratch(c: &mut Criterion) {
    // Benchmark building a WAD from scratch (not round-tripping a parsed WAD).
    let payload_256 = vec![0u8; 256];
    let payload_4k = vec![0u8; 4096];

    let mut group = c.benchmark_group("write/build_from_scratch");

    group.bench_function("10_lumps_256b", |b| {
        b.iter(|| {
            let mut builder = WadBuilder::new(WadKind::Pwad);
            for _ in 0..10 {
                builder.add_lump("BENCH", payload_256.clone());
            }
            builder.build().unwrap()
        });
    });

    group.bench_function("100_lumps_4kib", |b| {
        b.iter(|| {
            let mut builder = WadBuilder::new(WadKind::Pwad);
            for _ in 0..100 {
                builder.add_lump("BENCH", payload_4k.clone());
            }
            builder.build().unwrap()
        });
    });

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let cases = [
        ("small", helpers::small_wad()),
        ("medium", helpers::medium_wad()),
        ("large", helpers::large_wad()),
    ];

    let mut group = c.benchmark_group("write/roundtrip");
    for (label, src_bytes) in &cases {
        group.throughput(Throughput::Bytes(src_bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            src_bytes,
            |b, src_bytes| {
                b.iter_batched(
                    || src_bytes.clone(),
                    |input| {
                        let wad = Wad::from_bytes(input).unwrap();
                        wad.to_builder().build().unwrap()
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

fn bench_freedoom_roundtrip(c: &mut Criterion) {
    let Some(path) = helpers::freedoom_wad_file() else {
        return;
    };

    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    // Validate parse and write paths up-front; skip gracefully if the fixture is corrupt.
    let Ok(wad) = Wad::from_bytes(bytes.clone()) else {
        return;
    };
    if wad.to_builder().build().is_err() {
        return;
    }
    let mut group = c.benchmark_group("freedoom");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("roundtrip", |b| {
        b.iter_batched(
            || bytes.clone(),
            |input| {
                let wad = Wad::from_bytes(input).unwrap();
                wad.to_builder().build().unwrap()
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_build_strict,
    bench_build_lenient,
    bench_build_from_scratch,
    bench_roundtrip,
    bench_freedoom_roundtrip,
);
criterion_main!(benches);
