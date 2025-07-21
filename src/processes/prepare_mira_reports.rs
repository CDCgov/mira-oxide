#![allow(dead_code, unused_imports)]
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
use glob::glob;
use polars::prelude::*;
use serde::{self, Deserialize, de::DeserializeOwned};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Stdin, Write, stdin, stdout},
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(about = "Package for aggregating MIRA outputs into json files")]
pub struct ReportsArgs {
    #[arg(short = 'i', long)]
    /// Optional input fasta
    irma_path: PathBuf,

    #[arg(short = 's', long)]
    /// Optional input fasta
    samplesheet: PathBuf,

    #[arg(short = 'q', long)]
    /// Optional input fasta
    qc_yaml: PathBuf,

    #[arg(short = 'p', long)]
    /// Optional input fasta
    platform: String,

    #[arg(short = 'w', long)]
    /// Optional output delimited file
    workdir_path: PathBuf,

    #[arg(short = 'c', long, default_value = ",")]
    /// Use the provider delimiter for separating fields. Default is ','
    irma_config: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
pub struct Samplesheet {
    sample_id: String,
    sample_type: String,
}

#[derive(Debug, Deserialize)]
struct QCSettings {
    med_cov: u32,
    minor_vars: u32,
    allow_stop_codons: bool,
    perc_ref_covered: u32,
    negative_control_perc: u32,
    negative_control_perc_exception: u32,
    positive_control_minimum: u32,
    padded_consensus: bool,
    #[serde(default)]
    med_spike_cov: Option<u32>,
    #[serde(default)]
    perc_ref_spike_covered: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct QCConfig {
    #[serde(rename = "ont-flu")]
    ont_flu: QCSettings,
    #[serde(rename = "ont-sc2-spike")]
    ont_sc2_spike: QCSettings,
    #[serde(rename = "illumina-flu")]
    illumina_flu: QCSettings,
    #[serde(rename = "illumina-sc2")]
    illumina_sc2: QCSettings,
    #[serde(rename = "ont-sc2")]
    ont_sc2: QCSettings,
    #[serde(rename = "illumina-rsv")]
    illumina_rsv: QCSettings,
    #[serde(rename = "ont-rsv")]
    ont_rsv: QCSettings,
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

fn read_csv_to_dataframe(file_path: &PathBuf) -> Result<DataFrame, Box<dyn Error>> {
    // Read the CSV file into a DataFrame
    let df = CsvReader::from_path(file_path)?
        .infer_schema(None)
        .has_header(true)
        .finish()?;

    Ok(df)
}

fn read_yaml<R: std::io::Read>(reader: R) -> Result<QCConfig, Box<dyn std::error::Error>> {
    let mut contents = String::new();
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut contents)?;
    let config: QCConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

fn coverage_df(irma_path: &PathBuf) -> Result<DataFrame, Box<dyn Error>> {
    // Define the pattern to match text files
    let pattern = format!(
        "{}/*/IRMA/*/tables/*coverage.txt",
        irma_path.to_string_lossy()
    );
    println!("{}", pattern);

    // Initialize an empty DataFrame to hold the combined data
    let mut combined_cov_df: Option<DataFrame> = None;

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                // Read the CSV file into a DataFrame
                let file_path = path.to_str().unwrap();
                println!("Reading file: {}", file_path);

                let df = CsvReader::from_path(file_path)?.has_header(true).finish()?;

                // Combine the DataFrame with the existing one
                combined_cov_df = match combined_cov_df {
                    Some(existing_df) => Some(existing_df.vstack(&df)?),
                    None => Some(df),
                };
            }
            Err(e) => println!("Error reading file: {}", e),
        }
    }

    // Return the combined DataFrame or an error if no data was found
    if let Some(df) = combined_cov_df {
        Ok(df)
    } else {
        Err("No files found or no data to combine.".into())
    }
}

pub fn prepare_mira_reports_process(args: ReportsArgs) -> Result<(), Box<dyn Error>> {
    // Read in samplesheet
    let samplesheet = read_csv_to_dataframe(&args.samplesheet)?;

    // Read in qc yaml
    let qc_yaml_path = create_reader(Some(args.qc_yaml))?;
    let qc_config: QCConfig = read_yaml(qc_yaml_path)?;

    //cov df
    let cov_df = coverage_df(&args.irma_path)?;

    //println!("{:?}", samplesheet);
    println!("{:?}", qc_config);
    println!("{:?}", samplesheet);
    println!("{:?}", cov_df);

    Ok(())
}
