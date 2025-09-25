use crate::io::data_ingest::ReadsData;
use crate::utils::data_processing::{
    AASequences, NTSequences, extract_field, extract_string_fields_as_int,
};
use arrow::array::Float64Array;
use arrow::{
    array::{ArrayRef, Float32Array, Int32Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::arrow::ArrowWriter;
use std::{error::Error, fs::File, sync::Arc};

use super::data_ingest::{AllelesData, CoverageData, IndelsData};

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

/// Write the coverage data to parquet file.
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

/// Write the alleles data to parquet file.
pub fn write_alleles_to_parquet(
    alleles_data: &[AllelesData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(alleles_data, |item| item.sample_id.clone());
    let reference_vec = extract_field(alleles_data, |item| item.reference.clone());
    let sample_position_vec = extract_field(alleles_data, |item| item.sample_position);
    let coverage_vec = extract_field(alleles_data, |item| item.coverage);
    let consensus_allele_vec = extract_field(alleles_data, |item| item.consensus_allele.clone());
    let minority_allele_vec = extract_field(alleles_data, |item| item.minority_allele.clone());
    let consensus_count_vec = extract_field(alleles_data, |item| item.consensus_count);
    let minority_count_vec = extract_field(alleles_data, |item| item.minority_count);
    let minority_frequency_vec = extract_field(alleles_data, |item| item.minority_frequency);
    let runid_vec = extract_field(alleles_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(alleles_data, |item| item.instrument.clone());

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let reference_array: ArrayRef = Arc::new(StringArray::from(reference_vec));
    let sample_position_array: ArrayRef = Arc::new(Int32Array::from(sample_position_vec));
    let coverage_array: ArrayRef = Arc::new(Int32Array::from(coverage_vec));
    let consensus_allele_array: ArrayRef = Arc::new(StringArray::from(consensus_allele_vec));
    let minority_allele_array: ArrayRef = Arc::new(StringArray::from(minority_allele_vec));
    let consensus_count_array: ArrayRef = Arc::new(Int32Array::from(consensus_count_vec));
    let minority_count_array: ArrayRef = Arc::new(Int32Array::from(minority_count_vec));
    let minority_frequency_array: ArrayRef = Arc::new(Float64Array::from(minority_frequency_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("reference", DataType::Utf8, true),
        Field::new("sample_position", DataType::Int32, true),
        Field::new("coverage", DataType::Int32, true),
        Field::new("consensus_allele", DataType::Utf8, true),
        Field::new("minority_allele", DataType::Utf8, true),
        Field::new("consensus_count", DataType::Int32, true),
        Field::new("minority_count", DataType::Int32, true),
        Field::new("minority_frequency", DataType::Float64, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            reference_array,
            sample_position_array,
            coverage_array,
            consensus_allele_array,
            minority_allele_array,
            consensus_count_array,
            minority_count_array,
            minority_frequency_array,
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

/// Write the alleles data to parquet file.
pub fn write_indels_to_parquet(
    indels_data: &[IndelsData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(indels_data, |item| item.sample_id.clone());
    let sample_upstream_position_vec =
        extract_field(indels_data, |item| item.sample_upstream_position.clone());
    let reference_name_vec = extract_field(indels_data, |item| item.reference_name.clone());
    let context_vec = extract_field(indels_data, |item| item.context.clone());
    let length_vec = extract_field(indels_data, |item| item.length);
    let insert_vec = extract_field(indels_data, |item| item.insert.clone());
    let count_vec = extract_field(indels_data, |item| item.count);
    let upstream_base_coverage_vec = extract_field(indels_data, |item| item.total);
    let frequency_vec = extract_field(indels_data, |item| item.frequency);
    let runid_vec = extract_field(indels_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(indels_data, |item| item.instrument.clone());

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let sample_upstream_position_array: ArrayRef =
        Arc::new(StringArray::from(sample_upstream_position_vec));
    let reference_name_array: ArrayRef = Arc::new(StringArray::from(reference_name_vec));
    let context_array: ArrayRef = Arc::new(StringArray::from(context_vec));
    let length_array: ArrayRef = Arc::new(Int32Array::from(length_vec));
    let insert_array: ArrayRef = Arc::new(StringArray::from(insert_vec));
    let count_array: ArrayRef = Arc::new(Int32Array::from(count_vec));
    let upstream_base_coverage_array: ArrayRef =
        Arc::new(Int32Array::from(upstream_base_coverage_vec));
    let frequency_array: ArrayRef = Arc::new(Float64Array::from(frequency_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("sample_upstream_position", DataType::Utf8, true),
        Field::new("reference_name", DataType::Utf8, true),
        Field::new("context", DataType::Utf8, true),
        Field::new("length", DataType::Int32, true),
        Field::new("insert", DataType::Utf8, true),
        Field::new("count", DataType::Int32, true),
        Field::new("upstream_base_coverage", DataType::Int32, true),
        Field::new("frequency", DataType::Float64, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            sample_upstream_position_array,
            reference_name_array,
            context_array,
            length_array,
            insert_array,
            count_array,
            upstream_base_coverage_array,
            frequency_array,
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

/// Write the nt seqeunce data to parquet file.
pub fn write_nt_seq_to_parquet(
    nt_seq_data: &[NTSequences],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<String> = extract_field(nt_seq_data, |item| item.sample_id.clone());
    let sequence_vec = extract_field(nt_seq_data, |item| item.sequence.clone());
    let reference_vec = extract_field(nt_seq_data, |item| item.reference.clone());
    let qc_decision_vec = extract_field(nt_seq_data, |item| item.qc_decision.clone());
    let runid_vec = extract_field(nt_seq_data, |item| item.runid.clone());
    let instrument_vec = extract_field(nt_seq_data, |item| item.instrument.clone());

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let sequence_array: ArrayRef = Arc::new(StringArray::from(sequence_vec));
    let reference_array: ArrayRef = Arc::new(StringArray::from(reference_vec));
    let qc_decision_array: ArrayRef = Arc::new(StringArray::from(qc_decision_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("reference", DataType::Utf8, true),
        Field::new("qc_decision", DataType::Utf8, true),
        Field::new("sequence", DataType::Utf8, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            reference_array,
            qc_decision_array,
            sequence_array,
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

/// Write the aa seqeunce data to parquet file.
pub fn write_aa_seq_to_parquet(
    aa_seq_data: &[AASequences],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<String> = extract_field(aa_seq_data, |item| item.sample_id.clone());
    let sequence_vec = extract_field(aa_seq_data, |item| item.sequence.clone());
    let protein_vec = extract_field(aa_seq_data, |item| item.protein.clone());
    let qc_decision_vec = extract_field(aa_seq_data, |item| item.qc_decision.clone());
    let runid_vec = extract_field(aa_seq_data, |item| item.runid.clone());
    let instrument_vec = extract_field(aa_seq_data, |item| item.instrument.clone());

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let sequence_array: ArrayRef = Arc::new(StringArray::from(sequence_vec));
    let protein_array: ArrayRef = Arc::new(StringArray::from(protein_vec));
    let qc_decision_array: ArrayRef = Arc::new(StringArray::from(qc_decision_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("reference", DataType::Utf8, true),
        Field::new("qc_decision", DataType::Utf8, true),
        Field::new("sequence", DataType::Utf8, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            protein_array,
            qc_decision_array,
            sequence_array,
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
