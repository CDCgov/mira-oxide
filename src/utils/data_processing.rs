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
use zoe::prelude::{Len, Nucleotides};

use crate::processes::prepare_mira_reports::SamplesheetI;
use crate::processes::prepare_mira_reports::SamplesheetO;

use crate::io::data_ingest::{
    CoverageData, DaisSeqData, MinorVariantsData, QCSettings, ReadsData, SeqData,
};

/// vtype struct
#[derive(Serialize, Debug, Clone)]
pub struct ProcessedRecord {
    pub sample_id: Option<String>,
    pub original_ref: String,
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
    pub runid: String,
    pub instrument: String,
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

//Melted Reads vec
#[derive(Debug)]
pub struct MeltedRecord {
    sample_id: String,
    reference: String,
    reads_mapped: i32,
    total_reads: i32,
    pass_qc: i32,
}

/// Processed Cov Calcs
#[derive(Debug, Default)]
pub struct ProcessedCoverage {
    pub sample: String,
    pub reference: String,
    pub median_coverage: i32,
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
    pub percent_reference_coverage: Option<f64>,
    pub median_coverage: Option<i32>,
    pub count_minor_snv_at_or_over_5_pct: Option<i32>,
    pub spike_percent_coverage: Option<f64>,
    pub spike_median_coverage: Option<i32>,
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

/// Nt Sequences Struct
#[derive(Serialize, Deserialize, Debug)]
pub struct NTSequences {
    pub sample_id: String,
    pub sequence: String,
    pub target_ref: Option<String>,
    pub reference: String,
    pub qc_decision: String,
    pub runid: String,
    pub instrument: String,
}

/// Nt Sequences Struct
#[derive(Serialize, Deserialize, Debug)]
pub struct AASequences {
    pub sample_id: String,
    pub sequence: String,
    pub protein: Option<String>,
    pub reference: String,
    pub qc_decision: String,
    pub runid: String,
    pub instrument: String,
}

// Processed Seqs Struct
#[derive(Debug)]
pub struct ProcessedSequences {
    pub passed_seqs: Vec<SeqData>,
    pub failed_seqs: Vec<SeqData>,
}

// Nextclade Seqs Struct
#[derive(Debug)]
pub struct NextcladeSequences {
    pub influenza_a_h3n2_ha: Vec<SeqData>,
    pub influenza_a_h1n1pdm_ha: Vec<SeqData>,
    pub influenza_b_victoria_ha: Vec<SeqData>,
    pub influenza_a_h1n1pdm_na: Vec<SeqData>,
    pub influenza_a_h3n2_na: Vec<SeqData>,
    pub influenza_b_victoria_na: Vec<SeqData>,
    pub rsv_a: Vec<SeqData>,
    pub rsv_b: Vec<SeqData>,
    pub sars_cov_2: Vec<SeqData>,
}

// Transform Cov for Heatmap Struct
#[derive(Debug, Clone)]
pub struct TransformedData {
    pub sample_id: Option<String>,
    pub ref_id: String,
    pub coverage_depth: i32,
}

/////////////// Traits ///////////////
/// check for sample type and if not there add ""
pub trait HasSampleType {
    fn sample_type(&self) -> String;
}

/////////////// Imps ///////////////
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
            let stripped_ref = data.record.strip_prefix("4-").unwrap();
            let (vtype, ref_type, subtype) = read_record2type(&data.record);
            let processed_record = ProcessedRecord {
                sample_id: data.sample_id.clone(),
                original_ref: stripped_ref.to_string(),
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
) -> (Vec<String>, Vec<String>, HashMap<String, String>) {
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

    let mut segcolor: HashMap<String, String> = HashMap::new();
    for (i, seg) in segset.iter().enumerate() {
        if i < color_palette.len() {
            segcolor.insert(seg.clone(), color_palette[i].to_owned());
        }
    }

    (segments, segset, segcolor)
}

//////////////// Functions used to process the variants found in dais outputs ///////////////
// Function to calculate the aa variants - this is specifically for flu right now
pub fn compute_dais_variants(
    ref_seqs_data: &[DaisSeqData],
    sample_seqs_data: &[DaisSeqData],
    runid: &str,
    instrument: &str,
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
                    runid: runid.to_owned(),
                    instrument: instrument.to_owned(),
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
    runid: &str,
    instrument: &str,
    virus: &str,
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

    // Filter dais var data based on virus type
    let filtered_data: Vec<DaisSeqData> = if virus == "sc2-spike" {
        unique_data
            .into_values()
            .filter(|entry| entry.protein == "S")
            .collect()
    } else {
        unique_data.into_values().collect()
    };

    // Convert DaisSeqData to DaisVarsData and collect into a Vec
    let result: Vec<DaisVarsData> = filtered_data
        .into_iter()
        .map(|entry| DaisVarsData {
            sample_id: entry.sample_id,
            ctype: entry.ctype,
            reference_id: entry.reference.clone(),
            protein: entry.protein.clone(),
            aa_variant_count: entry.insertions_shift_frame.parse::<i32>().unwrap_or(0),
            aa_variants: entry.insertion.clone(),
            runid: runid.to_owned(),
            instrument: instrument.to_owned(),
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
pub fn extract_subtype_flu(
    dais_vars: &[DaisVarsData],
    coverage_data: &[ProcessedCoverage],
) -> Result<Vec<Subtype>, Box<dyn Error>> {
    let mut subtype_data: Vec<Subtype> = Vec::new();
    let mut sample_hemagglutinin_map: HashMap<String, String> = HashMap::new();
    let mut sample_neuraminidase_map: HashMap<String, String> = HashMap::new();

    // Map sample_id -> HA percent_reference_covered
    let mut ha_coverage_map: HashMap<String, f64> = HashMap::new();
    for cov in coverage_data {
        if cov.reference.contains("HA") {
            ha_coverage_map.insert(
                cov.sample.clone(),
                cov.percent_reference_covered.unwrap_or(0.0),
            );
        }
    }

    // Collect all sample IDs
    let mut all_sample_ids: HashSet<String> = HashSet::new();
    for entry in dais_vars {
        let hold_sample = entry.sample_id.clone().ok_or("Missing sample_id")?;
        let sample_id = hold_sample[..hold_sample.len() - 2].to_string();
        all_sample_ids.insert(sample_id);
    }

    // HA extraction
    for entry in dais_vars {
        let hold_sample = entry.sample_id.clone().ok_or("Missing sample_id")?;
        let sample_ha = hold_sample[..hold_sample.len() - 2].to_string();

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

    // NA extraction
    for entry in dais_vars {
        let hold_sample = entry.sample_id.clone().ok_or("Missing sample_id")?;
        let sample_na = hold_sample[..hold_sample.len() - 2].to_string();

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

    // Combine HA and NA, apply HA coverage rule
    for sample_id in all_sample_ids {
        let ha = sample_hemagglutinin_map
            .get(&sample_id)
            .cloned()
            .unwrap_or_default();

        let na = sample_neuraminidase_map
            .get(&sample_id)
            .cloned()
            .unwrap_or_default();

        let combined = format!("{ha}{na}");

        let mut subtype = if combined.is_empty() {
            "Undetermined".to_string()
        } else {
            combined
        };

        // Downgrade if HA coverage < 100
        if subtype.as_str() == "BYAM" {
            let ha_coverage = ha_coverage_map.get(&sample_id).copied().unwrap_or(0.0);
            if ha_coverage < 100.0 {
                subtype = "Undetermined".to_string();
            }
        }

        subtype_data.push(Subtype {
            sample_id: Some(sample_id),
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
            subtype: entry.ctype.clone(),
        });
    }

    Ok(subtype_data)
}

pub fn extract_subtype_rsv(dais_vars: &[DaisVarsData]) -> Result<Vec<Subtype>, Box<dyn Error>> {
    let mut subtype_data: Vec<Subtype> = Vec::new();

    for entry in dais_vars {
        subtype_data.push(Subtype {
            sample_id: entry.sample_id.clone(),
            subtype: entry.ctype.clone(),
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

fn calculate_median(values: &[i32]) -> i32 {
    let mut sorted_values = values.to_vec();
    sorted_values.sort_unstable();
    let len = sorted_values.len();
    if len == 0 {
        return 0; // Return 0 for empty input
    }
    if len.is_multiple_of(2) {
        // For even-length arrays, calculate the average of the two middle values
        i32::midpoint(sorted_values[len / 2 - 1], sorted_values[len / 2])
    } else {
        // For odd-length arrays, return the middle value
        sorted_values[len / 2]
    }
}

pub fn process_wgs_coverage_data<S: BuildHasher>(
    coverage_vec: &[CoverageData],
    ref_lens: &HashMap<String, usize, S>,
) -> Result<Vec<ProcessedCoverage>, Box<dyn Error>> {
    // Filter out invalid consensus values
    let filtered_coverage: Vec<_> = coverage_vec
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
    let mut coverage_vec_grouped: HashMap<(String, String), Vec<i32>> = HashMap::new();
    for row in coverage_vec {
        let key = (
            row.sample_id.clone().unwrap_or_default(),
            row.reference_name.clone(),
        );
        coverage_vec_grouped
            .entry(key)
            .or_default()
            .push(row.coverage_depth);
    }

    let mut coverage_vec_processed: HashMap<(String, String), i32> = HashMap::new();
    for (key, depths) in coverage_vec_grouped {
        let median_coverage = calculate_median(&depths);
        coverage_vec_processed.insert(key, median_coverage);
    }

    // Combine results into ProcessedCoverage
    let mut processed_coverage = Vec::new();

    for ((sample, reference), &median_coverage) in &coverage_vec_processed {
        let percent_reference_covered = cov_ref_lens_processed
            .iter()
            .find(|(s, r, _)| s == sample && r == reference)
            .map_or(Some(0.0), |(_, _, percent)| *percent); // Default value if not found

        processed_coverage.push(ProcessedCoverage {
            sample: sample.clone(),
            reference: reference.clone(),
            median_coverage, // Already an i32
            percent_reference_covered,
        });
    }

    Ok(processed_coverage)
}

pub fn process_position_coverage_data(
    coverage_vec: &[CoverageData],
    position_1: i32,
    position_2: i32,
) -> Result<Vec<ProcessedCoverage>, Box<dyn Error>> {
    // Filter rows where position is between position_1 and position_2
    let filtered_coverage: Vec<_> = coverage_vec
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

    // Calculate percent reference covered
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

    let mut med_coverage_vec_processed: HashMap<(String, String), i32> = HashMap::new();
    for (key, depths) in sample_med_cov_grouped {
        let median_coverage = calculate_median(&depths);
        med_coverage_vec_processed.insert(key, median_coverage);
    }

    // Combine results into ProcessedCoverage
    let mut processed_coverage = Vec::new();

    for ((sample, reference), &median_coverage) in &med_coverage_vec_processed {
        let percent_reference_covered = cov_ref_lens_processed
            .iter()
            .find(|(s, r, _)| s == sample && r == reference)
            .map_or(Some(0.0), |(_, _, percent)| *percent); // Default value if not found

        processed_coverage.push(ProcessedCoverage {
            sample: sample.clone(),
            reference: reference.clone(),
            median_coverage, // Already an i32
            percent_reference_covered,
        });
    }

    Ok(processed_coverage)
}

/// Count filtered minor variants for each unique `sample_id` and reference - used in IRMA summary below
#[must_use]
pub fn count_minor_variants(data: &[MinorVariantsData]) -> Vec<VariantCountData> {
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
/// Combine all vec to create IRMA summary
#[allow(clippy::too_many_arguments)]
pub fn create_irma_summary_vec(
    sample_list: &[String],
    reads_count_vec: &[MeltedRecord],
    calc_cov_vec: &[ProcessedCoverage],
    filtered_minor_vars_vec: &[MinorVariantsData],
    subtype_vec: &[Subtype],
    metadata: &Metadata,
    pos_calc_cov_vec: Option<&[ProcessedCoverage]>,
) -> Result<Vec<IRMASummary>, Box<dyn Error>> {
    let mut irma_summary: Vec<IRMASummary> = Vec::new();
    let filtered_minor_vars_count = count_minor_variants(filtered_minor_vars_vec);

    // Populate irma_summary with initial data from reads_count_vec
    for sample in sample_list {
        let mut found_match = false;
        for entry in reads_count_vec {
            if *sample == entry.sample_id {
                found_match = true;
                irma_summary.push(IRMASummary {
                    sample_id: Some(entry.sample_id.clone()),
                    reference: Some(entry.reference.clone()),
                    total_reads: Some(entry.total_reads),
                    pass_qc: Some(entry.pass_qc),
                    reads_mapped: Some(entry.reads_mapped),
                    percent_reference_coverage: None,
                    median_coverage: None,
                    count_minor_snv_at_or_over_5_pct: Some(0),
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
        // If no match was found, push the default IRMASummary entry that will indicate failed
        if !found_match {
            irma_summary.push(IRMASummary {
                sample_id: Some(sample.clone()),
                reference: Some("Undetermined".to_owned()),
                total_reads: Some(0),
                pass_qc: Some(0),
                reads_mapped: Some(0),
                percent_reference_coverage: Some(0.0),
                median_coverage: Some(0),
                count_minor_snv_at_or_over_5_pct: Some(0),
                spike_percent_coverage: None,
                spike_median_coverage: None,
                pass_fail_reason: Some("Fail".to_owned()),
                subtype: Some("Undetermined".to_owned()),
                mira_module: Some(metadata.module.clone()),
                runid: Some(metadata.runid.clone()),
                instrument: Some(metadata.instrument.clone()),
            });
        }
    }

    //Update irma_summary with data from other dataframes
    for sample in &mut irma_summary {
        for entry in calc_cov_vec {
            if sample.sample_id == Some(entry.sample.clone())
                && sample.reference == Some(entry.reference.clone())
            {
                sample.percent_reference_coverage = entry.percent_reference_covered;
                sample.median_coverage = Some(entry.median_coverage);
            }
        }

        if let Some(result) = pos_calc_cov_vec {
            if let Some(entry) = result.iter().find(|entry| {
                sample.sample_id == Some(entry.sample.clone())
                    && sample.reference == Some(entry.reference.clone())
            }) {
                sample.spike_percent_coverage =
                    Some(entry.percent_reference_covered.unwrap_or(0.0));
                sample.spike_median_coverage = Some(entry.median_coverage);
            } else {
                sample.spike_percent_coverage = Some(0.0);
                sample.spike_median_coverage = Some(0);
            }
        }

        for entry in &filtered_minor_vars_count {
            if sample.sample_id == entry.sample_id.clone()
                && sample.reference == Some(entry.reference.clone())
            {
                sample.count_minor_snv_at_or_over_5_pct = Some(entry.minor_variant_count);
            }
        }

        if let Some(entry) = subtype_vec
            .iter()
            .find(|entry| entry.sample_id == sample.sample_id)
        {
            sample.subtype = Some(entry.subtype.clone());
        } else {
            sample.subtype = Some("Undetermined".to_string());
        }
    }

    Ok(irma_summary)
}

/// Helper function to map flu segment to number for mixed sample chcking
fn get_seg_name(flu_type: &str, segment: &str) -> Option<&'static str> {
    match (flu_type, segment) {
        ("A", "2") | ("B", "1") => Some("PB1"),
        ("A", "1") | ("B", "2") => Some("PB2"),
        ("A" | "B", "3") => Some("PA"),
        ("A" | "B", "4") => Some("HA"),
        ("A" | "B", "5") => Some("NP"),
        ("A" | "B", "6") => Some("NA"),
        ("A" | "B", "7") => Some("M"),
        ("A" | "B", "8") => Some("NS"),
        _ => None,
    }
}

/// Combine all qc info and add to IRMA summary
#[allow(clippy::too_many_lines)]
impl IRMASummary {
    pub fn add_pass_fail_qc(
        &mut self,
        dais_vars: &[DaisVarsData],
        seq_vec: &[SeqData],
        qc_values: &QCSettings,
    ) -> Result<Vec<IRMASummary>, Box<dyn Error>> {
        let irma_summary: Vec<IRMASummary> = Vec::new();

        for entry in seq_vec {
            if let (Some((entry_sample, segment)), Some(sample_id), Some(reference)) = (
                entry.name.split_once('_'),
                self.sample_id.as_deref(),
                self.reference.as_deref(),
            ) {
                if entry_sample != sample_id {
                    continue;
                }

                let flu_type = match reference.chars().next() {
                    Some('A') => "A",
                    Some('B') => "B",
                    _ => continue,
                };

                if let Some(seg_name) = get_seg_name(flu_type, segment)
                    && reference.contains(seg_name)
                {
                    let nt_seq1: Nucleotides = entry.sequence.clone().into();
                    let seq_len = nt_seq1.len();
                    let mut mix_base_count = 0;

                    for base in nt_seq1 {
                        if !matches!(base, b'A' | b'T' | b'C' | b'G' | b'N' | b'-') {
                            mix_base_count += 1;
                        }
                    }

                    println!(
                        "{:?}:{:?}:{}",
                        self.sample_id, self.reference, mix_base_count
                    );
                }
            }
        }

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

        if let Some(coverage) = self.percent_reference_coverage
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
            && med_cov < qc_values.med_cov.try_into().unwrap()
        {
            let new_entry = format!("Median coverage < {}", qc_values.med_cov);
            if let Some(ref mut pf_reason) = self.pass_fail_reason {
                append_with_delim(pf_reason, &new_entry, ';');
            } else {
                self.pass_fail_reason = Some(new_entry);
            }
        }

        if let Some(minor_snv) = self.count_minor_snv_at_or_over_5_pct
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

        if let Some(spike_coverage) = self.spike_percent_coverage
            && let Some(perc_ref_spike_covered) = qc_values.perc_ref_spike_covered
            && spike_coverage < f64::from(perc_ref_spike_covered)
        {
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

        if let Some(spike_med_cov) = self.spike_median_coverage
            && let Some(spike_med_covered) = qc_values.med_spike_cov
            && spike_med_cov < spike_med_covered.try_into().unwrap()
        {
            let new_entry = format!("Median coverage of S gene < {}", qc_values.med_cov);
            if let Some(ref mut pf_reason) = self.pass_fail_reason {
                append_with_delim(pf_reason, &new_entry, ';');
            } else {
                self.pass_fail_reason = Some(new_entry);
            }
        }

        if self.pass_fail_reason.is_none() {
            self.pass_fail_reason = Some("Pass".to_string());
        }

        Ok(irma_summary)
    }
}

/// Matching sequences to samples and references for `nt_seq_vec`
pub fn create_nt_seq_vec(
    seq_data: &[SeqData],
    vtype_vec: &[ProcessedRecord],
    irma_summary_vec: &[IRMASummary],
    virus: &str,
    runid: &str,
    instrument: &str,
) -> Result<Vec<NTSequences>, Box<dyn Error>> {
    let mut nt_seq_vec: Vec<NTSequences> = Vec::new();

    if virus == "flu" {
        //Split name and segemnt unmber by last underscore
        for entry in seq_data {
            let parts: Vec<&str> = entry.name.rsplitn(2, '_').collect();
            if parts.len() != 2 {
                continue;
            }

            let segment_number = parts[0];
            let sample_id = parts[1].to_string();

            for sample in vtype_vec {
                if let Some(vtype_sample_id) = &sample.sample_id
                    && sample_id == *vtype_sample_id
                {
                    //A vs B segemnt identificaiton logic
                    let segment = if sample.vtype == "A" {
                        match segment_number {
                            "1" => Some("PB2"),
                            "2" => Some("PB1"),
                            "3" => Some("PA"),
                            "4" => Some("HA"),
                            "5" => Some("NP"),
                            "6" => Some("NA"),
                            "7" => Some("MP"),
                            "8" => Some("NS"),
                            _ => None,
                        }
                    } else if sample.vtype == "B" {
                        match segment_number {
                            "1" => Some("PB1"),
                            "2" => Some("PB2"),
                            "3" => Some("PA"),
                            "4" => Some("HA"),
                            "5" => Some("NP"),
                            "6" => Some("NA"),
                            "7" => Some("MP"),
                            "8" => Some("NS"),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    let mut assigned_segment = String::new();
                    if let Some(segment) = segment {
                        assigned_segment = (*segment).to_string();
                    }

                    for record in irma_summary_vec {
                        if let Some(record_sample_id) = &record.sample_id
                            && sample_id == *record_sample_id
                            && record.reference == Some(sample.original_ref.clone())
                            && assigned_segment == sample.ref_type
                        {
                            nt_seq_vec.push(NTSequences {
                                sample_id: sample_id.clone(),
                                sequence: entry.sequence.clone(),
                                target_ref: Some(assigned_segment.clone()),
                                reference: sample.original_ref.clone(),
                                qc_decision: record
                                    .pass_fail_reason
                                    .clone()
                                    .unwrap_or_else(String::new),
                                runid: runid.to_owned(),
                                instrument: instrument.to_owned(),
                            });
                        }
                    }
                }
            }
        }
    } else {
        for entry in seq_data {
            for record in irma_summary_vec {
                if let Some(record_sample_id) = &record.sample_id
                    && entry.name == *record_sample_id
                {
                    nt_seq_vec.push(NTSequences {
                        sample_id: record_sample_id.clone(),
                        sequence: entry.sequence.clone(),
                        target_ref: None,
                        reference: record.reference.clone().unwrap_or(String::new()),
                        qc_decision: record.pass_fail_reason.clone().unwrap_or_else(String::new),
                        runid: runid.to_owned(),
                        instrument: instrument.to_owned(),
                    });
                }
            }
        }
    }
    Ok(nt_seq_vec)
}

//Take NTSequences and divide them into seqs that pass and seqs that fail
//Pre step for printing the pass/fail amended concensus
pub fn divide_nt_into_pass_fail_vec(
    nt_seq_vec: &[NTSequences],
    platform: &str,
    virus: &str,
) -> Result<ProcessedSequences, Box<dyn Error>> {
    let mut pass_vec: Vec<SeqData> = Vec::new();
    let mut fail_vec: Vec<SeqData> = Vec::new();

    for entry in nt_seq_vec {
        let is_pass = match (platform, virus) {
            ("illumina", "flu") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.reference.contains("HA")
                        && !entry.reference.contains("NA"))
            }
            ("illumina", "sc2-wgs") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.qc_decision.contains("Premature stop codon 'S'"))
            }
            ("illumina", "rsv") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.reference.contains('F')
                        && !entry.reference.contains('G'))
            }
            ("ont", _) => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';'))
            }
            _ => {
                return Err(format!(
                    "Unhandled case for platform '{platform}' and virus '{virus}'"
                )
                .into());
            }
        };

        if is_pass {
            pass_vec.push(SeqData {
                name: format!("{} | {}", entry.sample_id.clone(), entry.reference),
                sequence: entry.sequence.clone(),
            });
        } else {
            fail_vec.push(SeqData {
                name: format!(
                    "{} | {} | {}",
                    entry.sample_id.clone(),
                    entry.reference,
                    entry.qc_decision
                ),
                sequence: entry.sequence.clone(),
            });
        }
    }

    let processed_nt_seqs = ProcessedSequences {
        passed_seqs: pass_vec,
        failed_seqs: fail_vec,
    };

    Ok(processed_nt_seqs)
}

pub fn create_aa_seq_vec(
    aa_data: &[DaisSeqData],
    irma_summary_vec: &[IRMASummary],
    virus: &str,
    runid: &str,
    instrument: &str,
) -> Result<Vec<AASequences>, Box<dyn Error>> {
    let mut aa_seq_vec: Vec<AASequences> = Vec::new();

    if virus == "flu" {
        for entry in aa_data {
            if let Some(sample_id_str) = entry.sample_id.as_ref() {
                let parts: Vec<&str> = sample_id_str.rsplitn(2, '_').collect();
                if parts.len() != 2 {
                    continue;
                }

                let sample_id = parts[1].to_string();

                for sample in irma_summary_vec {
                    if Some(sample_id.clone()) == sample.sample_id
                        && Some(entry.ctype.clone()) == sample.reference
                    {
                        aa_seq_vec.push(AASequences {
                            sample_id: sample_id.clone(),
                            sequence: entry.aa_seq.clone(),
                            protein: Some(entry.protein.clone()),
                            reference: sample.reference.clone().unwrap_or_else(String::new),
                            qc_decision: sample
                                .pass_fail_reason
                                .clone()
                                .unwrap_or_else(String::new),
                            runid: runid.to_owned(),
                            instrument: instrument.to_owned(),
                        });
                    }
                }
            }
        }
    } else {
        for entry in aa_data {
            if let Some(sample_id_str) = entry.sample_id.as_ref() {
                for sample in irma_summary_vec {
                    if Some(sample_id_str.clone()) == sample.sample_id
                        && Some(entry.ctype.clone()) == sample.reference
                    {
                        aa_seq_vec.push(AASequences {
                            sample_id: sample_id_str.clone(),
                            sequence: entry.aa_seq.clone(),
                            protein: Some(entry.protein.clone()),
                            reference: sample.reference.clone().unwrap_or_else(String::new),
                            qc_decision: sample
                                .pass_fail_reason
                                .clone()
                                .unwrap_or_else(String::new),
                            runid: runid.to_owned(),
                            instrument: instrument.to_owned(),
                        });
                    }
                }
            }
        }
    }
    Ok(aa_seq_vec)
}

//Take AASequences and divide them into seqs that pass and seqs that fail
//Pre step for printing the pass/fail amino acid concensus
#[allow(clippy::too_many_lines)]
pub fn divide_aa_into_pass_fail_vec(
    nt_seq_vec: &[AASequences],
    platform: &str,
    virus: &str,
) -> Result<ProcessedSequences, Box<dyn Error>> {
    let mut pass_vec: Vec<SeqData> = Vec::new();
    let mut fail_vec: Vec<SeqData> = Vec::new();

    for entry in nt_seq_vec {
        let is_pass = match (platform, virus) {
            ("illumina", "flu") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.reference.contains("HA")
                        && !entry.reference.contains("NA"))
            }
            ("illumina", "sc2-wgs") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.qc_decision.contains("Premature stop codon 'S'"))
            }
            ("illumina", "rsv") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.reference.contains('F')
                        && !entry.reference.contains('G'))
            }
            ("ont", _) => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';'))
            }
            _ => {
                return Err(format!(
                    "Unhandled case for platform '{platform}' and virus '{virus}'"
                )
                .into());
            }
        };

        if is_pass {
            pass_vec.push(SeqData {
                name: format!(
                    "{} | {}",
                    entry.sample_id.clone(),
                    entry.protein.clone().unwrap_or_else(String::new),
                ),
                sequence: entry.sequence.clone(),
            });
        } else {
            fail_vec.push(SeqData {
                name: format!(
                    "{} | {} | {}",
                    entry.sample_id.clone(),
                    entry.protein.clone().unwrap_or_else(String::new),
                    entry.qc_decision
                ),
                sequence: entry.sequence.clone(),
            });
        }
    }

    let processed_aa_seqs = ProcessedSequences {
        passed_seqs: pass_vec,
        failed_seqs: fail_vec,
    };

    Ok(processed_aa_seqs)
}

// Creating seq vecs for nextclade fastas
pub fn divide_nt_into_nextclade_vec(
    nt_seq_vec: &[NTSequences],
    platform: &str,
    virus: &str,
) -> Result<NextcladeSequences, Box<dyn Error>> {
    let mut nextclade_seqs = NextcladeSequences {
        influenza_a_h3n2_ha: Vec::new(),
        influenza_a_h1n1pdm_ha: Vec::new(),
        influenza_b_victoria_ha: Vec::new(),
        influenza_a_h1n1pdm_na: Vec::new(),
        influenza_a_h3n2_na: Vec::new(),
        influenza_b_victoria_na: Vec::new(),
        rsv_a: Vec::new(),
        rsv_b: Vec::new(),
        sars_cov_2: Vec::new(),
    };

    for entry in nt_seq_vec {
        let is_pass = match (platform, virus) {
            ("illumina", "flu") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.reference.contains("HA")
                        && !entry.reference.contains("NA"))
            }
            ("illumina", "sc2-wgs") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.qc_decision.contains("Premature stop codon 'S'"))
            }
            ("illumina", "rsv") => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';')
                        && !entry.reference.contains('F')
                        && !entry.reference.contains('G'))
            }
            ("ont", _) => {
                entry.qc_decision == "Pass"
                    || (entry.qc_decision.contains("Premature stop codon")
                        && !entry.qc_decision.contains(';'))
            }
            _ => {
                return Err(format!(
                    "Unhandled case for platform '{platform}' and virus '{virus}'"
                )
                .into());
            }
        };

        if !is_pass {
            continue;
        }

        let seq = SeqData {
            name: format!("{} | {}", entry.sample_id, entry.reference),
            sequence: entry.sequence.clone(),
        };

        match entry.reference.as_str() {
            "A_HA_H3" => nextclade_seqs.influenza_a_h3n2_ha.push(seq),
            "A_HA_H1" => nextclade_seqs.influenza_a_h1n1pdm_ha.push(seq),
            "B_HA" => nextclade_seqs.influenza_b_victoria_ha.push(seq),
            "A_NA_N1" => nextclade_seqs.influenza_a_h1n1pdm_na.push(seq),
            "A_NA_N2" => nextclade_seqs.influenza_a_h3n2_na.push(seq),
            "B_NA" => nextclade_seqs.influenza_b_victoria_na.push(seq),
            "RSV_AD" => nextclade_seqs.rsv_a.push(seq),
            "RSV_BD" => nextclade_seqs.rsv_b.push(seq),
            "SARS-CoV-2" => nextclade_seqs.sars_cov_2.push(seq),
            _ => {}
        }
    }

    Ok(nextclade_seqs)
}

///////////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////// Functions for Figures ////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////////

#[must_use]
pub fn transform_coverage_to_heatmap(
    coverage_data: &[CoverageData],
    virus: &str,
) -> Vec<TransformedData> {
    // Filter for SC2-Spike region if virus is "sc2-spike"
    let position_1 = 21563;
    let position_2 = 25384;

    let filtered_data: Vec<&CoverageData> = if virus.to_lowercase() == "sc2-spike" {
        coverage_data
            .iter()
            .filter(|row| row.position > position_1 && row.position < position_2)
            .collect()
    } else {
        coverage_data.iter().collect()
    };

    // Group by sample_id and reference_name, and calculate median coverage depth
    let mut grouped_data: HashMap<(Option<String>, String), Vec<i32>> = HashMap::new();
    for data in filtered_data {
        let key = (data.sample_id.clone(), data.reference_name.clone());
        grouped_data
            .entry(key)
            .or_default()
            .push(data.coverage_depth);
    }

    let mut median_data: Vec<(Option<String>, String, i32)> = Vec::new();
    for ((sample_id, reference_name), depths) in grouped_data {
        let median_depth = calculate_median(&depths);
        median_data.push((sample_id, reference_name, median_depth));
    }

    // Split Reference_Name into Subtype, Segment, and Group
    let mut transformed_data: Vec<TransformedData> = Vec::new();
    for (sample_id, reference_name, coverage_depth) in median_data {
        let parts: Vec<&str> = reference_name.split('_').collect();
        let segment = if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            reference_name.clone()
        };

        transformed_data.push(TransformedData {
            sample_id,
            ref_id: segment,
            coverage_depth,
        });
    }

    transformed_data
}
