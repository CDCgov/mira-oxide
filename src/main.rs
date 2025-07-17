#![allow(unreachable_patterns)]
use crate::processes::{
    all_sample_hd::*, all_sample_nt_diffs::*, find_chemistry::*, plotter::*,
    prepare_mira_reports::*, variants_of_interest::*,
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
    /// Find Chemistry
    FindChemistry(FindChemArgs),
    /// Hamming
    Hamming(HammingArgs),
    /// nt diffs
    NTDiffs(NTDiffsArgs),
    /// Plotter
    Plotter(PlotterArgs),
    /// Prepare MIRA report
    PrepareMiraReports(ReportArgs),
}

fn main() {
    let args = Cli::parse();
    let module = module_path!();

    match args.command {
        Commands::VariantsOfInterest(cmd_args) => {
            variants_of_interest_process(cmd_args).expect(&format!("{module}::VariantsOfInterest"))
        }
        Commands::FindChemistry(cmd_args) => {
            find_chemistry_process(cmd_args).unwrap_or_die(&format!("{module}::FindChemistry"))
        }
        Commands::Hamming(cmd_args) => {
            all_sample_hd_process(cmd_args).unwrap_or_die(&format!("{module}::Hamming"))
        }
        Commands::NTDiffs(cmd_args) => all_sample_nt_diffs_process(cmd_args),
        Commands::Plotter(cmd_args) => {
            plotter_process(cmd_args).expect(&format!("{module}::Plotter"))
        }
        _ => {
            eprintln!("mira-oxide: unrecognized command {:?}", args.command);
            std::process::exit(1)
        }
    }
}

mod processes;
pub use crate::processes::*;
