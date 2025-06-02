#![allow(dead_code, unused_imports)]
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
use serde::{self, Deserialize, de::DeserializeOwned};
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

#[derive(Deserialize, Debug)]
pub struct Samplesheet {
    sample_id: String,
    sample_type: String,
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

fn read_csv<T: DeserializeOwned, R: std::io::Read>(
    reader: R,
    has_headers: bool,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b',')
        .from_reader(reader);

    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let record: T = result?;
        records.push(record);
    }

    Ok(records)
}
fn main() {
    let args = APDArgs::parse();

    //read in samplesheet
    let samplesheet_path = create_reader(args.samplesheet).unwrap();
    let samplesheet: Vec<Samplesheet> = read_csv(samplesheet_path, false).unwrap();

    println!("{:?}", samplesheet)
}
