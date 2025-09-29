#![allow(dead_code, unused_imports)]
use crate::utils::data_processing::{
    DaisVarsData, ProcessedCoverage, Subtype, collect_analysis_metadata, collect_negatives,
    collect_sample_id, compute_cvv_dais_variants, compute_dais_variants, create_aa_seq_vec,
    create_irma_summary_vec, create_nt_seq_vec, create_vtype_data, divide_aa_into_pass_fail_vec,
    divide_nt_into_pass_fail_vec, extract_field, extract_subtype_flu, extract_subtype_sc2,
    melt_reads_data, process_position_coverage_data, process_wgs_coverage_data, return_seg_data,
};
use crate::{
    io::{
        data_ingest::{
            DaisSeqData, QCConfig, QCSettings, allele_data_collection,
            amended_consensus_data_collection, coverage_data_collection, create_reader,
            dais_ref_seq_data_collection, dais_sequence_data_collection, get_reference_lens,
            indels_data_collection, read_csv, read_yaml, reads_data_collection,
            run_info_collection,
        },
        write_csv_files::write_out_all_csv_mira_reports,
        write_fasta_files::write_out_all_fasta_files,
        write_json_files::{negative_qc_statement, write_out_all_json_files},
        write_parquet_files::{
            write_aa_seq_to_parquet, write_alleles_to_parquet, write_coverage_to_parquet,
            write_indels_to_parquet, write_irma_summary_to_parquet, write_minor_vars_to_parquet,
            write_nt_seq_to_parquet, write_reads_to_parquet, write_run_info_to_parquet,
        },
    },
    utils::data_processing::extract_subtype_rsv,
};
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
use serde::{self, Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
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
    /// The file path to the samples folders with IRMA outputs.
    irma_path: PathBuf,

    #[arg(short = 'o', long)]
    /// The file path where the `prepare_mira_report` outputs will be saved.
    output_path: PathBuf,

    #[arg(short = 's', long)]
    /// The filepath to the input samplesheet
    samplesheet: PathBuf,

    #[arg(short = 'q', long)]
    /// The file path to the qc yaml
    qc_yaml: PathBuf,

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

    #[arg(short = 'c', long, default_value = "default-config")]
    /// (Optional) The IRMA configuration used for processing.
    irma_config: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SamplesheetI {
    #[serde(rename = "Sample ID")]
    pub sample_id: String,
    #[serde(rename = "Sample Type")]
    pub sample_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SamplesheetO {
    #[serde(rename = "Barcode #")]
    pub barcode: String,
    #[serde(rename = "Sample ID")]
    pub sample_id: String,
    #[serde(rename = "Sample Type")]
    pub sample_type: Option<String>,
}

enum Samplesheet {
    Illumina(Vec<SamplesheetI>),
    ONT(Vec<SamplesheetO>),
}

#[allow(clippy::too_many_lines)]
pub fn prepare_mira_reports_process(args: ReportsArgs) -> Result<(), Box<dyn Error>> {
    /////////////// Read in and process data from IRMA and Dais ///////////////
    // Read in samplesheet
    let samplesheet_path = create_reader(args.samplesheet)?;
    let samplesheet = if &args.platform == "illumina" {
        let illumina_samplesheet: Vec<SamplesheetI> = read_csv(samplesheet_path, true)?;
        Samplesheet::Illumina(illumina_samplesheet)
    } else {
        let ont_samplesheet: Vec<SamplesheetO> = read_csv(samplesheet_path, true)?;
        Samplesheet::ONT(ont_samplesheet)
    };

    // Get sample ids from the samplesheet
    let sample_list = match samplesheet {
        Samplesheet::Illumina(ref sheet) => collect_sample_id(sheet),
        Samplesheet::ONT(ref sheet) => collect_sample_id(sheet),
    };

    // Get the negative controls from the samplesheet
    let neg_control_list = match samplesheet {
        Samplesheet::Illumina(ref sheet) => collect_negatives(sheet),
        Samplesheet::ONT(ref sheet) => collect_negatives(sheet),
    };

    // Read in qc yaml
    let qc_yaml_path = create_reader(args.qc_yaml)?;
    let qc_config: QCConfig = read_yaml(qc_yaml_path)?;

    // Read in IRMA data
    let coverage_data = coverage_data_collection(&args.irma_path, &args.platform, &args.runid)?;
    let read_data = reads_data_collection(&args.irma_path, &args.platform, &args.runid)?;
    let vtype_data = create_vtype_data(&read_data);
    let allele_data = allele_data_collection(&args.irma_path, &args.platform, &args.runid)?;
    let indel_data = indels_data_collection(&args.irma_path, &args.platform, &args.runid)?;
    let run_info = run_info_collection(&args.irma_path, &args.platform, &args.runid)?;
    let seq_data = amended_consensus_data_collection(&args.irma_path, &args.virus)?;
    let ref_lengths = match get_reference_lens(&args.irma_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error getting reference lengths: {e}");
            return Err(e);
        }
    };
    let (segments, segset, segcolor) = return_seg_data(extract_field(&coverage_data, |item| {
        item.reference_name.clone()
    }));

    //Read in DAIS-ribosome data
    let dais_seq_data = dais_sequence_data_collection(&args.irma_path)?;
    let mut dais_ref_data: Vec<DaisSeqData> = Vec::new();
    if args.virus.to_lowercase() == "flu" || args.virus.to_lowercase() == "rsv" {
        dais_ref_data = dais_ref_seq_data_collection(&args.workdir_path, &args.virus)?;
    } else if args.virus.to_lowercase() == "sc2-wgs" || args.virus.to_lowercase() == "sc2-spike" {
        dais_ref_data = dais_ref_seq_data_collection(&args.workdir_path, "sc2")?;
    }

    //////////////////////////////// Processing ingested IRMA and Dais data ////////////////////////////////
    //Calculate AA variants for aavars.csv and dais_vars.json
    let mut dais_vars_data: Vec<DaisVarsData> = Vec::new();
    if args.virus.to_lowercase() == "flu" {
        dais_vars_data =
            compute_dais_variants(&dais_ref_data, &dais_seq_data, &args.runid, &args.platform)?;
    } else if args.virus.to_lowercase() == "sc2-wgs"
        || args.virus.to_lowercase() == "sc2-spike"
        || args.virus.to_lowercase() == "rsv"
    {
        dais_vars_data = compute_cvv_dais_variants(
            &dais_ref_data,
            &dais_seq_data,
            &args.runid,
            &args.platform,
            &args.virus,
        )?;
    }

    // Calculating the % coverage and median coverage for summary
    let melted_reads_vec = melt_reads_data(&read_data);
    let mut calculated_cov_vec: Vec<ProcessedCoverage> = Vec::new();
    let mut calculated_position_cov_vec: Vec<ProcessedCoverage> = Vec::new();

    if args.virus.to_lowercase() == "flu"
        || args.virus.to_lowercase() == "rsv"
        || args.virus.to_lowercase() == "sc2-spike"
    {
        calculated_cov_vec = process_wgs_coverage_data(&coverage_data, &ref_lengths)?;
    } else if args.virus.to_lowercase() == "sc2-wgs" {
        calculated_cov_vec = process_wgs_coverage_data(&coverage_data, &ref_lengths)?;
        calculated_position_cov_vec = process_position_coverage_data(&coverage_data, 21563, 25384)?;
    }

    //Gather subtype information
    //todo: add rsv handling - fix sc2-spike handling
    let mut subtype_data: Vec<Subtype> = Vec::new();
    if args.virus.to_lowercase() == "flu" {
        subtype_data = extract_subtype_flu(&dais_vars_data)?;
    } else if args.virus.to_lowercase() == "sc2-wgs" || args.virus.to_lowercase() == "sc2-spike" {
        subtype_data = extract_subtype_sc2(&dais_vars_data)?;
    } else if args.virus.to_lowercase() == "rsv" {
        subtype_data = extract_subtype_rsv(&dais_vars_data)?;
    }

    //Gather Anlysis Metadata for irma_summary
    let analysis_metadata = collect_analysis_metadata(
        &args.workdir_path,
        &args.platform,
        &args.virus,
        &args.irma_config,
        &args.runid,
    )?;

    //Build prelim irma summary "dataframe"
    let mut irma_summary = create_irma_summary_vec(
        &sample_list,
        &melted_reads_vec,
        &calculated_cov_vec,
        &allele_data.filtered_alleles,
        &indel_data,
        &subtype_data,
        &analysis_metadata,
        Some(&calculated_position_cov_vec),
    )?;

    let mut qc_values = QCSettings {
        med_cov: 0,
        minor_vars: 0,
        allow_stop_codons: false,
        perc_ref_covered: 0,
        negative_control_perc: 0,
        negative_control_perc_exception: 0,
        positive_control_minimum: 0,
        padded_consensus: false,
        med_spike_cov: None,
        perc_ref_spike_covered: None,
    };
    // Set qc values based on given virus and platform
    if args.virus.to_lowercase() == "flu" {
        if args.platform.to_lowercase() == "illumina" {
            qc_values = qc_config.illumina_flu;
        } else {
            qc_values = qc_config.ont_flu;
        }
    } else if args.virus.to_lowercase() == "sc2-wgs" {
        if args.platform.to_lowercase() == "illumina" {
            qc_values = qc_config.illumina_sc2;
        } else {
            qc_values = qc_config.ont_sc2;
        }
    } else if args.virus.to_lowercase() == "sc2-spike" {
        qc_values = qc_config.ont_sc2_spike;
    } else if args.virus.to_lowercase() == "rsv" {
        if args.platform.to_lowercase() == "illumina" {
            qc_values = qc_config.illumina_rsv;
        } else {
            qc_values = qc_config.ont_rsv;
        }
    }

    // Add pass fail information to irma summary
    for sample in &mut irma_summary {
        if sample.pass_fail_reason.is_none() {
            sample.add_pass_fail_qc(&dais_vars_data, &seq_data, &qc_values)?;
        }
    }

    // Construct seq info and add pass fail information
    let nt_seq_vec = create_nt_seq_vec(
        &seq_data,
        &vtype_data,
        &irma_summary,
        &args.virus,
        &args.runid,
        &args.platform,
    )?;
    let aa_seq_vec = create_aa_seq_vec(
        &dais_seq_data,
        &irma_summary,
        &args.virus,
        &args.runid,
        &args.platform,
    )?;

    //Sort into passing and failing
    let processed_nt_seq = divide_nt_into_pass_fail_vec(&nt_seq_vec, &args.platform, &args.virus)?;
    let processed_aa_seq = divide_aa_into_pass_fail_vec(&aa_seq_vec, &args.platform, &args.virus)?;

    //////////////////////////////// Write all files ////////////////////////////////

    write_out_all_fasta_files(
        &args.output_path,
        &processed_nt_seq.passed_seqs,
        &processed_nt_seq.failed_seqs,
        &processed_aa_seq.passed_seqs,
        &processed_aa_seq.failed_seqs,
        &args.runid,
    )?;

    write_out_all_csv_mira_reports(
        &args.output_path,
        &coverage_data,
        &read_data,
        &allele_data,
        &indel_data,
        &irma_summary,
        &nt_seq_vec,
        &aa_seq_vec,
        &run_info,
        &args.runid,
        &args.virus,
    )?;

    write_out_all_json_files(
        &args.output_path,
        &coverage_data,
        &read_data,
        &vtype_data,
        &allele_data,
        &indel_data,
        &dais_vars_data,
        &neg_control_list,
        &irma_summary,
        &nt_seq_vec,
        &ref_lengths,
        &segments,
        &segset,
        &segcolor,
        &args.virus,
    )?;

    // Write fields to parq if flag given
    // Why separate you ask? parquet set up it niche
    if args.parq {
        write_coverage_to_parquet(
            &coverage_data,
            &format!(
                "{}/{}_coverage.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
        write_reads_to_parquet(
            &read_data,
            &format!("{}/{}_reads.parq", &args.output_path.display(), args.runid),
        )?;
        write_alleles_to_parquet(
            &allele_data.all_alleles,
            &format!(
                "{}/{}_all_alleles.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
        write_indels_to_parquet(
            &indel_data,
            &format!("{}/{}_indels.parq", &args.output_path.display(), args.runid),
        )?;
        write_minor_vars_to_parquet(
            &allele_data.filtered_alleles,
            &format!(
                "{}/{}_minor_variants.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
        write_irma_summary_to_parquet(
            &irma_summary,
            &args.virus,
            &format!(
                "{}/{}_summary.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
        write_nt_seq_to_parquet(
            &nt_seq_vec,
            &format!(
                "{}/{}_amended_consensus.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
        write_aa_seq_to_parquet(
            &aa_seq_vec,
            &format!(
                "{}/{}_amino_acid_consensus.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
        write_run_info_to_parquet(
            &run_info,
            &format!(
                "{}/{}_irma_config.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
    }

    Ok(())
}
