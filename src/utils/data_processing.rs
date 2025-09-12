#![allow(clippy::cast_precision_loss)]
use serde::{self, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::File,
    hash::BuildHasher,
    io::{self, BufRead},
    path::Path,
};

use crate::processes::prepare_mira_reports::SamplesheetI;
use crate::processes::prepare_mira_reports::SamplesheetO;

use super::data_ingest::{
    AllelesData, CoverageData, DaisSeqData, IndelsData, QCSettings, ReadsData, SeqData,
};

/// vtype struct
#[derive(Serialize, Debug, Clone)]
pub struct ProcessedRecord {
    pub sample_id: Option<String>,
    pub vtype: String,
    pub ref_type: String,
    pub subtype: String,
}

/// Dais Variants Struct
#[derive(Serialize, Deserialize, Debug)]
pub struct DaisVarsData {
    pub sample_id: Option<String>,
    pub ctype: String,
    pub reference_id: String,
    pub protein: String,
    pub aa_variant_count: i32,
    pub aa_variants: String,
}

/// Subtype Struct
#[derive(Serialize, Deserialize, Debug)]
pub struct Subtype {
    pub sample_id: Option<String>,
    pub subtype: String,
}

/// Analysis Metadata
#[derive(Debug)]
pub struct Metadata {
    pub module: String,
    pub runid: String,
    pub instrument: String,
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

/// IRMA struct
#[derive(Serialize, Debug, Clone)]
pub struct IRMASummary {
    pub sample_id: Option<String>,
    pub total_reads: Option<i32>,
    pub pass_qc: Option<i32>,
    pub reads_mapped: Option<i32>,
    pub reference: Option<String>,
    pub precent_reference_coverage: Option<f64>,
    pub median_coverage: Option<f64>,
    pub count_minor_snv: Option<i32>,
    pub count_minor_indel: Option<i32>,
    pub spike_percent_coverage: Option<f64>,
    pub spike_median_coverage: Option<f64>,
    pub pass_fail_reason: Option<String>,
    pub subtype: Option<String>,
    pub mira_module: Option<String>,
    pub runid: Option<String>,
    pub instrument: Option<String>,
}

/// Variant Count struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VariantCountData {
    #[serde(rename = "Sample")]
    pub sample_id: Option<String>,
    #[serde(rename = "Reference_Name")]
    pub reference: String,
    pub minor_variant_count: i32,
}

/////////////// Traits ///////////////
/// check for sample type and if not there add ""
pub trait HasSampleType {
    fn sample_type(&self) -> String;
}

impl HasSampleType for SamplesheetI {
    fn sample_type(&self) -> String {
        self.sample_type.clone().unwrap_or_default()
    }
}

impl HasSampleType for SamplesheetO {
    fn sample_type(&self) -> String {
        self.sample_type.clone().unwrap_or_default()
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

/// Functions to convert values in a vector of structs to vector
/// Some perform type converions
pub fn extract_field<V, T, U, F>(data: V, extractor: F) -> Vec<U>
where
    V: AsRef<[T]>,
    F: Fn(&T) -> U,
{
    data.as_ref().iter().map(extractor).collect()
}

pub fn extract_string_fields_as_float<V, T, F>(data: V, extractor: F) -> Vec<f32>
where
    V: AsRef<[T]>,
    F: Fn(&T) -> &str,
{
    data.as_ref()
        .iter()
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
pub fn filter_struct_by_ids<T>(samples: &[T], ids: &[String]) -> Vec<T>
where
    T: Serialize + Clone,
    T: HasSampleId,
{
    samples
        .iter()
        .filter(|sample| ids.contains(sample.sample_id()))
        .cloned()
        .collect()
}

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

// Function to append a new string with a comma
pub fn append_with_delim(base: &mut String, new_entry: &str, delim: char) {
    if base.is_empty() {
        base.push_str(new_entry);
    } else {
        base.push(delim);
        base.push_str(new_entry);
    }
}

pub fn collect_sample_id<T>(samples: &[T]) -> Vec<String>
where
    T: Serialize + Clone,
    T: HasSampleId, // Custom trait to ensure T has a sample_id field
{
    let mut sample_list = Vec::new();

    // Skip the first element (header) and iterate over the rest
    for entry in samples {
        sample_list.push(entry.sample_id().clone());
    }

    sample_list
}

pub fn collect_negatives<T>(samples: &[T]) -> Vec<String>
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

/////////////// Functions for manipulating IRMA data ///////////////
/// Breaking up the records column into three string for the `create_vtype_data` function
fn read_record2type(record: &str) -> (String, String, String) {
    let parts: Vec<&str> = record.split('_').collect();
    if parts.len() >= 2 {
        let vtype = parts[0][2..].to_string();
        let ref_type = parts[1].to_string();
        let subtype = if ref_type == "HA" || ref_type == "NA" {
            (*parts.last().unwrap_or(&"")).to_string()
        } else {
            String::new()
        };
        (vtype, ref_type, subtype)
    } else {
        let fallback = record[2..].to_string();
        (fallback.clone(), fallback.clone(), fallback.clone())
    }
}

/// Converting info for read data into vtype
#[must_use]
pub fn create_vtype_data(reads_data: &Vec<ReadsData>) -> Vec<ProcessedRecord> {
    let mut processed_records = Vec::new();

    for data in reads_data {
        // Filter records where the first character of 'record' is '4'
        if data.record.starts_with('4') {
            let (vtype, ref_type, subtype) = read_record2type(&data.record);
            let processed_record = ProcessedRecord {
                sample_id: data.sample_id.clone(),
                vtype,
                ref_type,
                subtype,
            };
            processed_records.push(processed_record);
        }
    }

    processed_records
}

// Function to process reference names and generate segments, segset, and segcolor
#[must_use]
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
    ref_seqs_data: &[DaisSeqData],
    sample_seqs_data: &[DaisSeqData],
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
                        append_with_delim(&mut aa_vars, &variant, ',');
                    }
                }

                let dais_vars_entry = DaisVarsData {
                    sample_id: sample_entry.sample_id.clone(),
                    ctype: sample_entry.ctype.clone(),
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
    ref_seqs_data: &[DaisSeqData],
    sample_seqs_data: &[DaisSeqData],
) -> Result<Vec<DaisVarsData>, Box<dyn Error>> {
    let mut merged_data = merge_sequences(ref_seqs_data, sample_seqs_data);

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
            ctype: entry.ctype,
            reference_id: entry.reference.clone(),
            protein: entry.protein.clone(),
            aa_variant_count: entry.insertions_shift_frame.parse::<i32>().unwrap_or(0),
            aa_variants: entry.insertion.clone(),
        })
        .collect();

    Ok(result)
}

/// Merge sequences based on Coordspace and Protein - used by `compute_cvv_dais_variants` fn
fn merge_sequences(
    ref_seqs_data: &[DaisSeqData],
    sample_seqs_data: &[DaisSeqData],
) -> std::vec::Vec<DaisSeqData> {
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

    merged_data
}

/// Compute AA variants - used by `compute_cvv_dais_variants` fn
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
            append_with_delim(&mut aa_vars, &variant, ',');
        }
    }

    aa_vars
}

/// Get subtypes for flu
pub fn extract_subtype_flu(dais_vars: &[DaisVarsData]) -> Result<Vec<Subtype>, Box<dyn Error>> {
    let mut subtype_data: Vec<Subtype> = Vec::new();
    let mut sample_hemagglutinin_map: HashMap<String, String> = HashMap::new();
    let mut sample_neuraminidase_map: HashMap<String, String> = HashMap::new();

    for entry in dais_vars {
        let hold_sample = entry.sample_id.clone().ok_or("Missing sample_id")?;
        let sample_ha: String = hold_sample[..hold_sample.len() - 2].to_string();
        let ha = if entry.protein == "HA" {
            match entry.reference_id.as_str() {
                "CALI07" => "H1",
                "ANNARBOR60" => "H2",
                "HK4801" => "H3",
                "VT1203" => "H5",
                "ANHUI01" => "H7",
                "BGD0994" => "H9",
                "BRISBANE60" => "BVIC",
                "PHUKET3073" => "BYAM",
                _ => "",
            }
        } else {
            ""
        };
        if !ha.is_empty() {
            sample_hemagglutinin_map.insert(sample_ha, ha.to_string());
        }
    }

    for entry in dais_vars {
        let hold_sample = entry.sample_id.clone().ok_or("Missing sample_id")?;
        let sample_na: String = hold_sample[..hold_sample.len() - 2].to_string();
        let na = if entry.protein == "NA" {
            match entry.reference_id.as_str() {
                "CALI07" => "N1",
                "HK4801" => "N2",
                "ONTARIO6118" => "N4",
                "RU1526" | "ALASKA4733" => "N5",
                "SICHUAN26221" => "N6",
                "NL219" => "N7",
                "ASTRAKHAN3212" => "N8",
                "ANHUI01" => "N9",
                _ => "",
            }
        } else {
            ""
        };
        if !na.is_empty() {
            sample_neuraminidase_map.insert(sample_na, na.to_string());
        }
    }
    //Combine ha and na from the hashmaps where sample IDs match
    let mut all_samples: HashMap<String, String> = HashMap::new();

    for (sample_id, ha) in &sample_hemagglutinin_map {
        all_samples.insert(sample_id.clone(), ha.clone());
    }

    for (sample_id, na) in &sample_neuraminidase_map {
        if let Some(existing) = all_samples.get_mut(sample_id) {
            *existing = format!("{existing}{na}");
        } else {
            all_samples.insert(sample_id.clone(), na.clone());
        }
    }

    //Process all_samples to determine subtype
    for (sample_id, combined) in all_samples {
        let subtype = if combined.is_empty() {
            "Undetermined".to_string()
        } else {
            combined
        };

        subtype_data.push(Subtype {
            sample_id: Some(sample_id.clone()),
            subtype,
        });
    }

    Ok(subtype_data)
}

pub fn extract_subtype_sc2(dais_vars: &[DaisVarsData]) -> Result<Vec<Subtype>, Box<dyn Error>> {
    let mut subtype_data: Vec<Subtype> = Vec::new();

    for entry in dais_vars {
        subtype_data.push(Subtype {
            sample_id: entry.sample_id.clone(),
            subtype: entry.reference_id.clone(),
        });
    }

    Ok(subtype_data)
}

//////////////// Functions used to create irma_summary ///////////////
/// Flip orientation of the reads structs
#[must_use]
pub fn melt_reads_data(records: &[ReadsData]) -> Vec<MeltedRecord> {
    let mut result = Vec::new();
    let mut sample_data: HashMap<String, (i32, i32)> = HashMap::new();

    for record in records {
        if let Some(sample_id) = &record.sample_id {
            if record.record == "1-initial" {
                sample_data.entry(sample_id.clone()).or_insert((0, 0)).0 = record.reads;
            } else if record.record == "2-passQC" {
                sample_data.entry(sample_id.clone()).or_insert((0, 0)).1 = record.reads;
            }
        }
    }

    for record in records {
        if let Some(sample_id) = &record.sample_id
            && record.record.starts_with("4-")
            && let Some(&(total_reads, pass_qc)) = sample_data.get(sample_id)
        {
            let reference = record
                .record
                .strip_prefix("4-")
                .unwrap_or(&record.record)
                .to_string();
            result.push(MeltedRecord {
                sample_id: sample_id.clone(),
                reference,
                reads_mapped: record.reads,
                total_reads,
                pass_qc,
            });
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
    if len.is_multiple_of(2) {
        f64::from(sorted_values[len / 2 - 1] + sorted_values[len / 2]) / 2.0
    } else {
        f64::from(sorted_values[len / 2])
    }
}

/// Coverage dataframe calculations
pub fn process_wgs_coverage_data<S: BuildHasher>(
    coverage_df: &[CoverageData],
    ref_lens: &HashMap<String, usize, S>,
) -> Result<Vec<ProcessedCoverage>, Box<dyn Error>> {
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
    for row in coverage_df {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        coverage_df_grouped
            .entry(key)
            .or_default()
            .push(row.coverage_depth);
    }

    let mut coverage_df_processed: HashMap<(String, String), f64> = HashMap::new();
    for (key, depths) in coverage_df_grouped {
        let median_coverage = calculate_median(&depths);
        coverage_df_processed.insert(key, median_coverage);
    }

    // Combine results into ProcessedCoverage
    let mut processed_coverage = Vec::new();

    for ((sample, reference), &median_coverage) in &coverage_df_processed {
        let percent_reference_covered = cov_ref_lens_processed
            .iter()
            .find(|(s, r, _)| s == sample && r == reference)
            .map_or(Some(0.0), |(_, _, percent)| *percent); // Default value if not found

        processed_coverage.push(ProcessedCoverage {
            sample: sample.clone(),
            reference: reference.clone(),
            median_coverage,
            percent_reference_covered,
        });
    }

    Ok(processed_coverage)
}

pub fn process_position_coverage_data(
    coverage_df: &[CoverageData],
    position_1: i32,
    position_2: i32,
) -> Result<Vec<ProcessedCoverage>, Box<dyn Error>> {
    // Filter rows where position is between 21563 and 25384
    let filtered_coverage: Vec<_> = coverage_df
        .iter()
        .filter(|row| row.position > position_1 && row.position < position_2)
        .collect();

    let filtered_coverage: Vec<_> = filtered_coverage
        .iter()
        .filter(|row| !["-", "N", "a", "c", "t", "g"].contains(&row.consensus.as_str()))
        .collect();

    let mut cov_sample_lens: HashMap<(String, String), usize> = HashMap::new();
    for row in &filtered_coverage {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        *cov_sample_lens.entry(key).or_insert(0) += 1;
    }

    let ref_len = (position_1 - position_2).abs();

    //Calculate percent ref covered
    let cov_ref_lens_processed: Vec<_> = cov_sample_lens
        .into_iter()
        .map(|((sample, reference_name), maplen)| {
            let percent_reference_covered = (maplen as f64 / f64::from(ref_len)) * 100.0;
            (
                sample,
                reference_name,
                Some((percent_reference_covered * 100.0).round() / 100.0),
            )
        })
        .collect();

    // Calculate median coverage
    let mut sample_med_cov_grouped: HashMap<(String, String), Vec<i32>> = HashMap::new();
    for row in &filtered_coverage {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );

        sample_med_cov_grouped
            .entry(key)
            .or_default()
            .push(row.coverage_depth);
    }

    let mut med_coverage_df_processed: HashMap<(String, String), f64> = HashMap::new();
    for (key, depths) in sample_med_cov_grouped {
        let median_coverage = calculate_median(&depths);
        med_coverage_df_processed.insert(key, median_coverage);
    }

    // Combine results into ProcessedCoverage
    let mut processed_coverage = Vec::new();

    for ((sample, reference), &median_coverage) in &med_coverage_df_processed {
        let percent_reference_covered = cov_ref_lens_processed
            .iter()
            .find(|(s, r, _)| s == sample && r == reference)
            .map_or(Some(0.0), |(_, _, percent)| *percent); // Default value if not found

        println!("{median_coverage:?}");

        processed_coverage.push(ProcessedCoverage {
            sample: sample.clone(),
            reference: reference.clone(),
            median_coverage,
            percent_reference_covered,
        });
    }

    Ok(processed_coverage)
}

/// Count minority alleles for each unique `sample_id` and reference - used in IRMA summary below
#[must_use]
pub fn count_minority_alleles(data: &[AllelesData]) -> Vec<VariantCountData> {
    let mut counts: HashMap<(Option<String>, String), i32> = HashMap::new();

    for entry in data {
        let key = (entry.sample_id.clone(), entry.reference.clone());
        *counts.entry(key).or_insert(0) += 1;
    }

    let mut result = Vec::new();
    for ((sample_id, reference), minor_variant_count) in counts {
        result.push(VariantCountData {
            sample_id,
            reference,
            minor_variant_count,
        });
    }

    result
}

/// Count minority alleles for each unique `sample_id` and reference - used in IRMA summary below
#[must_use]
pub fn count_minority_indels(data: &[IndelsData]) -> Vec<VariantCountData> {
    let mut counts: HashMap<(Option<String>, String), i32> = HashMap::new();

    for entry in data {
        //Alleles were already filtered, but have to filter indels for >= 0.2 freq here.
        if entry.frequency >= 0.2 {
            let key = (entry.sample_id.clone(), entry.reference_name.clone());
            *counts.entry(key).or_insert(0) += 1;
        }
    }

    let mut result = Vec::new();
    for ((sample_id, reference), minor_variant_count) in counts {
        result.push(VariantCountData {
            sample_id,
            reference,
            minor_variant_count,
        });
    }

    result
}

pub fn collect_analysis_metadata(
    work_path: &Path,
    platform: &str,
    virus: &str,
    irma_config: &String,
    input_runid: &str,
) -> Result<Metadata, Box<dyn Error>> {
    let mut descript_dict = HashMap::new();
    let description_path = format!("{}/DESCRIPTION", work_path.display());

    // Open the file for reading
    let file = File::open(&description_path)?;
    let reader = io::BufReader::new(file);

    // Read the file line by line
    for line in reader.lines().map_while(Result::ok) {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 2 {
            descript_dict.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
        }
    }

    // Construct the modulestring
    let version = descript_dict
        .get("Version")
        .ok_or("Version key not found in DESCRIPTION file")?;

    let modulestring = format!("MIRA-NF-v{version} {platform}-{virus} {irma_config}");

    let analysis_metadata = Metadata {
        module: modulestring,
        runid: input_runid.to_owned(),
        instrument: platform.to_owned(),
    };
    Ok(analysis_metadata)
}

/////////////// Final IRMA summary file creation ///////////////
/// Combine all df to create IRMA summary
#[allow(clippy::too_many_arguments)]
pub fn create_irma_summary_df(
    sample_list: &[String],
    reads_count_df: &[MeltedRecord],
    calc_cov_df: &[ProcessedCoverage],
    alleles_df: &[AllelesData],
    indels_df: &[IndelsData],
    subtype_df: &[Subtype],
    metadata: &Metadata,
    pos_calc_cov_df: Option<&[ProcessedCoverage]>,
) -> Result<Vec<IRMASummary>, Box<dyn Error>> {
    let mut irma_summary: Vec<IRMASummary> = Vec::new();
    let allele_count_data = count_minority_alleles(alleles_df);
    let indel_count_data = count_minority_indels(indels_df);

    // Populate irma_summary with initial data from reads_count_df
    for sample in sample_list {
        let mut found_match = false;
        for entry in reads_count_df {
            if *sample == entry.sample_id {
                found_match = true;
                irma_summary.push(IRMASummary {
                    sample_id: Some(entry.sample_id.clone()),
                    reference: Some(entry.reference.clone()),
                    total_reads: Some(entry.total_reads),
                    pass_qc: Some(entry.pass_qc),
                    reads_mapped: Some(entry.reads_mapped),
                    precent_reference_coverage: None,
                    median_coverage: None,
                    count_minor_snv: Some(0),
                    count_minor_indel: Some(0),
                    spike_percent_coverage: None,
                    spike_median_coverage: None,
                    pass_fail_reason: None,
                    subtype: None,
                    mira_module: Some(metadata.module.clone()),
                    runid: Some(metadata.runid.clone()),
                    instrument: Some(metadata.instrument.clone()),
                });
            }
        }
        // If no match was found, push the default IRMASummary entry
        if !found_match {
            irma_summary.push(IRMASummary {
                sample_id: Some(sample.clone()),
                reference: Some("Undetermined".to_owned()),
                total_reads: Some(0),
                pass_qc: Some(0),
                reads_mapped: Some(0),
                precent_reference_coverage: Some(0.0),
                median_coverage: Some(0.0),
                count_minor_snv: Some(0),
                count_minor_indel: Some(0),
                spike_percent_coverage: None,
                spike_median_coverage: None,
                pass_fail_reason: Some("Fail".to_owned()),
                subtype: Some("Undetermined".to_owned()),
                mira_module: None,
                runid: None,
                instrument: None,
            });
        }
    }

    //Update irma_summary with data from other dataframes
    for sample in &mut irma_summary {
        for entry in calc_cov_df {
            if sample.sample_id == Some(entry.sample.clone())
                && sample.reference == Some(entry.reference.clone())
            {
                sample.precent_reference_coverage = entry.percent_reference_covered;
                sample.median_coverage = Some(entry.median_coverage);
            }
        }

        if pos_calc_cov_df.is_some() {
            if let Some(result) = pos_calc_cov_df {
                for entry in result {
                    if sample.sample_id == Some(entry.sample.clone())
                        && sample.reference == Some(entry.reference.clone())
                    {
                        sample.spike_percent_coverage = entry.percent_reference_covered;
                        sample.spike_median_coverage = Some(entry.median_coverage);
                    }
                }
            }
        }

        for entry in &allele_count_data {
            if sample.sample_id == entry.sample_id.clone()
                && sample.reference == Some(entry.reference.clone())
            {
                sample.count_minor_snv = Some(entry.minor_variant_count);
            }
        }

        for entry in &indel_count_data {
            if sample.sample_id == entry.sample_id.clone()
                && sample.reference == Some(entry.reference.clone())
            {
                sample.count_minor_indel = Some(entry.minor_variant_count);
            }
        }

        for entry in subtype_df {
            if sample.sample_id == entry.sample_id.clone() {
                sample.subtype = Some(entry.subtype.clone());
            }
        }
    }

    Ok(irma_summary)
}

/// Combine all df to create IRMA summary
impl IRMASummary {
    pub fn add_pass_fail_qc(
        &mut self,
        dais_vars: &[DaisVarsData],
        _seq_df: &[SeqData],
        qc_values: &QCSettings,
    ) -> Result<Vec<IRMASummary>, Box<dyn Error>> {
        let irma_summary: Vec<IRMASummary> = Vec::new();

        let _premature_stop_codon_df = String::new();
        if !qc_values.allow_stop_codons {
            for entry in dais_vars {
                if self.sample_id == entry.sample_id
                    && self.reference == Some(entry.ctype.clone())
                    && entry.aa_variants.contains('*')
                {
                    self.pass_fail_reason =
                        format!("Premature stop codon '{}'", entry.protein).into();
                }
            }
        }

        if let Some(coverage) = self.precent_reference_coverage
            && coverage < qc_values.perc_ref_covered.into()
        {
            let new_entry = format!(
                "Less than {}% of reference covered",
                qc_values.perc_ref_covered
            );
            if let Some(ref mut pf_reason) = self.pass_fail_reason {
                append_with_delim(pf_reason, &new_entry, ';');
            } else {
                self.pass_fail_reason = Some(new_entry);
            }
        }

        if let Some(med_cov) = self.median_coverage
            && med_cov < qc_values.perc_ref_covered.into()
        {
            let new_entry = format!("Median coverage < {}", qc_values.med_cov);
            if let Some(ref mut pf_reason) = self.pass_fail_reason {
                append_with_delim(pf_reason, &new_entry, ';');
            } else {
                self.pass_fail_reason = Some(new_entry);
            }
        }

        if let Some(minor_snv) = self.count_minor_snv
            && minor_snv > qc_values.minor_vars.try_into().unwrap()
        {
            let new_entry = format!(
                "Count of minor variants at or over 5% > {}",
                qc_values.minor_vars
            );
            if let Some(ref mut pf_reason) = self.pass_fail_reason {
                append_with_delim(pf_reason, &new_entry, ';');
            } else {
                self.pass_fail_reason = Some(new_entry);
            }
        }

        if self.pass_fail_reason.is_none() {
            self.pass_fail_reason = Some("Pass".to_string());
        }

        if let Some(spike_coverage) = self.spike_percent_coverage {
            if let Some(perc_ref_spike_covered) = qc_values.perc_ref_spike_covered {
                if spike_coverage < f64::from(perc_ref_spike_covered) {
                    let new_entry = format!(
                        "Less than {}% of S gene reference covered",
                        qc_values.perc_ref_covered
                    );
                    if let Some(ref mut pf_reason) = self.pass_fail_reason {
                        append_with_delim(pf_reason, &new_entry, ';');
                    } else {
                        self.pass_fail_reason = Some(new_entry);
                    }
                }
            }
        }

        if let Some(spike_med_cov) = self.spike_median_coverage {
            if let Some(spike_med_covered) = qc_values.med_spike_cov {
                if spike_med_cov < f64::from(spike_med_covered) {
                    let new_entry = format!("Median coverage of S gene < {}", qc_values.med_cov);
                    if let Some(ref mut pf_reason) = self.pass_fail_reason {
                        append_with_delim(pf_reason, &new_entry, ';');
                    } else {
                        self.pass_fail_reason = Some(new_entry);
                    }
                }
            }
        }

        if self.pass_fail_reason.is_none() {
            self.pass_fail_reason = Some("Pass".to_string());
        }

        Ok(irma_summary)
    }
}
