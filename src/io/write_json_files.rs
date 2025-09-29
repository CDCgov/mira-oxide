use std::{collections::HashMap, error::Error, fs::File, io::Write, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::utils::data_processing::{
    DaisVarsData, IRMASummary, NTSequences, ProcessedRecord, filter_struct_by_ids,
};
use std::collections::HashSet;

use super::data_ingest::{AllelesData, CoverageData, IndelsData, ReadsData};

//////////////// Function to collection and write out all CSV files ///////////////
/////////////// Structs ///////////////
// PassQC struct
#[derive(Serialize, Deserialize, Debug)]
pub struct ReadQC {
    pub sample_id: String,
    pub percent_mapping: f64,
}

/////////////// Functions to write to json and csv files ///////////////
/// Function to serialize a vector of structs into split-oriented JSON with precision and indexing
pub fn write_structs_to_split_json_file<T: Serialize>(
    file_path: &str,
    data: &[T],
    columns: &[&str],
    struct_values: &[&str],
) -> Result<(), Box<dyn Error>> {
    // Create the "split-oriented" JSON structure
    let split_json = json!({
        "columns": columns,
        "index": (0..data.len()).collect::<Vec<_>>(),
        "data": data.iter().map(|item| {
            // Serialize each struct into a JSON value
            let serialized = serde_json::to_value(item).unwrap();
            let object = serialized.as_object().unwrap();

            // Extract fields in the order specified by `columns`
            struct_values.iter().map(|&struct_value| {
                if let Some(value) = object.get(struct_value) {
                    if value == "NA" {
                        json!(null)
                    } else {
                        value.clone()
                    }
                } else {
                    json!(null)
                }
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    });

    std::fs::write(file_path, serde_json::to_string_pretty(&split_json)?)?;

    println!("Split-oriented JSON written to {file_path}");

    Ok(())
}

pub fn write_irma_summary_to_pass_fail_json_file(
    file_path: &str,
    data: &[IRMASummary],
) -> Result<(), Box<dyn Error>> {
    // Extract unique sample_id and reference values
    let unique_sample_ids: Vec<String> = data
        .iter()
        .filter_map(|item| item.sample_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let unique_references: Vec<String> = data
        .iter()
        .filter_map(|item| item.reference.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Create a mapping of (sample_id, reference) to pass_fail_reason
    let mut pass_fail_map: HashMap<(String, String), Option<String>> = HashMap::new();
    for item in data {
        if let (Some(sample_id), Some(reference)) = (&item.sample_id, &item.reference) {
            pass_fail_map.insert(
                (sample_id.clone(), reference.clone()),
                item.pass_fail_reason.clone(),
            );
        }
    }

    // Build the "data" field for the JSON
    let data_matrix: Vec<Vec<Option<String>>> = unique_sample_ids
        .iter()
        .map(|sample_id| {
            unique_references
                .iter()
                .map(|reference| {
                    pass_fail_map
                        .get(&(sample_id.clone(), reference.clone()))
                        .cloned()
                        .unwrap_or(None)
                })
                .collect()
        })
        .collect();

    // Create the "split-oriented" JSON structure
    let split_json = json!({
        "columns": unique_references,
        "index": unique_sample_ids,
        "data": data_matrix,
    });

    // Write the JSON to the specified file
    std::fs::write(file_path, serde_json::to_string_pretty(&split_json)?)?;

    println!("Split-oriented JSON written to {file_path}");

    Ok(())
}

#[allow(clippy::implicit_hasher)]
/// make `ref_data.json` - has unique set up
pub fn write_ref_data_json<S: ::std::hash::BuildHasher>(
    file_path: &str,
    ref_lens: &HashMap<String, usize, S>,
    segments: &[String],
    segset: &[String],
    segcolor: &HashMap<String, &str>,
) -> Result<(), Box<dyn Error>> {
    let json_data = json!({
        "ref_lens": ref_lens,
        "segments": segments,
        "segset": segset,
        "segcolor": segcolor,
    });

    // Write JSON to a file
    let mut file = File::create(file_path)?;
    file.write_all(serde_json::to_string_pretty(&json_data)?.as_bytes())?;

    println!("Data written to ref_data.json");

    Ok(())
}

pub fn negative_qc_statement(
    output_file: &str,
    reads_data: &[ReadsData],
    neg_control_list: &[String],
) -> Result<(), Box<dyn Error>> {
    let filtered_reads_data = filter_struct_by_ids(reads_data, neg_control_list);

    let mut results = Vec::new();

    for sample in &filtered_reads_data {
        if sample.record == "1-initial" {
            // Find all corresponding "3-match" and "3-altmatch" records for the same sample_id
            if let Some(sample_id) = &sample.sample_id {
                let reads_stage_1 = sample.reads;

                let total_reads_stage_3: i32 = filtered_reads_data
                    .iter()
                    .filter(|d| {
                        d.sample_id == Some(sample_id.clone())
                            && (d.record == "3-match" || d.record == "3-altmatch")
                    })
                    .map(|d| d.reads)
                    .sum();

                if total_reads_stage_3 > 0 {
                    let percent_mapping =
                        (f64::from(total_reads_stage_3) / f64::from(reads_stage_1) * 100.0).round();

                    results.push(ReadQC {
                        sample_id: sample_id.clone(),
                        percent_mapping,
                    });
                }
            }
        }
    }

    // Categorize results into "passes QC" and "FAILS QC"
    let mut passes_qc = HashMap::new();
    let mut fails_qc = HashMap::new();

    for qc in results {
        let percent_mapping_str = format!("{:.2}", qc.percent_mapping);
        if qc.percent_mapping < 1.0 {
            passes_qc.insert(qc.sample_id, percent_mapping_str);
        } else {
            fails_qc.insert(qc.sample_id, percent_mapping_str);
        }
    }

    let mut output = HashMap::new();
    output.insert("passes QC".to_string(), passes_qc);
    output.insert("FAILS QC".to_string(), fails_qc);

    // Write the JSON to a file
    let json_output = json!(output);
    let mut file = File::create(output_file)?;
    file.write_all(json_output.to_string().as_bytes())?;

    println!("JSON written to {output_file}");
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::implicit_hasher
)]
pub fn write_out_all_json_files<S: ::std::hash::BuildHasher>(
    output_path: &Path,
    coverage_data: &[CoverageData],
    read_data: &[ReadsData],
    vtype_data: &[ProcessedRecord],
    allele_data: &[AllelesData],
    indel_data: &[IndelsData],
    dais_vars_data: &[DaisVarsData],
    neg_control_list: &[String],
    irma_summary: &[IRMASummary],
    nt_seq_vec: &[NTSequences],
    ref_lengths: &HashMap<String, usize, S>,
    segments: &[String],
    segset: &[String],
    segcolor: &HashMap<String, &str>,
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
    ];

    write_structs_to_split_json_file(
        &format!("{}/coverage.json", output_path.display()),
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
        &format!("{}/reads.json", output_path.display()),
        read_data,
        &reads_columns,
        &reads_struct_values,
    )?;

    // Writing out vtype data (json only)
    let vtype_columns = vec!["sample_id", "vtype", "ref_type", "subtype"];
    write_structs_to_split_json_file(
        &format!("{}/vtype.json", output_path.display()),
        vtype_data,
        &vtype_columns,
        &vtype_columns,
    )?;

    // Writing out allele
    let allele_columns = vec![
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

    write_structs_to_split_json_file(
        &format!("{}/alleles.json", output_path.display()),
        allele_data,
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
        &format!("{}/indels.json", output_path.display()),
        indel_data,
        &indels_columns,
        &indels_struct_values,
    )?;

    // Write out ref_data.json
    write_ref_data_json(
        &format!("{}/ref_data.json", output_path.display()),
        ref_lengths,
        segments,
        segset,
        segcolor,
    )?;

    // write out the dais_vars.json
    let aavars_columns = vec![
        "sample_id",
        "reference_id",
        "protein",
        "aa_variant_count",
        "aa_variants",
    ];

    write_structs_to_split_json_file(
        &format!("{}/dais_vars.json", output_path.display()),
        dais_vars_data,
        &aavars_columns,
        &aavars_columns,
    )?;

    negative_qc_statement(
        &format!("{}/qc_statement.json", output_path.display()),
        read_data,
        neg_control_list,
    )?;

    // write out the summary.json and the {runid}_summary.csv
    let summary_columns: Vec<&str> = if virus == "sc2-wgs" {
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
        &format!("{}/irma_summary.json", output_path.display()),
        irma_summary,
        &summary_columns,
        &summary_columns,
    )?;

    write_irma_summary_to_pass_fail_json_file(
        &format!("{}/pass_fail_qc.json", output_path.display()),
        irma_summary,
    )?;

    // write out the nt_sequences.json
    let nt_seq_columns: Vec<&str> = if virus == "flu" {
        vec!["sample_id", "sequence", "target_ref", "reference"]
    } else {
        vec!["sample_id", "sequence", "reference"]
    };

    write_structs_to_split_json_file(
        &format!("{}/nt_sequences.json", output_path.display()),
        nt_seq_vec,
        &nt_seq_columns,
        &nt_seq_columns,
    )?;

    Ok(())
}
