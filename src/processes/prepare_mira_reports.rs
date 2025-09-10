#![allow(dead_code, unused_imports)]
use crate::utils::{
    data_ingest::{
        DaisSeqData, QCConfig, QCSettings, allele_data_collection,
        amended_consensus_data_collection, coverage_data_collection, create_reader,
        dais_deletion_data_collection, dais_insertion_data_collection,
        dais_ref_seq_data_collection, dais_sequence_data_collection, get_reference_lens,
        indels_data_collection, read_csv, read_yaml, reads_data_collection,
    },
    data_processing::{
        DaisVarsData, ProcessedCoverage, Subtype, collect_analysis_metadata, collect_negatives,
        collect_sample_id, compute_cvv_dais_variants, compute_dais_variants,
        create_prelim_irma_summary_df, create_vtype_data, extract_field, extract_subtype_flu,
        extract_subtype_sc2, melt_reads_data, process_position_coverage_data,
        process_wgs_coverage_data, return_seg_data,
    },
    writing_outputs::negative_qc_statement,
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

    #[arg(short = 'v', long)]
    /// The virus the the data was generated from
    virus: String,

    #[arg(short = 'r', long)]
    /// The run id
    runid: String,

    #[arg(short = 'w', long)]
    /// The file path to the working directory
    workdir_path: PathBuf,

    #[arg(short = 'c', long, default_value = "default-config")]
    /// the irma config used for IRMA
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

//todo: split ingest, proccessing and writing out
#[allow(clippy::too_many_lines)]
pub fn prepare_mira_reports_process(args: ReportsArgs) -> Result<(), Box<dyn Error>> {
    ///////////////////////////////////////////////////////////////////////////
    /////////////// Read in and process data from IRMA and Dais ///////////////
    ///////////////////////////////////////////////////////////////////////////
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
    let allele_data = allele_data_collection(&args.irma_path)?;
    let indel_data = indels_data_collection(&args.irma_path)?;

    let seq_data = amended_consensus_data_collection(&args.irma_path, &args.virus)?;
    let ref_lengths = match get_reference_lens(&args.irma_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error getting reference lengths: {e}");
            return Err(e);
        }
    };
    let (segments, segset, segcolor) =
        return_seg_data(extract_field(coverage_data.clone(), |item| {
            item.reference_name.clone()
        }));

    //Read in DAIS-ribosome data
    let dais_ins_data = dais_insertion_data_collection(&args.irma_path)?;
    let dais_del_data = dais_deletion_data_collection(&args.irma_path)?;
    let dais_seq_data = dais_sequence_data_collection(&args.irma_path)?;
    let mut dais_ref_data: Vec<DaisSeqData> = Vec::new();
    if args.virus.to_lowercase() == "flu" || args.virus.to_lowercase() == "rsv" {
        dais_ref_data = dais_ref_seq_data_collection(&args.workdir_path, &args.virus)?;
    } else if args.virus.to_lowercase() == "sc2-wgs" {
        dais_ref_data = dais_ref_seq_data_collection(&args.workdir_path, "sc2")?;
    }
    //TODO: remove print statements at end
    //println!("{vtype_data:?}");
    //println!("{qc_config:?}")
    //println!("cov data: {coverage_data:?}");
    //println!("Allele data: {allele_data:?}");
    //println!("Indel data: {indel_data:?}");
    //println!("Seq data: {seq_data:#?}");
    //println!("Seq data: {:#?}", seq_data);
    //println!("dais ins data: {dais_ins_data:#?}");
    //println!("dais del data: {dais_del_data:#?}");
    //println!("dais seq data: {dais_seq_data:#?}");
    //println!("dais ref data: {dais_ref_data:#?}");
    //println!("ref length data: {ref_lengths:#?}");

    //Calculate AA variants for aavars.csv and dais_vars.json
    let mut dais_vars_data: Vec<DaisVarsData> = Vec::new();

    if args.virus.to_lowercase() == "flu" {
        dais_vars_data = compute_dais_variants(&dais_ref_data, &dais_seq_data)?;
    } else if args.virus.to_lowercase() == "sc2-wgs"
        || args.virus.to_lowercase() == "sc2"
        || args.virus.to_lowercase() == "rsv"
    {
        dais_vars_data = compute_cvv_dais_variants(&dais_ref_data, &dais_seq_data)?;
    }

    negative_qc_statement(
        "/home/xpa3/mira-oxide/test/qc_statement.json",
        &read_data,
        &neg_control_list,
    )?;

    // Calculating the % coverage and median coverage for summary
    let melted_reads_df = melt_reads_data(&read_data);
    let mut calculated_cov_df: Vec<ProcessedCoverage> = Vec::new();
    let mut calculated_position_cov_df: Vec<ProcessedCoverage> = Vec::new();

    if args.virus.to_lowercase() == "flu" || args.virus.to_lowercase() == "rsv" {
        calculated_cov_df = process_wgs_coverage_data(&coverage_data, &ref_lengths);
    } else if args.virus.to_lowercase() == "sc2-spike" {
        calculated_cov_df =
            process_position_coverage_data(&coverage_data, &ref_lengths, 21563, 25384);
    } else if args.virus.to_lowercase() == "sc2-wgs" {
        calculated_cov_df = process_wgs_coverage_data(&coverage_data, &ref_lengths);
        calculated_position_cov_df =
            process_position_coverage_data(&coverage_data, &ref_lengths, 21563, 25384);
    }

    //Gather subtype information
    //todo: add rsv handling - fix sc2-spike handling
    let mut subtype_data: Vec<Subtype> = Vec::new();
    if args.virus.to_lowercase() == "flu" {
        subtype_data = extract_subtype_flu(&dais_vars_data)?;
    } else if args.virus.to_lowercase() == "sc2-wgs" || args.virus.to_lowercase() == "sc2-spike" {
        subtype_data = extract_subtype_sc2(&dais_vars_data)?;
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
    //More will be added and analyzed before final irma summary created
    //todo: fix the fix sc2-wgs spike protein coverage handling in summary
    let mut irma_summary = create_prelim_irma_summary_df(
        &sample_list,
        &melted_reads_df,
        &calculated_cov_df,
        &allele_data,
        &indel_data,
        &subtype_data,
        analysis_metadata,
    )?;

    //todo: see how this works with the padded amended consensus
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

    for sample in &mut irma_summary {
        sample.create_final_irma_summary_df(&dais_vars_data, &seq_data, &qc_values);
    }
    //println!("{qc_values:?}");
    //todo:remove before end
    //println!("{dais_vars_data:?}");
    //println!("{melted_reads_df:?}");
    //println!("{calculated_cov_df:?}");
    //println!("{calculated_position_cov_df:?}");
    println!("{irma_summary:?}");
    //println!("{subtype_data:?}");

    /////////////////////////////////////////////////////////////////////////////
    /////////////// Write the structs to JSON files and CSV files ///////////////
    /////////////////////////////////////////////////////////////////////////////
    /*
        // Writing out Coverage data
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

        // Writing out allele csv and json file
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

        // Write out ref_data.json
        write_ref_data_json(
            "/home/xpa3/mira-oxide/test/ref_data.json",
            &ref_lengths,
            &segments,
            &segset,
            &segcolor,
        )?;

        // write out the dais_vars.json and the {runid}_aavars.csv
        let aavars_columns = vec![
            "sample_id",
            "reference_id",
            "protein",
            "aa_variant_count",
            "aa_variants",
        ];

        write_structs_to_split_json_file(
            "/home/xpa3/mira-oxide/test/dais_vars.json",
            &dais_vars_data,
            &aavars_columns,
            &aavars_columns,
        )?;

        write_structs_to_csv_file(
            &format!("/home/xpa3/mira-oxide/test/{}_aavars.csv", &args.runid),
            &dais_vars_data,
            &aavars_columns,
            &aavars_columns,
        )?;

        // write out the summary.json and the {runid}_summary.csv
        let summary_columns: Vec<&str> = if args.virus.to_lowercase() == "sc2-wgs" {
            vec![
                "sample_id",
                "total_reads",
                "pass_qc",
                "reads_mapped",
                "reference",
                "precent_reference_coverage",
                "median_coverage",
                "count_minor_snv",
                "count_minor_indel",
                "spike_percent_coverage",
                "spike_median_coverage",
                "pass_fail_reason",
                "subtype",
                "mira_module",
                "runid",
                "instrument",
            ]
        } else {
            vec![
                "sample_id",
                "total_reads",
                "pass_qc",
                "reads_mapped",
                "reference",
                "precent_reference_coverage",
                "median_coverage",
                "count_minor_snv",
                "count_minor_indel",
                "pass_fail_reason",
                "subtype",
                "mira_module",
                "runid",
                "instrument",
            ]
        };

        write_structs_to_split_json_file(
            "/home/xpa3/mira-oxide/test/irma_summary.json",
            &irma_summary,
            &summary_columns,
            &summary_columns,
        )?;

        write_structs_to_csv_file(
            &format!("/home/xpa3/mira-oxide/test/{}_summary.csv", &args.runid),
            &irma_summary,
            &summary_columns,
            &summary_columns,
        )?;

        /////////////// Write the structs to parquet files if flag invoked ///////////////
        // Write fields to parq if flag given
        write_reads_to_parquet(&read_data, "/home/xpa3/mira-oxide/test/read_data.parquet")?;
    */
    Ok(())
}
