use crate::utils::data_ingest;
use either::Either;
use glob::glob;
use serde::{self, Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use super::data_ingest::DaisSeqData;

/// Dais Variants
#[derive(Serialize, Deserialize, Debug)]
pub struct DaisVarsData {
    pub sample_id: Option<String>,
    pub reference_id: String,
    pub protein: String,
    pub aa_variant_count: i32,
    pub aa_variants: String,
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

// Function to process reference names and generate segments, segset, and segcolor
pub fn return_seg_data(
    reference_names: Vec<String>,
) -> (Vec<String>, Vec<String>, HashMap<String, &'static str>) {
    let mut segments: Vec<String> = reference_names.into_iter().collect();
    segments.sort();
    segments.dedup();

    let color_palette = vec![
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
