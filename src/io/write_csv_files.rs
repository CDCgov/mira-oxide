use csv::Writer;
use serde::Serialize;
use serde_json::Value;
use std::{error::Error, path::Path};

use crate::{
    processes::summary_report_update::UpdatedIRMASummary,
    utils::data_processing::{AASequences, DaisVarsData, IRMASummary, NTSequences},
};

use super::data_ingest::{AlleleDataCollection, CoverageData, IndelsData, ReadsData, RunInfo};

//////////////// Function to write CSV ///////////////
pub fn write_structs_to_csv_file<T: Serialize>(
    file_path: &str,
    data: &[T],
    columns: &[&str],
    struct_values: &[&str],
) -> Result<(), Box<dyn Error>> {
    let mut csv_writer = Writer::from_path(file_path)?;

    csv_writer.write_record(columns)?;

    for line in data {
        // Serialize the struct into a JSON object
        // This was the most effectient way to select columns for csv file
        let json_value: Value = serde_json::to_value(line)?;

        // Extract the specified fields from the JSON object
        let row: Vec<String> = struct_values
            .iter()
            .map(|field| {
                json_value
                    .get(*field)
                    .map_or(String::new(), |v| v.to_string().replace('"', ""))
            })
            .collect();

        csv_writer.write_record(row)?;
    }

    csv_writer.flush()?;
    println!(" -> CSV written to {file_path}");

    Ok(())
}

//////////////// Function to collection and write out all CSV files ///////////////
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn write_out_all_csv_mira_reports(
    output_path: &Path,
    coverage_data: &[CoverageData],
    read_data: &[ReadsData],
    allele_data: &AlleleDataCollection,
    indel_data: &[IndelsData],
    dais_vars: &[DaisVarsData],
    irma_summary: &[IRMASummary],
    nt_seq_vec: &[NTSequences],
    aa_seq_vec: &[AASequences],
    run_info: &[RunInfo],
    runid: &str,
    virus: &str,
) -> Result<(), Box<dyn Error>> {
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
        "Run_ID",
        "Instrument",
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
        "consensus_average_quality",
        "run_id",
        "instrument",
    ];
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_coverage.csv", output_path.display()),
        coverage_data,
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
        "Run_ID",
        "Instrument",
    ];
    let reads_columns = vec![
        "sample_id",
        "record",
        "reads",
        "patterns",
        "pairs_and_windows",
        "stage",
        "run_id",
        "instrument",
    ];
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_reads.csv", output_path.display()),
        read_data,
        &reads_columns,
        &reads_struct_values,
    )?;

    // Writing out allele data
    let allele_columns = if virus == "sc2-spike" {
        vec![
            "sample",
            "reference",
            "reference_position",
            "depth",
            "consensus_allele",
            "minority_allele",
            "consensus_count",
            "minority_count",
            "minority_frequency",
            "run_id",
            "instrument",
        ]
    } else {
        vec![
            "sample",
            "reference",
            "sample_position",
            "depth",
            "consensus_allele",
            "minority_allele",
            "consensus_count",
            "minority_count",
            "minority_frequency",
            "run_id",
            "instrument",
        ]
    };

    let allele_struct_values = if virus == "sc2-spike" {
        vec![
            "Sample",
            "Reference_Name",
            "HMM_Position",
            "Total",
            "Consensus_Allele",
            "Minority_Allele",
            "Consensus_Count",
            "Minority_Count",
            "Minority_Frequency",
            "Run_ID",
            "Instrument",
        ]
    } else {
        vec![
            "Sample",
            "Reference_Name",
            "Position",
            "Total",
            "Consensus_Allele",
            "Minority_Allele",
            "Consensus_Count",
            "Minority_Count",
            "Minority_Frequency",
            "Run_ID",
            "Instrument",
        ]
    };

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_all_alleles.csv", output_path.display()),
        &allele_data.all_alleles,
        &allele_columns,
        &allele_struct_values,
    )?;

    // Writing out indel
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
        "Run_ID",
        "Instrument",
    ];
    let indels_columns = vec![
        "sample",
        "sample_upstream_position",
        "reference_name",
        "context",
        "length",
        "insert",
        "count",
        "upstream_base_coverage",
        "frequency",
        "runid",
        "instrument",
    ];
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_indels.csv", output_path.display()),
        indel_data,
        &indels_columns,
        &indels_struct_values,
    )?;

    let minor_variant_columns = vec![
        "sample",
        "reference",
        "sample_upstream_position",
        "depth",
        "consensus_allele",
        "minority_allele",
        "consensus_count",
        "minority_count",
        "minority_frequency",
        "run_id",
        "instrument",
    ];

    //Write out variants.csv - filtered by snv > 0.05
    let minor_variant_struct_values = if virus == "sc2-spike" {
        vec![
            "Sample",
            "Reference_Name",
            "HMM_Position",
            "Total",
            "Consensus_Allele",
            "Minority_Allele",
            "Consensus_Count",
            "Minority_Count",
            "Minority_Frequency",
            "Run_ID",
            "Instrument",
        ]
    } else {
        vec![
            "Sample",
            "Reference_Name",
            "Position",
            "Total",
            "Consensus_Allele",
            "Minority_Allele",
            "Consensus_Count",
            "Minority_Count",
            "Minority_Frequency",
            "Run_ID",
            "Instrument",
        ]
    };

    write_structs_to_csv_file(
        &format!(
            "{}/mira_{runid}_filtered_variants.csv",
            output_path.display()
        ),
        &allele_data.filtered_alleles,
        &minor_variant_columns,
        &minor_variant_struct_values,
    )?;

    // write out the aavars.csv
    let aavars_columns = vec![
        "sample_id",
        "reference_id",
        "protein",
        "aa_variant_count",
        "aa_variants",
    ];

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_aavars.csv", output_path.display()),
        dais_vars,
        &aavars_columns,
        &aavars_columns,
    )?;

    // write out the mira_{runid}_summary.csv
    let summary_columns: Vec<&str> = if virus == "sc2-wgs" {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
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
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
        ]
    };

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_summary.csv", output_path.display()),
        irma_summary,
        &summary_columns,
        &summary_columns,
    )?;

    // write out the amended_consensus
    let seq_columns = vec![
        "sample_id",
        "reference",
        "qc_decision",
        "sequence",
        "runid",
        "instrument",
    ];

    write_structs_to_csv_file(
        &format!(
            "{}/mira_{runid}_amended_consensus.csv",
            output_path.display()
        ),
        nt_seq_vec,
        &seq_columns,
        &seq_columns,
    )?;

    // write out the amino_acid_consensus
    // Note that the struct values are different but the column values are the same
    let aa_struct_values = vec![
        "sample_id",
        "protein",
        "qc_decision",
        "sequence",
        "runid",
        "instrument",
    ];

    write_structs_to_csv_file(
        &format!(
            "{}/mira_{runid}_amino_acid_consensus.csv",
            output_path.display()
        ),
        aa_seq_vec,
        &seq_columns,
        &aa_struct_values,
    )?;

    // Writing out run information
    let run_info_struct_values = vec![
        "program_name",
        "PROGRAM",
        "Iterative Refinement Meta-Assembler (IRMA)",
        "Run_ID",
        "Instrument",
        "Timestamp",
    ];
    let run_info_columns = vec![
        "program_name",
        "program",
        "irma",
        "runid",
        "instrument",
        "timestamp",
    ];
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_irma_config.csv", output_path.display()),
        run_info,
        &run_info_columns,
        &run_info_struct_values,
    )?;

    Ok(())
}

//////////////// Function to collection and write out all CSV files ///////////////
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn write_out_updated_summary_csv(
    summary_data: &[UpdatedIRMASummary],
    virus: &str,
    runid: &str,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let summary_struct_values: Vec<&str> = if virus == "sc2-wgs" {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "spike_percent_coverage",
            "spike_median_coverage",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
            "nextclade_field_1",
            "nextclade_field_2",
            "nextclade_field_3",
        ]
    } else if virus == "flu" {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
            "nextclade_field_1",
            "nextclade_field_2",
            "nextclade_field_3",
        ]
    } else {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
            "nextclade_field_1",
            "nextclade_field_2",
        ]
    };

    let summary_columns: Vec<&str> = if virus == "sc2-wgs" {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "spike_percent_coverage",
            "spike_median_coverage",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
            "clade",
            "clade_who",
            "nextclade_pango",
        ]
    } else if virus == "flu" {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
            "clade",
            "short_clade",
            "subclade",
        ]
    } else {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "percent_reference_coverage",
            "median_coverage",
            "count_minor_snv_at_or_over_5_pct",
            "count_minor_snv_at_or_over_20_pct",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
            "clade",
            "g_clade",
        ]
    };

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_summary.csv", output_path.display()),
        summary_data,
        &summary_columns,
        &summary_struct_values,
    )?;
    Ok(())
}
