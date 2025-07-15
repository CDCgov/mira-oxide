use clap::{Parser, Subcommand};

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
    FindChemistry(FindChemistryArgs),
    /// Hamming
    Hamming(HammingArgs),
    /// nt diffs
    NTDiffs(NTDiffArgs),
    /// Plotter
    Plotter(PlotterArgs),
}

fn main() {
    let args = Cli::parse();
    let module = module_path!();

    matchs args.command {
        Commands::VariantsOfInterest(cmd_args) => {variants_of_interest_process(cmd_args).unwrap_or_die(&format!("{module}::VariantsOfInterest"))}
        Commands::FindChemistry(cmd_args) => {find_chemistry_process(cmd_args).unwrap_or_die(&format!("{module}::FindChemistry"))}
        Commands::Hamming(cmd_args) => {hamming_process(cmd_args).unwrap_or_die(&format!("{module}::Hamming"))}
        Commands::NTDiffs(cmd_args) => {ntdiff_process(cmd_args).unwrap_or_die(&format!("{module}::NTDiffs"))}
        Commands::Plotter(cmd_args) => {plotter_process(cmd_args).unwrap_or_die(&format!("{module}::Plotter"))}
        _ => {
            eprintln!("mira-oxide: unrecognized command {:?}", args.command);
            std::process::exit(1)
        }
    }
}

mod processes;
pub use crate::processes::*;