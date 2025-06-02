use clap::Parser;
use either::Either;
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Stdin, Write, stdin, stdout},
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(about = "Package for aggregating MIRA outputs into json files")]
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

fn create_reader(path: Option<PathBuf>) -> std::io::Result<BufReader<Either<File, Stdin>>> {
    let reader = if let Some(ref file_path) = path {
        let file = OpenOptions::new().read(true).open(file_path)?;
        BufReader::new(Either::Left(file))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    Ok(reader)
}

fn main() {
    let args = APDArgs::parse();

    //read in samplesheet
    let samplesheet = create_reader(args.samplesheet)?;
}
