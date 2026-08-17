//! idgames corpus harvest tool (epic #401).
//!
//! Operational spec: `xtask/DESIGN.md`; decision record: ADR-0030. This
//! crate is its own cargo workspace, deliberately excluded from the root
//! workspace — see the note in `xtask/Cargo.toml`.

mod api;
mod cache;
mod lslar;
mod mirror;
mod outliers;
mod phase1;
mod schema;
mod scope;
mod stats;
mod zips;

use clap::{Parser, Subcommand};

/// idgames corpus harvest tool (`xtask/DESIGN.md`).
#[derive(Debug, Parser)]
#[command(name = "xtask")]
struct Cli {
    /// Restrict the run to one archive directory, e.g. `levels/doom/a/`
    /// (dev flag, DESIGN.md §4.6).
    #[arg(long, global = true, value_name = "PATH")]
    root: Option<String>,

    /// Process at most N entries (dev flag, DESIGN.md §4.6).
    #[arg(long, global = true, value_name = "N")]
    limit: Option<usize>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Phase 1 — enumerate the /idgames tree, enrich with API metadata (DESIGN.md §4).
    HarvestApi,
    /// Phase 2 — true WAD sizes via HTTP range reads of zip central directories (DESIGN.md §5).
    HarvestZips,
    /// §6.4 — curated modern-outliers analysis (DESIGN.md §6.4).
    HarvestOutliers,
    /// Phase 3 — statistics and the sweep corpus manifest (DESIGN.md §6).
    Stats,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    match cli.command {
        Command::HarvestApi => phase1::run(cli.root.as_deref(), cli.limit),
        Command::HarvestZips => zips::run(cli.root.as_deref(), cli.limit),
        Command::HarvestOutliers => outliers::run(cli.root.as_deref(), cli.limit),
        Command::Stats => stats::run(cli.root.as_deref(), cli.limit),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn cli_structure_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn subcommands_parse_by_their_kebab_case_names() {
        let cases = [
            ("harvest-api", "HarvestApi"),
            ("harvest-zips", "HarvestZips"),
            ("harvest-outliers", "HarvestOutliers"),
            ("stats", "Stats"),
        ];
        for (name, expected) in cases {
            let cli = Cli::try_parse_from(["xtask", name]).unwrap();
            let actual = match cli.command {
                Command::HarvestApi => "HarvestApi",
                Command::HarvestZips => "HarvestZips",
                Command::HarvestOutliers => "HarvestOutliers",
                Command::Stats => "Stats",
            };
            assert_eq!(actual, expected, "subcommand `{name}`");
        }
    }

    #[test]
    fn dev_flags_are_global_and_optional() {
        let cli = Cli::try_parse_from(["xtask", "harvest-api"]).unwrap();
        assert_eq!(cli.root, None);
        assert_eq!(cli.limit, None);

        // Global flags must parse in the subcommand position too.
        let cli = Cli::try_parse_from([
            "xtask",
            "harvest-api",
            "--root",
            "levels/doom/a/",
            "--limit",
            "5",
        ])
        .unwrap();
        assert_eq!(cli.root.as_deref(), Some("levels/doom/a/"));
        assert_eq!(cli.limit, Some(5));
    }
}
