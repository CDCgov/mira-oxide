use crate::io::data_ingest::ReadsData;
use crate::utils::data_processing::{extract_field, extract_string_fields_as_int};
use arrow::array::Float64Array;
use arrow::{
    array::{ArrayRef, Float32Array, Int32Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::arrow::ArrowWriter;
use std::{error::Error, fs::File, sync::Arc};

use super::data_ingest::CoverageData;

/////////////// Functions to write parquet files out ///////////////
pub fn extract_string_fields_as_float<V, T, F>(data: V, extractor: F) -> Vec<f32>
where
    V: AsRef<[T]>,
    F: Fn(&T) -> Option<String>,
{
    data.as_ref()
        .iter()
        .map(|item| {
            if let Some(field) = extractor(item) {
                field.parse::<f32>().unwrap_or(0.0)
            } else {
                0.0
            }
        })
        .collect()
}

/// Write the reads data to parquet file.
pub fn write_coverage_to_parquet(
    coverage_data: &[CoverageData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(coverage_data, |item| item.sample_id.clone());
    let ref_name_vec = extract_field(coverage_data, |item| item.reference_name.clone());
    let position_vec = extract_field(coverage_data, |item| item.position);
    let coverage_depth_vec = extract_field(coverage_data, |item| item.coverage_depth);
    let consensus_vec = extract_field(coverage_data, |item| item.consensus.clone());
    let deletions_vec = extract_field(coverage_data, |item| item.deletions);
    let ambiguous_vec = extract_field(coverage_data, |item| item.ambiguous);
    let consensus_count_vec = extract_field(coverage_data, |item| item.consensus_count);
    let consensus_avg_quality_vec = extract_field(coverage_data, |item| item.consensus_avg_quality);
    let runid_vec = extract_field(coverage_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(coverage_data, |item| item.instrument.clone());

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let ref_name_array: ArrayRef = Arc::new(StringArray::from(ref_name_vec));
    let positions_array: ArrayRef = Arc::new(Int32Array::from(position_vec));
    let coverage_depth_array: ArrayRef = Arc::new(Int32Array::from(coverage_depth_vec));
    let consensus_array: ArrayRef = Arc::new(StringArray::from(consensus_vec));
    let deletions_array: ArrayRef = Arc::new(Int32Array::from(deletions_vec));
    let ambiguous_array: ArrayRef = Arc::new(Int32Array::from(ambiguous_vec));
    let consensus_count_array: ArrayRef = Arc::new(Int32Array::from(consensus_count_vec));
    let consensus_avg_quality_array: ArrayRef =
        Arc::new(Float64Array::from(consensus_avg_quality_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("reference_name", DataType::Utf8, true),
        Field::new("position", DataType::Int32, true),
        Field::new("coverage_depth", DataType::Int32, true),
        Field::new("consensus", DataType::Utf8, true),
        Field::new("deletions", DataType::Int32, true),
        Field::new("ambiguous", DataType::Int32, true),
        Field::new("consensus_count", DataType::Int32, true),
        Field::new("consensus_avg_quality", DataType::Float64, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));
    println!("{schema:?}");

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            ref_name_array,
            positions_array,
            coverage_depth_array,
            consensus_array,
            deletions_array,
            ambiguous_array,
            consensus_count_array,
            consensus_avg_quality_array,
            runid_array,
            instrument_array,
        ],
    )?;

    // Write the RecordBatch to a Parquet file
    let file = File::create(output_file)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(&record_batch)?;
    writer.close()?;

    println!("PARQUET written to {output_file}");

    Ok(())
}

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
    let patterns_vec = extract_string_fields_as_float(reads_data, |item| item.patterns.clone());
    let pairs_and_windows_vec =
        extract_string_fields_as_float(reads_data, |item| item.pairs_and_windows.clone());
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

    println!("PARQUET written to {output_file}");

    Ok(())
}
