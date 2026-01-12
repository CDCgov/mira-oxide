#![allow(dead_code, unused_imports)]
use std::{error::Error, path::PathBuf};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    about = "Package for adding the nextclade clade results back into the summary.csv and sumary.parq files created by MIRA"
)]
pub struct SummaryUpdateArgs {
    #[arg(short = 'i', long)]
    /// The file path to the samples folders with nextclade outputs.
    nextclade_path: PathBuf,

    #[arg(short = 'o', long)]
    /// The file path where the `prepare_mira_report` outputs will be saved.
    output_path: PathBuf,

    #[arg(short = 's', long)]
    /// The filepath to the input summary csv
    summary_csv: PathBuf,

    #[arg(short = 'p', long)]
    /// The platform used to generate the data.
    /// Options: illumina or ont
    platform: String,

    #[arg(short = 'v', long)]
    /// The virus the the data was generated from.
    /// Options: flu, sc2-wgs, sc2-spike or rsv
    virus: String,

    #[arg(short = 'r', long)]
    /// The run id. Used to create custom file names associated with `run_id`.
    runid: String,

    #[arg(short = 'w', long)]
    /// The file path to the user's cloned MIRA-NF repo.
    workdir_path: PathBuf,

    #[arg(short = 'f', long)]
    /// (Optional) A flag to indicate whether to create parquet files.
    parq: bool,
}

pub fn summary_report_update_process(args: &SummaryUpdateArgs) -> Result<(), Box<dyn Error>> {
    let _ = args;
    Ok(())
}
