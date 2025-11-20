use crate::io::data_ingest::CoverageData;
use std::collections::HashMap;

/// Transformed struct for the output
#[derive(Debug, Clone)]
pub struct TransformedData {
    pub sample_id: Option<String>,
    pub segment: String,
    pub coverage_depth: i32,
}

/// Helper function to calculate the median of a list of integers
fn calculate_median(values: &[i32]) -> i32 {
    let mut sorted_values = values.to_vec();
    sorted_values.sort_unstable();
    let len = sorted_values.len();
    if len % 2 == 0 {
        (sorted_values[len / 2 - 1] + sorted_values[len / 2]) / 2
    } else {
        sorted_values[len / 2]
    }
}

pub fn coverage_to_heatap(
    coverage_data: &[CoverageData],
    virus: &str,
    //output_file: &str,
) -> Vec<TransformedData> {
    let filtered_data: Vec<&CoverageData>;

    // Filter for SC2-Spike region if virus is "sc2-spike"
    if virus.to_lowercase() == "sc2-spike" {
        filtered_data = coverage_data
            .iter()
            .filter(|data| (21563..=25384).contains(&data.position))
            .collect();
    } else {
        filtered_data = coverage_data.iter().collect();
    }

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
            segment,
            coverage_depth,
        });
    }
    println!("{:?}", transformed_data);

    transformed_data
}
