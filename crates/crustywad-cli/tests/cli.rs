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
        .stdout(predicate::str::starts_with("index,filepos,size,name\n"))
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

/// A PWAD holding one Doom 64 nested-WAD map (`MAP01`): a single marker lump
/// whose bytes are themselves a WAD carrying the 13 sub-lumps `read_doom64_map`
/// expects. Record bytes mirror the library test suite's `common::d64_*`
/// helpers; the map assembles strict-clean as `MapFormat::Doom64`.
fn write_doom64_map_wad() -> NamedTempFile {
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

    // One sidedef: x/y offsets (i16), upper/lower/middle texture index,
    // sector (u16).
    let mut sidedefs = Vec::new();
    for v in [0_u16; 6] {
        sidedefs.extend_from_slice(&v.to_le_bytes());
    }

    // One sector: floor/ceiling height (i16), floor/ceiling flat index, five
    // color refs, special, tag, flags (u16).
    let mut sectors = Vec::new();
    sectors.extend_from_slice(&0_i16.to_le_bytes());
    sectors.extend_from_slice(&128_i16.to_le_bytes());
    for v in [0_u16; 10] {
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
    write_wad(*b"PWAD", &[("MAP01", &nested)])
}

#[test]
fn convert_doom64_source_notes_texture_gap_without_lenient_hint() {
    // A Doom 64-sourced map is refused by the UDMF writer in BOTH strictness
    // modes (`UnsupportedSourceFormat`, ADR-0021 §5), so the strict-mode
    // "re-run with --lenient" hint would be a lie (#264): the error must carry
    // the texture-layer note instead.
    let wad = write_doom64_map_wad();
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
        .stderr(predicate::str::contains("Doom64"))
        .stderr(predicate::str::contains(
            "note: this map's source format cannot be converted until crustywad has texture support",
        ))
        .stderr(predicate::str::contains("--lenient").not());
}

#[test]
fn convert_doom64_source_notes_texture_gap_in_lenient_mode_too() {
    // The texture-layer note is about a capability gap, not strictness, so it
    // prints in lenient mode as well.
    let wad = write_doom64_map_wad();
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
        .stderr(predicate::str::contains("cannot convert map MAP01 to udmf"))
        .stderr(predicate::str::contains(
            "note: this map's source format cannot be converted until crustywad has texture support",
        ));
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
