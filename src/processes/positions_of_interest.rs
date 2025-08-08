#![allow(unreachable_patterns)]
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Tool for observing codon and amino acid differences at a given poistion")]
pub struct PositionsArgs {
    #[arg(short = 'i', long)]
    /// Optional input fasta
    input_file: PathBuf,

    #[arg(short = 'r', long)]
    /// Optional input fasta
    ref_file: PathBuf,

    #[arg(short = 'm', long)]
    /// Optional input fasta
    muts_file: PathBuf,

    #[arg(short = 'o', long)]
    /// Optional output delimited file
    output_xsv: Option<PathBuf>,

    #[arg(short = 'd', long, default_value = ",")]
    /// Use the provider delimiter for separating fields. Default is ','
    output_delimiter: String,
}

pub fn positions_of_interest_process(args: PositionsArgs) -> Result<(), Box<dyn Error>> {
    print!("building");
    Ok(())
}
