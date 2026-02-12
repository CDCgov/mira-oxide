use crate::io::data_ingest::{AllAllelesData, ReadsData};
use crate::processes::prepare_mira_reports::Samplesheet;
use crate::processes::summary_report_update::UpdatedIRMASummary;
use crate::utils::data_processing::{AASequences, IRMASummary, NTSequences, extract_field};
use arrow::array::Float64Array;
use arrow::{
    array::{ArrayRef, Float32Array, Int32Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::arrow::ArrowWriter;
use std::{error::Error, fs::File, sync::Arc};

use super::data_ingest::{CoverageData, IndelsData, MinorVariantsData, RunInfo};

/////////////// Functions to write parquet files out ///////////////

pub fn extract_string_fields_as_int<V, T, F>(data: V, extractor: F) -> Vec<i32>
where
    V: AsRef<[T]>,
    F: Fn(&T) -> &str,
{
    data.as_ref()
        .iter()
        .map(|item| {
            let field = extractor(item);
            if field.is_empty() {
                0
            } else {
                field.parse::<i32>().unwrap_or(0)
            }
        })
        .collect()
}

pub fn extract_option_string_fields_as_option_int<V, T, F>(
    data: V,
    extractor: F,
) -> Vec<Option<i32>>
where
    V: AsRef<[T]>,
    F: Fn(&T) -> Option<String>,
{
    data.as_ref()
        .iter()
        .map(|item| {
            if let Some(field) = extractor(item) {
                if field.is_empty() {
                    Some(0)
                } else {
                    field.parse::<i32>().ok()
                }
            } else {
                None
            }
        })
        .collect()
}

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

    println!(" -> PARQUET written to {output_file}");

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

    println!(" -> PARQUET written to {output_file}");

    Ok(())
}

/// Write the alleles data to parquet file.
/// TODO: fix the columns for this
pub fn write_alleles_to_parquet(
    alleles_data: &[AllAllelesData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(alleles_data, |item| item.sample_id.clone());
    let reference_vec = extract_field(alleles_data, |item| item.reference.clone());
    let position_vec = extract_field(alleles_data, |item| item.position);
    let allele_vec = extract_field(alleles_data, |item| item.allele.clone());
    let allele_count_vec = extract_field(alleles_data, |item| item.allele_count.clone());
    let total_count_vec = extract_field(alleles_data, |item| item.total_count.clone());
    let allele_frequency_vec = extract_field(alleles_data, |item| item.allele_frequency);
    let average_quality_vec = extract_field(alleles_data, |item| item.average_quality.clone());
    let confidence_not_machine_error_vec = extract_field(alleles_data, |item| {
        item.confidence_not_machine_error.clone()
    });
    let allele_type_vec = extract_field(alleles_data, |item| item.allele_type.clone());
    let runid_vec = extract_field(alleles_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(alleles_data, |item| item.instrument.clone());
    let reference_upstream_position_vec =
        extract_field(alleles_data, |item| item.reference_upstream_position);

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let reference_array: ArrayRef = Arc::new(StringArray::from(reference_vec));
    let position_array: ArrayRef = Arc::new(Int32Array::from(position_vec));
    let allele_array: ArrayRef = Arc::new(StringArray::from(allele_vec));
    let allele_count_array: ArrayRef = Arc::new(Int32Array::from(allele_count_vec));
    let total_count_array: ArrayRef = Arc::new(Int32Array::from(total_count_vec));
    let allele_frequency_array: ArrayRef = Arc::new(Float64Array::from(allele_frequency_vec));
    let average_quality_array: ArrayRef = Arc::new(StringArray::from(average_quality_vec));
    let confidence_not_machine_error_array: ArrayRef =
        Arc::new(StringArray::from(confidence_not_machine_error_vec));
    let allele_type_array: ArrayRef = Arc::new(StringArray::from(allele_type_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));
    let reference_upstream_position_array: ArrayRef =
        Arc::new(Int32Array::from(reference_upstream_position_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("reference", DataType::Utf8, true),
        Field::new("position", DataType::Int32, true),
        Field::new("allele", DataType::Utf8, true),
        Field::new("allele_count", DataType::Int32, true),
        Field::new("total_count", DataType::Int32, true),
        Field::new("allele_frequency", DataType::Float64, true),
        Field::new("average_quality", DataType::Utf8, true),
        Field::new("confidence_not_machine_error", DataType::Utf8, true),
        Field::new("allele_type", DataType::Utf8, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
        Field::new("reference_upstream_position", DataType::Int32, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            reference_array,
            position_array,
            allele_array,
            allele_count_array,
            total_count_array,
            allele_frequency_array,
            average_quality_array,
            confidence_not_machine_error_array,
            allele_type_array,
            runid_array,
            instrument_array,
            reference_upstream_position_array,
        ],
    )?;

    // Write the RecordBatch to a Parquet file
    let file = File::create(output_file)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(&record_batch)?;
    writer.close()?;

    println!(" -> PARQUET written to {output_file}");

    Ok(())
}

/// Write the indels data to parquet file.
pub fn write_indels_to_parquet(
    indels_data: &[IndelsData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(indels_data, |item| item.sample_id.clone());
    let sample_upstream_position_vec =
        extract_option_string_fields_as_option_int(indels_data, |item| {
            item.sample_upstream_position.clone()
        });
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
        Arc::new(Int32Array::from(sample_upstream_position_vec));
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
        Field::new("sample_upstream_position", DataType::Int32, true),
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

    println!(" -> PARQUET written to {output_file}");

    Ok(())
}

/// Write the dais variant data to parquet file. TODO FIX
pub fn write_minor_vars_to_parquet(
    minor_vars_data: &[MinorVariantsData],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(minor_vars_data, |item| item.sample_id.clone());
    let reference_vec = extract_field(minor_vars_data, |item| item.reference.clone());
    let sample_position_vec = extract_field(minor_vars_data, |item| item.sample_position);
    let coverage_vec = extract_field(minor_vars_data, |item| item.coverage);
    let consensus_allele_vec = extract_field(minor_vars_data, |item| item.consensus_allele.clone());
    let minority_allele_vec = extract_field(minor_vars_data, |item| item.minority_allele.clone());
    let consensus_count_vec = extract_field(minor_vars_data, |item| item.consensus_count);
    let minority_count_vec = extract_field(minor_vars_data, |item| item.minority_count);
    let minority_frequency_vec = extract_field(minor_vars_data, |item| item.minority_frequency);
    let runid_vec = extract_field(minor_vars_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(minor_vars_data, |item| item.instrument.clone());

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

    println!(" -> PARQUET written to {output_file}");

    Ok(())
}
/// Write the irma summary data to parquet file.
pub fn write_irma_summary_to_parquet(
    irma_summary_data: &[IRMASummary],
    virus: &str,
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    if irma_summary_data.is_empty() {
        return Err("Input data is empty".into());
    }

    let sample_ids_vec: Vec<Option<String>> =
        extract_field(irma_summary_data, |item| item.sample_id.clone());
    let total_reads_vec = extract_field(irma_summary_data, |item| item.total_reads);
    let pass_qc_vec = extract_field(irma_summary_data, |item| item.pass_qc);
    let reads_mapped_vec = extract_field(irma_summary_data, |item| item.reads_mapped);
    let reference_vec = extract_field(irma_summary_data, |item| item.reference.clone());
    let percent_reference_coverage_vec =
        extract_field(irma_summary_data, |item| item.percent_reference_coverage);
    let median_coverage_vec = extract_field(irma_summary_data, |item| item.median_coverage);
    let count_minor_snv_at_or_over_5_pct_vec = extract_field(irma_summary_data, |item| {
        item.count_minor_snv_at_or_over_5_pct
    });
    let spike_percent_coverage_vec =
        extract_field(irma_summary_data, |item| item.spike_percent_coverage);
    let spike_median_coverage_vec =
        extract_field(irma_summary_data, |item| item.spike_median_coverage);
    let pass_fail_reason_vec =
        extract_field(irma_summary_data, |item| item.pass_fail_reason.clone());
    let subtype_vec = extract_field(irma_summary_data, |item| item.subtype.clone());
    let mira_module_vec = extract_field(irma_summary_data, |item| item.mira_module.clone());
    let runid_vec = extract_field(irma_summary_data, |item| item.runid.clone());
    let instrument_vec = extract_field(irma_summary_data, |item| item.instrument.clone());

    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let total_reads_array: ArrayRef = Arc::new(Int32Array::from(total_reads_vec));
    let pass_qc_array: ArrayRef = Arc::new(Int32Array::from(pass_qc_vec));
    let reads_mapped_array: ArrayRef = Arc::new(Int32Array::from(reads_mapped_vec));
    let reference_array: ArrayRef = Arc::new(StringArray::from(reference_vec));
    let percent_reference_coverage_array: ArrayRef =
        Arc::new(Float64Array::from(percent_reference_coverage_vec));
    let median_coverage_array: ArrayRef = Arc::new(Int32Array::from(median_coverage_vec));
    let count_minor_snv_at_or_over_5_pct_array: ArrayRef =
        Arc::new(Int32Array::from(count_minor_snv_at_or_over_5_pct_vec));
    let pass_fail_reason_array: ArrayRef = Arc::new(StringArray::from(pass_fail_reason_vec));
    let subtype_array: ArrayRef = Arc::new(StringArray::from(subtype_vec));
    let mira_module_array: ArrayRef = Arc::new(StringArray::from(mira_module_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

    let mut fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("total_reads", DataType::Int32, true),
        Field::new("pass_qc", DataType::Int32, true),
        Field::new("reads_mapped", DataType::Int32, true),
        Field::new("reference", DataType::Utf8, true),
        Field::new("percent_reference_coverage", DataType::Float64, true),
        Field::new("median_coverage", DataType::Int32, true),
        Field::new("count_minor_snv_at_or_over_5_pct", DataType::Int32, true),
        Field::new("pass_fail_reason", DataType::Utf8, true),
        Field::new("mira_module", DataType::Utf8, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("instrument", DataType::Utf8, true),
        Field::new("subtype", DataType::Utf8, true),
    ];

    let mut arrays = vec![
        sample_array,
        total_reads_array,
        pass_qc_array,
        reads_mapped_array,
        reference_array,
        percent_reference_coverage_array,
        median_coverage_array,
        count_minor_snv_at_or_over_5_pct_array,
        pass_fail_reason_array,
        mira_module_array,
        runid_array,
        instrument_array,
        subtype_array,
    ];

    if virus == "sc2-wgs" {
        fields.push(Field::new(
            "spike_percent_coverage",
            DataType::Float64,
            true,
        ));
        fields.push(Field::new("spike_median_coverage", DataType::Int32, true));

        arrays.push(Arc::new(Float64Array::from(spike_percent_coverage_vec)));
        arrays.push(Arc::new(Int32Array::from(spike_median_coverage_vec)));
    }

    let schema = Arc::new(Schema::new(fields));
    let record_batch = RecordBatch::try_new(schema.clone(), arrays)?;

    let file = File::create(output_file)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(&record_batch)?;
    writer.close()?;

    println!(" -> PARQUET written to {output_file}");

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

    println!(" -> PARQUET written to {output_file}");

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

    println!(" -> PARQUET written to {output_file}");

    Ok(())
}

/// Write the aa seqeunce data to parquet file.
pub fn write_run_info_to_parquet(
    run_info_data: &[RunInfo],
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    // Convert values in struct to vector of values
    let program_name_vec: Vec<Option<String>> =
        extract_field(run_info_data, |item| item.program_name.clone());
    let program_vec = extract_field(run_info_data, |item| item.program.clone());
    let irma_vec = extract_field(run_info_data, |item| item.irma.clone());
    let runid_vec = extract_field(run_info_data, |item| item.run_id.clone());
    let instrument_vec = extract_field(run_info_data, |item| item.instrument.clone());
    let assembly_time_vec = extract_field(run_info_data, |item| item.timestamp.clone());

    // Convert the vectors into Arrow columns
    let program_name_array: ArrayRef = Arc::new(StringArray::from(program_name_vec));
    let program_array: ArrayRef = Arc::new(StringArray::from(program_vec));
    let irma_array: ArrayRef = Arc::new(StringArray::from(irma_vec));
    let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
    let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));
    let assembly_time_array: ArrayRef = Arc::new(StringArray::from(assembly_time_vec));

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("program_name", DataType::Utf8, true),
        Field::new("program", DataType::Utf8, true),
        Field::new("irma", DataType::Utf8, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("machine", DataType::Utf8, true),
        Field::new("assembly_time", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            program_name_array,
            program_array,
            irma_array,
            runid_array,
            instrument_array,
            assembly_time_array,
        ],
    )?;

    // Write the RecordBatch to a Parquet file
    let file = File::create(output_file)?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(&record_batch)?;
    writer.close()?;

    println!(" -> PARQUET written to {output_file}");

    Ok(())
}

/// Write the samplesheet data to a Parquet file
pub fn write_samplesheet_to_parquet(
    samplesheet: Samplesheet,
    output_file: &str,
    runid: &str,
    instrument: &str,
) -> Result<(), Box<dyn Error>> {
    match samplesheet {
        Samplesheet::Illumina(data) => {
            // Extract fields from SamplesheetI
            let sample_id_vec: Vec<String> = extract_field(&data, |item| item.sample_id.clone());
            let sample_type_vec: Vec<Option<String>> =
                extract_field(&data, |item| item.sample_type.clone());

            // Add runid and instrument fields
            let runid_vec: Vec<String> = vec![runid.to_string(); data.len()];
            let instrument_vec: Vec<String> = vec![instrument.to_string(); data.len()];

            // Convert the vectors into Arrow columns
            let sample_id_array: ArrayRef = Arc::new(StringArray::from(sample_id_vec));
            let sample_type_array: ArrayRef = Arc::new(StringArray::from(sample_type_vec));
            let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
            let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

            // Define the schema for the Arrow IPC file
            let fields = vec![
                Field::new("Sample ID", DataType::Utf8, false),
                Field::new("Sample Type", DataType::Utf8, true),
                Field::new("Run ID", DataType::Utf8, false),
                Field::new("Instrument", DataType::Utf8, false),
            ];
            let schema = Arc::new(Schema::new(fields));

            // Create a RecordBatch
            let record_batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    sample_id_array,
                    sample_type_array,
                    runid_array,
                    instrument_array,
                ],
            )?;

            // Write the RecordBatch to a Parquet file
            let file = File::create(output_file)?;
            let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
            writer.write(&record_batch)?;
            writer.close()?;

            println!(" -> PARQUET written to {output_file}");
        }
        Samplesheet::ONT(data) => {
            // Extract fields from SamplesheetO
            let barcode_vec: Vec<String> = extract_field(&data, |item| item.barcode.clone());
            let sample_id_vec: Vec<String> = extract_field(&data, |item| item.sample_id.clone());
            let sample_type_vec: Vec<Option<String>> =
                extract_field(&data, |item| item.sample_type.clone());

            // Add runid and instrument fields
            let runid_vec: Vec<String> = vec![runid.to_string(); data.len()];
            let instrument_vec: Vec<String> = vec![instrument.to_string(); data.len()];

            // Convert the vectors into Arrow columns
            let barcode_array: ArrayRef = Arc::new(StringArray::from(barcode_vec));
            let sample_id_array: ArrayRef = Arc::new(StringArray::from(sample_id_vec));
            let sample_type_array: ArrayRef = Arc::new(StringArray::from(sample_type_vec));
            let runid_array: ArrayRef = Arc::new(StringArray::from(runid_vec));
            let instrument_array: ArrayRef = Arc::new(StringArray::from(instrument_vec));

            // Define the schema for the Arrow IPC file
            let fields = vec![
                Field::new("Barcode #", DataType::Utf8, false),
                Field::new("Sample ID", DataType::Utf8, false),
                Field::new("Sample Type", DataType::Utf8, true),
                Field::new("Run ID", DataType::Utf8, false),
                Field::new("Instrument", DataType::Utf8, false),
            ];
            let schema = Arc::new(Schema::new(fields));

            // Create a RecordBatch
            let record_batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    barcode_array,
                    sample_id_array,
                    sample_type_array,
                    runid_array,
                    instrument_array,
                ],
            )?;

            // Write the RecordBatch to a Parquet file
            let file = File::create(output_file)?;
            let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
            writer.write(&record_batch)?;
            writer.close()?;

            println!(" -> PARQUET written to {output_file}");
        }
    }

    Ok(())
}

/// Write the updated IRMA summary data to a parquet file
#[allow(clippy::too_many_lines)]
pub fn write_updated_irma_summary_to_parquet(
    summary_data: &[UpdatedIRMASummary],
    virus: &str,
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    if summary_data.is_empty() {
        return Err("Input data is empty".into());
    }

    // Common fields
    let sample_id_vec = extract_field(summary_data, |i| i.sample_id.clone());
    let total_reads_vec = extract_field(summary_data, |i| i.total_reads);
    let pass_qc_vec = extract_field(summary_data, |i| i.pass_qc);
    let reads_mapped_vec = extract_field(summary_data, |i| i.reads_mapped);
    let reference_vec = extract_field(summary_data, |i| i.reference.clone());
    let percent_reference_coverage_vec =
        extract_field(summary_data, |i| i.percent_reference_coverage);
    let median_coverage_vec = extract_field(summary_data, |i| i.median_coverage);
    let count_minor_snv_at_or_over_5_pct_vec =
        extract_field(summary_data, |i| i.count_minor_snv_at_or_over_5_pct);
    let pass_fail_reason_vec = extract_field(summary_data, |i| i.pass_fail_reason.clone());
    let subtype_vec = extract_field(summary_data, |i| i.subtype.clone());
    let mira_module_vec = extract_field(summary_data, |i| i.mira_module.clone());
    let runid_vec = extract_field(summary_data, |i| i.runid.clone());
    let instrument_vec = extract_field(summary_data, |i| i.instrument.clone());

    let mut fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("total_reads", DataType::Int32, true),
        Field::new("pass_qc", DataType::Int32, true),
        Field::new("reads_mapped", DataType::Int32, true),
        Field::new("reference", DataType::Utf8, true),
        Field::new("percent_reference_coverage", DataType::Float64, true),
        Field::new("median_coverage", DataType::Int32, true),
        Field::new("count_minor_snv", DataType::Int32, true),
    ];

    let mut arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(sample_id_vec)) as ArrayRef,
        Arc::new(Int32Array::from(total_reads_vec)) as ArrayRef,
        Arc::new(Int32Array::from(pass_qc_vec)) as ArrayRef,
        Arc::new(Int32Array::from(reads_mapped_vec)) as ArrayRef,
        Arc::new(StringArray::from(reference_vec)) as ArrayRef,
        Arc::new(Float64Array::from(percent_reference_coverage_vec)) as ArrayRef,
        Arc::new(Int32Array::from(median_coverage_vec)) as ArrayRef,
        Arc::new(Int32Array::from(count_minor_snv_at_or_over_5_pct_vec)) as ArrayRef,
    ];

    // SC2 spike fields
    if virus == "sc2-wgs" {
        let spike_percent_coverage_vec = extract_field(summary_data, |i| i.spike_percent_coverage);
        let spike_median_coverage_vec = extract_field(summary_data, |i| i.spike_median_coverage);

        fields.push(Field::new(
            "spike_percent_coverage",
            DataType::Float64,
            true,
        ));
        fields.push(Field::new("spike_median_coverage", DataType::Int32, true));

        arrays.push(Arc::new(Float64Array::from(spike_percent_coverage_vec)) as ArrayRef);
        arrays.push(Arc::new(Int32Array::from(spike_median_coverage_vec)) as ArrayRef);
    }

    // Common trailing fields
    fields.extend([
        Field::new("pass_fail_reason", DataType::Utf8, true),
        Field::new("mira_module", DataType::Utf8, true),
        Field::new("runid", DataType::Utf8, true),
        Field::new("instrument", DataType::Utf8, true),
        Field::new("subtype", DataType::Utf8, true),
    ]);

    arrays.extend(vec![
        Arc::new(StringArray::from(pass_fail_reason_vec)) as ArrayRef,
        Arc::new(StringArray::from(mira_module_vec)) as ArrayRef,
        Arc::new(StringArray::from(runid_vec)) as ArrayRef,
        Arc::new(StringArray::from(instrument_vec)) as ArrayRef,
        Arc::new(StringArray::from(subtype_vec)) as ArrayRef,
    ]);

    // Virus-specific Nextclade fields
    match virus {
        "sc2-wgs" => {
            let clade_vec = extract_field(summary_data, |i| i.nextclade_field_1.clone());
            let clade_who_vec = extract_field(summary_data, |i| i.nextclade_field_2.clone());
            let pango_vec = extract_field(summary_data, |i| i.nextclade_field_3.clone());

            fields.extend([
                Field::new("clade", DataType::Utf8, true),
                Field::new("clade_who", DataType::Utf8, true),
                Field::new("nextclade_pango", DataType::Utf8, true),
            ]);

            arrays.extend(vec![
                Arc::new(StringArray::from(clade_vec)) as ArrayRef,
                Arc::new(StringArray::from(clade_who_vec)) as ArrayRef,
                Arc::new(StringArray::from(pango_vec)) as ArrayRef,
            ]);
        }
        "flu" => {
            let clade_vec = extract_field(summary_data, |i| i.nextclade_field_1.clone());
            let short_clade_vec = extract_field(summary_data, |i| i.nextclade_field_2.clone());
            let subclade_vec = extract_field(summary_data, |i| i.nextclade_field_3.clone());

            fields.extend([
                Field::new("clade", DataType::Utf8, true),
                Field::new("short_clade", DataType::Utf8, true),
                Field::new("subclade", DataType::Utf8, true),
            ]);

            arrays.extend(vec![
                Arc::new(StringArray::from(clade_vec)) as ArrayRef,
                Arc::new(StringArray::from(short_clade_vec)) as ArrayRef,
                Arc::new(StringArray::from(subclade_vec)) as ArrayRef,
            ]);
        }
        _ => {
            let clade_vec = extract_field(summary_data, |i| i.nextclade_field_1.clone());
            let g_clade_vec = extract_field(summary_data, |i| i.nextclade_field_2.clone());

            fields.extend([
                Field::new("clade", DataType::Utf8, true),
                Field::new("g_clade", DataType::Utf8, true),
            ]);

            arrays.extend(vec![
                Arc::new(StringArray::from(clade_vec)) as ArrayRef,
                Arc::new(StringArray::from(g_clade_vec)) as ArrayRef,
            ]);
        }
    }

    // Write parquet
    let schema = Arc::new(Schema::new(fields));
    let record_batch = RecordBatch::try_new(schema.clone(), arrays)?;

    let file = File::create(output_file)?;
    let mut writer = ArrowWriter::try_new(file, schema, None)?;
    writer.write(&record_batch)?;
    writer.close()?;

    println!(" -> PARQUET written to {output_file}");
    Ok(())
}
