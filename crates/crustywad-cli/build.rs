//! Build script for `crustywad-cli`.
//!
//! Generates shell completion files for `cwad` (bash, zsh, fish) at build
//! time using `clap_complete`, and a man page (`cwad.1`) using `clap_mangen`.
//! The generated files are placed in `$OUT_DIR` and are not part of the
//! source tree.

#[allow(dead_code)]
#[path = "src/cli.rs"]
mod cli;

use std::env;
use std::io::Error;
use std::path::PathBuf;

use clap::CommandFactory as _;
use clap_complete::{Shell, generate_to};
use clap_mangen::Man;

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=src/cli.rs");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Shell completions
    let completions_dir = out.join("completions");
    std::fs::create_dir_all(&completions_dir)?;
    let mut cmd = cli::Cli::command();
    generate_to(Shell::Bash, &mut cmd, "cwad", &completions_dir)?;
    generate_to(Shell::Zsh, &mut cmd, "cwad", &completions_dir)?;
    generate_to(Shell::Fish, &mut cmd, "cwad", &completions_dir)?;

    // Man page
    let man_dir = out.join("man");
    std::fs::create_dir_all(&man_dir)?;
    let cmd = cli::Cli::command();
    let man = Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    std::fs::write(man_dir.join("cwad.1"), buf)?;

    Ok(())
}
