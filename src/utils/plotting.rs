use plotly::{
    Plot, Scatter,
    common::{Fill, Line, Mode, Title},
    layout::{Axis, AxisType, Layout, Shape, ShapeLine, ShapeType},
};
use std::{collections::HashMap, error::Error};

use crate::io::data_ingest::CoverageData;

#[allow(clippy::too_many_lines, clippy::double_must_use)]
#[must_use]
pub fn create_sample_coverage_fig<S: ::std::hash::BuildHasher>(
    sample: &str,
    data: &[CoverageData],
    segments: &[String],
    segcolor: &HashMap<String, String, S>,
    cov_linear_y: bool,
) -> Result<Plot, Box<dyn Error>> {
    let mut plot = Plot::new();

    // Filter data for the given sample
    let sample_data: Vec<&CoverageData> = data
        .iter()
        .filter(|d| d.sample_id.as_deref() == Some(sample))
        .collect();

    if sample_data.is_empty() {
        return Ok(plot);
    }

    // Determine the maximum coverage depth
    let max_coverage_depth = sample_data
        .iter()
        .map(|d| d.coverage_depth)
        .max()
        .unwrap_or(0);

    // ORF positions for different segments
    let orf_positions = if segments.contains(&"SARS-CoV-2".to_string()) {
        Some(vec![
            ("orf1ab", (266, 21556)),
            ("S", (21563, 25385)),
            ("ORF3a", (25393, 26221)),
            ("E", (26245, 26473)),
            ("M", (26523, 27192)),
            ("ORF6", (27202, 27388)),
            ("ORF7a", (27394, 27759)),
            ("ORF7b", (27756, 27887)),
            ("ORF8", (27894, 28260)),
            ("N", (28274, 29534)),
            ("ORF10", (29558, 29675)),
            ("ORF9b", (28284, 28577)),
        ])
    } else {
        None
    };

    // Add ORF boxes to the plot
    if let Some(orf_positions) = orf_positions {
        let oy = f64::from(max_coverage_depth) / 10.0;
        let _ya = if cov_linear_y {
            0.0 - (f64::from(max_coverage_depth) / 20.0)
        } else {
            0.9
        };

        for (orf, (start, end)) in orf_positions {
            let x = vec![start, end, end, start, start];
            let y = vec![oy, oy, 0.0, 0.0, oy];
            let color = "rgba(0, 128, 0, 0.5)";

            let trace = Scatter::new(x, y)
                .mode(Mode::Lines)
                .fill(Fill::ToSelf)
                .line(Line::new().color(color))
                .name(orf);

            plot.add_trace(trace);
        }
    }

    // Add coverage data for each segment
    for segment in segments {
        let segment_data: Vec<&CoverageData> = sample_data
            .iter()
            .copied()
            .filter(|d| &d.reference_name == segment)
            .collect();

        if !segment_data.is_empty() {
            let x: Vec<i32> = segment_data.iter().map(|d| d.position).collect();
            let y: Vec<i32> = segment_data.iter().map(|d| d.coverage_depth).collect();
            let color = segcolor
                .get(segment)
                .cloned()
                .unwrap_or_else(|| "blue".to_string());

            let trace = Scatter::new(x, y)
                .mode(Mode::Lines)
                .line(Line::new().color(color.clone()))
                .name(segment);
            plot.add_trace(trace);
        }
    }

    // Add a horizontal line for median coverage
    let median_coverage = 100;
    let x0 = 0;
    let x1 = sample_data.iter().map(|d| d.position).max().unwrap_or(0);
    let y0 = median_coverage;
    let y1 = median_coverage;

    let shape = Shape::new()
        .shape_type(ShapeType::Line)
        .x0(x0)
        .x1(x1)
        .y0(y0)
        .y1(y1)
        .line(
            ShapeLine::new()
                .color("black")
                .dash(plotly::common::DashType::Dash)
                .width(2.0),
        );

    // Y-axis scaling
    let yaxis_type = if cov_linear_y {
        AxisType::Linear
    } else {
        AxisType::Log
    };

    let ymax = if cov_linear_y {
        f64::from(max_coverage_depth)
    } else {
        f64::from(max_coverage_depth).powf(1.0 / 10.0)
    };

    // Set full layout
    plot.set_layout(
        Layout::new()
            .title(Title::with_text(sample.to_string()))
            .height(600)
            .x_axis(Axis::new().title(Title::with_text("Position")))
            .y_axis(
                Axis::new()
                    .title(Title::with_text("Coverage_Depth"))
                    .type_(yaxis_type)
                    .range(vec![0.0, ymax]),
            )
            .shapes(vec![shape]),
    );

    Ok(plot)
}

#[allow(clippy::implicit_hasher, clippy::needless_pass_by_value)]
pub fn create_coverage_plot(
    data: &[CoverageData],
    segments: Vec<String>,
    segcolor: &HashMap<String, String>,
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    let samples: Vec<String> = data
        .iter()
        .filter_map(|d| d.sample_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    println!("Building coverage plots for {} samples", samples.len());

    for sample in samples {
        let coverage_fig = create_sample_coverage_fig(&sample, data, &segments, segcolor, true)?;
        let file_name = format!("{output_file}/coveragefig_{sample}_linear.json");
        let json_output = serde_json::to_string_pretty(&coverage_fig)?;
        std::fs::write(&file_name, json_output)?;
        println!("  -> saved {file_name}");
    }

    println!(" --> All coverage JSONs saved");
    Ok(())
}
