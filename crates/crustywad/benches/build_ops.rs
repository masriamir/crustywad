#![allow(missing_docs)]

//! Criterion benchmarks for the `nodebuild` node-lump builders and the
//! engine-playable one-shot (`add_doom_map_with_nodes`).
//!
//! Gated behind `required-features = ["nodebuild"]` so the existing write
//! benches stay buildable with the `write` feature alone.

mod helpers;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use crustywad::map::Map;
use crustywad::map::build::{
    NodeBuildOptions, add_doom_map_with_nodes, build_blockmap, build_nodes,
};
use crustywad::{Wad, WadBuilder, WadKind, WriteOptions};

/// Number of upward teeth in the synthetic comb polygon. Each tooth adds four
/// vertices and two reflex (concave) corners, so a larger count yields a deeper
/// BSP tree with more partitions to build.
const TOOTH_COUNT: usize = 30;
/// Horizontal half-period of the comb (unit width of a tooth or a gap).
const STEP: i16 = 32;
/// Base wall height; teeth rise to `BASE_Y + TOOTH_H` above it.
const BASE_Y: i16 = 256;
/// How far each tooth rises above the base line.
const TOOTH_H: i16 = 128;

/// Encodes the five raw Doom map lump byte buffers for a single-sector concave
/// "comb" polygon: a base rectangle with [`TOOTH_COUNT`] rectangular teeth cut
/// into its top edge. It is a simple (non-self-intersecting) polygon with many
/// reflex corners, so the BSP pass must place many partitions — real closed
/// geometry, unlike the collinear chain the write benches use.
///
/// A **single** sector is deliberate: a mixed-sector subsector (which strict
/// [`build_nodes`] rejects, ADR-0024 §7) requires two sectors meeting at a bare
/// vertex, so a one-sector map builds strict-clean by construction. Every value
/// is a whole number in `i16` range, so the map narrows cleanly under strict
/// [`WriteOptions`].
struct SyntheticBspLumps {
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
}

fn synthetic_bsp_map_lumps() -> SyntheticBspLumps {
    // Build the polygon's vertex ring, counterclockwise, matching the winding
    // the strict-clean square/L-room build tests use (interior on the right of
    // each one-sided wall).
    let span = 2 * i16::try_from(TOOTH_COUNT).expect("TOOTH_COUNT fits in i16") * STEP;
    let mut ring: Vec<(i16, i16)> = vec![(0, 0), (span, 0)];
    // Walk the top edge right-to-left; each tooth is an upward rectangular bump.
    for t in 0..i16::try_from(TOOTH_COUNT).expect("TOOTH_COUNT fits in i16") {
        let x_right = span - 2 * t * STEP;
        let x_left = span - (2 * t + 1) * STEP;
        ring.push((x_right, BASE_Y));
        ring.push((x_right, BASE_Y + TOOTH_H));
        ring.push((x_left, BASE_Y + TOOTH_H));
        ring.push((x_left, BASE_Y));
    }
    ring.push((0, BASE_Y));

    let mut vertexes = Vec::new();
    for &(x, y) in &ring {
        vertexes.extend_from_slice(&x.to_le_bytes());
        vertexes.extend_from_slice(&y.to_le_bytes());
    }

    let mut linedefs = Vec::new();
    let mut sidedefs = Vec::new();
    let count = u16::try_from(ring.len()).expect("vertex count fits in u16");
    for i in 0..count {
        let v1 = i;
        let v2 = (i + 1) % count; // close the loop on the last edge

        linedefs.extend_from_slice(&v1.to_le_bytes());
        linedefs.extend_from_slice(&v2.to_le_bytes());
        linedefs.extend_from_slice(&0x0001u16.to_le_bytes()); // impassable, one-sided
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // special
        linedefs.extend_from_slice(&0u16.to_le_bytes()); // sector tag
        linedefs.extend_from_slice(&i.to_le_bytes()); // right sidedef
        linedefs.extend_from_slice(&0xffffu16.to_le_bytes()); // no left (one-sided)

        sidedefs.extend_from_slice(&0i16.to_le_bytes());
        sidedefs.extend_from_slice(&0i16.to_le_bytes());
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
        sidedefs.extend_from_slice(b"STARTAN3");
        sidedefs.extend_from_slice(&0u16.to_le_bytes()); // the single sector
    }

    let mut sectors = Vec::new();
    sectors.extend_from_slice(&0i16.to_le_bytes());
    sectors.extend_from_slice(&128i16.to_le_bytes());
    sectors.extend_from_slice(b"FLOOR4_8");
    sectors.extend_from_slice(b"CEIL3_5\0");
    sectors.extend_from_slice(&160i16.to_le_bytes());
    sectors.extend_from_slice(&0i16.to_le_bytes());
    sectors.extend_from_slice(&0i16.to_le_bytes());

    // One player-1 start well inside the base rectangle.
    let mut things = Vec::new();
    things.extend_from_slice(&(STEP).to_le_bytes());
    things.extend_from_slice(&(BASE_Y / 2).to_le_bytes());
    things.extend_from_slice(&0u16.to_le_bytes());
    things.extend_from_slice(&1u16.to_le_bytes());
    things.extend_from_slice(&0x0007u16.to_le_bytes()); // all skills

    SyntheticBspLumps {
        things,
        linedefs,
        sidedefs,
        vertexes,
        sectors,
    }
}

/// Assembles the synthetic BSP fixture once, outside every timed closure.
fn assemble_fixture() -> Map {
    let lumps = synthetic_bsp_map_lumps();
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
    let group = wad.map_group("MAP01").unwrap();
    Map::assemble(&wad, &group).unwrap()
}

fn element_count(map: &Map) -> u64 {
    (map.vertices().len()
        + map.linedefs().len()
        + map.sidedefs().len()
        + map.sectors().len()
        + map.things().len()) as u64
}

/// The classic BSP pass: SEGS/SSECTORS/NODES from the assembled map.
fn bench_build_nodes(c: &mut Criterion) {
    let map = assemble_fixture();
    let mut group = c.benchmark_group("build/nodes");
    group.throughput(Throughput::Elements(element_count(&map)));
    group.bench_function("build_nodes", |b| {
        b.iter(|| build_nodes(&map, &NodeBuildOptions::strict()).unwrap());
    });
    group.finish();
}

/// The packed 128-unit collision grid.
fn bench_build_blockmap(c: &mut Criterion) {
    let map = assemble_fixture();
    let mut group = c.benchmark_group("build/blockmap");
    group.throughput(Throughput::Elements(element_count(&map)));
    group.bench_function("build_blockmap", |b| {
        b.iter(|| build_blockmap(&map, &NodeBuildOptions::strict()).unwrap());
    });
    group.finish();
}

/// The engine-playable one-shot: data lumps + all three builders into a builder.
fn bench_add_doom_map_with_nodes(c: &mut Criterion) {
    let map = assemble_fixture();
    let mut group = c.benchmark_group("build/one_shot");
    group.throughput(Throughput::Elements(element_count(&map)));
    group.bench_function("add_doom_map_with_nodes", |b| {
        b.iter(|| {
            let mut builder = WadBuilder::new(WadKind::Pwad);
            add_doom_map_with_nodes(
                &mut builder,
                "MAP01",
                &map,
                &WriteOptions::strict(),
                &NodeBuildOptions::strict(),
            )
            .unwrap();
            builder
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_build_nodes,
    bench_build_blockmap,
    bench_add_doom_map_with_nodes,
);
criterion_main!(benches);
