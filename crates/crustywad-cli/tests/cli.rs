//! Integration tests for the `cwad` CLI binary.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

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
        .stdout(predicate::str::contains("\"lumps\""));
}

#[test]
fn info_csv_format() {
    let wad = write_wad(*b"IWAD", &[("PLAYPAL", &[1, 2, 3])]);
    Command::cargo_bin("cwad")
        .unwrap()
        .args(["-F", "csv", "info", wad.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("kind,lumps"))
        .stdout(predicate::str::contains("Iwad,1"));
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
        .stdout(predicate::str::contains("lumps: 1"));
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
        .stdout(predicate::str::contains("lumps: 2"));
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
