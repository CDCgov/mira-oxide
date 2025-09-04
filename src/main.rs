#![warn(clippy::pedantic)]
#![allow(
    unreachable_patterns,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]
use crate::processes::{
    all_sample_hd::{HammingArgs, all_sample_hd_process},
    all_sample_nt_diffs::{NTDiffsArgs, all_sample_nt_diffs_process},
    find_chemistry::{FindChemArgs, find_chemistry_process},
    plotter::{PlotterArgs, plotter_process},
    positions_of_interest::{PositionsArgs, positions_of_interest_process},
    variants_of_interest::{VariantsArgs, variants_of_interest_process},
};
use clap::{Parser, Subcommand};
use processes::prepare_mira_reports::{ReportsArgs, prepare_mira_reports_process};
use zoe::prelude::OrFail;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Variants of Interest
    VariantsOfInterest(VariantsArgs),
    /// Positions of Interest
    PositionsOfInterest(PositionsArgs),
    /// Find Chemistry
    FindChemistry(FindChemArgs),
    /// Hamming
    Hamming(HammingArgs),
    /// nt diffs
    NTDiffs(NTDiffsArgs),
    /// Plotter
    Plotter(PlotterArgs),
    /// Prepare MIRA report
    PrepareMiraReports(ReportsArgs),
}

fn main() {
    let args = Cli::parse();
    let module = module_path!();

    match args.command {
        Commands::VariantsOfInterest(cmd_args) => {
            variants_of_interest_process(cmd_args)
                .unwrap_or_else(|_| panic!("{module}::VariantsOfInterest"));
        }
        Commands::PositionsOfInterest(cmd_args) => positions_of_interest_process(cmd_args)
            .unwrap_or_else(|_| panic!("{module}::PositionsOfInterest")),

        Commands::FindChemistry(cmd_args) => {
            find_chemistry_process(&cmd_args).unwrap_or_die(&format!("{module}::FindChemistry"));
        }
        Commands::Hamming(cmd_args) => {
            all_sample_hd_process(&cmd_args).unwrap_or_die(&format!("{module}::Hamming"));
        }
        Commands::NTDiffs(cmd_args) => all_sample_nt_diffs_process(&cmd_args),
        Commands::Plotter(cmd_args) => {
            plotter_process(cmd_args).unwrap_or_else(|_| panic!("{module}::Plotter"));
        }
        Commands::PrepareMiraReports(cmd_args) => prepare_mira_reports_process(cmd_args)
            .unwrap_or_else(|_| panic!("{module}::PrepareMiraReports")),
        _ => {
            eprintln!("mira-oxide: unrecognized command {:?}", args.command);
            std::process::exit(1)
        }
    }
}

pub mod processes;
pub mod utils;
