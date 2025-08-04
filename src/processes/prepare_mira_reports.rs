#![allow(dead_code, unused_imports)]
use crate::utils::{data_ingest::*, writing_outputs::*};
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
use serde::{self, Deserialize, de::DeserializeOwned};
use std::sync::Arc;
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
    /// The file path to the IRMA outputs
    irma_path: PathBuf,

    #[arg(short = 's', long)]
    /// The filepath to the input samplesheet
    samplesheet: PathBuf,

    #[arg(short = 'q', long)]
    /// The file path to the qc yaml
    qc_yaml: PathBuf,

    #[arg(short = 'p', long)]
    /// The platform used to generate the data
    platform: String,

    #[arg(short = 'r', long)]
    /// The run id
    runid: String,

    #[arg(short = 'w', long)]
    /// The file path to the working directory
    workdir_path: PathBuf,

    #[arg(short = 'c', long, default_value = "default config")]
    /// the irma config used for IRMA
    irma_config: Option<String>,
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
    /////////////// Read in all data ///////////////
    // Read in samplesheet
    let samplesheet_path = create_reader(args.samplesheet)?;
    let samplesheet: Vec<SamplesheetO> = read_csv(samplesheet_path, false)?;

    // Read in qc yaml
    let qc_yaml_path = create_reader(args.qc_yaml)?;
    let qc_config: QCConfig = read_yaml(qc_yaml_path)?;

    //Read in IRMA data
    let coverage_data = coverage_data_collection(&args.irma_path, &args.platform, &args.runid)?;
    let read_data = reads_data_collection(&args.irma_path, &args.platform, &args.runid)?;
    let vtype_data = create_vtype_data(&read_data);
    let allele_data = allele_data_collection(&args.irma_path)?;
    let indel_data = indels_data_collection(&args.irma_path)?;
    let seq_data = amended_consensus_data_collection(&args.irma_path, "flu");

    //Read in DAIS-ribosome data
    let dais_ins_data = dias_insertion_data_collection(&args.irma_path);
    let dais_del_data = dias_deletion_data_collection(&args.irma_path);
    let dais_seq_data = dias_sequence_data_collection(&args.irma_path);
    let dais_ref_data = dias_ref_seq_data_collection(&args.workdir_path, "flu");

    //TODO: remove these at end
    //println!("{samplesheet:?}");
    //println!("{qc_config:?}")
    //println!("cov data: {coverage_data:?}");
    //println!("Allele data: {allele_data:?}");
    //println!("Indel data: {indel_data:?}");
    //println!("Seq data: {seq_data:#?}");
    //println!("dais ins data: {dais_ins_data:#?}");
    //println!("dais del data: {dais_del_data:#?}");
    //println!("dais seq data: {dais_seq_data:#?}");
    //println!("dais seq data: {dais_ref_data:#?}");

    /////////////// Write the structs to JSON files and CSV files ///////////////
    // Writing out coverage data
    let coverage_struct_values = vec![
        "Sample",
        "Reference_Name",
        "Position",
        "Coverage Depth",
        "Consensus",
        "Deletions",
        "Ambiguous",
        "Consensus_Count",
        "Consensus_Average_Quality",
    ];

    let coverage_columns = vec![
        "sample_id",
        "reference",
        "reference_position",
        "depth",
        "consensus",
        "deletions",
        "ambiguous",
        "consensus_count",
        "consensus_quality",
    ];

    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/coverage_data.json",
        &coverage_data,
        &coverage_columns,
        &coverage_struct_values,
    )?;
    write_structs_to_csv_file(
        "/home/xpa3/mira-oxide/test/coverage_data.csv",
        &coverage_data,
        &coverage_columns,
        &coverage_struct_values,
    )?;

    // Writing out reads data
    let reads_struct_values = vec![
        "Sample",
        "Record",
        "Reads",
        "Patterns",
        "PairsAndWidows",
        "Stage",
    ];
    let reads_columns = vec![
        "sample_id",
        "record",
        "reads",
        "patterns",
        "pairs_and_windows",
        "stage",
    ];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/reads.json",
        &read_data,
        &reads_columns,
        &reads_struct_values,
    )?;
    write_structs_to_csv_file(
        "/home/xpa3/mira-oxide/test/reads.csv",
        &read_data,
        &reads_columns,
        &reads_struct_values,
    )?;

    // Writing out vtype data (json only)
    let vtype_columns = vec!["sample_id", "vtype", "ref_type", "subtype"];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/vtype.json",
        &vtype_data,
        &vtype_columns,
        &vtype_columns,
    )?;

    // Writing out allele csv and josn file
    let allele_struct_values = vec![
        "Sample",
        "Upstream_Position",
        "Reference_Name",
        "Context",
        "Length",
        "Insert",
        "Count",
        "Total",
        "Frequency",
    ];
    let allele_columns = vec![
        "sample",
        "sample_upstream_position",
        "reference",
        "context",
        "length",
        "insert",
        "count",
        "upstream_base_coverage",
        "frequency",
    ];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/alleles.json",
        &allele_data,
        &allele_columns,
        &allele_struct_values,
    )?;

    write_structs_to_csv_file(
        "/home/xpa3/mira-oxide/test/alleles.csv",
        &allele_data,
        &allele_columns,
        &allele_struct_values,
    )?;

    // Writing out indel csv and josn file
    let indels_struct_values = vec![
        "Sample",
        "Upstream_Position",
        "Reference_Name",
        "Context",
        "Length",
        "Insert",
        "Count",
        "Total",
        "Frequency",
    ];
    let indels_columns = vec![
        "sample",
        "sample_upstream_position",
        "reference",
        "context",
        "length",
        "insert",
        "count",
        "upstream_base_coverage",
        "frequency",
    ];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/indels.json",
        &indel_data,
        &indels_columns,
        &indels_struct_values,
    )?;

    write_structs_to_csv_file(
        "/home/xpa3/mira-oxide/test/indels.csv",
        &indel_data,
        &indels_columns,
        &indels_struct_values,
    )?;

    // Write fields to parq if flag given
    write_reads_to_parquet(&read_data, "/home/xpa3/mira-oxide/test/read_data.parquet")?;

    Ok(())
}
