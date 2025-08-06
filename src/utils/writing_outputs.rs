use crate::utils::data_ingest::ReadsData;
use crate::utils::data_processing::*;
use arrow::array::{ArrayRef, Float32Array, Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use csv::Writer;
use parquet::arrow::ArrowWriter;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::{error::Error, fs::File};

/////////////// Functions to write to json and csv files ///////////////
/// Function to serialize a vector of structs into split-oriented JSON with precision and indexing
pub fn write_structs_to_split_json_file<T: Serialize>(
    file_path: &str,
    data: &Vec<T>,
    columns: &Vec<&str>,
    struct_values: &Vec<&str>,
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

/// Write to CSV
pub fn write_structs_to_csv_file<T: Serialize>(
    file_path: &str,
    data: &Vec<T>,
    columns: &Vec<&str>,
    struct_values: &Vec<&str>,
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
    println!("CSV written to {file_path}");

    Ok(())
}

/// make ref_data.json - has unique set up
pub fn write_ref_data_json(
    file_path: &str,
    ref_lens: &HashMap<String, usize>,
    segments: &Vec<String>,
    segset: &Vec<String>,
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
/////////////// Functions to write parquet files out ///////////////

/// Write the reads data to parquet file.
pub fn write_reads_to_parquet(
    reads_data: &Vec<ReadsData>,
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(reads_data.clone(), |item| item.sample_id.clone());
    let record_vec = extract_field(reads_data.clone(), |item| item.record.clone());
    let reads_vec = extract_field(reads_data.clone(), |item| item.reads);
    let patterns_vec = extract_string_fields_as_float(reads_data.clone(), |item| &item.patterns);
    let pairs_and_windows_vec =
        extract_string_fields_as_float(reads_data.clone(), |item| &item.pairs_and_windows);
    let stages_vec = extract_string_fields_as_int(reads_data.clone(), |item| {
        item.stage.as_deref().unwrap_or("")
    });
    let runid_vec = extract_field(reads_data.clone(), |item| item.run_id.clone());
    let instrument_vec = extract_field(reads_data.clone(), |item| item.instrument.clone());

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
