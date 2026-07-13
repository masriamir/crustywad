#![allow(missing_docs)]

mod helpers;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use crustywad::map::{Map, write_doom_map};
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

/// Element counts shared by the two synthetic map generators below, so the
/// Doom-sourced and UDMF-sourced conversion benchmarks report comparable
/// throughput.
const MAP_VERTEX_COUNT: usize = 500;
const MAP_SECTOR_COUNT: usize = 50;

/// The five raw Doom map lump byte buffers produced by
/// [`synthetic_doom_map_lumps`].
struct SyntheticDoomLumps {
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
}

/// Builds a synthetic classic-Doom map's five lump byte buffers: a linear
/// chain of [`MAP_VERTEX_COUNT`] vertices joined by one-sided linedefs (and
/// their sidedefs), cycling through [`MAP_SECTOR_COUNT`] sectors, with one
/// thing per sector. Every value is a whole number in range, so the map
/// converts cleanly under strict [`WriteOptions`].
fn synthetic_doom_map_lumps() -> SyntheticDoomLumps {
    let mut vertexes = Vec::new();
    for i in 0..MAP_VERTEX_COUNT {
        let x = i16::try_from(i).expect("MAP_VERTEX_COUNT fits in i16");
        vertexes.extend_from_slice(&x.to_le_bytes());
        vertexes.extend_from_slice(&0i16.to_le_bytes());
    }

    let mut linedefs = Vec::new();
    let mut sidedefs = Vec::new();
    for i in 0..MAP_VERTEX_COUNT - 1 {
        let sector = u16::try_from(i % MAP_SECTOR_COUNT).expect("bounded by MAP_SECTOR_COUNT");
        sidedefs.extend_from_slice(&0i16.to_le_bytes());
        sidedefs.extend_from_slice(&0i16.to_le_bytes());
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"STARTAN3");
        sidedefs.extend_from_slice(&sector.to_le_bytes());

        let v1 = u16::try_from(i).expect("MAP_VERTEX_COUNT fits in u16");
        let v2 = v1 + 1;
        linedefs.extend_from_slice(&v1.to_le_bytes());
        linedefs.extend_from_slice(&v2.to_le_bytes());
        linedefs.extend_from_slice(&1u16.to_le_bytes()); // impassable
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // special
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // sector tag
        linedefs.extend_from_slice(&v1.to_le_bytes()); // right sidedef
        linedefs.extend_from_slice(&0xffffu16.to_le_bytes()); // one-sided
    }

    let mut sectors = Vec::new();
    let mut things = Vec::new();
    for _ in 0..MAP_SECTOR_COUNT {
        sectors.extend_from_slice(&0i16.to_le_bytes());
        sectors.extend_from_slice(&128i16.to_le_bytes());
        sectors.extend_from_slice(b"FLOOR4_8");
        sectors.extend_from_slice(b"CEIL3_5\0");
        sectors.extend_from_slice(&160i16.to_le_bytes());
        sectors.extend_from_slice(&0i16.to_le_bytes());
        sectors.extend_from_slice(&0i16.to_le_bytes());

        things.extend_from_slice(&0i16.to_le_bytes());
        things.extend_from_slice(&0i16.to_le_bytes());
        things.extend_from_slice(&0u16.to_le_bytes());
        things.extend_from_slice(&1u16.to_le_bytes());
        things.extend_from_slice(&0x0007u16.to_le_bytes()); // skill 1-3
    }

    SyntheticDoomLumps {
        things,
        linedefs,
        sidedefs,
        vertexes,
        sectors,
    }
}

/// Builds a synthetic UDMF `TEXTMAP` string carrying the same element counts
/// (and equally loss-free values) as [`synthetic_doom_map_lumps`], so the two
/// conversion benchmarks below are directly comparable.
fn synthetic_udmf_textmap() -> String {
    use std::fmt::Write as _;

    let mut text = String::from("namespace = \"doom\";\n");
    for i in 0..MAP_VERTEX_COUNT {
        let _ = writeln!(text, "vertex {{ x = {i}; y = 0; }}");
    }
    for _ in 0..MAP_SECTOR_COUNT {
        text.push_str("sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n");
    }
    for i in 0..MAP_VERTEX_COUNT - 1 {
        let sector = i % MAP_SECTOR_COUNT;
        let v2 = i + 1;
        let _ = writeln!(
            text,
            "sidedef {{ sector = {sector}; texturemiddle = \"STARTAN3\"; }}"
        );
        let _ = writeln!(text, "linedef {{ v1 = {i}; v2 = {v2}; sidefront = {i}; }}");
    }
    for _ in 0..MAP_SECTOR_COUNT {
        text.push_str(
            "thing { x = 0; y = 0; type = 1; skill1 = true; skill2 = true; skill3 = true; }\n",
        );
    }
    text
}

/// Assembles the synthetic Doom-sourced map once, outside the timed closure,
/// then benchmarks [`write_doom_map`] alone — the narrowing/serialization
/// cost with assembly excluded.
fn bench_write_doom_map(c: &mut Criterion) {
    let lumps = synthetic_doom_map_lumps();
    let wad_bytes = helpers::build_wad(
        *b"PWAD",
        &[
            ("MAP01", &[][..]),
            ("THINGS", &lumps.things),
            ("LINEDEFS", &lumps.linedefs),
            ("SIDEDEFS", &lumps.sidedefs),
            ("VERTEXES", &lumps.vertexes),
            ("SECTORS", &lumps.sectors),
        ],
    );
    let wad = Wad::from_bytes(wad_bytes).unwrap();
    let group_lumps = wad.map_group("MAP01").unwrap();
    let map = Map::assemble(&wad, &group_lumps).unwrap();

    let element_count = (map.vertices().len()
        + map.linedefs().len()
        + map.sidedefs().len()
        + map.sectors().len()
        + map.things().len()) as u64;

    let mut group = c.benchmark_group("write/doom_map");
    group.throughput(Throughput::Elements(element_count));
    group.bench_function("write_doom_map", |b| {
        b.iter(|| write_doom_map(&map, &WriteOptions::strict()).unwrap());
    });
    group.finish();
}

/// Assembles the synthetic UDMF-sourced map once, outside the timed closure,
/// then benchmarks the full UDMF → Doom conversion (`write_doom_map` run on a
/// `Map` whose source format is [`MapFormat::Udmf`](crustywad::map::MapFormat::Udmf),
/// exercising the UDMF-specific id-sentinel and flag-synthesis paths).
fn bench_udmf_to_doom_conversion(c: &mut Criterion) {
    let textmap = synthetic_udmf_textmap();
    let wad_bytes = helpers::build_wad(
        *b"PWAD",
        &[
            ("MAP01", &[][..]),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", &[][..]),
        ],
    );
    let wad = Wad::from_bytes(wad_bytes).unwrap();
    let group_lumps = wad.map_group("MAP01").unwrap();
    let map = Map::assemble(&wad, &group_lumps).unwrap();

    let element_count = (map.vertices().len()
        + map.linedefs().len()
        + map.sidedefs().len()
        + map.sectors().len()
        + map.things().len()) as u64;

    let mut group = c.benchmark_group("write/udmf_to_doom");
    group.throughput(Throughput::Elements(element_count));
    group.bench_function("write_doom_map", |b| {
        b.iter(|| write_doom_map(&map, &WriteOptions::strict()).unwrap());
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
    bench_write_doom_map,
    bench_udmf_to_doom_conversion,
);
criterion_main!(benches);
