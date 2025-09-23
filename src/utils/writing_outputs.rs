use crate::io::data_ingest::ReadsData;
use crate::utils::data_processing::{
    extract_field, extract_string_fields_as_float, extract_string_fields_as_int,
    filter_struct_by_ids,
};
use arrow::{
    array::{ArrayRef, Float32Array, Int32Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::arrow::ArrowWriter;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::{collections::HashMap, error::Error, fs::File, io::Write, sync::Arc};

use super::data_processing::IRMASummary;

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
                    value.clone()
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

/// make `ref_data.json` - has unique set up
pub fn write_ref_data_json<S: ::std::hash::BuildHasher>(
    file_path: &str,
    ref_lens: &HashMap<String, usize, S>,
    segments: &[String],
    segset: &[String],
    segcolor: &HashMap<String, &str, S>,
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
/////////////// Functions to write parquet files out ///////////////

/// Write the reads data to parquet file.
pub fn write_reads_to_parquet(
    reads_data: &[ReadsData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(reads_data, |item| item.sample_id.clone());
    let record_vec = extract_field(reads_data, |item| item.record.clone());
    let reads_vec = extract_field(reads_data, |item| item.reads);
    let patterns_vec = extract_string_fields_as_float(reads_data, |item| &item.patterns);
    let pairs_and_windows_vec =
        extract_string_fields_as_float(reads_data, |item| &item.pairs_and_windows);
    let stages_vec =
        extract_string_fields_as_int(reads_data, |item| item.stage.as_deref().unwrap_or(""));
    let runid_vec = extract_field(reads_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(reads_data, |item| item.instrument.clone());

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let record_array: ArrayRef = Arc::new(StringArray::from(record_vec));
    let reads_array: ArrayRef = Arc::new(Int32Array::from(reads_vec));
    let patterns_array: ArrayRef = Arc::new(Float32Array::from(patterns_vec));
    let pairs_and_windows_array: ArrayRef = Arc::new(Float32Array::from(pairs_and_windows_vec));
    let stage_array: ArrayRef = Arc::new(Int32Array::from(stages_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("stage", DataType::Utf8, true),
        Field::new("readcount", DataType::Int32, true),
        Field::new("patterns", DataType::Float32, true),
        Field::new("pairsandwindows", DataType::Float32, true),
        Field::new("stagenum", DataType::Int32, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            record_array,
            reads_array,
            patterns_array,
            pairs_and_windows_array,
            stage_array,
            runid_array,
            instrument_array,
        ],
    )?;

    // Write the RecordBatch to a Parquet file
    let file = File::create(output_file)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(&record_batch)?;
    writer.close()?;

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
