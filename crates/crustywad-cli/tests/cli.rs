//! Integration tests for the `cwad` CLI binary.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Minimal WAD builder (mirrors common::build_wad in the library test suite)
// ---------------------------------------------------------------------------

fn encode_i32(value: usize) -> [u8; 4] {
    i32::try_from(value)
        .expect("test fixture values should fit within i32")
        .to_le_bytes()
}

fn build_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut directory = Vec::new();
    let directory_offset = 12 + lumps.iter().map(|(_, b)| b.len()).sum::<usize>();

    for (name, bytes) in lumps {
        let filepos = 12 + payload.len();
        payload.extend_from_slice(bytes);
        directory.extend_from_slice(&encode_i32(filepos));
        directory.extend_from_slice(&encode_i32(bytes.len()));
        let mut encoded = [0_u8; 8];
        for (slot, byte) in name.as_bytes().iter().take(8).enumerate() {
            encoded[slot] = *byte;
        }
        directory.extend_from_slice(&encoded);
    }

    let mut wad = Vec::new();
    wad.extend_from_slice(&kind);
    wad.extend_from_slice(&encode_i32(lumps.len()));
    wad.extend_from_slice(&encode_i32(directory_offset));
    wad.extend_from_slice(&payload);
    wad.extend_from_slice(&directory);
    wad
}

fn write_wad(kind: [u8; 4], lumps: &[(&str, &[u8])]) -> NamedTempFile {
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), build_wad(kind, lumps)).expect("wad should be written");
    file
}

fn write_bytes(data: &[u8]) -> NamedTempFile {
    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), data).expect("bytes should be written");
    file
}

/// Explodes a WAD file into `NAME=<tempfile>` build specs (one per lump, in
/// directory order), plus the backing temp files (returned so the caller keeps
/// them alive for the duration of the `build` invocation).
fn explode_wad_to_build_specs(path: &std::path::Path) -> (Vec<String>, Vec<NamedTempFile>) {
    let bytes = std::fs::read(path).expect("wad readable");
    let wad = crustywad::Wad::from_bytes(bytes).expect("wad parses");
    let mut specs = Vec::new();
    let mut files = Vec::new();
    for lump in wad.lumps() {
        let f = NamedTempFile::new().expect("tempfile");
        std::fs::write(f.path(), wad.lump_data(lump)).expect("write lump bytes");
        specs.push(format!("{}={}", lump.name(), f.path().to_str().unwrap()));
        files.push(f);
    }
    (specs, files)
}

// ---------------------------------------------------------------------------
// `cwad info`
// ---------------------------------------------------------------------------

#[test]
fn info_json_format() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\""))
        .stdout(predicate::str::contains("\"lumps\""))
        .stdout(predicate::str::contains("\"data_size\""))
        .stdout(predicate::str::contains("\"maps\""));
}

#[test]
fn info_csv_format() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind,lumps,data_size,maps"))
        .stdout(predicate::str::contains("Iwad,1,3,"));
}

#[test]
fn info_csv_format_with_maps() {
    // Two real maps (each marker has a data run) pin the multi-map CSV
    // rendering: space-separated, in directory order. The trailing bare E9M9
    // marker has no data run, so structural detection (#253) excludes it —
    // the exact newline-terminated row asserts that too.
    let wad = write_wad(
        *b"IWAD",
        &[
            ("E1M1", &[]),
            ("THINGS", &[0; 4]),
            ("E1M2", &[]),
            ("THINGS", &[0; 4]),
            ("E9M9", &[]),
        ],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind,lumps,data_size,maps"))
        .stdout(predicate::str::contains("Iwad,5,8,E1M1 E1M2\n"));
}

#[test]
fn info_iwad() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Iwad"))
        .stdout(predicate::str::contains("lumps:     1"))
        .stdout(predicate::str::contains("data size: 3 bytes"));
}

#[test]
fn info_pwad() {
    let wad = write_wad(*b"PWAD", &[("PATCH1", &[0]), ("PATCH2", &[0, 0])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pwad"))
        .stdout(predicate::str::contains("lumps:     2"))
        .stdout(predicate::str::contains("data size: 3 bytes"));
}

#[test]
fn info_maps_doom1_style() {
    // E1M1 groups with its THINGS data run; the trailing bare E1M2 marker has
    // no data run, so structural detection (#253) excludes it.
    let wad = write_wad(
        *b"IWAD",
        &[("E1M1", &[]), ("THINGS", &[0; 10]), ("E1M2", &[])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        // The exact, newline-terminated maps line pins the complete list —
        // E1M2's absence included.
        .stdout(predicate::str::contains("maps:      E1M1\n"));
}

// Structural detection (#253): `cwad info` delegates to `Wad::map_groups`, so
// a map is a marker lump followed by a recognized data run — whatever its
// name — plus Doom 64 nested-WAD `MAPxx` lumps. A stray map-named lump with
// no data run is NOT a map (divergence from the old name-pattern heuristic,
// decided in #253).
#[test]
fn info_maps_match_map_groups_across_formats() {
    // Minimal Doom 64 nested-WAD map: a complete WAD (one empty THINGS
    // sub-lump) stored as the MAP04 lump's data.
    let nested = build_wad(*b"IWAD", &[("THINGS", &[])]);
    let wad = write_wad(
        *b"IWAD",
        &[
            ("E1M1", &[]),
            ("THINGS", &[0; 10]),
            ("WEIRDMAP", &[]),
            ("VERTEXES", &[0; 4]),
            ("MAP04", &nested),
            ("MAP99", &[]), // stray marker, no data run: not a map
        ],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        // The exact, newline-terminated maps line pins the complete list —
        // MAP99's absence included.
        .stdout(predicate::str::contains(
            "maps:      E1M1, WEIRDMAP, MAP04\n",
        ));
}

#[test]
fn info_maps_doom2_style() {
    // MAP01 groups with its data run; the bare trailing MAP02 marker (no data
    // run, no nested magic) is excluded by structural detection (#253).
    let wad = write_wad(
        *b"IWAD",
        &[("MAP01", &[]), ("THINGS", &[0; 10]), ("MAP02", &[])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        // The exact, newline-terminated maps line pins the complete list —
        // MAP02's absence included.
        .stdout(predicate::str::contains("maps:      MAP01\n"));
}

#[test]
fn info_maps_json_includes_array() {
    let wad = write_wad(*b"IWAD", &[("MAP01", &[]), ("THINGS", &[0; 4])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"maps\":[\"MAP01\"]"));
}

#[test]
fn info_no_maps_json_empty_array() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"maps\":[]"));
}

#[test]
fn info_no_maps_human_no_maps_line() {
    // When there are no maps the "maps:" line should be absent.
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("maps:").not());
}

#[test]
fn info_data_size_sums_all_lumps() {
    // Three lumps: 4 + 8 + 2 = 14 bytes total data.
    let wad = write_wad(*b"PWAD", &[("A", &[0; 4]), ("B", &[0; 8]), ("C", &[0; 2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("data size: 14 bytes"));
}

#[test]
fn info_lenient_emits_warning_for_bad_magic() {
    let mut bytes = build_wad(*b"NOPE", &[("TEST", &[1])]);
    // Keep lump count and directory offset valid so lenient mode produces a warning, not a hard error.
    bytes[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());

    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &bytes).expect("bytes should be written");

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["--lenient", "info", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));
}

// ---------------------------------------------------------------------------
// `cwad list`
// ---------------------------------------------------------------------------

#[test]
fn list_json_format() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[9])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"index\""))
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"PLAYPAL\""));
}

#[test]
fn list_csv_format() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[9])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("index,filepos,size,name"))
        .stdout(predicate::str::contains("PLAYPAL"));
}

#[test]
fn list_json_escapes_special_chars_in_lump_name() {
    // Lump name with chars requiring JSON escaping: \ " \n \r \t \x01 P A
    // (exactly 8 bytes — the WAD name field width).
    // Exercises every escape arm in json_string().
    let wad = write_wad(*b"IWAD", &[("\\\"\n\r\t\x01PA", &[1])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "-F",
            "json",
            "list",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\\\\")) // \ → \\
        .stdout(predicate::str::contains("\\\"")) // " → \"
        .stdout(predicate::str::contains("\\n")) // newline
        .stdout(predicate::str::contains("\\r")) // carriage return
        .stdout(predicate::str::contains("\\t")) // tab
        .stdout(predicate::str::contains("\\u0001")); // control char U+0001
}

#[test]
fn list_csv_quotes_lump_name_with_comma() {
    // Lump name containing a comma; csv_field() must wrap it in double-quotes.
    let wad = write_wad(*b"IWAD", &[("A,B", &[1])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "-F",
            "csv",
            "list",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"A,B\""));
}

#[test]
fn list_shows_lump_names_and_indices() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[9, 9, 9]), ("COLORMAP", &[0])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PLAYPAL"))
        .stdout(predicate::str::contains("COLORMAP"))
        .stdout(predicate::str::contains("0000"));
}

#[test]
fn list_empty_wad() {
    let wad = write_wad(*b"PWAD", &[]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn list_lenient_emits_warning_for_bad_magic() {
    let mut bytes = build_wad(*b"NOPE", &[("LUMP", &[0xAB])]);
    bytes[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());

    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &bytes).expect("bytes should be written");

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["--lenient", "list", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));
}

// ---------------------------------------------------------------------------
// `cwad validate --deep` (#251)
// ---------------------------------------------------------------------------

/// The five classic map data lumps, all zero-length — a structurally valid,
/// empty map (no records means no cross-references to dangle).
fn empty_map_lumps(marker: &str) -> Vec<(String, Vec<u8>)> {
    [
        marker, "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SECTORS",
    ]
    .iter()
    .map(|name| ((*name).to_string(), Vec::new()))
    .collect()
}

fn write_wad_owned(kind: [u8; 4], lumps: &[(String, Vec<u8>)]) -> NamedTempFile {
    let borrowed: Vec<(&str, &[u8])> = lumps
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    write_wad(kind, &borrowed)
}

#[test]
fn validate_deep_ok_on_clean_maps() {
    let wad = write_wad_owned(*b"IWAD", &empty_map_lumps("E1M1"));
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "--deep", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 map(s) validated"));
}

#[test]
fn validate_deep_names_the_failing_map_and_continues() {
    // E1M1's LINEDEFS is 13 bytes (mid-record: not a multiple of 14) — fails
    // in both modes. E1M2 is clean; deep validation must report it too rather
    // than stopping at the first failure.
    let mut lumps = empty_map_lumps("E1M1");
    lumps[2].1 = vec![0; 13];
    lumps.extend(empty_map_lumps("E1M2"));
    let wad = write_wad_owned(*b"IWAD", &lumps);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "--deep", wad.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error: map E1M1"))
        // "1 of 2" proves continuation: the denominator counts every group,
        // and E1M2 can only reach the summary if validation proceeded past
        // the E1M1 failure (its per-map success row is JSON/CSV-only).
        .stderr(predicate::str::contains("1 of 2 map(s) failed validation"));
}

#[test]
fn validate_deep_json_emits_per_map_rows_and_summary() {
    let mut lumps = empty_map_lumps("E1M1");
    lumps[2].1 = vec![0; 13];
    lumps.extend(empty_map_lumps("E1M2"));
    let wad = write_wad_owned(*b"IWAD", &lumps);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "validate",
            "--deep",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"map\":\"E1M1\",\"ok\":false"))
        .stdout(predicate::str::contains("\"map\":\"E1M2\",\"ok\":true"))
        .stdout(predicate::str::contains(
            "\"ok\":false,\"error\":\"1 of 2 map(s) failed validation\"",
        ));
}

#[test]
fn validate_deep_csv_emits_per_map_rows() {
    let mut lumps = empty_map_lumps("E1M1");
    lumps[2].1 = vec![0; 13];
    lumps.extend(empty_map_lumps("E1M2"));
    let wad = write_wad_owned(*b"IWAD", &lumps);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "csv",
            "validate",
            "--deep",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("map,ok,error\n"))
        .stdout(predicate::str::contains("E1M2,true,\n"));
}

#[test]
fn validate_deep_lenient_recovers_with_warnings_and_exits_zero() {
    // One linedef referencing vertex 9 with only 2 vertices: strict-fatal,
    // lenient-recoverable (clamp + warning).
    let mut lumps = empty_map_lumps("E1M1");
    let mut linedef = Vec::new();
    for v in [0u16, 9, 0, 0, 0, 0xffff, 0xffff] {
        linedef.extend(v.to_le_bytes());
    }
    lumps[2].1 = linedef;
    lumps[4].1 = vec![0; 8]; // two 4-byte vertices at (0,0)
    let wad = write_wad_owned(*b"IWAD", &lumps);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "validate",
            "--deep",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: map E1M1"));
    // The same WAD fails deep validation in strict mode.
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "--deep", wad.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error: map E1M1"));
}

#[test]
fn validate_deep_json_and_csv_success_summaries() {
    // All-maps-pass summaries per format: JSON emits the per-map row plus the
    // same {"ok":true} object shallow mode prints; CSV emits only the header
    // and per-map rows (no summary pair).
    let wad = write_wad_owned(*b"IWAD", &empty_map_lumps("E1M1"));
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "validate",
            "--deep",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"map\":\"E1M1\",\"ok\":true"))
        .stdout(predicate::str::contains("{\"ok\":true}\n"));
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "csv",
            "validate",
            "--deep",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_match("^map,ok,error\nE1M1,true,\n$").unwrap());
}

#[test]
fn validate_deep_prints_container_warnings_after_the_summary() {
    // An unknown magic parses only leniently, with a container-level warning;
    // deep validation prints it after the summary (ADR-0008 §3).
    let wad = write_wad_owned(*b"WADX", &empty_map_lumps("E1M1"));
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "validate",
            "--deep",
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 map(s) validated"))
        .stderr(predicate::str::contains("warning:"));
}

#[test]
fn validate_deep_covers_doom64_nested_maps() {
    // A Doom 64 nested-WAD map whose container is missing every record
    // sub-lump but THINGS: strict deep validation fails naming the map.
    let nested = build_wad(*b"IWAD", &[("THINGS", &[])]);
    let wad = write_wad(*b"IWAD", &[("MAP01", &nested)]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "--deep", wad.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error: map MAP01"));
}

#[test]
fn validate_without_deep_ignores_map_contents() {
    // The same corrupt LINEDEFS that fails --deep passes a shallow validate:
    // the directory is well-formed, and shallow validation stops there.
    let mut lumps = empty_map_lumps("E1M1");
    lumps[2].1 = vec![0; 13];
    let wad = write_wad_owned(*b"IWAD", &lumps);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", wad.path().to_str().unwrap()])
        .assert()
        .success();
}

// `cwad validate`
// ---------------------------------------------------------------------------

#[test]
fn validate_csv_format_ok() {
    let wad = write_wad(*b"IWAD", &[]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "validate", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("ok\ntrue\n");
}

#[test]
fn validate_json_format_error() {
    let file = write_bytes(b"NOTAWAD");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "validate", file.path().to_str().unwrap()])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("\"error\""));
}

#[test]
fn validate_csv_format_error() {
    let file = write_bytes(b"NOTAWAD");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "validate", file.path().to_str().unwrap()])
        .assert()
        .code(2)
        .stdout("ok\nfalse\n");
}

#[test]
fn validate_clean_wad_exits_0() {
    let wad = write_wad(*b"IWAD", &[]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", wad.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn validate_clean_wad_human_output() {
    let wad = write_wad(*b"IWAD", &[]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("ok:"));
}

#[test]
fn validate_json_format_ok() {
    let wad = write_wad(*b"IWAD", &[]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "validate", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\""));
}

#[test]
fn validate_missing_file_exits_2() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.wad");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", missing.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn validate_corrupt_wad_exits_2() {
    let file = write_bytes(b"NOTAWADX");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

// ---------------------------------------------------------------------------
// `cwad validate` (lenient / warnings)
// ---------------------------------------------------------------------------

#[test]
fn validate_lenient_emits_warning_for_bad_magic() {
    let mut bytes = build_wad(*b"NOPE", &[("TEST", &[1])]);
    bytes[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());

    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &bytes).expect("bytes should be written");

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["--lenient", "validate", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));
}

// ---------------------------------------------------------------------------
// `cwad diff`
// ---------------------------------------------------------------------------

#[test]
fn diff_identical_wads_exits_0() {
    let wad = write_wad(
        *b"IWAD",
        &[("THINGS", &[1, 2, 3]), ("LINEDEFS", &[4, 5, 6])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad.path().to_str().unwrap(),
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty());
}

#[test]
fn diff_different_lump_data_exits_1() {
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1, 2, 3])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[9, 9, 9])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Changed"))
        .stdout(predicate::str::contains("THINGS"));
}

#[test]
fn diff_lump_only_in_first_exits_1() {
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1]), ("LINEDEFS", &[2])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[1])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Only in"))
        .stdout(predicate::str::contains("LINEDEFS"));
}

#[test]
fn diff_lump_only_in_second_exits_1() {
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[1]), ("NEWLUMP", &[99])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Only in"))
        .stdout(predicate::str::contains("NEWLUMP"));
}

#[test]
fn diff_missing_file_exits_2() {
    let wad = write_wad(*b"IWAD", &[("THINGS", &[1])]);
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.wad");
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad.path().to_str().unwrap(),
            missing.to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

#[test]
fn diff_json_format_identical() {
    let wad = write_wad(*b"IWAD", &[("THINGS", &[1, 2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "diff",
            wad.path().to_str().unwrap(),
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty());
}

#[test]
fn diff_json_format_differences() {
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"kind\""))
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"THINGS\""));
}

#[test]
fn diff_csv_format_identical() {
    let wad = write_wad(*b"IWAD", &[("THINGS", &[1])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "csv",
            "diff",
            wad.path().to_str().unwrap(),
            wad.path().to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty());
}

#[test]
fn diff_csv_format_differences() {
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "csv",
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("kind,name"))
        .stdout(predicate::str::contains("changed,THINGS"));
}

#[test]
fn diff_json_format_only_in_first_and_second() {
    let wad1 = write_wad(*b"IWAD", &[("ALPHA", &[1])]);
    let wad2 = write_wad(*b"IWAD", &[("BETA", &[2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"only_in_first\""))
        .stdout(predicate::str::contains("\"only_in_second\""));
}

#[test]
fn diff_csv_format_only_in_first_and_second() {
    let wad1 = write_wad(*b"IWAD", &[("ALPHA", &[1])]);
    let wad2 = write_wad(*b"IWAD", &[("BETA", &[2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "csv",
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("only_in_first,ALPHA"))
        .stdout(predicate::str::contains("only_in_second,BETA"));
}

#[test]
fn diff_lenient_emits_warning_for_bad_magic() {
    let mut bytes1 = build_wad(*b"NOPE", &[("TEST", &[1])]);
    bytes1[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes1[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());
    let mut bytes2 = build_wad(*b"NOPE", &[("TEST", &[1])]);
    bytes2[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes2[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());

    let file1 = NamedTempFile::new().expect("tempfile should be created");
    let file2 = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file1.path(), &bytes1).expect("bytes should be written");
    std::fs::write(file2.path(), &bytes2).expect("bytes should be written");

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "diff",
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("warning"));
}

#[test]
fn diff_multiple_differences_all_reported() {
    let wad1 = write_wad(
        *b"IWAD",
        &[("THINGS", &[1]), ("LINEDEFS", &[2]), ("SIDEDEFS", &[3])],
    );
    let wad2 = write_wad(
        *b"IWAD",
        &[("THINGS", &[9]), ("NEWLUMP", &[7]), ("SIDEDEFS", &[3])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("THINGS"))
        .stdout(predicate::str::contains("LINEDEFS"))
        .stdout(predicate::str::contains("NEWLUMP"));
}

#[test]
fn diff_duplicate_lump_count_differs_exits_1() {
    // WAD1 has THINGS twice; WAD2 has THINGS once with the same data.
    // The per-name sequence comparison must detect the count difference and report Changed.
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1, 2, 3]), ("THINGS", &[1, 2, 3])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("THINGS"));
}

#[test]
fn diff_duplicate_lumps_same_count_and_data_exits_0() {
    // Both WADs have THINGS twice with identical data — must be identical.
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1, 2, 3]), ("THINGS", &[4, 5, 6])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[1, 2, 3]), ("THINGS", &[4, 5, 6])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty());
}

#[test]
fn diff_duplicate_lumps_order_differs_exits_1() {
    // Both WADs have THINGS twice but the data vectors are in a different order —
    // the per-name sequence comparison must detect the difference.
    let wad1 = write_wad(*b"IWAD", &[("THINGS", &[1, 2, 3]), ("THINGS", &[4, 5, 6])]);
    let wad2 = write_wad(*b"IWAD", &[("THINGS", &[4, 5, 6]), ("THINGS", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("THINGS"));
}

#[test]
fn diff_unique_name_reorder_exits_0() {
    // Directory order of distinct lump names is intentionally not significant —
    // A,B vs B,A with identical data should exit 0.
    let wad1 = write_wad(*b"IWAD", &[("ALPHA", &[1, 2]), ("BETA", &[3, 4])]);
    let wad2 = write_wad(*b"IWAD", &[("BETA", &[3, 4]), ("ALPHA", &[1, 2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// Exit codes and error paths
// ---------------------------------------------------------------------------

#[test]
fn help_flag_exits_successfully() {
    Command::cargo_bin("cwad")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn missing_file_exits_2() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("file.wad");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", missing.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn invalid_magic_strict_mode_exits_2() {
    let file = write_bytes(b"BOGUS_DATA_NOT_A_WAD");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn unknown_subcommand_exits_3() {
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["notasubcommand"])
        .assert()
        .code(3);
}

#[test]
fn invalid_format_value_exits_3() {
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "notaformat", "info", "/dev/null"])
        .assert()
        .code(3);
}

#[test]
fn missing_required_arg_exits_3() {
    Command::cargo_bin("cwad")
        .unwrap()
        .arg("info")
        .assert()
        .code(3);
}

// ---------------------------------------------------------------------------
// `cwad merge`
// ---------------------------------------------------------------------------

#[test]
fn merge_two_wads_contains_all_lumps_in_order() {
    let wad1 = write_wad(*b"IWAD", &[("ALPHA", &[1, 2, 3]), ("BETA", &[4])]);
    let wad2 = write_wad(*b"PWAD", &[("GAMMA", &[5, 6])]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify all three lump names are present.
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("ALPHA"))
        .stdout(predicate::str::contains("BETA"))
        .stdout(predicate::str::contains("GAMMA"));

    // Verify lump order: ALPHA before BETA, BETA before GAMMA.
    let stdout = Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "list", out.path().to_str().unwrap()])
        .output()
        .unwrap()
        .stdout;
    let text = String::from_utf8(stdout).unwrap();
    let alpha_pos = text.find("ALPHA").unwrap();
    let beta_pos = text.find("BETA").unwrap();
    let gamma_pos = text.find("GAMMA").unwrap();
    assert!(alpha_pos < beta_pos, "ALPHA should come before BETA");
    assert!(beta_pos < gamma_pos, "BETA should come before GAMMA");

    // Verify total lump count.
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("lumps:     3"));
}

#[test]
fn merge_output_is_parseable_wad() {
    let wad1 = write_wad(*b"PWAD", &[("TEST", &[0xAB, 0xCD])]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            wad1.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", out.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn merge_default_kind_is_pwad() {
    let wad1 = write_wad(*b"IWAD", &[("LUMP", &[1])]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            wad1.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pwad"));
}

#[test]
fn merge_kind_flag_iwad() {
    let wad1 = write_wad(*b"PWAD", &[("LUMP", &[1])]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            wad1.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
            "--kind",
            "iwad",
        ])
        .assert()
        .success();

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Iwad"));
}

#[test]
fn merge_missing_input_exits_2() {
    let out = NamedTempFile::new().unwrap();
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.wad");

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            missing.to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

#[test]
fn merge_preserves_lump_data() {
    let data1: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
    let data2: &[u8] = &[0xCA, 0xFE];
    let wad1 = write_wad(*b"PWAD", &[("LUMP1", data1)]);
    let wad2 = write_wad(*b"PWAD", &[("LUMP2", data2)]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            wad1.path().to_str().unwrap(),
            wad2.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let bytes = std::fs::read(out.path()).unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 2);
    assert_eq!(wad.lump(0).unwrap().name(), "LUMP1");
    assert_eq!(wad.lump(1).unwrap().name(), "LUMP2");
    assert_eq!(wad.lump_data(wad.lump(0).unwrap()), data1);
    assert_eq!(wad.lump_data(wad.lump(1).unwrap()), data2);
}

#[test]
fn merge_lenient_non_ascii_name_still_fails_write_validation_exits_3() {
    // A non-ASCII lump name decodes successfully under lenient *read* (with a
    // warning), but `WriteError::NonAsciiName` is rejected in both write
    // strictness modes, so `--lenient merge` must still exit 3 rather than
    // silently succeeding or falling through to the generic I/O exit code 2.
    let wad = write_wad(*b"PWAD", &[("É", &[1])]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "merge",
            wad.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(out.path().to_str().unwrap()));
}

#[test]
fn merge_no_inputs_exits_3() {
    // `inputs` is required to have at least one path; omitting it entirely is
    // a clap usage error, not a silent zero-lump merge.
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["merge", "--output", out.path().to_str().unwrap()])
        .assert()
        .code(3);
}

#[test]
fn merge_lenient_emits_warning_for_bad_magic() {
    let mut bytes = build_wad(*b"NOPE", &[("TEST", &[1])]);
    bytes[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());

    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &bytes).expect("bytes should be written");
    let out = NamedTempFile::new().expect("tempfile should be created");

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "merge",
            file.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"))
        .stderr(predicate::str::contains(file.path().to_str().unwrap()));
}

// ---------------------------------------------------------------------------
// `cwad extract`
// ---------------------------------------------------------------------------

#[test]
fn extract_all_lumps_creates_files() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3]), ("COLORMAP", &[4, 5])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out_dir.path().join("PLAYPAL.bin").exists());
    assert!(out_dir.path().join("COLORMAP.bin").exists());
    assert_eq!(
        std::fs::read(out_dir.path().join("PLAYPAL.bin")).unwrap(),
        vec![1, 2, 3]
    );
    assert_eq!(
        std::fs::read(out_dir.path().join("COLORMAP.bin")).unwrap(),
        vec![4, 5]
    );
}

#[test]
fn extract_named_lump_only_extracts_that_lump() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3]), ("COLORMAP", &[4, 5])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
            "--lump",
            "PLAYPAL",
        ])
        .assert()
        .success();

    assert!(out_dir.path().join("PLAYPAL.bin").exists());
    assert!(!out_dir.path().join("COLORMAP.bin").exists());
}

#[test]
fn extract_named_lump_not_found_exits_2() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
            "--lump",
            "NOTEXIST",
        ])
        .assert()
        .code(2);
}

#[test]
fn extract_missing_wad_exits_2() {
    let out_dir = TempDir::new().unwrap();
    let missing_dir = TempDir::new().unwrap();
    let missing = missing_dir.path().join("missing.wad");

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            missing.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

#[test]
fn extract_mixed_case_lump_names_deduplicated() {
    // "PATCH" and "patch" differ only in case; after uppercasing they collide,
    // so the second gets the _1 suffix instead of silently overwriting on
    // case-insensitive filesystems (Windows/macOS).
    let wad = write_wad(*b"PWAD", &[("PATCH", &[0xAA]), ("patch", &[0xBB])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.path().join("PATCH.bin").exists(),
        "PATCH.bin not written"
    );
    assert!(
        out_dir.path().join("PATCH_1.bin").exists(),
        "PATCH_1.bin not written"
    );
    assert_eq!(
        std::fs::read(out_dir.path().join("PATCH.bin")).unwrap(),
        vec![0xAAu8]
    );
    assert_eq!(
        std::fs::read(out_dir.path().join("PATCH_1.bin")).unwrap(),
        vec![0xBBu8]
    );
}

#[test]
fn extract_windows_reserved_lump_name_gets_prefixed() {
    // Lump names that are Windows device names (CON, NUL, COM1, LPT1, …) must
    // be prefixed with '_' so extraction succeeds on all platforms.
    let wad = write_wad(*b"IWAD", &[("CON", &[0xCC]), ("NUL", &[0xDD])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        out_dir.path().join("_CON.bin").exists(),
        "_CON.bin not written"
    );
    assert!(
        out_dir.path().join("_NUL.bin").exists(),
        "_NUL.bin not written"
    );
    assert_eq!(
        std::fs::read(out_dir.path().join("_CON.bin")).unwrap(),
        vec![0xCCu8]
    );
    assert_eq!(
        std::fs::read(out_dir.path().join("_NUL.bin")).unwrap(),
        vec![0xDDu8]
    );
}

#[test]
fn extract_lump_flag_without_value_exits_3() {
    // `--lump` requires a NAME argument; omitting the value is a clap parse
    // error, which the main() dispatch maps to exit code 3.
    let out_dir = TempDir::new().unwrap();
    let missing_dir = TempDir::new().unwrap();
    let missing = missing_dir.path().join("missing.wad");
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            missing.to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
            "--lump",
        ])
        .assert()
        .code(3);
}

#[test]
fn extract_duplicate_lump_names_writes_unique_files() {
    // Two lumps with the same name — second should get an occurrence-count suffix.
    let wad = write_wad(*b"PWAD", &[("PATCH", &[0xAA]), ("PATCH", &[0xBB])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let first = out_dir.path().join("PATCH.bin");
    let second = out_dir.path().join("PATCH_1.bin");
    assert!(first.exists(), "PATCH.bin not written");
    assert!(second.exists(), "PATCH_1.bin not written");
    assert_eq!(
        std::fs::read(&first).unwrap(),
        vec![0xAAu8],
        "PATCH.bin has wrong content"
    );
    assert_eq!(
        std::fs::read(&second).unwrap(),
        vec![0xBBu8],
        "PATCH_1.bin has wrong content"
    );
}

#[test]
fn extract_empty_lump_creates_empty_file() {
    // Marker/namespace lumps have zero bytes; they must still be written.
    let wad = write_wad(*b"IWAD", &[("SS_START", &[])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let path = out_dir.path().join("SS_START.bin");
    assert!(path.exists());
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
}

#[test]
fn extract_empty_wad_exits_0() {
    let wad = write_wad(*b"IWAD", &[]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn extract_sanitizes_path_separator_in_lump_name() {
    // A lump name containing '/' must not create subdirectories or escape the
    // output directory; the slash is replaced with '_' by sanitize_lump_name.
    let wad = write_wad(*b"IWAD", &[("A/B", &[0x42])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // The sanitized file must exist inside the output directory.
    assert!(
        out_dir.path().join("A_B.bin").exists(),
        "A_B.bin not written"
    );
    // No subdirectory should have been created by the path separator.
    assert!(
        !out_dir.path().join("A").is_dir(),
        "path separator created a subdirectory"
    );
}

#[test]
fn extract_bad_output_checked_before_wad_io() {
    // With a nonexistent WAD and a bad --output, the output check fires first.
    // If the order were reversed, stderr would mention the WAD, not the output.
    let not_a_dir = NamedTempFile::new().unwrap();
    let missing_dir = TempDir::new().unwrap();
    let missing = missing_dir.path().join("missing.wad");
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            missing.to_str().unwrap(),
            "--output",
            not_a_dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "does not exist or is not a directory",
        ));
}

#[test]
fn extract_output_not_a_directory_exits_2() {
    // Pass a regular file (not a directory) as --output to verify the upfront
    // is_dir() check fires with exit 2 rather than a confusing write-time error.
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1])]);
    let not_a_dir = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            not_a_dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

#[test]
fn extract_empty_lump_name_writes_unnamed_file() {
    // An all-null lump name sanitizes to the empty string, which falls back to
    // "UNNAMED" so the file can be written without an empty filename.
    let wad = write_wad(*b"IWAD", &[("", &[0x99])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let path = out_dir.path().join("UNNAMED.bin");
    assert!(path.exists(), "UNNAMED.bin not written");
    assert_eq!(std::fs::read(&path).unwrap(), vec![0x99u8]);
}

#[test]
fn extract_json_format_outputs_filenames_as_json() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filename\""))
        .stdout(predicate::str::contains("\"PLAYPAL.bin\""));
}

#[test]
fn extract_csv_format_outputs_header_and_filenames() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1]), ("COLORMAP", &[2])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "csv",
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("filename"))
        .stdout(predicate::str::contains("PLAYPAL.bin"))
        .stdout(predicate::str::contains("COLORMAP.bin"));
}

#[test]
fn extract_lenient_emits_warning_for_bad_magic() {
    let mut bytes = build_wad(*b"NOPE", &[("PLAYPAL", &[1])]);
    bytes[4..8].copy_from_slice(&1_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&(12_i32 + 1_i32).to_le_bytes());

    let file = NamedTempFile::new().expect("tempfile should be created");
    std::fs::write(file.path(), &bytes).expect("bytes should be written");
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "extract",
            file.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));
}

#[test]
fn extract_printed_summary_to_stdout() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("PLAYPAL"));
}

// ---------------------------------------------------------------------------
// `cwad build`
// ---------------------------------------------------------------------------

#[test]
fn build_empty_pwad_exits_0() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "--kind",
            "pwad",
            "-o",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    // Output file must be a valid WAD.
    let bytes = std::fs::read(out.path()).unwrap();
    assert!(crustywad::Wad::from_bytes(bytes).is_ok());
}

#[test]
fn build_empty_iwad_exits_0() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "--kind",
            "iwad",
            "-o",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();
    let bytes = std::fs::read(out.path()).unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.kind(), crustywad::WadKind::Iwad);
    assert_eq!(wad.lump_count(), 0);
}

#[test]
fn build_default_kind_is_pwad() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["build", "-o", out.path().to_str().unwrap()])
        .assert()
        .success();
    let bytes = std::fs::read(out.path()).unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.kind(), crustywad::WadKind::Pwad);
}

#[test]
fn build_with_lump_files_produces_correct_lumps() {
    // Create two lump data files.
    let lump1 = NamedTempFile::new().unwrap();
    std::fs::write(lump1.path(), b"\x01\x02\x03").unwrap();
    let lump2 = NamedTempFile::new().unwrap();
    std::fs::write(lump2.path(), b"\xAA\xBB").unwrap();

    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "--kind",
            "pwad",
            "-o",
            out.path().to_str().unwrap(),
            &format!("PLAYPAL={}", lump1.path().to_str().unwrap()),
            &format!("COLORMAP={}", lump2.path().to_str().unwrap()),
        ])
        .assert()
        .success();

    let bytes = std::fs::read(out.path()).unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 2);
    assert_eq!(wad.lump(0).unwrap().name(), "PLAYPAL");
    assert_eq!(wad.lump(1).unwrap().name(), "COLORMAP");
    assert_eq!(wad.lump_data(wad.lump(0).unwrap()), b"\x01\x02\x03");
    assert_eq!(wad.lump_data(wad.lump(1).unwrap()), b"\xAA\xBB");
}

#[test]
fn build_human_output_reports_lump_count() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["build", "-o", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("lumps: 0"));
}

#[test]
fn build_json_output_ok_true() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "build", "-o", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));
}

#[test]
fn build_csv_output_reports_lump_count() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "build", "-o", out.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("lumps: 0"));
}

#[test]
fn build_missing_lump_file_exits_2() {
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "-o",
            out.path().to_str().unwrap(),
            "BADLUMP=/nonexistent/path/lump.bin",
        ])
        .assert()
        .code(2);
}

#[test]
fn build_invalid_lump_spec_exits_3() {
    // A lump spec without '=' is a usage error.
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["build", "-o", out.path().to_str().unwrap(), "NOEQUALSSIGN"])
        .assert()
        .code(3);
}

#[test]
fn build_empty_lump_name_exits_3() {
    // A spec with an empty name (leading '=') is a usage error.
    let lump = NamedTempFile::new().unwrap();
    std::fs::write(lump.path(), b"\x01").unwrap();

    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "-o",
            out.path().to_str().unwrap(),
            &format!("={}", lump.path().to_str().unwrap()),
        ])
        .assert()
        .code(3);
}

#[test]
fn build_empty_lump_file_path_exits_3() {
    // A spec with an empty file path (trailing '=') is a usage error.
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["build", "-o", out.path().to_str().unwrap(), "PLAYPAL="])
        .assert()
        .code(3);
}

#[test]
fn build_lenient_truncates_long_name_and_warns() {
    // In lenient mode, a name over 8 bytes is truncated with a warning instead
    // of rejected outright.
    let lump = NamedTempFile::new().unwrap();
    std::fs::write(lump.path(), b"\x01").unwrap();

    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "build",
            "-o",
            out.path().to_str().unwrap(),
            &format!("NAMEISWAYTOOLONG={}", lump.path().to_str().unwrap()),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));

    let bytes = std::fs::read(out.path()).unwrap();
    let wad = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(wad.lump_count(), 1);
}

#[test]
fn build_missing_output_arg_exits_3() {
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["build"])
        .assert()
        .code(3);
}

#[test]
fn build_lump_name_too_long_exits_3() {
    // A lump name over 8 bytes fails WadBuilder validation (strict mode) — this is a
    // usage error (bad input), not an I/O failure, so it must exit 3, not 2.
    let lump = NamedTempFile::new().unwrap();
    std::fs::write(lump.path(), b"\x01").unwrap();

    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "-o",
            out.path().to_str().unwrap(),
            &format!("NAMEISWAYTOOLONG={}", lump.path().to_str().unwrap()),
        ])
        .assert()
        .code(3);
}

#[test]
fn build_nodes_builds_playable_lumps_and_preserves_non_map_lumps() {
    // A hand-packed Doom map with empty node lumps + a trailing non-map lump
    // (COLORMAP), exploded into `NAME=FILE` build specs.
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);

    // The Doom group was rebuilt with real nodes (overwriting the empty lumps).
    assert!(
        !lump_bytes(out.path(), "SEGS").is_empty(),
        "SEGS should be rebuilt non-empty"
    );
    assert!(
        !lump_bytes(out.path(), "SSECTORS").is_empty(),
        "SSECTORS should be rebuilt non-empty"
    );
    assert!(
        !lump_bytes(out.path(), "BLOCKMAP").is_empty(),
        "BLOCKMAP should be rebuilt non-empty"
    );
    // REJECT is a 1-byte all-clear table for the single-sector room.
    assert!(
        !lump_bytes(out.path(), "REJECT").is_empty(),
        "REJECT should be rebuilt non-empty"
    );
    // NODES is emitted but legitimately empty for a convex single-subsector
    // room (the engine's `numnodes == 0` path), so assert it is present rather
    // than non-empty — the full classic node-lump set is synthesized.
    assert!(
        lump_names(out.path()).iter().any(|n| n == "NODES"),
        "NODES lump should be present"
    );
    // The trailing non-map lump passed through verbatim.
    assert_eq!(lump_bytes(out.path(), "COLORMAP"), vec![4_u8, 5, 6]);
    // Engine-playable: the output assembles strict-clean.
    assert_maps_assemble_strict_clean(out.path());
}

#[test]
fn build_nodes_with_no_map_is_a_noop() {
    // A single non-map lump: no Doom group, so --nodes does nothing but note it.
    let lump = write_bytes(&[1_u8, 2, 3]);
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "build",
            "--nodes",
            "-o",
            out.path().to_str().unwrap(),
            &format!("PLAYPAL={}", lump.path().to_str().unwrap()),
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains(
            "no buildable map groups found; --nodes had no effect",
        ));
    assert_eq!(lump_bytes(out.path(), "PLAYPAL"), vec![1_u8, 2, 3]);
    // No map means no SEGS lump was added at all (not merely an empty one) —
    // `lump_bytes` panics on a missing lump, so check absence via `lump_names`.
    assert!(
        !lump_names(out.path()).iter().any(|n| n == "SEGS"),
        "no map means no node lumps were added"
    );
}

#[test]
fn build_nodes_skips_hexen_group_with_note() {
    // A Hexen map: skipped with a note; its lumps (incl. BEHAVIOR) pass through.
    let fixture = write_hexen_map_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("is a Hexen map"))
        .stderr(predicate::str::contains("#352"));
    // A Hexen map carries a BEHAVIOR lump; it must survive the pass-through.
    assert!(
        lump_names(out.path()).iter().any(|n| n == "BEHAVIOR"),
        "Hexen BEHAVIOR lump should be preserved"
    );
}

#[test]
fn build_nodes_refuses_a_map_that_fails_to_assemble() {
    // Explode a valid Doom map, then blank out VERTEXES so the linedefs
    // reference out-of-range vertices and strict assembly fails during the
    // --nodes rebuild (exit 3, before any output is written).
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (mut specs, mut files) = explode_wad_to_build_specs(fixture.path());
    let empty = write_bytes(&[]);
    for spec in &mut specs {
        if spec.starts_with("VERTEXES=") {
            *spec = format!("VERTEXES={}", empty.path().to_str().unwrap());
        }
    }
    files.push(empty); // keep the backing file alive for the command's duration
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("failed to assemble map"));
}

#[test]
fn build_nodes_lenient_reports_assembly_repair_warnings() {
    // Truncate a valid Doom map's VERTEXES to a single vertex, so its linedefs
    // reference out-of-range-but-clampable vertices. In --lenient mode assembly
    // repairs them and records a warning, which the --nodes rebuild surfaces on
    // stderr (the node build then fails on the degenerate geometry, so exit code
    // is not asserted — only that the assembly-repair warning is reported).
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (mut specs, mut files) = explode_wad_to_build_specs(fixture.path());
    let one_vertex = write_bytes(&[0, 0, 0, 0]); // a single VERTEXES record (x=0, y=0)
    for spec in &mut specs {
        if spec.starts_with("VERTEXES=") {
            *spec = format!("VERTEXES={}", one_vertex.path().to_str().unwrap());
        }
    }
    files.push(one_vertex);
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "--lenient".to_string(),
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .stderr(predicate::str::contains("warning:"));
}

#[test]
fn build_nodes_skips_doom64_group_with_note() {
    // A Doom 64 map: skipped with a note (#353); no classic node build applies.
    let fixture = write_doom64_textured_map_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("is a Doom 64 map"))
        .stderr(predicate::str::contains("#353"));
}

#[test]
fn build_nodes_builds_znodes_for_udmf_group() {
    // A UDMF map: build --nodes builds GL nodes into a fresh ZNODES lump
    // inserted immediately after TEXTMAP; the auto-format note fires and the
    // TEXTMAP bytes survive byte-identical.
    let fixture = write_udmf_square_room_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("building GL nodes"));

    // The group grew a ZNODES lump inserted immediately after TEXTMAP; the
    // trailing COLORMAP passes through.
    assert_eq!(
        lump_names(out.path()),
        vec!["MAP01", "TEXTMAP", "ZNODES", "ENDMAP", "COLORMAP"]
    );
    // TEXTMAP bytes are byte-identical to the input map text.
    assert_eq!(
        lump_bytes(out.path(), "TEXTMAP"),
        udmf_square_room().into_bytes()
    );
    // The ZNODES lump is a GL stream; the square room resolves to the minimal
    // XGLN dialect under the default (auto) GL format.
    let znodes = lump_bytes(out.path(), "ZNODES");
    assert!(
        znodes.starts_with(b"XGLN"),
        "ZNODES should be an XGLN stream, got {:?}",
        &znodes[..znodes.len().min(4)]
    );
}

#[test]
fn build_nodes_replaces_stale_znodes_and_preserves_port_lumps() {
    // A UDMF group carrying a stale/garbage ZNODES plus a DIALOGUE port lump
    // between TEXTMAP and ENDMAP: build --nodes replaces the ZNODES bytes in
    // place and preserves DIALOGUE (bytes and position) verbatim; the lump
    // count is unchanged.
    let textmap = udmf_square_room();
    let fixture = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ZNODES", b"JUNK"),
            ("DIALOGUE", &[7, 8, 9]),
            ("ENDMAP", b""),
        ],
    );
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);

    // Same lumps, same order, same count — the stale ZNODES was replaced in
    // place, not appended.
    assert_eq!(
        lump_names(out.path()),
        vec!["MAP01", "TEXTMAP", "ZNODES", "DIALOGUE", "ENDMAP"]
    );
    // The ZNODES bytes are a real GL stream now, not the garbage payload.
    let znodes = lump_bytes(out.path(), "ZNODES");
    assert_ne!(znodes, b"JUNK");
    assert!(
        znodes.starts_with(b"XGLN"),
        "ZNODES should be an XGLN stream, got {:?}",
        &znodes[..znodes.len().min(4)]
    );
    // The DIALOGUE port lump survived verbatim.
    assert_eq!(lump_bytes(out.path(), "DIALOGUE"), vec![7_u8, 8, 9]);
    // TEXTMAP bytes are byte-identical.
    assert_eq!(
        lump_bytes(out.path(), "TEXTMAP"),
        udmf_square_room().into_bytes()
    );
}

#[test]
fn build_nodes_skips_udmf_group_for_non_gl_format_with_note() {
    // --node-format xnod has no GL stream, so a UDMF group is passed through
    // untouched with a note (#384).
    let fixture = write_udmf_square_room_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "--node-format".to_string(),
        "xnod".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("not valid for ZNODES"))
        .stderr(predicate::str::contains("#384"));

    // Passed through untouched: no ZNODES lump was added.
    assert_eq!(
        lump_names(out.path()),
        vec!["MAP01", "TEXTMAP", "ENDMAP", "COLORMAP"]
    );
}

#[test]
fn build_nodes_refuses_udmf_map_that_fails_to_assemble() {
    // A UDMF map whose TEXTMAP parses but whose linedef `sidefront` points past
    // the sidedef count: strict assembly rejects the dangling reference, so the
    // --nodes build refuses (exit 3) while naming the map.
    let textmap = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 128; y = 0; }\n",
        "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 99; blocking = true; }\n",
    );
    let fixture = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
        ],
    );
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("failed to assemble map"));
}

#[test]
fn build_nodes_lenient_udmf_echoes_assembly_repair_warning() {
    // Lenient: the same dangling `sidefront` is clamped and recorded as an
    // assembly warning, which the build-walk echoes on stderr before the node
    // build then fails on the degenerate single-linedef geometry (exit 3).
    let textmap = concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 128; y = 0; }\n",
        "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 99; blocking = true; }\n",
    );
    let fixture = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
        ],
    );
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "--lenient".to_string(),
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("warning: MAP01"))
        .stderr(predicate::str::contains(
            "failed to build nodes for map MAP01",
        ));
}

#[test]
fn build_nodes_refuses_udmf_mixed_sector_fan_and_hints_lenient() {
    // The UDMF mixed-sector fan assembles strict-clean but its GL node build
    // fails (MixedSectorSubsector); the build refuses (exit 3), names the map,
    // and — the error being lenient-recoverable — hints --lenient.
    let fixture = write_udmf_mixed_sector_fan_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "failed to build nodes for map MAP01",
        ))
        .stderr(predicate::str::contains("re-run with --lenient"));
}

#[test]
fn build_nodes_lenient_recovers_udmf_mixed_sector_fan() {
    // Lenient: the UDMF fan's mixed-sector subsector is tolerated, so the GL
    // node build succeeds and surfaces its recovery warning on stderr (the
    // build-walk warning-echo path).
    let fixture = write_udmf_mixed_sector_fan_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "--lenient".to_string(),
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("warning: MAP01"));
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn build_nodes_skips_udmf_group_for_znod_format_with_note() {
    // --node-format znod (zlib-only, non-GL) has no ZNODES carrier, so a UDMF
    // group is passed through untouched with a note naming the requested format.
    let fixture = write_udmf_square_room_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "--node-format".to_string(),
        "znod".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("znod"))
        .stderr(predicate::str::contains("not valid for ZNODES"));

    // Passed through untouched: no ZNODES lump was added.
    assert_eq!(
        lump_names(out.path()),
        vec!["MAP01", "TEXTMAP", "ENDMAP", "COLORMAP"]
    );
}

#[test]
fn build_nodes_udmf_honors_explicit_gl_dialect() {
    // An explicit --node-format xgl3 forces the XGL3 dialect for the ZNODES
    // stream (overriding the auto XGLN selection).
    let fixture = write_udmf_square_room_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "--node-format".to_string(),
        "xgl3".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);

    let znodes = lump_bytes(out.path(), "ZNODES");
    assert!(
        znodes.starts_with(b"XGL3"),
        "ZNODES should be an XGL3 stream, got {:?}",
        &znodes[..znodes.len().min(4)]
    );
}

#[test]
fn build_nodes_handles_mixed_doom_and_udmf_groups() {
    // A single WAD carrying BOTH a Doom map group (empty node lumps) and a UDMF
    // map group, plus a loose non-map lump: build --nodes (default format)
    // rebuilds the Doom group's classic node lumps and builds a GL ZNODES for
    // the UDMF group, while the non-map lump survives verbatim.
    let doom_fixture = write_doom_square_room_empty_nodes_wad();
    let doom_bytes = std::fs::read(doom_fixture.path()).unwrap();
    let doom_wad = crustywad::Wad::from_bytes(doom_bytes).expect("doom fixture parses");
    // The Doom fixture is [MAP01 .. BLOCKMAP, COLORMAP]; take it whole, then
    // append a UDMF group so the WAD holds one group of each format.
    let mut lumps: Vec<(String, Vec<u8>)> = doom_wad
        .lumps()
        .iter()
        .map(|l| (l.name().to_string(), doom_wad.lump_data(l).to_vec()))
        .collect();
    lumps.push(("MAP02".to_string(), Vec::new()));
    lumps.push(("TEXTMAP".to_string(), udmf_square_room().into_bytes()));
    lumps.push(("ENDMAP".to_string(), Vec::new()));

    let combined = write_wad_owned(*b"PWAD", &lumps);
    let (specs, _files) = explode_wad_to_build_specs(combined.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);

    // Doom group: classic node lumps rebuilt (SEGS non-empty, NODES present).
    assert!(
        !lump_bytes(out.path(), "SEGS").is_empty(),
        "the Doom group's SEGS should be rebuilt non-empty"
    );
    assert!(
        lump_names(out.path()).iter().any(|n| n == "NODES"),
        "the Doom group's NODES lump should be present"
    );
    // UDMF group: a GL ZNODES stream was built (XGLN under the default format).
    let znodes = lump_bytes(out.path(), "ZNODES");
    assert!(
        znodes.starts_with(b"XGLN"),
        "the UDMF group's ZNODES should be an XGLN stream, got {:?}",
        &znodes[..znodes.len().min(4)]
    );
    // The loose non-map lump survived verbatim.
    assert_eq!(lump_bytes(out.path(), "COLORMAP"), vec![4_u8, 5, 6]);
}

#[test]
fn build_without_nodes_leaves_packed_node_lumps_untouched() {
    // Regression: without --nodes, packed (empty) node lumps are not rebuilt.
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);

    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "without --nodes, the packed empty SEGS stays empty"
    );
}

#[test]
fn build_nodes_refuses_mixed_sector_fan_and_hints_lenient() {
    // Strict: the fan assembles cleanly but `add_doom_map_with_nodes` fails
    // with `MixedSectorSubsector`; the build refuses (exit 3), names the map,
    // and — because the error IS lenient-recoverable (#264) — hints `--lenient`.
    let fixture = write_doom_mixed_sector_fan_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "failed to build nodes for map MAP01",
        ))
        .stderr(predicate::str::contains("re-run with --lenient"));
}

#[test]
fn build_nodes_lenient_recovers_mixed_sector_fan() {
    // Lenient: the fan is tolerated (ADR-0024 §7), so the same build succeeds
    // and produces populated node lumps despite the mixed-sector subsector.
    let fixture = write_doom_mixed_sector_fan_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "--lenient".to_string(),
        "build".to_string(),
        "--nodes".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);
    assert!(
        !lump_bytes(out.path(), "SEGS").is_empty(),
        "lenient node build populates SEGS despite the fan"
    );
}

#[test]
fn build_nodes_with_node_format_xgl3_emits_the_gl_stream() {
    // build --nodes --node-format xgl3: the rebuilt Doom group's SSECTORS
    // carries a single XGL3 stream (ADR-0026), SEGS/NODES stay empty, the
    // output re-assembles strict-clean, and the trailing non-map lump
    // (COLORMAP) is preserved.
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "--node-format".to_string(),
        "xgl3".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0);

    let ssectors = lump_bytes(out.path(), "SSECTORS");
    assert!(
        ssectors.starts_with(b"XGL3"),
        "SSECTORS should be an XGL3 stream, got {:?}",
        &ssectors[..ssectors.len().min(4)]
    );
    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "GL nodes leave SEGS empty"
    );
    assert!(
        lump_bytes(out.path(), "NODES").is_empty(),
        "GL nodes leave NODES empty"
    );
    assert_eq!(lump_bytes(out.path(), "COLORMAP"), vec![4_u8, 5, 6]);
    assert_maps_assemble_strict_clean(out.path());
}

#[test]
fn build_node_format_without_nodes_is_noted_and_ignored() {
    // Mirror convert_node_format_without_nodes_is_noted_and_ignored: a
    // --node-format given without --nodes is noted and ignored rather than
    // silently dropped.
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--node-format".to_string(),
        "xnod".to_string(), // no --nodes
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(0)
        .stderr(predicate::str::contains(
            "--node-format has no effect without --nodes",
        ));

    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "no --nodes: packed empty SEGS stays untouched"
    );
}

#[cfg(not(feature = "extended-nodes-zlib"))]
#[test]
fn build_node_format_zgln_without_feature_errors_clearly() {
    let fixture = write_doom_square_room_empty_nodes_wad();
    let (specs, _files) = explode_wad_to_build_specs(fixture.path());
    let out = NamedTempFile::new().unwrap();

    let mut args = vec![
        "build".to_string(),
        "--nodes".to_string(),
        "--node-format".to_string(),
        "zgln".to_string(),
        "-o".to_string(),
        out.path().to_str().unwrap().to_string(),
    ];
    args.extend(specs);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(&args)
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "--node-format zgln requires cwad built with the extended-nodes-zlib feature",
        ));
}

// ---------------------------------------------------------------------------
// Hardening: invalid-file regression tests
//
// Each malformed fixture is applied to every read subcommand that accepts a
// WAD path: `info`, `list`, `validate`, and `diff` (as one of the inputs).
// All must exit non-zero in strict mode (the default).
// ---------------------------------------------------------------------------

/// Returns a 6-byte truncated WAD — not even a complete 12-byte header.
fn truncated_wad_bytes() -> Vec<u8> {
    b"IWAD\x00\x00".to_vec()
}

/// Returns a 12-byte WAD whose header claims 5 lumps but provides no directory
/// bytes — any attempt to read a directory entry past the file end must fail.
fn header_only_no_directory_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"IWAD");
    b.extend_from_slice(&5_i32.to_le_bytes()); // numlumps = 5
    b.extend_from_slice(&12_i32.to_le_bytes()); // infotableofs = 12 (= end of file)
    b
}

/// Returns a 12-byte file whose four-byte magic is `XWAD` (not `IWAD`/`PWAD`).
fn wrong_magic_bytes() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"XWAD");
    b.extend_from_slice(&0_i32.to_le_bytes()); // numlumps = 0
    b.extend_from_slice(&12_i32.to_le_bytes()); // infotableofs = 12
    b
}

// --- truncated WAD ---

#[test]
fn hardening_truncated_wad_info_exits_nonzero() {
    let file = write_bytes(&truncated_wad_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_truncated_wad_list_exits_nonzero() {
    let file = write_bytes(&truncated_wad_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_truncated_wad_validate_exits_nonzero() {
    let file = write_bytes(&truncated_wad_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_truncated_wad_diff_exits_nonzero() {
    let good = write_wad(*b"IWAD", &[]);
    let bad = write_bytes(&truncated_wad_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            good.path().to_str().unwrap(),
            bad.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

// --- valid header but no directory bytes ---

#[test]
fn hardening_header_only_no_directory_info_exits_nonzero() {
    let file = write_bytes(&header_only_no_directory_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_header_only_no_directory_list_exits_nonzero() {
    let file = write_bytes(&header_only_no_directory_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_header_only_no_directory_validate_exits_nonzero() {
    let file = write_bytes(&header_only_no_directory_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_header_only_no_directory_diff_exits_nonzero() {
    let good = write_wad(*b"IWAD", &[]);
    let bad = write_bytes(&header_only_no_directory_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            good.path().to_str().unwrap(),
            bad.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

// --- wrong magic (XWAD) in strict mode ---

#[test]
fn hardening_wrong_magic_strict_info_exits_nonzero() {
    let file = write_bytes(&wrong_magic_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_wrong_magic_strict_list_exits_nonzero() {
    let file = write_bytes(&wrong_magic_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_wrong_magic_strict_validate_exits_nonzero() {
    let file = write_bytes(&wrong_magic_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_wrong_magic_strict_diff_exits_nonzero() {
    let good = write_wad(*b"IWAD", &[]);
    let bad = write_bytes(&wrong_magic_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            good.path().to_str().unwrap(),
            bad.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

// --- zero-size (empty) file ---

#[test]
fn hardening_zero_size_file_info_exits_nonzero() {
    let file = write_bytes(b"");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_zero_size_file_list_exits_nonzero() {
    let file = write_bytes(b"");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_zero_size_file_validate_exits_nonzero() {
    let file = write_bytes(b"");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_zero_size_file_diff_exits_nonzero() {
    let good = write_wad(*b"IWAD", &[]);
    let bad = write_bytes(b"");
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            good.path().to_str().unwrap(),
            bad.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

// ---------------------------------------------------------------------------
// Hardening: exit-code consistency
// ---------------------------------------------------------------------------

#[test]
fn hardening_validate_corrupt_exits_2() {
    // `validate` on a structurally corrupt WAD must exit 2 (parse error), not
    // exit 0 (success). `validate` never returns 1 (unlike `diff`, which uses
    // 1 for semantic differences) — this test only guards against 2 vs 0.
    let file = write_bytes(&truncated_wad_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", file.path().to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_validate_nonexistent_exits_2() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("hardening_test_unique.wad");
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", missing.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn hardening_info_valid_wad_exits_0() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .code(0);
}

#[test]
fn hardening_list_valid_wad_exits_0() {
    let wad = write_wad(*b"IWAD", &[("COLORMAP", &[2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", wad.path().to_str().unwrap()])
        .assert()
        .code(0);
}

// ---------------------------------------------------------------------------
// Hardening: format consistency under error conditions
// ---------------------------------------------------------------------------

#[test]
fn hardening_info_json_corrupt_wad_exits_nonzero_not_panic() {
    // `info --format json` on a corrupt WAD must exit non-zero without
    // panicking. The `info` subcommand propagates parse errors through
    // `run()` → `main()` which prints a human-readable "error: …" on stderr
    // and exits 2; stderr must not contain a Rust panic backtrace header or
    // an unwrap-on-error message.
    let file = write_bytes(&truncated_wad_bytes());
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", file.path().to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("thread '").not())
        .stderr(predicate::str::contains("unwrap()").not());
}

#[test]
fn hardening_list_csv_valid_wad_has_header_row() {
    // `list --format csv` must emit a header row before any data rows.
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[9]), ("COLORMAP", &[0, 1, 2])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        // The trailing `audio` column carries the per-lump audio annotation
        // (empty here — neither lump is a detected audio format).
        .stdout(predicate::str::starts_with(
            "index,filepos,size,name,audio\n",
        ))
        .stdout(predicate::str::contains("PLAYPAL"))
        .stdout(predicate::str::contains("COLORMAP"));
}

// ---------------------------------------------------------------------------
// Hardening: write command edge cases
// ---------------------------------------------------------------------------

#[test]
fn hardening_build_no_lumps_exits_0_valid_wad() {
    // `build` with no lump arguments must exit 0 and produce a structurally
    // valid empty WAD.
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["build", "-o", out.path().to_str().unwrap()])
        .assert()
        .code(0);

    let bytes = std::fs::read(out.path()).unwrap();
    assert!(
        crustywad::Wad::from_bytes(bytes).is_ok(),
        "build with no lumps must produce a valid WAD"
    );
}

#[test]
fn hardening_merge_one_input_exits_0() {
    // Merging a single WAD is valid — the output should be parseable and
    // contain the same lump as the input.
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            wad.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .code(0);

    // The merged output must be a valid WAD with the same lump data, not just
    // the same lump name.
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", out.path().to_str().unwrap()])
        .assert()
        .success();

    let bytes = std::fs::read(out.path()).unwrap();
    let merged = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(merged.lump_count(), 1);
    let lump = merged.lump(0).unwrap();
    assert_eq!(lump.name(), "PLAYPAL");
    assert_eq!(merged.lump_data(lump), &[1, 2, 3]);
}

#[test]
fn hardening_extract_truncated_wad_exits_nonzero() {
    // `extract` on a truncated WAD must fail rather than writing partial output.
    let bad = write_bytes(&truncated_wad_bytes());
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            bad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);

    let entries: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("output dir should be readable")
        .collect();
    assert!(
        entries.is_empty(),
        "extract must not write partial output on failure"
    );
}

#[test]
fn hardening_merge_truncated_wad_exits_nonzero() {
    // `merge` must exit non-zero when any input WAD is structurally corrupt.
    let bad = write_bytes(&truncated_wad_bytes());
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            bad.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .code(2);
}

// ---------------------------------------------------------------------------
// `cwad convert`
// ---------------------------------------------------------------------------

/// The five classic Doom map data lumps, as raw bytes.
struct DoomMapLumps {
    things: Vec<u8>,
    linedefs: Vec<u8>,
    sidedefs: Vec<u8>,
    vertexes: Vec<u8>,
    sectors: Vec<u8>,
}

/// The five classic Doom map data lumps for a minimal one-sector, one-linedef,
/// one-thing map (mirrors the fixture used in the guide's conversion page).
fn doom_map_lumps() -> DoomMapLumps {
    let mut vertexes = Vec::new();
    for v in [0_i16, 0, 64, 0] {
        vertexes.extend_from_slice(&v.to_le_bytes());
    }

    let mut linedefs = Vec::new();
    for v in [0_u16, 1, 1, 0, 0, 0, 0xffff] {
        linedefs.extend_from_slice(&v.to_le_bytes());
    }

    let mut sidedefs = Vec::new();
    sidedefs.extend_from_slice(&0_i16.to_le_bytes());
    sidedefs.extend_from_slice(&0_i16.to_le_bytes());
    sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedefs.extend_from_slice(b"-\0\0\0\0\0\0\0");
    sidedefs.extend_from_slice(b"STARTAN3");
    sidedefs.extend_from_slice(&0_u16.to_le_bytes());

    let mut sectors = Vec::new();
    sectors.extend_from_slice(&0_i16.to_le_bytes());
    sectors.extend_from_slice(&128_i16.to_le_bytes());
    sectors.extend_from_slice(b"FLOOR4_8");
    sectors.extend_from_slice(b"CEIL3_5\0");
    sectors.extend_from_slice(&160_i16.to_le_bytes());
    sectors.extend_from_slice(&0_i16.to_le_bytes());
    sectors.extend_from_slice(&0_i16.to_le_bytes());

    let mut things = Vec::new();
    for v in [32_i16, 32, 0, 1, 7] {
        things.extend_from_slice(&v.to_le_bytes());
    }

    DoomMapLumps {
        things,
        linedefs,
        sidedefs,
        vertexes,
        sectors,
    }
}

/// A PWAD containing `PLAYPAL`, one classic Doom map named `map_name`, then
/// `COLORMAP`, so pass-through of non-map lumps (and their order) is testable.
fn write_doom_map_wad(map_name: &str) -> NamedTempFile {
    let m = doom_map_lumps();
    write_wad(
        *b"PWAD",
        &[
            ("PLAYPAL", &[1, 2, 3]),
            (map_name, b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &m.linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
            ("COLORMAP", &[4, 5, 6]),
        ],
    )
}

/// A UDMF `TEXTMAP` body equivalent to [`doom_map_lumps`]. When `zdoom_fields`
/// is set the thing carries a nonzero `height`, which the Doom format has no
/// slot for (ADR-0019 tier 3).
fn udmf_textmap(zdoom_fields: bool) -> String {
    let thing = if zdoom_fields {
        "thing { x = 32; y = 32; height = 16; type = 1; skill1 = true; skill2 = true; skill3 = true; }\n"
    } else {
        "thing { x = 32; y = 32; type = 1; skill1 = true; skill2 = true; skill3 = true; }\n"
    };
    format!(
        concat!(
            "namespace = \"doom\";\n",
            "vertex {{ x = 0; y = 0; }}\n",
            "vertex {{ x = 64; y = 0; }}\n",
            "sector {{ texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }}\n",
            "sidedef {{ sector = 0; }}\n",
            "linedef {{ v1 = 0; v2 = 1; sidefront = 0; blocking = true; }}\n",
            "{}"
        ),
        thing
    )
}

/// A PWAD containing a single UDMF map (`MAP01`) plus a trailing `COLORMAP`.
fn write_udmf_map_wad(zdoom_fields: bool) -> NamedTempFile {
    let textmap = udmf_textmap(zdoom_fields);
    write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
            ("COLORMAP", &[4, 5, 6]),
        ],
    )
}

/// Reads the data of the named lump from a WAD file.
fn lump_bytes(path: &std::path::Path, name: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).expect("output WAD should be readable");
    let wad = crustywad::Wad::from_bytes(bytes).expect("output WAD should parse");
    let lump = wad
        .lump_by_name(name)
        .unwrap_or_else(|| panic!("{name} should be present"));
    wad.lump_data(lump).to_vec()
}

/// Reads the lump names of a WAD file, in directory order.
fn lump_names(path: &std::path::Path) -> Vec<String> {
    let bytes = std::fs::read(path).expect("output WAD should be readable");
    let wad = crustywad::Wad::from_bytes(bytes).expect("output WAD should parse");
    wad.lumps()
        .iter()
        .map(|l| l.name().to_owned())
        .collect::<Vec<_>>()
}

#[test]
fn convert_doom_to_udmf_replaces_map_lumps_in_place() {
    let wad = write_doom_map_wad("MAP01");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 1 map to udmf"));

    // The map group is replaced in place: the original binary lumps are gone,
    // and the surrounding non-map lumps keep their directory order.
    assert_eq!(
        lump_names(out.path()),
        vec!["PLAYPAL", "MAP01", "TEXTMAP", "ENDMAP", "COLORMAP"]
    );
    // Non-map lumps pass through byte for byte, not merely by name — conversion
    // must not rewrite payloads it does not own.
    assert_eq!(lump_bytes(out.path(), "PLAYPAL"), vec![1, 2, 3]);
    assert_eq!(lump_bytes(out.path(), "COLORMAP"), vec![4, 5, 6]);
}

#[test]
fn convert_udmf_to_doom_emits_lump_run_and_nodes_warning() {
    let wad = write_udmf_map_wad(false);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
        ])
        .assert()
        .code(0)
        // The unconditional NodesNotBuilt warning (ADR-0019 §4) must be shown.
        .stderr(predicate::str::contains("MAP01: node lumps"))
        .stderr(predicate::str::contains("run a nodebuilder"));

    assert_eq!(
        lump_names(out.path()),
        vec![
            // The canonical Doom lump order, with the node lumps present but
            // zero-length (ADR-0019 §4).
            "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES",
            "SECTORS", "REJECT", "BLOCKMAP", "COLORMAP",
        ]
    );
}

/// A UDMF `TEXTMAP` body for a closed one-sector square room (four vertices,
/// four one-sided walls) — real geometry a node build turns into a non-empty
/// `SEGS`/`SSECTORS` run, unlike the single-linedef [`udmf_textmap`] fixture.
fn udmf_square_room() -> String {
    concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 128; y = 0; }\n",
        "vertex { x = 128; y = 128; }\n",
        "vertex { x = 0; y = 128; }\n",
        "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; blocking = true; }\n",
        "linedef { v1 = 1; v2 = 2; sidefront = 1; blocking = true; }\n",
        "linedef { v1 = 2; v2 = 3; sidefront = 2; blocking = true; }\n",
        "linedef { v1 = 3; v2 = 0; sidefront = 3; blocking = true; }\n",
        "thing { x = 64; y = 64; type = 1; skill1 = true; skill2 = true; skill3 = true; }\n",
    )
    .to_owned()
}

/// A PWAD holding a single UDMF square-room map (`MAP01`) plus a trailing
/// `COLORMAP`.
fn write_udmf_square_room_wad() -> NamedTempFile {
    let textmap = udmf_square_room();
    write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
            ("COLORMAP", &[4, 5, 6]),
        ],
    )
}

/// A UDMF `TEXTMAP` for a mixed-sector fan: two coincident one-sided walls
/// facing *different* sectors, which no seg line can separate. The classic BSP
/// pass rejects this in strict mode (`NodeBuildError::MixedSectorSubsector`) and
/// tolerates it in lenient mode (ADR-0024 §7 amendment) — the geometry the
/// retail masters themselves ship.
fn udmf_mixed_sector_fan() -> String {
    concat!(
        "namespace = \"doom\";\n",
        "vertex { x = 0; y = 0; }\n",
        "vertex { x = 64; y = 0; }\n",
        "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
        "sector { texturefloor = \"FLOOR4_8\"; textureceiling = \"CEIL3_5\"; }\n",
        "sidedef { sector = 0; texturemiddle = \"STARTAN3\"; }\n",
        "sidedef { sector = 1; texturemiddle = \"STARTAN3\"; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 0; blocking = true; }\n",
        "linedef { v1 = 0; v2 = 1; sidefront = 1; blocking = true; }\n",
        "thing { x = 32; y = 0; type = 1; skill1 = true; skill2 = true; skill3 = true; }\n",
    )
    .to_owned()
}

/// A PWAD holding the mixed-sector-fan map (`MAP01`).
fn write_udmf_mixed_sector_fan_wad() -> NamedTempFile {
    let textmap = udmf_mixed_sector_fan();
    write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
        ],
    )
}

/// Re-reads a WAD file and asserts every map group assembles strict-clean (no
/// assembly warnings), the engine-playable acceptance criterion.
fn assert_maps_assemble_strict_clean(path: &std::path::Path) {
    let bytes = std::fs::read(path).expect("output WAD should be readable");
    let wad = crustywad::Wad::from_bytes(bytes).expect("output WAD should parse");
    let groups = wad.map_groups();
    assert!(
        !groups.is_empty(),
        "output WAD should contain at least one map group"
    );
    for group in &groups {
        let map = crustywad::map::Map::assemble(&wad, group)
            .unwrap_or_else(|e| panic!("map {} should assemble: {e}", group.name));
        assert!(
            map.warnings().is_empty(),
            "map {} should assemble strict-clean, got warnings {:?}",
            group.name,
            map.warnings()
        );
    }
}

#[test]
fn convert_udmf_to_doom_with_nodes_builds_playable_lumps() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
        ])
        .assert()
        .code(0)
        // With --nodes the node lumps are built for real, so the unconditional
        // NodesNotBuilt warning must NOT appear (Global Constraint 4).
        .stderr(predicate::str::contains("run a nodebuilder").not());

    // The canonical Doom lump run, with the node lumps present.
    assert_eq!(
        lump_names(out.path()),
        vec![
            "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES",
            "SECTORS", "REJECT", "BLOCKMAP", "COLORMAP",
        ]
    );

    // A real geometry build yields non-empty SEGS (and the sibling node lumps),
    // unlike the zero-length lumps `add_doom_map` emits.
    assert!(
        !lump_bytes(out.path(), "SEGS").is_empty(),
        "SEGS should be non-empty after a node build"
    );
    assert!(!lump_bytes(out.path(), "SSECTORS").is_empty());
    assert!(!lump_bytes(out.path(), "BLOCKMAP").is_empty());

    // The output is engine-playable: its maps re-read and assemble strict-clean.
    assert_maps_assemble_strict_clean(out.path());
}

#[test]
fn convert_with_node_format_xnod_emits_xnod_stream() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "xnod",
        ])
        .assert()
        .code(0);

    // The extended writer packs everything into a single XNOD stream in NODES,
    // leaving SEGS/SSECTORS zero-length (ADR-0025).
    let nodes = lump_bytes(out.path(), "NODES");
    assert!(
        nodes.starts_with(b"XNOD"),
        "NODES should be an XNOD stream, got {:?}",
        &nodes[..nodes.len().min(4)]
    );
    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "extended nodes leave SEGS empty"
    );
    // The output re-reads and assembles (the uncompressed XNOD reader is always on).
    assert_maps_assemble_strict_clean(out.path());
}

#[test]
fn convert_with_node_format_classic_stays_classic() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "classic",
        ])
        .assert()
        .code(0);

    // Classic layout: real (non-empty) SEGS, and NODES is not an extended stream.
    assert!(
        !lump_bytes(out.path(), "SEGS").is_empty(),
        "classic build yields non-empty SEGS"
    );
    let nodes = lump_bytes(out.path(), "NODES");
    assert!(
        !nodes.starts_with(b"XNOD") && !nodes.starts_with(b"ZNOD"),
        "classic NODES is not an extended stream"
    );
    assert_maps_assemble_strict_clean(out.path());
}

#[test]
fn convert_node_format_without_nodes_is_noted_and_ignored() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--node-format",
            "xnod", // no --nodes
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains(
            "--node-format has no effect without --nodes",
        ))
        // Without --nodes, node lumps are zero-length and the NodesNotBuilt warning shows.
        .stderr(predicate::str::contains("run a nodebuilder"));

    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "no --nodes: SEGS stays zero-length"
    );
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn convert_with_node_format_znod_emits_znod_stream() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "znod",
        ])
        .assert()
        .code(0);

    let nodes = lump_bytes(out.path(), "NODES");
    assert!(nodes.starts_with(b"ZNOD"), "NODES should be a ZNOD stream");
    // Reading ZNOD back needs the zlib feature, which this test is gated on.
    assert_maps_assemble_strict_clean(out.path());
}

#[cfg(not(feature = "extended-nodes-zlib"))]
#[test]
fn convert_node_format_znod_without_feature_errors_clearly() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "znod",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "--node-format znod requires cwad built with the extended-nodes-zlib feature",
        ));
}

/// Every documented --node-format value parses (guards clap's kebab-casing
/// at digit boundaries: `xgl2` must not render as `xgl-2`).
#[test]
fn convert_node_format_accepts_every_documented_value() {
    for value in [
        "classic", "xnod", "znod", "xgln", "xgl2", "xgl3", "gl", "zgln", "zgl2", "zgl3", "zgl",
    ] {
        // --help-style parse check: an unknown value fails at clap level with
        // exit 2 before any file I/O; a known value proceeds far enough to
        // fail on the missing input file instead. Assert the clap layer
        // accepted the value by checking the error is NOT "invalid value".
        let assert = Command::cargo_bin("cwad")
            .unwrap()
            .args([
                "convert",
                "--to",
                "doom",
                "--nodes",
                "--node-format",
                value,
                "missing.wad",
                "-o",
                "out.wad",
            ])
            .assert()
            .failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
        assert!(
            !stderr.contains("invalid value"),
            "--node-format {value} rejected at the clap layer: {stderr}"
        );
    }
}

#[test]
fn convert_with_node_format_xgl3_emits_the_gl_stream_in_ssectors() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "xgl3",
        ])
        .assert()
        .code(0);

    // The GL writer packs everything into a single XGL3 stream in SSECTORS,
    // leaving SEGS/NODES zero-length (ADR-0026).
    let ssectors = lump_bytes(out.path(), "SSECTORS");
    assert!(
        ssectors.starts_with(b"XGL3"),
        "SSECTORS should be an XGL3 stream, got {:?}",
        &ssectors[..ssectors.len().min(4)]
    );
    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "GL nodes leave SEGS empty"
    );
    assert!(
        lump_bytes(out.path(), "NODES").is_empty(),
        "GL nodes leave NODES empty"
    );
    assert_maps_assemble_strict_clean(out.path());
}

/// The remaining explicit uncompressed GL dialects (`xgln`/`xgl2` — `xgl3`
/// has its own dedicated carrier-layout test above) drive the writer
/// end-to-end: the emitted SSECTORS stream carries the requested tag.
#[test]
fn convert_with_each_explicit_gl_dialect_emits_its_tag() {
    for (value, tag) in [("xgln", b"XGLN"), ("xgl2", b"XGL2")] {
        let wad = write_udmf_square_room_wad();
        let out = NamedTempFile::new().unwrap();
        Command::cargo_bin("cwad")
            .unwrap()
            .args([
                "convert",
                wad.path().to_str().unwrap(),
                "-o",
                out.path().to_str().unwrap(),
                "--to",
                "doom",
                "--nodes",
                "--node-format",
                value,
            ])
            .assert()
            .code(0);
        let ssectors = lump_bytes(out.path(), "SSECTORS");
        assert!(
            ssectors.starts_with(tag),
            "--node-format {value}: SSECTORS should start with {tag:?}"
        );
        assert_maps_assemble_strict_clean(out.path());
    }
}

/// The compressed twins (and the compressed auto-format, which resolves ZGLN
/// on this whole-unit map) emit their `Z*` tags.
#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn convert_with_each_compressed_gl_dialect_emits_its_tag() {
    for (value, tag) in [("zgln", b"ZGLN"), ("zgl2", b"ZGL2"), ("zgl", b"ZGLN")] {
        let wad = write_udmf_square_room_wad();
        let out = NamedTempFile::new().unwrap();
        Command::cargo_bin("cwad")
            .unwrap()
            .args([
                "convert",
                wad.path().to_str().unwrap(),
                "-o",
                out.path().to_str().unwrap(),
                "--to",
                "doom",
                "--nodes",
                "--node-format",
                value,
            ])
            .assert()
            .code(0);
        let ssectors = lump_bytes(out.path(), "SSECTORS");
        assert!(
            ssectors.starts_with(tag),
            "--node-format {value}: SSECTORS should start with {tag:?}"
        );
        assert_maps_assemble_strict_clean(out.path());
    }
}

#[test]
fn convert_with_node_format_gl_auto_selects_the_minimal_dialect() {
    // write_udmf_square_room_wad uses whole-unit coordinates and a handful of
    // linedefs, so nothing forces escalation past the minimal GL dialect.
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "gl",
        ])
        .assert()
        .code(0);

    let ssectors = lump_bytes(out.path(), "SSECTORS");
    assert!(
        ssectors.starts_with(b"XGLN"),
        "SSECTORS should be an XGLN stream, got {:?}",
        &ssectors[..ssectors.len().min(4)]
    );
    assert_maps_assemble_strict_clean(out.path());
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn convert_with_node_format_zgl3_emits_the_compressed_stream() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "zgl3",
        ])
        .assert()
        .code(0);

    let ssectors = lump_bytes(out.path(), "SSECTORS");
    assert!(
        ssectors.starts_with(b"ZGL3"),
        "SSECTORS should be a ZGL3 stream"
    );
    assert_maps_assemble_strict_clean(out.path());
}

#[cfg(not(feature = "extended-nodes-zlib"))]
#[test]
fn convert_node_format_zgl3_without_feature_errors_clearly() {
    let wad = write_udmf_square_room_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
            "--node-format",
            "zgl3",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "--node-format zgl3 requires cwad built with the extended-nodes-zlib feature",
        ));
}

#[test]
fn convert_to_doom_with_nodes_refuses_mixed_sector_fan_and_hints_lenient() {
    let wad = write_udmf_mixed_sector_fan_wad();

    // Strict: `add_doom_map_with_nodes` fails with `MixedSectorSubsector`; the
    // convert refuses (exit 3), names the map, and — because the error IS
    // lenient-recoverable (#264) — suggests `--lenient`.
    let strict_out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            strict_out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "cannot convert map MAP01 to doom: a convex subsector",
        ))
        .stderr(predicate::str::contains("spans multiple sectors"))
        .stderr(predicate::str::contains("re-run with --lenient"));

    // Lenient: the fan is tolerated (ADR-0024 §7), so the same conversion
    // succeeds and produces a playable map.
    let lenient_out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            lenient_out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
        ])
        .assert()
        .code(0);
    assert!(!lump_bytes(lenient_out.path(), "SEGS").is_empty());
}

/// A PWAD holding a single **Doom-format** square-room map (`MAP01`) whose
/// SEGS/SSECTORS/NODES/REJECT/BLOCKMAP lumps are present but zero-length — the
/// output `add_doom_map` produces (an editor's "run a nodebuilder" map) — plus
/// a trailing `COLORMAP` non-map lump. The geometry is the square room, so a
/// real node build turns the empty node lumps into a populated BSP.
fn write_doom_square_room_empty_nodes_wad() -> NamedTempFile {
    // Assemble the square room from its UDMF form, then serialize to the Doom
    // binary format with empty node lumps via the write path.
    let textmap = udmf_square_room();
    let src = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
        ],
    );
    let src_bytes = std::fs::read(src.path()).expect("source WAD readable");
    let src_wad = crustywad::Wad::from_bytes(src_bytes).expect("source WAD parses");
    let groups = src_wad.map_groups();
    let group = groups.first().expect("source has one map group");
    let map = crustywad::map::Map::assemble(&src_wad, group).expect("square room assembles");

    let mut builder = crustywad::WadBuilder::new(crustywad::WadKind::Pwad);
    crustywad::map::add_doom_map(
        &mut builder,
        "MAP01",
        &map,
        &crustywad::WriteOptions::strict(),
    )
    .expect("writes empty-node Doom map");
    builder.add_lump("COLORMAP", vec![4_u8, 5, 6]);
    let bytes = builder.build().expect("builds Doom WAD");

    let out = NamedTempFile::new().unwrap();
    std::fs::write(out.path(), &bytes).expect("write Doom fixture");
    out
}

/// A PWAD holding a single **Doom-format** mixed-sector-fan map (`MAP01`) with
/// empty node lumps — the fan geometry that assembles cleanly but that a node
/// build refuses in strict mode (a convex subsector spanning multiple sectors,
/// ADR-0024 §7). Mirrors [`write_doom_square_room_empty_nodes_wad`] exactly,
/// only sourcing the fan geometry from [`udmf_mixed_sector_fan`].
fn write_doom_mixed_sector_fan_empty_nodes_wad() -> NamedTempFile {
    let textmap = udmf_mixed_sector_fan();
    let src = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("TEXTMAP", textmap.as_bytes()),
            ("ENDMAP", b""),
        ],
    );
    let src_bytes = std::fs::read(src.path()).expect("source WAD readable");
    let src_wad = crustywad::Wad::from_bytes(src_bytes).expect("source WAD parses");
    let groups = src_wad.map_groups();
    let group = groups.first().expect("source has one map group");
    let map = crustywad::map::Map::assemble(&src_wad, group).expect("fan assembles");

    let mut builder = crustywad::WadBuilder::new(crustywad::WadKind::Pwad);
    crustywad::map::add_doom_map(
        &mut builder,
        "MAP01",
        &map,
        &crustywad::WriteOptions::strict(),
    )
    .expect("writes empty-node Doom map");
    let bytes = builder.build().expect("builds Doom WAD");

    let out = NamedTempFile::new().unwrap();
    std::fs::write(out.path(), &bytes).expect("write Doom fixture");
    out
}

#[test]
fn convert_doom_to_doom_with_nodes_rebuilds_empty_node_lumps() {
    // Baseline: the Doom-format input's node lumps are empty (editor output).
    let wad = write_doom_square_room_empty_nodes_wad();
    assert!(
        lump_bytes(wad.path(), "SEGS").is_empty(),
        "fixture precondition: input SEGS is empty"
    );

    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
            "--nodes",
        ])
        .assert()
        .code(0)
        // The map is rebuilt (not passed through), so it is counted and the
        // NodesNotBuilt warning is absent.
        .stdout(predicate::str::contains("converted 1 map to doom"))
        .stderr(predicate::str::contains("run a nodebuilder").not());

    // Same-format Doom map is now rebuilt: node lumps populated, non-map lump
    // still passed through in order.
    assert_eq!(
        lump_names(out.path()),
        vec![
            "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES",
            "SECTORS", "REJECT", "BLOCKMAP", "COLORMAP",
        ]
    );
    assert!(
        !lump_bytes(out.path(), "SEGS").is_empty(),
        "SEGS should be non-empty after a Doom->Doom node build"
    );
    assert!(!lump_bytes(out.path(), "SSECTORS").is_empty());
    assert_maps_assemble_strict_clean(out.path());
}

#[test]
fn convert_doom_to_doom_without_nodes_passes_through_empty_node_lumps() {
    // Contrast with the --nodes case: without --nodes, a same-format Doom map
    // passes through verbatim, keeping its empty node lumps.
    let wad = write_doom_square_room_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 0 maps to doom"));

    // Passed through unchanged: SEGS stays empty.
    assert!(
        lump_bytes(out.path(), "SEGS").is_empty(),
        "without --nodes the empty SEGS should pass through unchanged"
    );
}

#[test]
fn convert_to_udmf_with_nodes_emits_znodes() {
    // Doom-source WAD -> UDMF target with --nodes: the converted group carries a
    // built GL ZNODES stream, and the result validates deeply.
    let wad = write_doom_square_room_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
        ])
        .assert()
        .code(0)
        // The default (Classic) node-format resolves to the GL auto-format; the
        // run notes it once.
        .stderr(predicate::str::contains(
            "--to udmf --nodes builds GL nodes (gl auto-format) into ZNODES for each converted map",
        ));

    // The converted group is [marker, TEXTMAP, ZNODES, ENDMAP], with the
    // trailing non-map lump passed through in order.
    assert_eq!(
        lump_names(out.path()),
        vec!["MAP01", "TEXTMAP", "ZNODES", "ENDMAP", "COLORMAP"]
    );
    let znodes = lump_bytes(out.path(), "ZNODES");
    assert!(
        znodes.starts_with(b"XGLN"),
        "ZNODES should be an XGLN stream, got {:?}",
        &znodes[..znodes.len().min(4)]
    );

    // The synthesized UDMF map with built nodes passes deep validation.
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "--deep", out.path().to_str().unwrap()])
        .assert()
        .code(0);
}

#[test]
fn convert_to_udmf_with_nodes_passes_udmf_source_through_unchanged() {
    // A source group already in the target format (UDMF -> UDMF) passes through
    // untouched: --nodes does NOT retrofit a ZNODES onto it (that in-place
    // rebuild is tracked by #385). The per-group note makes the pass-through
    // honest despite the run-level "builds GL nodes into ZNODES" note.
    let wad = write_udmf_square_room_wad();
    let input_textmap = lump_bytes(wad.path(), "TEXTMAP");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains(
            "MAP01 is already UDMF; passed through unchanged (no ZNODES built — see #385)",
        ));

    // The group passed through byte-identical: same lumps in order, no ZNODES
    // inserted, and the TEXTMAP bytes are untouched.
    assert_eq!(
        lump_names(out.path()),
        vec!["MAP01", "TEXTMAP", "ENDMAP", "COLORMAP"]
    );
    assert!(
        !lump_names(out.path()).iter().any(|n| n == "ZNODES"),
        "no ZNODES should be built for an already-UDMF pass-through"
    );
    assert_eq!(lump_bytes(out.path(), "TEXTMAP"), input_textmap);
}

#[test]
fn convert_to_udmf_with_nodes_rejects_non_gl_format() {
    // A non-GL extended format has no ZNODES carrier: the UDMF target rejects it
    // up front (exit 3) before writing anything.
    let wad = write_doom_square_room_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
            "--node-format",
            "xnod",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("not valid for a UDMF target"))
        .stderr(predicate::str::contains("#384"));

    // Rejected before conversion, so nothing was written to the output file.
    assert_eq!(
        std::fs::metadata(out.path()).unwrap().len(),
        0,
        "no output should be written on the up-front rejection"
    );
}

#[test]
fn convert_to_udmf_with_nodes_refuses_mixed_sector_fan_and_hints_lenient() {
    // A Doom-source mixed-sector fan converts to UDMF, but the GL node build in
    // the one-shot `add_udmf_map_with_nodes` fails (MixedSectorSubsector): the
    // conversion refuses (exit 3), names the map, and — the error being
    // lenient-recoverable — hints --lenient.
    let wad = write_doom_mixed_sector_fan_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot convert map MAP01 to udmf"))
        .stderr(predicate::str::contains("--lenient"));
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn convert_to_udmf_with_nodes_rejects_znod_format() {
    // --node-format znod (zlib-only, non-GL) has no ZNODES carrier: the UDMF
    // target rejects it up front (exit 3) before writing anything.
    let wad = write_doom_square_room_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
            "--node-format",
            "znod",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("not valid for a UDMF target"));
}

#[cfg(feature = "extended-nodes-zlib")]
#[test]
fn convert_to_udmf_with_nodes_zgl3_compresses() {
    let wad = write_doom_square_room_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
            "--node-format",
            "zgl3",
        ])
        .assert()
        .code(0);

    let znodes = lump_bytes(out.path(), "ZNODES");
    assert!(
        znodes.starts_with(b"ZGL3"),
        "ZNODES should be a ZGL3 stream"
    );
}

#[cfg(not(feature = "extended-nodes-zlib"))]
#[test]
fn convert_to_udmf_with_nodes_zgl3_without_feature_errors_clearly() {
    let wad = write_doom_square_room_empty_nodes_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--nodes",
            "--node-format",
            "zgl3",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "--node-format zgl3 requires cwad built with the extended-nodes-zlib feature",
        ));
}

#[test]
fn convert_map_already_in_target_format_passes_through() {
    let wad = write_doom_map_wad("MAP01");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 0 maps to doom"));

    assert_eq!(
        lump_names(out.path()),
        vec![
            "PLAYPAL", "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SECTORS",
            "COLORMAP",
        ]
    );
}

#[test]
fn convert_map_filter_converts_only_the_named_map() {
    let m = doom_map_lumps();
    let wad = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &m.linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
            ("MAP02", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &m.linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
        ],
    );
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--map",
            "MAP02",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 1 map to udmf"));

    assert_eq!(
        lump_names(out.path()),
        vec![
            "MAP01", "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SECTORS", "MAP02", "TEXTMAP",
            "ENDMAP",
        ]
    );
}

#[test]
fn convert_strict_refuses_lossy_udmf_to_doom_with_exit_3() {
    // A UDMF thing with a nonzero `height` has no slot in the Doom format
    // (ADR-0019 tier 3): strict mode must refuse, name the field, and point at
    // `--lenient`.
    let wad = write_udmf_map_wad(true);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot convert map MAP01 to doom"))
        .stderr(predicate::str::contains("height"))
        .stderr(predicate::str::contains("--lenient"));
}

#[test]
fn convert_lenient_accepts_lossy_udmf_to_doom_and_warns() {
    let wad = write_udmf_map_wad(true);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("was dropped"))
        .stderr(predicate::str::contains("height"));

    assert!(lump_names(out.path()).contains(&"THINGS".to_owned()));
}

/// A PWAD containing a single Hexen-format map (`MAP01`) whose group carries a
/// `BEHAVIOR` lump (compiled ACS) alongside the five classic data lumps. The
/// `THINGS` and `LINEDEFS` lumps use the Hexen record layouts; the other three
/// are byte-identical across formats and are reused from [`doom_map_lumps`].
fn write_hexen_map_wad() -> NamedTempFile {
    let m = doom_map_lumps();

    // Hexen linedef: v1, v2, flags (u16), special + 5 args (u8), right, left (u16).
    let mut linedefs = Vec::new();
    for v in [0_u16, 1, 1] {
        linedefs.extend_from_slice(&v.to_le_bytes());
    }
    linedefs.extend_from_slice(&[0_u8; 6]);
    linedefs.extend_from_slice(&0_u16.to_le_bytes());
    linedefs.extend_from_slice(&0xffff_u16.to_le_bytes());

    // Hexen thing: tid, x, y, z, angle, type, flags (u16), special + 5 args (u8).
    let mut things = Vec::new();
    for v in [0_u16, 32, 32, 0, 0, 1, 7] {
        things.extend_from_slice(&v.to_le_bytes());
    }
    things.extend_from_slice(&[0_u8; 6]);

    write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("THINGS", &things),
            ("LINEDEFS", &linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
            ("BEHAVIOR", &[7, 7, 7, 7]),
        ],
    )
}

#[test]
fn convert_strict_refuses_to_drop_extra_group_lumps_with_exit_3() {
    // A `BEHAVIOR` lump is compiled ACS bound to the source map's specials: no
    // conversion can carry it across. Dropping it is data loss, so strict mode
    // must refuse, name the lump, and point at `--lenient`.
    let wad = write_hexen_map_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot convert map MAP01 to udmf"))
        .stderr(predicate::str::contains("BEHAVIOR"))
        .stderr(predicate::str::contains("--lenient"));
}

#[test]
fn convert_lenient_drops_extra_group_lumps_and_warns() {
    let wad = write_hexen_map_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("BEHAVIOR"));

    // The dropped lump is really gone — it is neither converted nor passed
    // through into the converted group.
    let names = lump_names(out.path());
    assert!(!names.contains(&"BEHAVIOR".to_owned()), "{names:?}");
    assert_eq!(names, vec!["MAP01", "TEXTMAP", "ENDMAP"]);
}

#[test]
fn convert_strict_accepts_a_group_with_no_extra_lumps() {
    // Regression guard for the strict refusal above: a plain Doom map group has
    // no extra lumps, so strict mode must convert it without complaint.
    let wad = write_doom_map_wad("MAP01");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 1 map to udmf"))
        .stderr(predicate::str::contains("cannot convert").not());
}

/// A Doom 64 nested-WAD map (a single marker lump whose bytes are themselves
/// a WAD carrying the 13 sub-lumps `read_doom64_map` expects), wrapped in an
/// outer `T_START..T_END` texture section whose two names hash to the values
/// carried by every sidedef/sector texture field (`SDOORA` = 2712, `SFLATAE`
/// = 4098 — the empirically validated vectors, ADR-0022 §1), so assembly
/// resolves every ref to a name and the only convert obstacle left is
/// colored lighting.
fn write_doom64_textured_map_wad() -> NamedTempFile {
    // Two vertices in 16.16 fixed point: (0, 0) and (64, 0).
    let mut vertexes = Vec::new();
    for v in [0_i32, 0, 64 << 16, 0] {
        vertexes.extend_from_slice(&v.to_le_bytes());
    }

    // One linedef: v1, v2 (u16), flags (u32), special, tag, right, left (u16).
    let mut linedefs = Vec::new();
    for v in [0_u16, 1] {
        linedefs.extend_from_slice(&v.to_le_bytes());
    }
    linedefs.extend_from_slice(&0_u32.to_le_bytes());
    for v in [0_u16, 7, 0, 0xffff] {
        linedefs.extend_from_slice(&v.to_le_bytes());
    }

    // One sidedef: x/y offsets (i16), upper/lower/middle texture hash
    // (all SDOORA), sector (u16).
    let mut sidedefs = Vec::new();
    for v in [0_i16; 2] {
        sidedefs.extend_from_slice(&v.to_le_bytes());
    }
    for v in [2712_u16, 2712, 2712, 0] {
        sidedefs.extend_from_slice(&v.to_le_bytes());
    }

    // One sector: floor/ceiling height (i16), floor/ceiling flat hash
    // (both SFLATAE), five color refs, special, tag, flags (u16).
    let mut sectors = Vec::new();
    sectors.extend_from_slice(&0_i16.to_le_bytes());
    sectors.extend_from_slice(&128_i16.to_le_bytes());
    for v in [4098_u16, 4098, 0, 0, 0, 0, 0, 0, 0, 0] {
        sectors.extend_from_slice(&v.to_le_bytes());
    }

    // One LIGHTS record: r, g, b, tag, two pad bytes.
    let lights = [0_u8; 6];

    let nested = build_wad(
        *b"IWAD",
        &[
            ("THINGS", &[]),
            ("LINEDEFS", &linedefs),
            ("SIDEDEFS", &sidedefs),
            ("VERTEXES", &vertexes),
            ("SECTORS", &sectors),
            ("LIGHTS", &lights),
            ("SEGS", &[]),
            ("SSECTORS", &[]),
            ("NODES", &[]),
            ("REJECT", &[]),
            ("BLOCKMAP", &[]),
            ("LEAFS", &[]),
            ("MACROS", &[]),
        ],
    );
    write_wad(
        *b"IWAD",
        &[
            ("T_START", &[]),
            ("SDOORA", &[]),
            ("SFLATAE", &[]),
            ("T_END", &[]),
            ("MAP01", &nested),
        ],
    )
}

#[test]
fn convert_doom64_strict_refuses_lighting_with_lenient_hint() {
    // ADR-0021 §5 amendment 3: the texture gate is gone; strict now refuses
    // for colored lighting (tier 3) with an HONEST --lenient hint (this
    // exact fixture converts under --lenient below), and the #264
    // texture-support note is retired.
    let wad = write_doom64_textured_map_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot convert map MAP01 to udmf"))
        .stderr(predicate::str::contains("colors"))
        .stderr(predicate::str::contains("--lenient"))
        .stderr(predicate::str::contains("texture support").not());
}

#[test]
fn convert_doom64_lenient_drops_lighting_and_writes_resolved_names() {
    let wad = write_doom64_textured_map_wad();
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 1 map to udmf"))
        .stderr(predicate::str::contains("colored lighting"));

    // The output WAD's TEXTMAP carries resolved NAMES. Assert the QUOTED
    // UDMF form — the bare name would also appear as a directory entry if
    // convert copies the input's texture lumps through, so quotes are what
    // prove TEXTMAP content.
    let bytes = std::fs::read(out.path()).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("\"SDOORA\""));
    assert!(text.contains("\"SFLATAE\""));
}

#[test]
fn convert_strict_refuses_frontless_linedef_to_udmf_with_lenient_hint() {
    // The counter-case to the two tests above: a front-sidedef 0xFFFF linedef
    // (ADR-0020) assembles strict-clean but has no valid UDMF `sidefront`, a
    // refusal lenient mode CAN recover (it writes `sidefront = -1`) — so the
    // `--lenient` hint must still appear for it after #264.
    let m = doom_map_lumps();
    let mut linedefs = Vec::new();
    for v in [0_u16, 1, 1, 0, 0, 0xffff, 0] {
        linedefs.extend_from_slice(&v.to_le_bytes());
    }
    let wad = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
        ],
    );
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot convert map MAP01 to udmf"))
        .stderr(predicate::str::contains("no front sidedef"))
        .stderr(predicate::str::contains(
            "note: re-run with --lenient to accept the data loss",
        ))
        .stderr(predicate::str::contains("texture support").not());
}

#[test]
fn convert_unknown_map_name_exits_3() {
    // A typo'd `--map` name must not look like success (it previously wrote a
    // verbatim copy and reported "converted 0 maps").
    let wad = write_doom_map_wad("MAP01");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--map",
            "TYPO",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(r#"map "TYPO" not found"#))
        .stderr(predicate::str::contains("available maps: MAP01"));
}

#[test]
fn convert_json_and_iwad_kind() {
    let wad = write_doom_map_wad("MAP01");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "-F",
            "json",
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--kind",
            "iwad",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            r#"{"ok":true,"converted":1,"format":"udmf"}"#,
        ));

    let bytes = std::fs::read(out.path()).unwrap();
    let converted = crustywad::Wad::from_bytes(bytes).unwrap();
    assert_eq!(converted.kind(), crustywad::WadKind::Iwad);
}

#[test]
fn convert_missing_input_exits_2() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("no_such_input.wad");
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            missing.to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "doom",
        ])
        .assert()
        .code(2);
}

#[test]
fn convert_lenient_reports_parse_warnings_from_the_input() {
    // A WAD with non-standard magic parses only in lenient mode, and the
    // resulting ParseWarning must reach the user (path-prefixed on stderr)
    // rather than being swallowed by the conversion.
    let m = doom_map_lumps();
    let wad = write_wad(
        *b"XWAD",
        &[
            ("MAP01", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &m.linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
        ],
    );
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--lenient",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("warning:"));
}

#[test]
fn convert_lenient_reports_assembly_warnings() {
    // A linedef pointing at a vertex that does not exist is a dangling
    // cross-reference: strict assembly rejects it, lenient assembly clamps it to
    // an in-range vertex and records a MapWarning. That repair changes the
    // geometry being written out, so `convert --lenient` must surface it rather
    // than silently emitting a repaired map that looks clean.
    let m = doom_map_lumps();
    let mut linedefs = m.linedefs.clone();
    // Point the linedef's start vertex (first u16) at index 999 — well past the
    // two vertices the map actually has.
    linedefs[0..2].copy_from_slice(&999_u16.to_le_bytes());

    let wad = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
        ],
    );
    let out = NamedTempFile::new().unwrap();

    // Strict refuses the dangling reference outright.
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(3);

    // Lenient converts, but must report the repair it made.
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--lenient",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("MAP01"))
        .stderr(predicate::str::contains("vertex"));
}

#[test]
fn convert_non_ascii_lump_name_fails_write_validation_exits_3() {
    // A non-ASCII lump name decodes under a lenient *read* but `WriteError::
    // NonAsciiName` is rejected in both write modes, so building the converted
    // WAD fails. Convert must surface that as a usage error (exit 3), not fall
    // through to the generic I/O exit 2 — the same contract `merge` honors.
    let m = doom_map_lumps();
    let wad = write_wad(
        *b"PWAD",
        &[
            ("É", &[1]),
            ("MAP01", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &m.linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &m.vertexes),
            ("SECTORS", &m.sectors),
        ],
    );
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("failed to build"));
}

#[test]
fn convert_map_filter_on_a_wad_with_no_maps_exits_3() {
    // `--map NAME` against a WAD that has no maps at all takes the "contains no
    // maps" branch: still exit 3, but the note must say so rather than printing
    // an empty list of available maps.
    let wad = write_wad(*b"PWAD", &[("PLAYPAL", &[1, 2, 3])]);
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--map",
            "MAP01",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("contains no maps"));
}

#[test]
fn convert_corrupt_map_exits_3() {
    // A map group whose VERTEXES lump has a trailing partial record cannot be
    // assembled: the conversion must fail with exit 3, not panic.
    let m = doom_map_lumps();
    let wad = write_wad(
        *b"PWAD",
        &[
            ("MAP01", b""),
            ("THINGS", &m.things),
            ("LINEDEFS", &m.linedefs),
            ("SIDEDEFS", &m.sidedefs),
            ("VERTEXES", &[0, 0, 0]), // 3 bytes: not a whole 4-byte vertex
            ("SECTORS", &m.sectors),
        ],
    );
    let out = NamedTempFile::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "convert",
            wad.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("failed to assemble map MAP01"))
        .stderr(predicate::str::contains("thread '").not());
}

// ---------------------------------------------------------------------------
// Retail sweep smoke (#281)
// ---------------------------------------------------------------------------

#[test]
fn convert_retail_doom64_map01_lenient_smoke() {
    // Env-gated retail smoke (the CLI crate has no fixture feature flag;
    // env-only gating with graceful skip mirrors the library sweep
    // suite). Exit 0 alone proves full texture resolution: any leftover
    // unresolved index would be a both-modes writer error.
    let Some(dir) = std::env::var_os("CRUSTYWAD_SWEEP_DIR") else {
        eprintln!("skipping: CRUSTYWAD_SWEEP_DIR not set");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if !dir.is_absolute() || !dir.is_dir() {
        eprintln!(
            "skipping: CRUSTYWAD_SWEEP_DIR is not an absolute path to a directory: {}",
            dir.display()
        );
        return;
    }
    let wad_path = dir.join("DOOM64.WAD");
    if !wad_path.is_file() {
        eprintln!("skipping: DOOM64.WAD not present in {}", dir.display());
        return;
    }

    let out = NamedTempFile::new().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "--lenient",
            "convert",
            wad_path.to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--to",
            "udmf",
            "--map",
            "MAP01",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("converted 1 map to udmf"))
        .stderr(predicate::str::contains("colored lighting"));

    // TEXTMAP carries quoted resolved names (quotes distinguish UDMF
    // content from pass-through directory entries).
    let bytes = std::fs::read(out.path()).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("texturefloor = \""));
}

// ---------------------------------------------------------------------------
// `cwad extract` audio-aware export (#304)
// ---------------------------------------------------------------------------

/// Fixture D1 (mirrors `crates/crustywad/tests/audio.rs`): a valid DMX lump — format
/// 3, rate 11025, length 52, the 20 PCM samples `0..=19` between two 16-byte
/// pads. 60 bytes total.
fn dmx_d1() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&3u16.to_le_bytes()); // format 3 (DMX digital sound)
    v.extend_from_slice(&11025u16.to_le_bytes()); // sample rate
    v.extend_from_slice(&52u32.to_le_bytes()); // length (16 + 20 + 16)
    v.extend_from_slice(&[0xAA; 16]); // leading pad
    v.extend(0u8..=19); // 20 samples
    v.extend_from_slice(&[0xBB; 16]); // trailing pad
    v
}

/// Fixture M1 (mirrors `crates/crustywad/tests/audio.rs`): a valid 23-byte MUS lump —
/// score length 7, score start 16, one instrument `[1]`, three events
/// (press key ch0 note 60 vel 100; release key ch0 note 60, delta 70;
/// score-end).
fn mus_m1() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&[0x4D, 0x55, 0x53, 0x1A]); // "MUS\x1a"
    v.extend_from_slice(&7u16.to_le_bytes()); // score length
    v.extend_from_slice(&16u16.to_le_bytes()); // score start
    v.extend_from_slice(&1u16.to_le_bytes()); // primary channels
    v.extend_from_slice(&0u16.to_le_bytes()); // secondary channels
    v.extend_from_slice(&1u16.to_le_bytes()); // instrument count
    v.extend_from_slice(&1u16.to_le_bytes()); // instrument [1]
    // events: 0x10 press-key ch0, key 0xBC (note 60 + velocity flag), vel 0x64;
    // 0x80 release-key ch0 with delta, note 0x3C, delta 0x46 (70); 0x60 end.
    v.extend_from_slice(&[0x10, 0xBC, 0x64, 0x80, 0x3C, 0x46, 0x60]);
    v
}

/// A minimal standard-MIDI lump: the `MThd` magic is all `AudioKind::detect`
/// needs to classify it as MIDI. 14-byte header + a tiny empty `MTrk`.
fn midi_mthd() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"MThd");
    v.extend_from_slice(&6u32.to_be_bytes()); // header size
    v.extend_from_slice(&0u16.to_be_bytes()); // format 0
    v.extend_from_slice(&1u16.to_be_bytes()); // one track
    v.extend_from_slice(&0x0060u16.to_be_bytes()); // division
    v.extend_from_slice(b"MTrk");
    v.extend_from_slice(&4u32.to_be_bytes()); // track length
    v.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]); // end-of-track
    v
}

/// A minimal RIFF/WAVE lump: `RIFF` + size + `WAVE` is all `AudioKind::detect`
/// needs to classify it as WAV.
fn wav_riff() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&4u32.to_le_bytes()); // riff size (just covers "WAVE")
    v.extend_from_slice(b"WAVE");
    v
}

/// The canonical 44-byte WAV header the DMX wrapper writes for fixture D1,
/// followed by the 20 samples. Every field hand-derived: mono, 8-bit, PCM,
/// rate 11025, `data_len = 20`, `byte_rate = 11025`, `block_align = 1`,
/// `riff_size = 36 + 20 = 56`.
fn expected_d1_wav() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&56u32.to_le_bytes()); // riff_size = 36 + 20
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&11025u32.to_le_bytes()); // sample rate
    v.extend_from_slice(&11025u32.to_le_bytes()); // byte rate = rate * 1 * 1
    v.extend_from_slice(&1u16.to_le_bytes()); // block align
    v.extend_from_slice(&8u16.to_le_bytes()); // bits per sample
    v.extend_from_slice(b"data");
    v.extend_from_slice(&20u32.to_le_bytes()); // data_len
    v.extend(0u8..=19); // 20 samples
    v
}

#[test]
fn extract_audio_writes_containers() {
    // DMX -> .wav (wrapped); MUS -> raw .mus; MThd -> .mid passthrough;
    // RIFF/WAVE -> .wav passthrough. Content-detected, not name-driven.
    let dmx = dmx_d1();
    let mus = mus_m1();
    let mthd = midi_mthd();
    let riff = wav_riff();
    let wad = write_wad(
        *b"IWAD",
        &[
            ("DMXSND", &dmx),
            ("MUSLMP", &mus),
            ("MIDILMP", &mthd),
            ("WAVLMP", &riff),
        ],
    );
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // DMX wrapped into a canonical 44-byte-header WAV + 20 samples (64 bytes).
    let wav = std::fs::read(out_dir.path().join("DMXSND.wav")).unwrap();
    assert_eq!(wav, expected_d1_wav());
    assert_eq!(wav.len(), 64);

    // MUS extracted raw as .mus (no .mid without --midi).
    assert_eq!(
        std::fs::read(out_dir.path().join("MUSLMP.mus")).unwrap(),
        mus
    );
    assert!(!out_dir.path().join("MUSLMP.mid").exists());

    // MThd / RIFF pass through unchanged under their container extensions.
    assert_eq!(
        std::fs::read(out_dir.path().join("MIDILMP.mid")).unwrap(),
        mthd
    );
    assert_eq!(
        std::fs::read(out_dir.path().join("WAVLMP.wav")).unwrap(),
        riff
    );

    // No raw .bin fallbacks were written for the audio lumps.
    assert!(!out_dir.path().join("DMXSND.bin").exists());
    assert!(!out_dir.path().join("MUSLMP.bin").exists());
}

#[test]
fn extract_midi_flag_converts_mus() {
    // With --midi the MUS lump also yields a converted format-0 SMF. The
    // expected bytes are hand-derived from the M1 event stream against the
    // mus2mid semantics; see the derivation below.
    let mus = mus_m1();
    let wad = write_wad(*b"IWAD", &[("MUSLMP", &mus)]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
            "--midi",
        ])
        .assert()
        .success();

    // The raw .mus is still written.
    assert_eq!(
        std::fs::read(out_dir.path().join("MUSLMP.mus")).unwrap(),
        mus
    );

    // Hand-derived expected MIDI for M1.
    //
    // Header (mus2mid.c:68-77), 22 bytes, with the track length (16)
    // backfilled at offset 18 (mus2mid.c:676):
    let mut expected = Vec::new();
    expected.extend_from_slice(b"MThd");
    expected.extend_from_slice(&6u32.to_be_bytes()); // header size
    expected.extend_from_slice(&0u16.to_be_bytes()); // MIDI type 0
    expected.extend_from_slice(&1u16.to_be_bytes()); // one track
    expected.extend_from_slice(&0x0046u16.to_be_bytes()); // resolution 70
    expected.extend_from_slice(b"MTrk");
    expected.extend_from_slice(&16u32.to_be_bytes()); // track length
    // Track events:
    //  - MUS channel 0's first use allocates MIDI channel 0 and emits the
    //    all-notes-off controller 0x7b, value 0 (mus2mid.c:405-409), prefixed
    //    by delta 0.
    expected.extend_from_slice(&[0x00, 0xB0, 0x7B, 0x00]);
    //  - press key: note-on 0x90|0, note 60 (0x3C), velocity 100 (0x64),
    //    delta 0 (mus2mid.c:158-191).
    expected.extend_from_slice(&[0x00, 0x90, 0x3C, 0x64]);
    //  - release key: note-off 0x80|0, note 60 (0x3C), velocity 0,
    //    delta 0 (mus2mid.c:194-226).
    expected.extend_from_slice(&[0x00, 0x80, 0x3C, 0x00]);
    //  - end of track: the queued delta 70 (0x46) then FF 2F 00
    //    (mus2mid.c:140-156).
    expected.extend_from_slice(&[0x46, 0xFF, 0x2F, 0x00]);
    assert_eq!(expected.len(), 38);

    let mid = std::fs::read(out_dir.path().join("MUSLMP.mid")).unwrap();
    assert_eq!(mid, expected);
}

#[test]
fn extract_malformed_mus_falls_back_to_raw() {
    // A lump that detects as MUS (the magic matches) but cannot be parsed even
    // leniently is extracted raw as .bin with a stderr warning. This MUS lump
    // is truncated to just the magic plus one byte, failing the 14-byte header
    // check in both strictness modes.
    let bad_mus = vec![0x4D, 0x55, 0x53, 0x1A, 0x00]; // "MUS\x1a" + one byte
    let wad = write_wad(*b"IWAD", &[("BADMUS", &bad_mus)]);
    let out_dir = TempDir::new().unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"))
        .stderr(predicate::str::contains("BADMUS"));

    // Fell back to a raw .bin write of the original bytes.
    assert_eq!(
        std::fs::read(out_dir.path().join("BADMUS.bin")).unwrap(),
        bad_mus
    );
    assert!(!out_dir.path().join("BADMUS.mus").exists());
}

// ---------------------------------------------------------------------------
// `cwad list` / `cwad info` audio annotations (#304)
// ---------------------------------------------------------------------------

#[test]
fn list_annotates_detected_audio_lumps_human() {
    let dmx = dmx_d1();
    let mus = mus_m1();
    let wad = write_wad(
        *b"IWAD",
        &[("DMXSND", &dmx), ("MUSLMP", &mus), ("PLAYPAL", &[1, 2, 3])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        // DMX: rate + sample count (20 samples between the pads).
        .stdout(predicate::str::contains(
            "[audio: Dmx rate=11025 samples=20]",
        ))
        // MUS: event count (press, release, score-end = 3 typed events).
        .stdout(predicate::str::contains("[audio: Mus events=3]"));
}

#[test]
fn list_annotates_detected_audio_lumps_json() {
    let dmx = dmx_d1();
    let wad = write_wad(*b"IWAD", &[("DMXSND", &dmx)]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""audio":{"kind":"Dmx","sample_rate":11025,"samples":20}"#,
        ));
}

#[test]
fn list_annotates_detected_audio_lumps_csv() {
    let dmx = dmx_d1();
    let wad = write_wad(*b"IWAD", &[("DMXSND", &dmx)]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("index,filepos,size,name,audio"))
        .stdout(predicate::str::contains("Dmx rate=11025 samples=20"));
}

#[test]
fn list_leaves_non_audio_lumps_unannotated() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("[audio:").not());
}

#[test]
fn info_summarizes_detected_audio_human() {
    let dmx = dmx_d1();
    let mus = mus_m1();
    let wad = write_wad(
        *b"IWAD",
        &[("DMXSND", &dmx), ("MUSLMP", &mus), ("PLAYPAL", &[1, 2, 3])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("audio:"))
        .stdout(predicate::str::contains("Dmx: 1"))
        .stdout(predicate::str::contains("Mus: 1"));
}

#[test]
fn info_summarizes_detected_audio_json() {
    let dmx = dmx_d1();
    let wad = write_wad(*b"IWAD", &[("DMXSND", &dmx)]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""audio":{"Dmx":1}"#));
}

#[test]
fn info_no_audio_lumps_empty_object_json() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""audio":{}"#));
}

#[test]
fn list_annotates_midi_wav_pcspeaker_and_bare_mus() {
    // Covers the remaining annotation arms: Midi and Wav with details,
    // PcSpeaker (kind-only), and a MUS whose 4 magic bytes detect but whose
    // header is too short to parse even leniently (kind-only fallback).
    let midi = midi_mthd();
    // A full canonical 48-byte WAV (PCM, mono, 22050 Hz, 16-bit, 4 data
    // bytes) so the annotation carries real fmt details.
    let mut wav: Vec<u8> = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&40u32.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&22050u32.to_le_bytes());
    wav.extend_from_slice(&44100u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&4u32.to_le_bytes());
    wav.extend_from_slice(&[0x00, 0x01, 0xFF, 0x7F]);
    let pcs: Vec<u8> = vec![0x00, 0x00, 0x02, 0x00, 10, 42];
    let bare_mus: Vec<u8> = vec![0x4D, 0x55, 0x53, 0x1A];
    // Detects as MIDI (4-byte magic) but fails even the lenient parse
    // (`0 < len < 14`), exercising the kind-only Midi fallback.
    let bare_midi: Vec<u8> = vec![b'M', b'T', b'h', b'd', 0, 0, 0, 6];
    let wad = write_wad(
        *b"IWAD",
        &[
            ("MIDILMP", &midi),
            ("WAVLMP", &wav),
            ("DPLMP", &pcs),
            ("SHORTMUS", &bare_mus),
            ("SHORTMID", &bare_midi),
        ],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("[audio: Midi tracks=1]"))
        .stdout(predicate::str::contains(
            "[audio: Wav rate=22050 channels=1 bits=16]",
        ))
        .stdout(predicate::str::contains("[audio: PcSpeaker]"))
        .stdout(predicate::str::contains("[audio: Mus]"))
        .stdout(predicate::str::contains("SHORTMID [audio: Midi]"));
}

#[test]
fn list_json_and_csv_cover_detail_and_bare_arms() {
    let midi = midi_mthd();
    let pcs: Vec<u8> = vec![0x00, 0x00, 0x00, 0x00];
    let wad = write_wad(*b"IWAD", &[("MIDILMP", &midi), ("DPLMP", &pcs)]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "json", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""audio":{"kind":"Midi","tracks":1}"#,
        ))
        .stdout(predicate::str::contains(r#""audio":{"kind":"PcSpeaker"}"#));
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "list", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Midi tracks=1"))
        .stdout(predicate::str::contains("PcSpeaker"));
}

#[test]
fn info_summarizes_all_audio_kinds() {
    let dmx = dmx_d1();
    let mus = mus_m1();
    let midi = midi_mthd();
    let wav = wav_riff();
    let pcs: Vec<u8> = vec![0x00, 0x00, 0x01, 0x00, 7];
    let wad = write_wad(
        *b"IWAD",
        &[
            ("DMXSND", &dmx),
            ("MUSLMP", &mus),
            ("MIDILMP", &midi),
            ("WAVLMP", &wav),
            ("DPLMP", &pcs),
        ],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dmx: 1"))
        .stdout(predicate::str::contains("PcSpeaker: 1"))
        .stdout(predicate::str::contains("Mus: 1"))
        .stdout(predicate::str::contains("Midi: 1"))
        .stdout(predicate::str::contains("Wav: 1"));
}

#[test]
fn extract_malformed_midi_falls_back_to_raw() {
    // Detects as MIDI (4-byte magic) but fails even the lenient parse
    // (`0 < len < 14`): the extract must warn and fall back to raw `.bin`,
    // matching the MUS fallback contract.
    let bare_midi: Vec<u8> = vec![b'M', b'T', b'h', b'd', 0, 0, 0, 6];
    let wad = write_wad(*b"IWAD", &[("SHORTMID", &bare_midi)]);
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            wad.path().to_str().unwrap(),
            "--output",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("could not index MIDI chunks"));
    assert!(dir.path().join("SHORTMID.bin").exists());
    assert!(!dir.path().join("SHORTMID.mid").exists());
}
