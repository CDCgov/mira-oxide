use crate::io::data_ingest::ReadsData;
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SampleSankeyJson {
    pub sample_id: String,
    pub json: serde_json::Value,
}

// Placeholder functions for returnStageColors and seg
fn return_stage_colors(_data: &[ReadsData]) -> HashMap<String, String> {
    // Implement logic to return stage colors
    HashMap::new()
}

fn seg(label: &str) -> std::string::String {
    // Implement logic to segment the label
    label.to_string()
}
#[allow(clippy::too_many_lines)]
fn dash_reads_to_sankey(data: &[ReadsData], virus: &str) -> Value {
    // Filter out rows where "Stage" is None or "Stage" is 0 or 5
    let filtered_data: Vec<_> = data
        .iter()
        .filter(|row| {
            row.stage
                .as_ref()
                .is_some_and(|stage| stage != "0" && stage != "5")
        })
        .cloned()
        .collect();

    // Generate stage colors (placeholder function)
    let reccolor = return_stage_colors(&filtered_data);

    // 🔹 Flu segment → color map
    let flu_pattern_colors: Vec<(&str, &str)> = vec![
        ("HA", "#5796D9"),  // Light Blue
        ("NA", "#7DDEEC"),  // Teal
        ("MP", "#B278B2"),  // Purple
        ("NP", "#FABF61"),  // Yellow
        ("NS", "#FF9C63"),  // Orange
        ("PA", "#F0695E"),  // Light Red
        ("PB1", "#0081A1"), // Dark Teal
        ("PB2", "#8F4A8F"), // Dark Purple
    ];

    // Convert to HashMap for easy lookups
    let flu_color_map: std::collections::HashMap<_, _> =
        flu_pattern_colors.iter().copied().collect();

    // Extract labels
    let labels: Vec<String> = filtered_data.iter().map(|row| row.record.clone()).collect();

    // Initialize positions and colors
    let mut x_pos = Vec::new();
    let mut y_pos = Vec::new();
    let mut color = Vec::new();

    for label in &labels {
        match label.chars().next().unwrap_or('0') {
            '1' => {
                x_pos.push(0.05);
                y_pos.push(0.1);
                color.push("#87B5E3".to_string());
            }
            '2' => {
                x_pos.push(0.2);
                y_pos.push(0.1);
                color.push("#3382CF".to_string());
            }
            '3' => {
                x_pos.push(0.35);
                y_pos.push(0.1);
                color.push("#0057B7".to_string());
            }
            _ => {
                // Default: stage 4→N nodes

                // 🔹 If virus == flu, try matching segment prefix
                if virus == "flu" {
                    let mut applied_flu_color = None;

                    // Try each prefix (HA, NA, ...)
                    for (seg_prefix, seg_color) in &flu_color_map {
                        if label.contains(seg_prefix) {
                            applied_flu_color = Some((*seg_color).to_string());
                            break;
                        }
                    }

                    if let Some(seg_color) = applied_flu_color {
                        x_pos.push(0.95);
                        y_pos.push(0.01);
                        color.push(seg_color);
                        continue; // Skip fallback logic
                    }
                }

                // 🔹 Fallback to reccolor or dark blue
                x_pos.push(0.95);
                y_pos.push(0.01);

                let fallback_color = "#032659".to_string();
                let seg_key = seg(label);
                let seg_color = reccolor.get(&seg_key).unwrap_or(&fallback_color);

                color.push(seg_color.clone());
            }
        }
    }

    // Initialize source/target/value for links
    let mut source = Vec::new();
    let mut target = Vec::new();
    let mut value = Vec::new();

    for row in &filtered_data {
        let stage = row.stage.as_ref().and_then(|s| s.parse::<u32>().ok());
        if let Some(stage) = stage {
            match stage {
                4 => {
                    source.push(labels.iter().position(|x| x == "3-match").unwrap());
                    target.push(labels.iter().position(|x| *x == row.record).unwrap());
                    value.push(row.reads);
                }
                3 => {
                    source.push(labels.iter().position(|x| x == "2-passQC").unwrap());
                    target.push(labels.iter().position(|x| *x == row.record).unwrap());
                    value.push(row.reads);
                }
                2 => {
                    source.push(labels.iter().position(|x| x == "1-initial").unwrap());
                    target.push(labels.iter().position(|x| *x == row.record).unwrap());
                    value.push(row.reads);
                }
                _ => {}
            }
        }
    }

    // Create Sankey JSON
    json!({
        "data": [{
            "type": "sankey",
            "arrangement": "snap",
            "node": {
                "pad": 15,
                "thickness": 20,
                "label": labels,
                "x": x_pos,
                "y": y_pos,
                "color": color,
                "hovertemplate": "%{label} %{value} reads <extra></extra>"
            },
            "link": {
                "source": source,
                "target": target,
                "value": value,
                "color": "#DBE8F7".to_string(),
                "hovertemplate": "<extra></extra>"
            }
        }]
    })
}

#[must_use]
pub fn reads_to_sankey_json(
    data: &[ReadsData],
    virus: &str,
    output_file: &str,
) -> Vec<SampleSankeyJson> {
    println!("Building read sankey plot");

    let unique_samples: Vec<_> = data
        .iter()
        .filter_map(|row| row.sample_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut json_vec = Vec::new();

    for sample in unique_samples {
        let sample_data: Vec<_> = data
            .iter()
            .filter(|row| row.sample_id.as_ref() == Some(&sample))
            .cloned()
            .collect();

        let sankeyfig = dash_reads_to_sankey(&sample_data, virus);

        let file_path = format!("{output_file}/readsfig_{sample}.json");
        std::fs::write(file_path.clone(), sankeyfig.to_string()).expect("Unable to write file");
        println!("  -> read sankey plot json saved to {file_path}");

        json_vec.push(SampleSankeyJson {
            sample_id: sample,
            json: sankeyfig,
        });
    }

    json_vec
}
