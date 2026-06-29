//! Build script for `crustywad-cli`.
//!
//! Generates shell completion files for `cwad` (bash, zsh, fish) at build
//! time using `clap_complete`.  The generated files are placed in `$OUT_DIR`
//! and are not part of the source tree.

#[allow(dead_code)]
#[path = "src/cli.rs"]
mod cli;

use std::env;
use std::io::Error;
use std::path::PathBuf;

use clap::CommandFactory as _;
use clap_complete::{Shell, generate_to};

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=src/cli.rs");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("completions");
    std::fs::create_dir_all(&out)?;
    let mut cmd = cli::Cli::command();
    generate_to(Shell::Bash, &mut cmd, "cwad", &out)?;
    generate_to(Shell::Zsh, &mut cmd, "cwad", &out)?;
    generate_to(Shell::Fish, &mut cmd, "cwad", &out)?;
    Ok(())
}
