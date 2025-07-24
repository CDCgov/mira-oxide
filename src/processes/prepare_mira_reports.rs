#![allow(dead_code, unused_imports)]
use crate::utils::dataframes::*;
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
    irma_path: PathBuf,

    #[arg(short = 's', long)]
    /// Optional input fasta
    samplesheet: PathBuf,

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
pub struct SamplesheetI {
    sample_id: String,
    sample_type: String,
}

#[derive(Deserialize, Debug)]
pub struct SamplesheetO {
    barcode: String,
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

fn read_yaml<R: std::io::Read>(reader: R) -> Result<QCConfig, Box<dyn std::error::Error>> {
    let mut contents = String::new();
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut contents)?;
    let config: QCConfig = serde_yaml_ng::from_str(&contents)?;
    Ok(config)
}

pub fn prepare_mira_reports_process(args: ReportsArgs) -> Result<(), Box<dyn Error>> {
    // Read in samplesheet
    let samplesheet_path = create_reader(Some(args.samplesheet))?;
    let samplesheet: Vec<SamplesheetO> = read_csv(samplesheet_path, false)?;

    // Read in qc yaml
    let qc_yaml_path = create_reader(args.qc_yaml)?;
    let qc_config: QCConfig = read_yaml(qc_yaml_path)?;

    //Read in data
    let coverage_data = coverage_data_collection(&args.irma_path)?;
    let read_data = reads_data_collection(&args.irma_path)?;
    let vtype_data = create_vtype_data(read_data);
    let allele_data = allele_data_collection(&args.irma_path)?;
    let indel_data = indels_data_collection(&args.irma_path)?;

    println!("{samplesheet:?}");
    println!("{qc_config:?}");
    println!("Coverage data: {coverage_data:?}");
    //println!("Reads data: {read_data:?}");
    println!("Reads data: {vtype_data:?}");
    println!("Allele data: {allele_data:?}");
    println!("Indel data: {indel_data:?}");

    Ok(())
}
