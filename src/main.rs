#![allow(unreachable_patterns)]
use crate::processes::{
    all_sample_hd::*, all_sample_nt_diffs::*, find_chemistry::*, nf_status::*, plotter::*,
    positions_of_interest::*, variants_of_interest::*,
};
use clap::{Parser, Subcommand};
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
    /// NF Status
    NFStatus(NFStatusArgs),
}

fn main() {
    let args = Cli::parse();
    let module = module_path!();

    match args.command {
        Commands::VariantsOfInterest(cmd_args) => variants_of_interest_process(cmd_args)
            .unwrap_or_else(|_| panic!("{module}::VariantsOfInterest")),
        Commands::PositionsOfInterest(cmd_args) => positions_of_interest_process(cmd_args)
            .unwrap_or_else(|_| panic!("{module}::PositionsOfInterest")),
        Commands::FindChemistry(cmd_args) => {
            find_chemistry_process(cmd_args).unwrap_or_die(&format!("{module}::FindChemistry"))
        }
        Commands::Hamming(cmd_args) => {
            all_sample_hd_process(cmd_args).unwrap_or_die(&format!("{module}::Hamming"))
        }
        Commands::NTDiffs(cmd_args) => all_sample_nt_diffs_process(cmd_args),
        Commands::Plotter(cmd_args) => {
            plotter_process(cmd_args).unwrap_or_else(|_| panic!("{module}::Plotter"))
        }
        Commands::NFStatus(cmd_args) => {
            nf_status_process(cmd_args).unwrap_or_else(|_| panic!("{module}::NFStatus"))
        }
        _ => {
            eprintln!("mira-oxide: unrecognized command {:?}", args.command);
            std::process::exit(1)
        }
    }
}

mod processes;
pub use crate::processes::*;
pub(crate) mod utils;
