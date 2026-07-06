#![allow(missing_docs)]

mod helpers;

use std::io::Write as _;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use crustywad::map::{
    Linedef, Node, Sector, Seg, Sidedef, Subsector, Thing, Vertex, parse_records,
};
use crustywad::{ParseOptions, Wad};

fn bench_parse_from_bytes(c: &mut Criterion) {
    let cases = [
        ("small", helpers::small_wad()),
        ("medium", helpers::medium_wad()),
        ("large", helpers::large_wad()),
    ];

    let mut group = c.benchmark_group("parse/from_bytes_strict");
    for (label, bytes) in &cases {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), bytes, |b, bytes| {
            b.iter_batched(
                || bytes.clone(),
                |input| Wad::from_bytes(input).unwrap(),
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();

    let mut group = c.benchmark_group("parse/from_bytes_lenient");
    for (label, bytes) in &cases {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), bytes, |b, bytes| {
            b.iter_batched(
                || bytes.clone(),
                |input| Wad::from_bytes_with_options(input, ParseOptions::lenient()).unwrap(),
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

fn bench_parse_from_path(c: &mut Criterion) {
    let medium = helpers::medium_wad();
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(&medium).unwrap();
    tmp.flush().unwrap();
    let path = tmp.path().to_owned();

    let mut group = c.benchmark_group("parse/from_path");
    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_function("medium_strict", |b| {
        b.iter(|| Wad::from_path(&path).unwrap());
    });
    group.bench_function("medium_lenient", |b| {
        b.iter(|| Wad::from_path_with_options(&path, ParseOptions::lenient()).unwrap());
    });
    group.finish();
}

fn bench_lump_access(c: &mut Criterion) {
    let wad = Wad::from_bytes(helpers::medium_wad()).unwrap();
    let lump = wad.lump(0).unwrap();

    let mut group = c.benchmark_group("lump_access");
    group.bench_function("lump_by_index", |b| {
        b.iter(|| wad.lump(std::hint::black_box(0)));
    });
    group.bench_function("lump_by_name_hit", |b| {
        b.iter(|| wad.lump_by_name(std::hint::black_box("BENCH")));
    });
    group.bench_function("lump_by_name_miss", |b| {
        b.iter(|| wad.lump_by_name(std::hint::black_box("MISSING")));
    });
    group.bench_function("lump_bytes", |b| {
        b.iter(|| wad.lump_bytes(std::hint::black_box(0)));
    });
    group.bench_function("lump_data", |b| {
        b.iter(|| wad.lump_data(std::hint::black_box(lump)));
    });
    group.bench_function("lumps_iter_count", |b| {
        b.iter(|| wad.lumps().iter().count());
    });
    group.finish();
}

fn bench_map_records(c: &mut Criterion) {
    // Each record type is benchmarked against 1 000 all-zero records.
    // Zeroed bytes parse without error for all record types (integer fields and Name8 fields alike).
    const N: usize = 1000;

    // Sizes in bytes: derived from on-disk format (little-endian packed, no padding).
    //   Thing     = 5 × 2 B                     = 10 B
    //   Linedef   = 7 × 2 B                     = 14 B
    //   Sidedef   = 2×i16 + 3×Name8 + u16       = 30 B
    //   Vertex    = 2 × 2 B                      =  4 B
    //   Seg       = 5×u16 + i16                  = 12 B
    //   Subsector = 2 × 2 B                      =  4 B
    //   Node      = 12×i16 + 2×u16               = 28 B
    //   Sector    = 2×i16 + 2×Name8 + 3×i16      = 26 B
    let thing_buf = vec![0u8; N * 10];
    let linedef_buf = vec![0u8; N * 14];
    let sidedef_buf = vec![0u8; N * 30];
    let vertex_buf = vec![0u8; N * 4];
    let seg_buf = vec![0u8; N * 12];
    let subsector_buf = vec![0u8; N * 4];
    let node_buf = vec![0u8; N * 28];
    let sector_buf = vec![0u8; N * 26];

    let mut group = c.benchmark_group("map_records");
    group.bench_function("Thing_x1000", |b| {
        b.iter(|| parse_records::<Thing>(&thing_buf).unwrap());
    });
    group.bench_function("Linedef_x1000", |b| {
        b.iter(|| parse_records::<Linedef>(&linedef_buf).unwrap());
    });
    group.bench_function("Sidedef_x1000", |b| {
        b.iter(|| parse_records::<Sidedef>(&sidedef_buf).unwrap());
    });
    group.bench_function("Vertex_x1000", |b| {
        b.iter(|| parse_records::<Vertex>(&vertex_buf).unwrap());
    });
    group.bench_function("Seg_x1000", |b| {
        b.iter(|| parse_records::<Seg>(&seg_buf).unwrap());
    });
    group.bench_function("Subsector_x1000", |b| {
        b.iter(|| parse_records::<Subsector>(&subsector_buf).unwrap());
    });
    group.bench_function("Node_x1000", |b| {
        b.iter(|| parse_records::<Node>(&node_buf).unwrap());
    });
    group.bench_function("Sector_x1000", |b| {
        b.iter(|| parse_records::<Sector>(&sector_buf).unwrap());
    });
    group.finish();
}

fn bench_freedoom(c: &mut Criterion) {
    let Some(path) = helpers::freedoom_wad_file() else {
        return;
    };

    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    // Validate up-front; skip gracefully if the fixture is corrupt or incompatible.
    let Ok(wad) = Wad::from_bytes(bytes.clone()) else {
        return;
    };
    let mut group = c.benchmark_group("freedoom");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("from_bytes", |b| {
        b.iter_batched(
            || bytes.clone(),
            |input| Wad::from_bytes(input).unwrap(),
            BatchSize::LargeInput,
        );
    });
    group.bench_function("lump_by_name_hit", |b| {
        b.iter(|| wad.lump_by_name(std::hint::black_box("E1M1")));
    });
    group.bench_function("lump_by_name_miss", |b| {
        b.iter(|| wad.lump_by_name(std::hint::black_box("MISSING")));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_parse_from_bytes,
    bench_parse_from_path,
    bench_lump_access,
    bench_map_records,
    bench_freedoom,
);
criterion_main!(benches);
