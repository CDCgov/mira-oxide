use crate::utils::data_processing::TransformedData;
use serde_json::json;

/// Function to create a heatmap JSON from coverage data
pub fn coverage_to_heatmap_json(coverage_data: &[TransformedData], virus: &str, output_file: &str) {
    let mut filtered_data = coverage_data.to_vec();

    // If virus is RSV, normalize segment names
    if virus.to_lowercase() == "rsv" {
        for data in &mut filtered_data {
            if data.ref_id.starts_with("RSV_") || data.ref_id.starts_with("RSV") {
                data.ref_id = "RSV".to_string();
            }
        }

        // Sort by Sample, Segment, and Coverage Depth, and keep the first unique Sample
        filtered_data.sort_by(|a, b| {
            (a.sample_id.clone(), a.ref_id.clone(), b.coverage_depth).cmp(&(
                b.sample_id.clone(),
                b.ref_id.clone(),
                a.coverage_depth,
            ))
        });
        filtered_data.dedup_by(|a, b| a.sample_id == b.sample_id);
    }

    // Prepare data for heatmap
    let mut x_values = Vec::new();
    let mut y_values = Vec::new();
    let mut z_values = Vec::new();

    for data in filtered_data {
        x_values.push(data.sample_id.clone().unwrap_or_default());
        y_values.push(data.ref_id.clone());
        z_values.push(data.coverage_depth);
    }

    // Define the color scale
    let colorscale = vec![
        (0.0, "rgb(247,252,240)"),
        (0.125, "rgb(224,243,219)"),
        (0.25, "rgb(204,235,197)"),
        (0.375, "rgb(168,221,181)"),
        (0.5, "rgb(123,204,196)"),
        (0.625, "rgb(78,179,211)"),
        (0.75, "rgb(43,140,190)"),
        (0.875, "rgb(8,88,158)"),
        (1.0, "rgb(0,68,27)"),
    ];

    // Create the heatmap
    let heatmap = json!({
        "type": "heatmap",
        "x": x_values,
        "y": y_values,
        "z": z_values,
        "colorscale": colorscale,
        "hovertemplate": "%{y} = %{z:,.0f}x <extra>%{x}</extra>",
        "zmin": 0,
        "zmid": 100,
        "zmax": 1000
    });

    // Create the layout
    let layout = json!({
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
    });

    // Combine data and layout into the final plot JSON
    let plot_json = json!({
        "data": [heatmap],
        "layout": layout
    });

    // Save JSON to file
    let file_path = format!("{output_file}/heatmap.json");
    std::fs::write(file_path.clone(), plot_json.to_string()).expect("Failed to write heatmap JSON");
    println!("  -> coverage heatmap json saved to {file_path}");
}
