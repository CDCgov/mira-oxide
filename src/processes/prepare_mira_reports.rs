#![allow(dead_code, unused_imports)]
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
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
    irma_path: Option<PathBuf>,

    #[arg(short = 's', long)]
    /// Optional input fasta
    samplesheet: Option<PathBuf>,

    #[arg(short = 'q', long)]
    /// Optional input fasta
    qc_yaml: Option<PathBuf>,

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

fn read_yaml<R: std::io::Read>(reader: R) -> Result<QCConfig, Box<dyn std::error::Error>> {
    let mut contents = String::new();
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut contents)?;
    let config: QCConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

pub fn prepare_mira_reports_process(args: ReportsArgs) -> Result<(), std::io::Error> {
    let args = ReportsArgs::parse();

    // Read in samplesheet
    let samplesheet_path = create_reader(args.samplesheet)?;
    let samplesheet: Vec<Samplesheet> = read_csv(samplesheet_path, false)?;

    // Read in qc yaml
    let qc_yaml_path = create_reader(args.qc_yaml)?;
    let qc_config: QCConfig = read_yaml(qc_yaml_path)?;

    //println!("{:?}", samplesheet);
    //println!("{:?}", qc_config);

    Ok(())
}
