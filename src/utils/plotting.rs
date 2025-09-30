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
        vec![
            ("orf1ab".to_string(), (266, 21556)),
            ("S".to_string(), (21563, 25385)),
            ("ORF3a".to_string(), (25393, 26221)),
            ("E".to_string(), (26245, 26473)),
            ("M".to_string(), (26523, 27192)),
            ("ORF6".to_string(), (27202, 27388)),
            ("ORF7a".to_string(), (27394, 27759)),
            ("ORF7b".to_string(), (27756, 27887)),
            ("ORF8".to_string(), (27894, 28260)),
            ("N".to_string(), (28274, 29534)),
            ("ORF10".to_string(), (29558, 29675)),
            ("ORF9b".to_string(), (28284, 28577)),
        ]
    } else if segments.contains(&"RSV_B".to_string()) {
        vec![
            ("NS1".to_string(), (99, 518)),
            ("NS2".to_string(), (626, 1000)),
            ("N".to_string(), (1140, 2315)),
            ("P".to_string(), (2348, 3073)),
            ("M".to_string(), (3263, 4033)),
            ("SH".to_string(), (4303, 4500)),
            ("G".to_string(), (4690, 5589)),
            ("F".to_string(), (5666, 7390)),
            ("M2-1".to_string(), (7618, 8205)),
            ("M2-2".to_string(), (8171, 8443)),
            ("L".to_string(), (8509, 15009)),
        ]
    } else if segments.contains(&"RSV_A".to_string()) {
        vec![
            ("NS1".to_string(), (99, 518)),
            ("NS2".to_string(), (628, 1002)),
            ("N".to_string(), (1141, 2316)),
            ("P".to_string(), (2347, 3072)),
            ("M".to_string(), (3262, 4032)),
            ("SH".to_string(), (4304, 4498)),
            ("G".to_string(), (4689, 5585)),
            ("F".to_string(), (5662, 7386)),
            ("M2-1".to_string(), (7607, 8191)),
            ("M2-2".to_string(), (8160, 8432)),
            ("L".to_string(), (8499, 14996)),
        ]
    } else {
        vec![]
    };

    // Predefined list of colors for ORFs
    let orf_colors = [
        "rgba(255, 0, 0, 0.5)",     // Red
        "rgba(0, 255, 0, 0.5)",     // Green
        "rgba(0, 0, 255, 0.5)",     // Blue
        "rgba(255, 255, 0, 0.5)",   // Yellow
        "rgba(255, 0, 255, 0.5)",   // Magenta
        "rgba(0, 255, 255, 0.5)",   // Cyan
        "rgba(128, 0, 128, 0.5)",   // Purple
        "rgba(128, 128, 0, 0.5)",   // Olive
        "rgba(0, 128, 128, 0.5)",   // Teal
        "rgba(128, 128, 128, 0.5)", // Gray
        "rgba(255, 128, 0, 0.5)",   // Orange
        "rgba(0, 128, 255, 0.5)",   // Light Blue
    ];

    // Add ORF boxes to the plot
    let oy = f64::from(max_coverage_depth) / 10.0;
    let _ya = if cov_linear_y {
        0.0 - (f64::from(max_coverage_depth) / 20.0)
    } else {
        0.9
    };

    for (i, (orf, (start, end))) in orf_positions.iter().enumerate() {
        let x = vec![*start, *end, *end, *start, *start];
        let y = vec![oy, oy, 0.0, 0.0, oy];
        let color = orf_colors
            .get(i % orf_colors.len())
            .unwrap_or(&"rgba(0, 128, 0, 0.5)");

        let trace = Scatter::new(x.clone(), y.clone()) // Clone the data to ensure ownership
            .mode(Mode::Lines)
            .fill(Fill::ToSelf)
            .line(Line::new().color((*color).to_string()))
            .name(orf.clone()); // Clone the ORF name to ensure ownership

        plot.add_trace(trace);
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
                .name(segment.clone());
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
