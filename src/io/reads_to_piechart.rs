use super::data_ingest::ReadsData;
use crate::constants::theme;
use serde_json::json;

/// Creates a barcode distribution figure - writes it to a file and returns the JSON object.
#[must_use]
pub fn create_barcode_distribution_figure(
    summaries: &[ReadsData],
    output_path: &str,
    write_file: bool,
) -> serde_json::Value {
    println!("Building barcode distribution stacked bar figure as JSON");

    // Prepare (sample, reads) pairs, ordered left-to-right by descending reads.
    let mut samples_reads: Vec<(String, i32)> = summaries
        .iter()
        .filter(|s| s.record == "1-initial")
        .map(|s| {
            (
                s.sample_id.clone().unwrap_or_else(|| "Unknown".to_string()),
                s.reads,
            )
        })
        .collect();
    samples_reads.sort_by(|a, b| b.1.cmp(&a.1));

    // Single blue scale across all bars: darkest CDC navy for the largest read
    // count, fading to a pale CDC blue for the smallest, partitioned by the
    // number of samples.
    let colors = blue_scale(samples_reads.len());

    let total_reads: f64 = samples_reads.iter().map(|(_, r)| *r as f64).sum();

    // Single horizontal 100% stacked bar: one trace per sample so each
    // contribution is its own segment. Bar labels, hover and legend are off;
    // sample/read/percent detail is placed as annotations above each segment.
    let category = "Barcode Distribution";
    let mut data = Vec::with_capacity(samples_reads.len());
    let mut annotations = Vec::with_capacity(samples_reads.len());
    let mut cumulative_percent = 0.0_f64;
    let mut max_text_chars = 0_usize;

    for (i, (sample_label, read_count)) in samples_reads.iter().enumerate() {
        let read_count = *read_count;
        let percent = if total_reads > 0.0 {
            (read_count as f64 / total_reads) * 100.0
        } else {
            0.0
        };

        data.push(json!({
            "type": "bar",
            "orientation": "h",
            "name": sample_label,
            "x": [read_count],
            "y": [category],
            "hoverinfo": "skip",
            "showlegend": false,
            "marker": { "color": colors[i].clone() },
        }));

        let center = cumulative_percent + percent / 2.0;

        // read count in thousands with two decimals
        let reads_k = read_count as f64 / 1000.0;
        let text = format!("{sample_label} {reads_k:.2}K ({percent:.1}%)");
        max_text_chars = max_text_chars.max(text.chars().count());

        annotations.push(json!({
            "x": center,
            "xref": "x",
            "y": 1.0,
            "yref": "paper",
            "yanchor": "bottom",
            "textangle": -80,
            "text": text,
            "showarrow": false,
            "font": { "family": theme::BODY_FONT, "color": theme::CHARCOAL, "size": 12 },
        }));

        cumulative_percent += percent;
    }

    // Headroom must clear the tallest rotated label: text projected vertically
    // (~sin(80deg) of its pixel length).
    let char_px = 7.0_f64; // approx glyph advance at font size 12
    let label_px = (max_text_chars as f64 * char_px * 0.985).round();
    let top_margin = (30.0 + label_px).round() as i64;
    let height = top_margin + 160;

    let mut layout = theme::layout_json();
    layout["barmode"] = json!("stack");
    layout["barnorm"] = json!("percent");
    layout["showlegend"] = json!(false);
    layout["annotations"] = json!(annotations);
    layout["margin"] = json!({ "t": top_margin, "l": 40, "r": 40, "b": 40 });
    layout["height"] = json!(height);
    layout["xaxis"] = json!({
        "title": { "text": "Percent of total reads" },
        "ticksuffix": "%",
        "range": [0, 100],
    });
    layout["yaxis"] = json!({
        "showticklabels": false,
    });

    let plot_json = json!({
        "data": data,
        "layout": layout
    });

    if write_file {
        let file_path = format!("{output_path}barcode_distribution.json");
        std::fs::write(&file_path, plot_json.to_string())
            .expect("Failed to write barcode distribution JSON");

        println!("  -> barcode distribution stacked bar figure saved to {file_path}");
    }

    plot_json
}

/// Build `n` hex colors evenly interpolated between a dark and pale CDC blue.
fn blue_scale(n: usize) -> Vec<String> {
    const DARK: (u8, u8, u8) = (0x03, 0x26, 0x59); // CDC navy
    const LIGHT: (u8, u8, u8) = (0xB8, 0xD4, 0xED); // pale CDC blue

    (0..n)
        .map(|i| {
            let t = if n > 1 {
                i as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let lerp =
                |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
            format!(
                "#{:02X}{:02X}{:02X}",
                lerp(DARK.0, LIGHT.0),
                lerp(DARK.1, LIGHT.1),
                lerp(DARK.2, LIGHT.2)
            )
        })
        .collect()
}
