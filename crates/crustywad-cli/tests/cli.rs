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
    let wad = write_wad(
        *b"IWAD",
        &[("E1M1", &[]), ("THINGS", &[0; 4]), ("E1M2", &[])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind,lumps,data_size,maps"))
        .stdout(predicate::str::contains("Iwad,3,4,E1M1 E1M2"));
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
    // E1M1 is a zero-size marker lump followed by THINGS etc. in real WADs, but
    // the map-detection logic only checks the lump name, not the size.
    let wad = write_wad(
        *b"IWAD",
        &[("E1M1", &[]), ("THINGS", &[0; 10]), ("E1M2", &[])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("maps:"))
        .stdout(predicate::str::contains("E1M1, E1M2"));
}

#[test]
fn info_maps_doom2_style() {
    let wad = write_wad(
        *b"IWAD",
        &[("MAP01", &[]), ("THINGS", &[0; 10]), ("MAP02", &[])],
    );
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("maps:"))
        .stdout(predicate::str::contains("MAP01, MAP02"));
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

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &bytes).unwrap();

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

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &bytes).unwrap();

    Command::cargo_bin("cwad")
        .unwrap()
        .args(["--lenient", "list", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"));
}

// ---------------------------------------------------------------------------
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
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"NOTAWAD").unwrap();
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
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"NOTAWAD").unwrap();
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
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["validate", "/nonexistent/path/to/missing.wad"])
        .assert()
        .code(2);
}

#[test]
fn validate_corrupt_wad_exits_2() {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"NOTAWADX").unwrap();
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

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &bytes).unwrap();

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
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "diff",
            wad.path().to_str().unwrap(),
            "/nonexistent/path/missing.wad",
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

    let file1 = NamedTempFile::new().unwrap();
    let file2 = NamedTempFile::new().unwrap();
    std::fs::write(file1.path(), &bytes1).unwrap();
    std::fs::write(file2.path(), &bytes2).unwrap();

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
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["info", "/nonexistent/path/file.wad"])
        .assert()
        .code(2);
}

#[test]
fn invalid_magic_strict_mode_exits_2() {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"BOGUS_DATA_NOT_A_WAD").unwrap();
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

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "merge",
            "/nonexistent/path/missing.wad",
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

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &bytes).unwrap();
    let out = NamedTempFile::new().unwrap();

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
        .stderr(predicate::str::contains("warning"));
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

    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            "/nonexistent/path/to/missing.wad",
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
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            "/nonexistent.wad",
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
    Command::cargo_bin("cwad")
        .unwrap()
        .args([
            "extract",
            "/nonexistent.wad",
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

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &bytes).unwrap();
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
