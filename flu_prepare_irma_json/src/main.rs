use clap::Parser;
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Stdin, Write, stdin, stdout},
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(about = "Tool for calculating amino acid difference tables")]
pub struct APDArgs {
    #[arg(short = 'i', long)]
    /// Optional input fasta
    irma_path: Option<PathBuf>,

    #[arg(short = 's', long)]
    /// Optional input fasta
    samplesheet: Option<PathBuf>,

    #[arg(short = 'p', long)]
    /// Optional input fasta
    platform: Option<String>,

    #[arg(short = 'w', long)]
    /// Optional output delimited file
    workdir_path: Option<PathBuf>,

    #[arg(short = 'c', long, default_value = ",")]
    /// Use the provider delimiter for separating fields. Default is ','
    irma_config: Option<PathBuf>,
}

fn main() {
    println!("Hello, world!");
}
