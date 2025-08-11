use serde::{self, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::processes::prepare_mira_reports::SamplesheetI;
use crate::processes::prepare_mira_reports::SamplesheetO;

use super::data_ingest::{DaisSeqData, ReadsData};

/// Dais Variants Struct
#[derive(Serialize, Deserialize, Debug)]
pub struct DaisVarsData {
    pub sample_id: Option<String>,
    pub reference_id: String,
    pub protein: String,
    pub aa_variant_count: i32,
    pub aa_variants: String,
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
