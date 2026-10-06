use csv::Writer;
use serde::Serialize;
use serde_json::Value;
use std::{error::Error, path::Path};

use crate::{
    processes::summary_report_update::UpdatedIRMASummary,
    utils::data_processing::{AASequences, DaisVarsData, IRMASummary, NTSequences},
};

use super::data_ingest::{
    CoverageData, IndelsData, MinorVariantDataCollection, ReadsData, RunInfo,
};

/// Writes selected fields from serializable structs to a CSV file.
///
/// The function writes `columns` as the CSV header, then serializes every item
/// in `data` to JSON and writes a row using the fields named in
/// `struct_values`. Each entry in `struct_values` corresponds positionally to
/// the output column at the same index. Fields that are absent from an item's
/// serialized JSON representation are written as empty cells.
///
/// ## Arguments
///
/// - `data` - Values to serialize and write as rows.
/// - `columns` - Column names to write as the CSV header.
/// - `struct_values` - JSON field names to extract from each serialized value.
///
/// ## Errors
///
/// Returns an error if the file cannot be created or written, a value in
/// `data` cannot be serialized to JSON, or a CSV record cannot be written.
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

/// Function to collect and write out all CSV files
#[allow(clippy::too_many_arguments)]
pub fn write_out_all_csv_mira_reports(
    output_path: &Path,
    coverage_data: &[CoverageData],
    read_data: &[ReadsData],
    minor_variant_data: &MinorVariantDataCollection,
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
    let (coverage_columns, coverage_struct_values) = COVERAGE_SCHEMA;
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_coverage.csv", output_path.display()),
        coverage_data,
        coverage_columns,
        coverage_struct_values,
    )?;

    // Writing out reads data
    let (reads_columns, reads_struct_values) = READS_SCHEMA;
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_reads.csv", output_path.display()),
        read_data,
        reads_columns,
        reads_struct_values,
    )?;

    // Writing out minor variants data
    let (minor_variants_columns, minor_vars_struct_values) = if virus == "sc2-spike" {
        MINOR_VARIANTS_SPIKE_SCHEMA
    } else {
        MINOR_VARIANTS_DEFAULT_SCHEMA
    };

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_minor_variants.csv", output_path.display()),
        &minor_variant_data.all_minor_variants,
        minor_variants_columns,
        minor_vars_struct_values,
    )?;

    // Writing out indel
    let (indels_columns, indels_struct_values) = INDELS_SCHEMA;
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_indels.csv", output_path.display()),
        indel_data,
        indels_columns,
        indels_struct_values,
    )?;

    // write out the aavars.csv
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_aavars.csv", output_path.display()),
        dais_vars,
        AAVARS_SCHEMA.0,
        AAVARS_SCHEMA.1,
    )?;

    // write out the mira_{runid}_summary.csv
    let (summary_columns, summary_struct_values) = if virus == "sc2-wgs" {
        SUMMARY_WGS_SCHEMA
    } else if virus == "flu" {
        SUMMARY_FLU_SCHEMA
    } else {
        SUMMARY_DEFAULT_SCHEMA
    };

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_summary.csv", output_path.display()),
        irma_summary,
        summary_columns,
        summary_struct_values,
    )?;

    // write out the amended_consensus
    write_structs_to_csv_file(
        &format!(
            "{}/mira_{runid}_amended_consensus.csv",
            output_path.display()
        ),
        nt_seq_vec,
        NT_SEQUENCE_SCHEMA.0,
        NT_SEQUENCE_SCHEMA.1,
    )?;

    // write out the amino_acid_consensus
    // Note that the struct values are different but the column values are the same
    write_structs_to_csv_file(
        &format!(
            "{}/mira_{runid}_amino_acid_consensus.csv",
            output_path.display()
        ),
        aa_seq_vec,
        AA_SEQUENCE_SCHEMA.0,
        AA_SEQUENCE_SCHEMA.1,
    )?;

    // Writing out run information
    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_irma_config.csv", output_path.display()),
        run_info,
        RUN_INFO_SCHEMA.0,
        RUN_INFO_SCHEMA.1,
    )?;

    Ok(())
}

/// Function to collect and write out all CSV files
pub fn write_out_updated_summary_csv(
    summary_data: &[UpdatedIRMASummary],
    virus: &str,
    runid: &str,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let (summary_columns, summary_struct_values) = if virus == "sc2-wgs" {
        UPDATED_SUMMARY_WGS_SCHEMA
    } else if virus == "flu" {
        UPDATED_SUMMARY_FLU_SCHEMA
    } else {
        UPDATED_SUMMARY_DEFAULT_SCHEMA
    };

    write_structs_to_csv_file(
        &format!("{}/mira_{runid}_summary.csv", output_path.display()),
        summary_data,
        summary_columns,
        summary_struct_values,
    )?;
    Ok(())
}

/// Type alias for combining the column names for a csv and the names of the
/// related JSON fields for populating the columns
type CsvSchema = (&'static [&'static str], &'static [&'static str]);

const COVERAGE_SCHEMA: CsvSchema = (
    &[
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
    ],
    &[
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
    ],
);

const READS_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "record",
        "reads",
        "patterns",
        "pairs_and_windows",
        "stage",
        "run_id",
        "instrument",
    ],
    &[
        "Sample",
        "Record",
        "Reads",
        "Patterns",
        "PairsAndWidows",
        "Stage",
        "Run_ID",
        "Instrument",
    ],
);

const MINOR_VARIANTS_SPIKE_SCHEMA: CsvSchema = (
    &[
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
    ],
    &[
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
    ],
);

const MINOR_VARIANTS_DEFAULT_SCHEMA: CsvSchema = (
    &[
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
    ],
    &[
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
    ],
);

const INDELS_SCHEMA: CsvSchema = (
    &[
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
    ],
    &[
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
    ],
);

const AAVARS_COLUMNS: &[&str] = &[
    "sample_id",
    "aa_reference_id",
    "positional_reference_id",
    "protein",
    "aa_variant_count",
    "aa_variants",
];

const AAVARS_SCHEMA: CsvSchema = (AAVARS_COLUMNS, AAVARS_COLUMNS);

const SUMMARY_WGS_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "spike_percent_coverage",
        "spike_median_coverage",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
    ],
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "spike_percent_coverage",
        "spike_median_coverage",
        "pass_fail_reason",
        "subtype",
        "mira_module",
        "runid",
        "instrument",
    ],
);

const SUMMARY_FLU_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "di_5prime;di_3prime",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
    ],
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "di_ratios_5prime_3prime",
        "pass_fail_reason",
        "subtype",
        "mira_module",
        "runid",
        "instrument",
    ],
);

const SUMMARY_DEFAULT_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
    ],
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "pass_fail_reason",
        "subtype",
        "mira_module",
        "runid",
        "instrument",
    ],
);

const SEQUENCE_COLUMNS: &[&str] = &[
    "sample_id",
    "reference",
    "qc_decision",
    "sequence",
    "runid",
    "instrument",
];

const NT_SEQUENCE_SCHEMA: CsvSchema = (SEQUENCE_COLUMNS, SEQUENCE_COLUMNS);

const AA_SEQUENCE_SCHEMA: CsvSchema = (
    SEQUENCE_COLUMNS,
    &[
        "sample_id",
        "protein",
        "qc_decision",
        "sequence",
        "runid",
        "instrument",
    ],
);

const RUN_INFO_COLUMNS: &[&str] = &[
    "program_name",
    "program",
    "irma",
    "runid",
    "instrument",
    "timestamp",
];

const RUN_INFO_SCHEMA: CsvSchema = (
    RUN_INFO_COLUMNS,
    &[
        "program_name",
        "PROGRAM",
        "Iterative Refinement Meta-Assembler (IRMA)",
        "Run_ID",
        "Instrument",
        "Timestamp",
    ],
);

const UPDATED_SUMMARY_WGS_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "spike_percent_coverage",
        "spike_median_coverage",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
        "clade",
        "clade_who",
        "nextclade_pango",
        "nextclade_version;dataset;tag",
    ],
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "spike_percent_coverage",
        "spike_median_coverage",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
        "nextclade_field_1",
        "nextclade_field_2",
        "nextclade_field_3",
        "nextclade_info",
    ],
);

const UPDATED_SUMMARY_FLU_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "di_5prime;di_3prime",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
        "subclade",
        "nextclade_alias",
        "nextclade_version;dataset;tag",
    ],
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "di_5prime;di_3prime",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
        "nextclade_field_1",
        "nextclade_field_2",
        "nextclade_info",
    ],
);

const UPDATED_SUMMARY_DEFAULT_SCHEMA: CsvSchema = (
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
        "clade",
        "nextclade_version;dataset;tag",
    ],
    &[
        "sample_id",
        "total_reads",
        "pass_qc",
        "reads_mapped",
        "reference",
        "percent_reference_coverage",
        "median_coverage",
        "count_minor_snv_at_or_over_5_pct",
        "pass_fail_reason",
        "subtype",
        "mira_version;module;irma_config",
        "runid",
        "instrument",
        "nextclade_field_1",
        "nextclade_info",
    ],
);
