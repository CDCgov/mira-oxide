use crate::constants::heatmap_ref::{FLU_SEGMENTS, RSV_GENOME, SC2_GENOME};
use crate::utils::data_processing::TransformedData;
use serde_json::json;

pub fn coverage_to_heatmap_json(
    coverage_data: &[TransformedData],
    sample_list: &Vec<String>,
    virus: &str,
    output_file: &str,
) {
    let filtered_data = normalize_rsv_segments(coverage_data, virus);

    let references = get_references_for_virus(virus);
    let completed_data = complete_data_for_samples(&filtered_data, sample_list, &references);

    let (x_values, y_values, z_values) = prepare_heatmap_axes(&completed_data);

    let colorscale = get_colorscale();

    let heatmap = build_heatmap_json(&x_values, &y_values, &z_values, &colorscale);
    let layout = build_layout_json(&colorscale);

    let plot_json = json!({
        "data": [heatmap],
        "layout": layout
    });

    let file_path = format!("{output_file}/heatmap.json");
    std::fs::write(&file_path, plot_json.to_string()).expect("Failed to write heatmap JSON");
    println!("  -> coverage heatmap json saved to {file_path}");
}

fn normalize_rsv_segments(coverage_data: &[TransformedData], virus: &str) -> Vec<TransformedData> {
    let mut filtered_data = coverage_data.to_vec();

    if virus.to_lowercase() == "rsv" {
        for data in &mut filtered_data {
            if data.ref_id.contains("AD") || data.ref_id.contains("BD") {
                data.ref_id = "RSV".to_string();
            }
        }
        filtered_data.sort_by(|a, b| {
            (a.sample_id.clone(), a.ref_id.clone(), a.coverage_depth).cmp(&(
                b.sample_id.clone(),
                b.ref_id.clone(),
                b.coverage_depth,
            ))
        });
        filtered_data.dedup_by(|a, b| a.sample_id == b.sample_id);
    }
    println!("{filtered_data:?}");
    filtered_data
}

fn get_references_for_virus(virus: &str) -> Vec<String> {
    match virus.to_lowercase().as_str() {
        "flu" => FLU_SEGMENTS.iter().map(ToString::to_string).collect(),
        "sc2-wgs" | "sc2-spike" => vec![SC2_GENOME.to_string()],
        "rsv" => vec![RSV_GENOME.to_string()],
        _ => vec![],
    }
}

fn complete_data_for_samples(
    filtered_data: &[TransformedData],
    sample_list: &[String],
    references: &[String],
) -> Vec<TransformedData> {
    let mut completed_data = Vec::new();

    for sample in sample_list {
        for reference in references {
            if let Some(data) = filtered_data.iter().find(|d| {
                d.sample_id.as_ref().map_or(false, |id| id == sample) && d.ref_id == *reference
            }) {
                completed_data.push(data.clone());
            } else if let Some(_data) = filtered_data
                .iter()
                .find(|d| d.sample_id.as_ref() == Some(sample) && d.ref_id == "Undetermined")
            {
                completed_data.push(TransformedData {
                    sample_id: Some(sample.clone()),
                    ref_id: reference.clone(),
                    coverage_depth: 0,
                });
            } else {
                completed_data.push(TransformedData {
                    sample_id: Some(sample.clone()),
                    ref_id: reference.clone(),
                    coverage_depth: 0,
                });
            }
        }
    }
    completed_data
}

fn prepare_heatmap_axes(
    completed_data: &[TransformedData],
) -> (Vec<String>, Vec<String>, Vec<u32>) {
    let mut x_values = Vec::new();
    let mut y_values = Vec::new();
    let mut z_values = Vec::new();

    for data in completed_data {
        x_values.push(data.sample_id.clone().unwrap_or_default());
        y_values.push(data.ref_id.clone());
        z_values.push(data.coverage_depth.try_into().unwrap());
    }
    (x_values, y_values, z_values)
}

fn get_colorscale() -> Vec<(f64, &'static str)> {
    vec![
        (0.0, "rgb(247,252,240)"),
        (0.125, "rgb(224,243,219)"),
        (0.25, "rgb(204,235,197)"),
        (0.375, "rgb(168,221,181)"),
        (0.5, "rgb(123,204,196)"),
        (0.625, "rgb(78,179,211)"),
        (0.75, "rgb(43,140,190)"),
        (0.875, "rgb(8,88,158)"),
        (1.0, "rgb(0,68,27)"),
    ]
}

fn build_heatmap_json(
    x_values: &[String],
    y_values: &[String],
    z_values: &[u32],
    colorscale: &[(f64, &str)],
) -> serde_json::Value {
    json!({
        "type": "heatmap",
        "x": x_values,
        "y": y_values,
        "z": z_values,
        "colorscale": colorscale,
        "hovertemplate": "%{y} = %{z:,.0f}x <extra>%{x}</extra>",
        "zmin": 0,
        "zmid": 100,
        "zmax": 1000
    })
}

fn build_layout_json(colorscale: &[(f64, &str)]) -> serde_json::Value {
    json!({
        "template": {
            "data": {
                "heatmap": [{
                    "type": "heatmap",
                    "colorscale": colorscale
                }]
            },
            "layout": {
                "paper_bgcolor": "white",
                "plot_bgcolor": "#E5ECF6"
            }
        },
        "legend": {
            "x": 0.4,
            "y": 1.2,
            "orientation": "h"
        },
        "xaxis": {
            "side": "top"
        }
    })
}
