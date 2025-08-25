use serde::{self, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::processes::prepare_mira_reports::SamplesheetI;
use crate::processes::prepare_mira_reports::SamplesheetO;

use super::data_ingest::{CoverageData, DaisSeqData, ReadsData};

/// Dais Variants Struct
#[derive(Serialize, Deserialize, Debug)]
pub struct DaisVarsData {
    pub sample_id: Option<String>,
    pub reference_id: String,
    pub protein: String,
    pub aa_variant_count: i32,
    pub aa_variants: String,
}

//Melted Reads df
#[derive(Debug)]
pub struct MeltedRecord {
    sample_id: String,
    reference: String,
    reads_mapped: i32,
    total_reads: i32,
    pass_qc: i32,
}

/// Processed Cov Calcs
#[derive(Debug)]
pub struct ProcessedCoverage {
    pub sample: String,
    pub reference: String,
    pub median_coverage: f64,
    pub percent_reference_covered: Option<f64>,
}

/////////////// Traits ///////////////
/// check for sample type and if not there add ""
pub trait HasSampleType {
    fn sample_type(&self) -> String;
}

impl HasSampleType for SamplesheetI {
    fn sample_type(&self) -> String {
        self.sample_type.clone().unwrap_or_else(|| "".to_string())
    }
}

impl HasSampleType for SamplesheetO {
    fn sample_type(&self) -> String {
        self.sample_type.clone().unwrap_or_else(|| "".to_string())
    }
}

/// Check for sample id
pub trait HasSampleId {
    fn sample_id(&self) -> &String;
}

impl HasSampleId for SamplesheetI {
    fn sample_id(&self) -> &String {
        &self.sample_id
    }
}

impl HasSampleId for SamplesheetO {
    fn sample_id(&self) -> &String {
        &self.sample_id
    }
}

impl HasSampleId for ReadsData {
    fn sample_id(&self) -> &String {
        self.sample_id.as_ref().unwrap_or_else(|| {
            static EMPTY_STRING: String = String::new();
            &EMPTY_STRING
        })
    }
}

/// Functions to convert values in a vecxtor of structs to vector
/// Some perform type converions
pub fn extract_field<T, U, F>(data: Vec<T>, extractor: F) -> Vec<U>
where
    F: Fn(&T) -> U,
{
    data.iter().map(extractor).collect()
}

pub fn extract_string_fields_as_float<T, F>(data: Vec<T>, extractor: F) -> Vec<f32>
where
    F: Fn(&T) -> &str,
{
    data.iter()
        .map(|item| {
            let field = extractor(item);
            if field.is_empty() {
                0.0
            } else {
                field.parse::<f32>().unwrap_or(0.0)
            }
        })
        .collect()
}

//Function to filter struct by sample id
//If using this with a Vec of structs then you need to add impl to HasSampleID trait above if not done already
pub fn filter_struct_by_ids<T>(samples: &Vec<T>, ids: Vec<String>) -> Vec<T>
where
    T: Serialize + Clone,
    T: HasSampleId,
{
    samples
        .iter()
        .filter(|sample| {
            if let sample_id = sample.sample_id() {
                ids.contains(sample_id)
            } else {
                false
            }
        })
        .cloned()
        .collect()
}

pub fn extract_string_fields_as_int<T, F>(data: Vec<T>, extractor: F) -> Vec<i32>
where
    F: Fn(&T) -> &str,
{
    data.iter()
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

// Function to append a new string with a comma
pub fn append_with_comma(base: &mut String, new_entry: &str) {
    if !base.is_empty() {
        base.push(',');
        base.push_str(new_entry);
    } else {
        base.push_str(new_entry);
    }
}

pub fn collect_negatives<T>(samples: &Vec<T>) -> Vec<String>
where
    T: Serialize + Clone,
    T: HasSampleType + HasSampleId, // Custom trait to ensure T has a `sample_type` and sample_id field
{
    let negative_keywords = ["- control", "negative", "negative_control", "ntc"];

    samples
        .iter()
        .filter_map(|item| {
            let sample_type = item.sample_type().to_lowercase();
            if negative_keywords
                .iter()
                .any(|keyword| sample_type.contains(keyword))
            {
                Some(item.sample_id().clone())
            } else {
                None
            }
        })
        .collect()
}

// Function to process reference names and generate segments, segset, and segcolor
pub fn return_seg_data(
    reference_names: Vec<String>,
) -> (Vec<String>, Vec<String>, HashMap<String, &'static str>) {
    let mut segments: Vec<String> = reference_names.into_iter().collect();
    segments.sort();
    segments.dedup();

    let color_palette = [
        "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f",
        "#bcbd22", "#17becf",
    ];

    let mut segset: Vec<String> = Vec::new();
    for segment in &segments {
        let parts: Vec<&str> = segment.split('_').collect();
        if parts.len() > 1 {
            segset.push(parts[1].to_string());
        } else {
            segset.push(segment.clone());
        }
    }

    let segset: Vec<String> = segset
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut segcolor: HashMap<String, &str> = HashMap::new();
    for (i, seg) in segset.iter().enumerate() {
        if i < color_palette.len() {
            segcolor.insert(seg.clone(), color_palette[i]);
        }
    }

    (segments, segset, segcolor)
}

//////////////// Functions used to process the variants found in dais outputs ///////////////
// Function to calculate the aa variants - this is specifically for flu right now
pub fn compute_dais_variants(
    ref_seqs_data: &Vec<DaisSeqData>,
    sample_seqs_data: &Vec<DaisSeqData>,
) -> Result<Vec<DaisVarsData>, Box<dyn Error>> {
    let mut dais_vars_data: Vec<DaisVarsData> = Vec::new();

    // Compute aa variants
    for sample_entry in sample_seqs_data {
        for ref_entry in ref_seqs_data {
            if sample_entry.reference == ref_entry.reference
                && sample_entry.protein == ref_entry.protein
            {
                let sample_aa_seq = sample_entry.aa_aln.clone();
                let ref_aa_seq = ref_entry.aa_aln.clone();
                let mut var_aa_count = 0;

                // Use .chars() to iterate over aa
                let mut aa_vars = String::new();
                for (index, (sample_aa, ref_aa)) in
                    sample_aa_seq.chars().zip(ref_aa_seq.chars()).enumerate()
                {
                    if sample_aa != ref_aa {
                        let pos = index + 1;
                        var_aa_count += 1;
                        let variant = format!("{ref_aa}{pos}{sample_aa}");
                        append_with_comma(&mut aa_vars, &variant);
                    }
                }

                let dais_vars_entry = DaisVarsData {
                    sample_id: sample_entry.sample_id.clone(),
                    reference_id: sample_entry.reference.clone(),
                    protein: sample_entry.protein.clone(),
                    aa_variant_count: var_aa_count,
                    aa_variants: aa_vars,
                };
                dais_vars_data.push(dais_vars_entry);
            }
        }
    }

    // Sort by protein, sample_id, and aa_variant_count
    dais_vars_data.sort_by(|a, b| {
        a.protein
            .cmp(&b.protein)
            .then(a.sample_id.cmp(&b.sample_id))
            .then(a.aa_variant_count.cmp(&b.aa_variant_count))
    });

    // Remove duplicates based on sample_id and protein, keeping the first entry
    let mut unique_entries = Vec::new();
    let mut seen = HashSet::new();

    for entry in dais_vars_data {
        let key = (entry.sample_id.clone(), entry.protein.clone());
        if seen.insert(key) {
            unique_entries.push(entry);
        }
    }

    Ok(unique_entries)
}

/// Compute CVV DAIS Variants
pub fn compute_cvv_dais_variants(
    ref_seqs_data: &Vec<DaisSeqData>,
    sample_seqs_data: &Vec<DaisSeqData>,
) -> Result<Vec<DaisVarsData>, Box<dyn Error>> {
    let mut merged_data = merge_sequences(ref_seqs_data, sample_seqs_data)?;

    // Compute AA Variants
    for entry in &mut merged_data {
        entry.insertion = compute_aa_variants(&entry.aa_aln, &entry.aa_seq);
    }

    for entry in &mut merged_data {
        entry.insertions_shift_frame = if entry.insertion.is_empty() {
            "0".to_string()
        } else {
            entry.insertion.split(',').count().to_string()
        };
    }

    // Filter and sort the data - keep first
    merged_data.sort_by(|a, b| {
        a.protein
            .cmp(&b.protein)
            .then(a.sample_id.cmp(&b.sample_id))
            .then(a.insertions_shift_frame.cmp(&b.insertions_shift_frame))
    });

    let mut unique_data = HashMap::new();
    for entry in merged_data {
        let key = (entry.sample_id.clone(), entry.protein.clone());
        unique_data.entry(key).or_insert(entry);
    }

    // Convert DaisSeqData to DaisVarsData and collect into a Vec
    let result: Vec<DaisVarsData> = unique_data
        .into_values()
        .map(|entry| DaisVarsData {
            sample_id: entry.sample_id,
            reference_id: entry.reference.clone(),
            protein: entry.protein.clone(),
            aa_variant_count: entry.insertions_shift_frame.parse::<i32>().unwrap_or(0),
            aa_variants: entry.insertion.clone(),
        })
        .collect();

    Ok(result)
}

/// Merge sequences based on Coordspace and Protein - used by compute_cvv_dais_variants fn
fn merge_sequences(
    ref_seqs_data: &Vec<DaisSeqData>,
    sample_seqs_data: &Vec<DaisSeqData>,
) -> Result<Vec<DaisSeqData>, Box<dyn Error>> {
    let mut merged_data = Vec::new();

    for sample_entry in sample_seqs_data {
        for ref_entry in ref_seqs_data {
            if sample_entry.reference == ref_entry.reference
                && sample_entry.protein == ref_entry.protein
            {
                // Create a new owned DaisSeqData instance - it was this or lifetimes...
                let merged_entry = DaisSeqData {
                    sample_id: sample_entry.sample_id.clone(),
                    ctype: sample_entry.ctype.clone(),
                    reference: sample_entry.reference.clone(),
                    protein: sample_entry.protein.clone(),
                    vh: sample_entry.vh.clone(),
                    aa_seq: ref_entry.aa_seq.clone(), // Use ref_entry's AA sequence
                    aa_aln: sample_entry.aa_aln.clone(),
                    cds_id: sample_entry.cds_id.clone(),
                    insertion: sample_entry.insertion.clone(),
                    insertions_shift_frame: sample_entry.insertions_shift_frame.clone(),
                    cds_sequence: sample_entry.cds_sequence.clone(),
                    aligned_cds_sequence: sample_entry.aligned_cds_sequence.clone(),
                    reference_nt_positions: sample_entry.reference_nt_positions.clone(),
                    sample_nt_positions: sample_entry.sample_nt_positions.clone(),
                };

                merged_data.push(merged_entry); // Push the owned value
            }
        }
    }

    Ok(merged_data)
}

/// Compute AA variants - used by compute_cvv_dais_variants fn
fn compute_aa_variants(aligned_aa_sequence: &str, ref_aa_sequence: &str) -> String {
    let mut aa_vars = String::new();

    for (index, (sample_aa, ref_aa)) in aligned_aa_sequence
        .chars()
        .zip(ref_aa_sequence.chars())
        .enumerate()
    {
        if sample_aa != ref_aa {
            let pos = index + 1;
            let variant = format!("{ref_aa}{pos}{sample_aa}");
            append_with_comma(&mut aa_vars, &variant);
        }
    }

    aa_vars
}
//////////////// Functions used to create irma_summary ///////////////
/// Flip orientation of the reads structs
pub fn melt_reads_data(records: &Vec<ReadsData>) -> Vec<MeltedRecord> {
    let mut result = Vec::new();
    let mut sample_data: HashMap<String, (i32, i32)> = HashMap::new(); // To store total_reads and pass_qc for each sample_id

    for record in records {
        if let Some(sample_id) = &record.sample_id {
            if record.record == "1-initial" {
                sample_data.entry(sample_id.clone()).or_insert((0, 0)).0 = record.reads; // Store total_reads
            } else if record.record == "2-passQC" {
                sample_data.entry(sample_id.clone()).or_insert((0, 0)).1 = record.reads; // Store pass_qc
            }
        }
    }

    for record in records {
        if let Some(sample_id) = &record.sample_id {
            if record.record.starts_with("4-") {
                if let Some(&(total_reads, pass_qc)) = sample_data.get(sample_id) {
                    let reference = record
                        .record
                        .strip_prefix("4-")
                        .unwrap_or(&record.record)
                        .to_string();
                    result.push(MeltedRecord {
                        sample_id: sample_id.to_string(),
                        reference,
                        reads_mapped: record.reads,
                        total_reads,
                        pass_qc,
                    });
                }
            }
        }
    }

    result
}

//Calculate Median - needed in coverage functions below
fn calculate_median(values: &[i32]) -> f64 {
    let mut sorted_values = values.to_vec();
    sorted_values.sort_unstable();
    let len = sorted_values.len();
    if len == 0 {
        return 0.0;
    }
    if len % 2 == 0 {
        (sorted_values[len / 2 - 1] + sorted_values[len / 2]) as f64 / 2.0
    } else {
        sorted_values[len / 2] as f64
    }
}

/// Coverage dataframe calculations
pub fn process_wgs_coverage_data(
    coverage_df: &Vec<CoverageData>,
    ref_lens: &HashMap<String, usize>,
) -> Vec<ProcessedCoverage> {
    // Filter out invalid consensus values
    let filtered_coverage: Vec<_> = coverage_df
        .iter()
        .filter(|row| !["-", "N", "a", "c", "t", "g"].contains(&row.consensus.as_str()))
        .collect();

    let mut cov_ref_lens: HashMap<(String, String), usize> = HashMap::new();
    for row in &filtered_coverage {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        *cov_ref_lens.entry(key).or_insert(0) += 1;
    }

    let cov_ref_lens_processed: Vec<_> = cov_ref_lens
        .into_iter()
        .map(|((sample, reference_name), maplen)| {
            let percent_reference_covered = ref_lens
                .get(&reference_name)
                .map(|&ref_len| (maplen as f64 / ref_len as f64) * 100.0);
            (
                sample,
                reference_name,
                percent_reference_covered.map(|x| (x * 100.0).round() / 100.0),
            )
        })
        .collect();

    // Calculate Median Coverage
    let mut coverage_df_grouped: HashMap<(String, String), Vec<i32>> = HashMap::new();
    for row in &filtered_coverage {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        coverage_df_grouped
            .entry(key)
            .or_insert_with(Vec::new)
            .push(row.coverage_depth);
    }

    let mut coverage_df_processed: HashMap<(String, String), f64> = HashMap::new();
    for (key, depths) in coverage_df_grouped {
        let median_coverage = calculate_median(&depths);
        coverage_df_processed.insert(key, median_coverage);
    }

    // Combine results into ProcessedCoverage
    let mut processed_coverage = Vec::new();
    for (sample, reference, percent_reference_covered) in cov_ref_lens_processed {
        let median_coverage = coverage_df_processed
            .get(&(sample.clone(), reference.clone()))
            .copied()
            .unwrap_or(0.0);

        processed_coverage.push(ProcessedCoverage {
            sample,
            reference,
            median_coverage,
            percent_reference_covered,
        });
    }

    processed_coverage
}

pub fn process_position_coverage_data(
    coverage_df: &Vec<CoverageData>,
    ref_lens: &HashMap<String, usize>,
    position_1: i32,
    position_2: i32,
) -> Vec<ProcessedCoverage> {
    // Filter rows where position is between 21563 and 25384
    let filtered_coverage: Vec<_> = coverage_df
        .into_iter()
        .filter(|row| row.position >= position_1 && row.position <= position_2)
        .collect();

    let filtered_coverage: Vec<_> = filtered_coverage
        .iter()
        .filter(|row| !["-", "N", "a", "c", "t", "g"].contains(&row.consensus.as_str()))
        .collect();

    let mut cov_ref_lens: HashMap<(String, String), usize> = HashMap::new();
    for row in &filtered_coverage {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        *cov_ref_lens.entry(key).or_insert(0) += 1;
    }

    let cov_ref_lens_processed: Vec<_> = cov_ref_lens
        .into_iter()
        .map(|((sample, reference_name), maplen)| {
            let percent_reference_covered = ref_lens
                .get(&reference_name)
                .map(|&ref_len| (maplen as f64 / ref_len as f64) * 100.0);
            (
                sample,
                reference_name,
                percent_reference_covered.map(|x| (x * 100.0).round() / 100.0),
            )
        })
        .collect();

    // Calculate Median Coverage
    let mut coverage_df_grouped: HashMap<(String, String), Vec<i32>> = HashMap::new();
    for row in &filtered_coverage {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        coverage_df_grouped
            .entry(key)
            .or_insert_with(Vec::new)
            .push(row.coverage_depth);
    }

    let mut coverage_df_processed: HashMap<(String, String), f64> = HashMap::new();
    for (key, depths) in coverage_df_grouped {
        let median_coverage = calculate_median(&depths);
        coverage_df_processed.insert(key, median_coverage);
    }

    // Combine results into ProcessedCoverage
    let mut processed_coverage = Vec::new();
    for (sample, reference, percent_reference_covered) in cov_ref_lens_processed {
        let median_coverage = coverage_df_processed
            .get(&(sample.clone(), reference.clone()))
            .copied()
            .unwrap_or(0.0);

        processed_coverage.push(ProcessedCoverage {
            sample,
            reference,
            median_coverage,
            percent_reference_covered,
        });
    }

    processed_coverage
}
