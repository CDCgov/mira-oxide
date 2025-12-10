use super::data_ingest::ReadsData;
use serde_json::json;

/// Creates a barcode distribution figure - writes it to a file and returns the JSON object.
#[must_use]
pub fn create_barcode_distribution_figure(
    summaries: &[ReadsData],
    output_path: &str,
) -> serde_json::Value {
    println!("Building barcode distribution pie figure");

    // Prepare vectors for samples and reads
    let mut samples = Vec::new();
    let mut reads = Vec::new();

    for summary in summaries {
        if summary.record == "1-initial" {
            samples.push(summary.sample_id.clone());
            reads.push(summary.reads);
        }
    }

    // Build the JSON structure
    let pie_data = json!({
        "domain": { "x": [0.0, 1.0], "y": [0.0, 1.0] },
        "hovertemplate": "Sample=%{label}<br>Reads=%{value}<extra></extra>",
        "labels": samples,
        "legendgroup": "",
        "name": "",
        "showlegend": true,
        "values": reads,
        "type": "pie",
        "textinfo": "percent+label",
        "textposition": "inside"
    });

    // Minimal layout
    let plot_json = json!({
        "data": [pie_data],
        "layout": {
            "margin": { "t": 60 }
        }
    });

    // Save to file
    let file_path = format!("{output_path}/barcode_distribution.json");
    std::fs::write(&file_path, plot_json.to_string())
        .expect("Failed to write barcode distribution JSON");

    println!("  -> barcode distribution pie figure saved to {file_path}");

    // Return the JSON object
    plot_json
}
