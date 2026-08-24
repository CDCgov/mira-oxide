#![allow(clippy::cast_precision_loss, clippy::struct_excessive_bools)]
use clap::Parser;
use csv::ReaderBuilder;
use glob::glob;
use plotly::{
    Layout, Plot, Scatter,
    common::{Mode, Title},
    configuration::{ImageButtonFormats, ToImageButtonOptions},
    layout::{Axis, GridPattern, LayoutGrid},
};
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

/// A raw JSON trace, used where the plotly crate's typed builders lack a field
/// we need (here: sankey link `customdata`).
#[derive(Clone, serde::Serialize)]
#[serde(transparent)]
struct RawTrace(serde_json::Value);

impl plotly::Trace for RawTrace {
    fn to_json(&self) -> String {
        serde_json::to_string(&self.0).unwrap_or_default()
    }
}

use crate::constants::theme;

/// Colors for IRMA indels overlaid on coverage subplots.
const INSERTION_COLOR: &str = "#2CA02C"; // green
const DELETION_COLOR: &str = "#9467BD"; // purple
/// Fallback minor-indel frequency thresholds when IRMA run_info.txt is absent.
const INDEL_FREQ_DEFAULT_ILLUMINA: f32 = 0.05;
const INDEL_FREQ_DEFAULT_ONT: f32 = 0.30;

/// Read IRMA run parameters (short name -> value) from `<irma>/logs/run_info.txt`.
fn read_run_info(irma_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(irma_dir.join("logs/run_info.txt")) {
        for line in content.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() >= 3 {
                map.insert(cols[1].to_string(), cols[2].trim().to_string());
            }
        }
    }
    map
}

/// One minor-SNV variant: (position, consensus allele, minority allele, consensus count, minority count, minority frequency).
type VariantRec = (u32, String, String, u32, u32, f32);
/// One insertion: (position, inserted bases, count, total, frequency).
type InsertionRec = (u32, String, u32, u32, f32);
/// One deletion: (position, length, context, count, total, frequency).
type DeletionRec = (u32, u32, String, u32, u32, f32);

/// Load IRMA minor-SNV variants per segment from `tables/*variants.txt`.
fn load_variant_data(
    input_directory: &Path,
) -> Result<HashMap<String, Vec<VariantRec>>, Box<dyn Error>> {
    let mut variants_data: HashMap<String, Vec<VariantRec>> = HashMap::new();
    for variant_path in (glob(&format!(
        "{}/tables/*variants.txt",
        input_directory.display()
    ))?)
    .flatten()
    {
        let file = File::open(&variant_path)?;
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(true)
            .from_reader(file);
        for result in rdr.records() {
            let record = result?;
            if record.len() >= 8 {
                let segment_name = record[0].to_string();
                let position: u32 = record[1].parse()?;
                let consensus_allele: String = record[3].to_string();
                let minority_allele: String = record[4].to_string();
                let consensus_count: u32 = record[5].parse()?;
                let minority_count: u32 = record[6].parse()?;
                let minority_frequency: f32 = record[8].parse()?;
                variants_data.entry(segment_name).or_default().push((
                    position,
                    consensus_allele,
                    minority_allele,
                    consensus_count,
                    minority_count,
                    minority_frequency,
                ));
            }
        }
    }
    Ok(variants_data)
}

/// Load IRMA insertions/deletions per segment. Per-type frequency thresholds are
/// sourced from run_info.txt (MIN_FI/MIN_FD), floored at a platform default so
/// low IRMA settings do not bury the coverage curve in indel noise.
#[allow(clippy::type_complexity)]
fn load_indel_data(
    input_directory: &Path,
) -> Result<
    (
        HashMap<String, Vec<InsertionRec>>,
        HashMap<String, Vec<DeletionRec>>,
    ),
    Box<dyn Error>,
> {
    let run_info = read_run_info(input_directory);
    let platform_default = if input_directory
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("ont"))
    {
        INDEL_FREQ_DEFAULT_ONT
    } else {
        INDEL_FREQ_DEFAULT_ILLUMINA
    };
    let insertion_min_frequency = run_info
        .get("MIN_FI")
        .and_then(|v| v.parse::<f32>().ok())
        .map_or(platform_default, |v| v.max(platform_default));
    let deletion_min_frequency = run_info
        .get("MIN_FD")
        .and_then(|v| v.parse::<f32>().ok())
        .map_or(platform_default, |v| v.max(platform_default));

    let mut insertions_data: HashMap<String, Vec<InsertionRec>> = HashMap::new();
    let mut deletions_data: HashMap<String, Vec<DeletionRec>> = HashMap::new();

    for ins_path in (glob(&format!(
        "{}/tables/*insertions.txt",
        input_directory.display()
    ))?)
    .flatten()
    {
        let file = File::open(&ins_path)?;
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(true)
            .from_reader(file);
        for result in rdr.records() {
            let record = result?;
            if record.len() >= 8 {
                let frequency: f32 = record[7].parse().unwrap_or(0.0);
                if frequency < insertion_min_frequency {
                    continue;
                }
                let segment = record[0].to_string();
                let position: u32 = record[1].parse()?;
                let insert = record[2].to_string();
                let count: u32 = record[5].parse()?;
                let total: u32 = record[6].parse()?;
                insertions_data
                    .entry(segment)
                    .or_default()
                    .push((position, insert, count, total, frequency));
            }
        }
    }

    for del_path in (glob(&format!(
        "{}/tables/*deletions.txt",
        input_directory.display()
    ))?)
    .flatten()
    {
        let file = File::open(&del_path)?;
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(true)
            .from_reader(file);
        for result in rdr.records() {
            let record = result?;
            if record.len() >= 8 {
                let frequency: f32 = record[7].parse().unwrap_or(0.0);
                if frequency < deletion_min_frequency {
                    continue;
                }
                let segment = record[0].to_string();
                let position: u32 = record[1].parse()?;
                let length: u32 = record[2].parse()?;
                let context = record[3].to_string();
                let count: u32 = record[5].parse()?;
                let total: u32 = record[6].parse()?;
                deletions_data
                    .entry(segment)
                    .or_default()
                    .push((position, length, context, count, total, frequency));
            }
        }
    }

    Ok((insertions_data, deletions_data))
}

/// Evenly spaced points along a vertical line at `x` from `y0` to `y1`, so hover
/// fires anywhere along the drawn line rather than only at its two ends.
fn vline(x: u32, y0: u32, y1: u32) -> (Vec<u32>, Vec<u32>) {
    const STEPS: u32 = 40;
    let (lo, hi) = (y0.min(y1), y0.max(y1));
    let span = hi - lo;
    (0..=STEPS).map(|i| (x, lo + span * i / STEPS)).unzip()
}

/// Draw the minor-SNV and indel vertical lines for one segment. Every trace
/// shares `legend_group` (the segment name) so a front end can highlight or
/// toggle a whole segment independently of Plotly's own legend controls.
#[allow(clippy::too_many_arguments)]
fn add_variant_indel_traces(
    plot: &mut Plot,
    segment_name: &str,
    segment_color: &'static str,
    variants_data: &HashMap<String, Vec<VariantRec>>,
    insertions_data: &HashMap<String, Vec<InsertionRec>>,
    deletions_data: &HashMap<String, Vec<DeletionRec>>,
    xaxis: &str,
    yaxis: &str,
    legend_group: &str,
) {
    // Minor-SNV variants: solid minority depth, dashed remainder up to total.
    if let Some(variants) = variants_data.get(segment_name) {
        for (
            position,
            consensus_allele,
            minority_allele,
            consensus_count,
            minority_count,
            minority_frequency,
        ) in variants
        {
            let total = *consensus_count + *minority_count;
            let hover = format!(
                "{}:{}:{} ({:.2}%)<extra></extra>",
                consensus_allele,
                position,
                minority_allele,
                *minority_frequency * 100.0
            );
            let (vx, vy) = vline(*position, 0, *minority_count);
            let minor_line = Scatter::new(vx, vy)
                .mode(Mode::Lines)
                .name(segment_name)
                .legend_group(legend_group)
                .line(plotly::common::Line::new().color(segment_color).width(4.0))
                .hover_template(hover.clone())
                .x_axis(xaxis)
                .y_axis(yaxis)
                .show_legend(false);
            plot.add_trace(minor_line);
            let (tx, ty) = vline(*position, *minority_count, total);
            let total_line = Scatter::new(tx, ty)
                .mode(Mode::Lines)
                .name(segment_name)
                .legend_group(legend_group)
                .line(
                    plotly::common::Line::new()
                        .color(segment_color)
                        .width(4.0)
                        .dash(plotly::common::DashType::Dot),
                )
                .hover_template(hover.clone())
                .x_axis(xaxis)
                .y_axis(yaxis)
                .show_legend(false);
            plot.add_trace(total_line);
        }
    }

    // Insertions: solid green major count, dashed green indel count.
    if let Some(insertions) = insertions_data.get(segment_name) {
        for (position, insert, count, total, frequency) in insertions {
            let major = total.saturating_sub(*count);
            let hover = format!(
                "-:{}:{} ({:.2}%)<extra></extra>",
                position,
                insert,
                *frequency * 100.0
            );
            let (mx, my) = vline(*position, 0, major);
            let major_line = Scatter::new(mx, my)
                .mode(Mode::Lines)
                .name("Insertion")
                .legend_group(legend_group)
                .line(
                    plotly::common::Line::new()
                        .color(INSERTION_COLOR)
                        .width(3.0),
                )
                .hover_template(hover.clone())
                .x_axis(xaxis)
                .y_axis(yaxis)
                .show_legend(false);
            plot.add_trace(major_line);
            let (ix, iy) = vline(*position, major, *total);
            let indel_line = Scatter::new(ix, iy)
                .mode(Mode::Lines)
                .name("Insertion")
                .legend_group(legend_group)
                .line(
                    plotly::common::Line::new()
                        .color(INSERTION_COLOR)
                        .width(3.0)
                        .dash(plotly::common::DashType::Dash),
                )
                .hover_template(hover.clone())
                .x_axis(xaxis)
                .y_axis(yaxis)
                .show_legend(false);
            plot.add_trace(indel_line);
        }
    }

    // Deletions: solid purple major count, dashed purple indel count.
    if let Some(deletions) = deletions_data.get(segment_name) {
        for (position, _length, context, count, total, _frequency) in deletions {
            let major = total.saturating_sub(*count);
            let dashes = "-".repeat(context.matches('-').count());
            let hover = format!("{context}:{position}:{dashes}<extra></extra>");
            let (mx, my) = vline(*position, 0, major);
            let major_line = Scatter::new(mx, my)
                .mode(Mode::Lines)
                .name("Deletion")
                .legend_group(legend_group)
                .line(plotly::common::Line::new().color(DELETION_COLOR).width(3.0))
                .hover_template(hover.clone())
                .x_axis(xaxis)
                .y_axis(yaxis)
                .show_legend(false);
            plot.add_trace(major_line);
            let (ix, iy) = vline(*position, major, *total);
            let indel_line = Scatter::new(ix, iy)
                .mode(Mode::Lines)
                .name("Deletion")
                .legend_group(legend_group)
                .line(
                    plotly::common::Line::new()
                        .color(DELETION_COLOR)
                        .width(3.0)
                        .dash(plotly::common::DashType::Dash),
                )
                .hover_template(hover.clone())
                .x_axis(xaxis)
                .y_axis(yaxis)
                .show_legend(false);
            plot.add_trace(indel_line);
        }
    }
}

// Add this function to generate consistent colors for segment names
#[must_use]
pub fn get_segment_color(segment_name: &str) -> &'static str {
    // Two CDC color families (same segment -> same color):
    //   set1 (red range)       -> HA, NA
    //   set2 (blue/teal range) -> PB2, PB1, PA, NP, MP, NS
    if segment_name.contains("HA") {
        theme::CDC_RED // #CC1B22
    } else if segment_name.contains("NA") {
        "#F0695E" // coral (light red)
    } else if segment_name.contains("PB2") {
        theme::CDC_NAVY // #032659
    } else if segment_name.contains("PB1") {
        theme::CDC_BLUE // #0057B7
    } else if segment_name.contains("PA") {
        theme::CDC_BLUE_1 // #3382CF
    } else if segment_name.contains("NP") {
        "#5796D9" // light blue
    } else if segment_name.contains("MP") {
        theme::CDC_TEAL // #0081A1
    } else if segment_name.contains("NS") {
        "#00B1CE" // light teal
    } else {
        // Other virus types (RSV, SC2) -> CDC colorway by hash.
        let hash = segment_name
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)));
        let cw = theme::colorway();
        cw[hash as usize % cw.len()]
    }
}

#[derive(Debug, Parser)]
#[command(version, about = "Generate plotly plots for IRMA output")]
pub struct PlotterArgs {
    #[arg(short = 'i', long, help = "Required")]
    irma_dir: PathBuf,

    #[arg(
        short = 'c',
        long,
        default_value_t = false,
        help = "Generate one coverage plot with all segments (Default: false)"
    )]
    coverage: bool,

    #[arg(
        short = 's',
        long,
        default_value_t = false,
        help = "Generate segmented coverage subplots, including minor variant annotation (Default: false)"
    )]
    coverage_seg: bool,

    #[arg(
        short = 'r',
        long,
        default_value_t = false,
        help = "Generate read assignment sankey diagram (Default: false)"
    )]
    read_flow: bool,

    #[arg(
        short = 'd',
        long,
        default_value_t = false,
        help = "Output plots immediately to browser (Default: false)"
    )]
    display: bool,

    #[arg(
        short = 't',
        long,
        default_value_t = false,
        help = "Output inline html to stdout (Default: false)"
    )]
    inline_html: bool,

    #[arg(
        short = 'o',
        long,
        help = "Output standalone HTML file path (Optional)"
    )]
    output: Option<PathBuf>,
}

pub fn generate_plot_coverage(input_directory: &Path) -> Result<Plot, Box<dyn Error>> {
    // Create a Plotly plot
    let mut plot = Plot::new();

    // Load variant/indel overlays once, keyed by segment.
    let variants_data = load_variant_data(input_directory)?;
    let (insertions_data, deletions_data) = load_indel_data(input_directory)?;

    // Iterate over all coverage files in the input directory
    for entry in glob(&format!(
        "{}/tables/*coverage.txt",
        input_directory.display()
    ))? {
        match entry {
            Ok(path) => {
                // Open the CSV file
                let file = File::open(&path)?;

                // Create a CSV reader
                let mut rdr = ReaderBuilder::new().delimiter(b'\t').from_reader(file);

                // Vectors to store the data
                let mut x_values = Vec::new();
                let mut y_values = Vec::new();

                // Read the CSV file
                for result in rdr.records() {
                    let record = result?;
                    let x: u32 = record[1].parse()?;
                    // Depth = base-call depth + deletion-spanning reads (true read depth).
                    let depth: u32 = record[2].parse()?;
                    let deletions: u32 = record.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
                    x_values.push(x);
                    y_values.push(depth + deletions);
                }

                // Extract segment name
                let segment_name = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split('-')
                    .next()
                    .unwrap();

                // Get color for this segment
                let segment_color = get_segment_color(segment_name);

                // Create a trace for the current CSV file with consistent color
                let trace = Scatter::new(x_values, y_values)
                    .mode(Mode::Lines)
                    .name(segment_name)
                    .legend_group(segment_name)
                    .line(plotly::common::Line::new().color(segment_color).width(3.0))
                    .hover_template("<b>Position:</b> %{x}<br><b>Depth:</b> %{y}<extra></extra>");

                plot.add_trace(trace);

                // Overlay minor-SNV variants and indels for this segment.
                add_variant_indel_traces(
                    &mut plot,
                    segment_name,
                    segment_color,
                    &variants_data,
                    &insertions_data,
                    &deletions_data,
                    "x",
                    "y",
                    segment_name,
                );
            }
            Err(e) => eprintln!("Error reading file: {e}"),
        }
    }

    // Set the figure title
    let layout = Layout::new()
        .template(theme::cdc_template())
        .title(format!(
            "Coverage | {}",
            input_directory
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .split('-')
                .next()
                .unwrap()
        ))
        .x_axis(Axis::new().title(Title::with_text("Position")))
        .y_axis(Axis::new().title(Title::with_text("Coverage")));
    plot.set_layout(layout);

    // Apply configuration to plot
    plot.set_configuration(
        plotly::Configuration::new()
            .responsive(true)
            .display_logo(false)
            .fill_frame(true)
            .to_image_button_options(
                ToImageButtonOptions::new()
                    .format(ImageButtonFormats::Svg)
                    .filename("coverage_plot"),
            ),
    );

    Ok(plot)
}

#[allow(clippy::type_complexity)]
pub fn generate_plot_coverage_seg(input_directory: &Path) -> Result<Plot, Box<dyn Error>> {
    // Init a Plotly plot
    let mut plot = Plot::new();

    // Track number of files for subplot layout
    let mut file_paths = Vec::new();

    // First, count files and collect paths
    for path in (glob(&format!(
        "{}/tables/*coverage.txt",
        input_directory.display()
    ))?)
    .flatten()
    {
        //file_count += 1;
        file_paths.push(path);
    }

    // Calculate grid dimensions for subplots
    let rows = 4; //((file_count + 2) as f64).sqrt().ceil() as usize;
    let cols = 2; //(file_count + rows - 1) / rows; // Ceiling division

    // Load variant/indel overlays once, keyed by segment.
    let variants_data = load_variant_data(input_directory)?;
    let (insertions_data, deletions_data) = load_indel_data(input_directory)?;

    // Segment title labels, positioned dynamically per subplot.
    let mut annotations = Vec::new();

    // Process each file and create a subplot
    for (idx, path) in file_paths.iter().enumerate() {
        // Extract segment name from file path
        let segment_name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("Unknown")
            .split('-')
            .next()
            .unwrap_or("Unknown")
            .to_string();

        // Get color for this segment
        let segment_color = get_segment_color(&segment_name);

        // Open the CSV file
        let file = File::open(path)?;

        // Create a CSV reader
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(true)
            .from_reader(file);

        // Vectors to store the data
        let mut x_values = Vec::new();
        let mut y_values = Vec::new();

        // Read the CSV file
        for result in rdr.records() {
            let record = result?;
            let x: u32 = record[1].parse()?;
            // Depth = base-call depth + deletion-spanning reads (true read depth).
            let depth: u32 = record[2].parse()?;
            let deletions: u32 = record.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
            x_values.push(x);
            y_values.push(depth + deletions);
        }

        // Pick the emptiest top corner (data coords) for the segment label so
        // it does not sit on top of the coverage curve or variant lines.
        let n = y_values.len();
        let max_y = y_values.iter().copied().max().unwrap_or(1);
        let min_x = x_values.iter().copied().min().unwrap_or(0);
        let max_x = x_values.iter().copied().max().unwrap_or(1);
        let mid = n / 2;
        let left_peak = y_values[..mid].iter().copied().max().unwrap_or(0);
        let right_peak = y_values[mid..].iter().copied().max().unwrap_or(0);
        let (label_x, label_x_anchor, region_peak) = if left_peak <= right_peak {
            (f64::from(min_x), plotly::common::Anchor::Left, left_peak)
        } else {
            (f64::from(max_x), plotly::common::Anchor::Right, right_peak)
        };
        // Vertical anchor also varies per subplot so the label clears the data:
        // low local coverage -> pin to the top; high local coverage -> float
        // just above the local curve.
        let (label_y, label_y_anchor) = if region_peak * 2 <= max_y {
            (f64::from(max_y), plotly::common::Anchor::Top)
        } else {
            (f64::from(region_peak), plotly::common::Anchor::Bottom)
        };

        // Create a trace for the current CSV file with consistent color
        let trace = Scatter::new(x_values, y_values.clone())
            .mode(Mode::Lines)
            .name(&segment_name)
            .legend_group(&segment_name)
            .line(plotly::common::Line::new().color(segment_color).width(3.0))
            .hover_template("<b>Position:</b> %{x}<br><b>Depth:</b> %{y}<extra></extra>")
            .show_legend(false);

        // Calculate row and column for this subplot (1-indexed)
        let row = idx / cols + 1;
        let col = idx % cols + 1;

        let xaxis = if col == 1 && row == 1 {
            "x".to_string()
        } else {
            format!("x{}", col + (row - 1) * cols)
        };

        let yaxis = if col == 1 && row == 1 {
            "y".to_string()
        } else {
            format!("y{}", col + (row - 1) * cols)
        };

        let trace = trace.x_axis(&xaxis).y_axis(&yaxis);

        // Add trace to plot
        plot.add_trace(trace);

        // Segment label anchored in the chosen corner of this subplot.
        let label = segment_name.split('_').nth(1).unwrap_or(&segment_name);
        annotations.push(
            plotly::layout::Annotation::new()
                .text(label)
                .x_ref(&xaxis)
                .y_ref(&yaxis)
                .x(label_x)
                .y(label_y)
                .x_anchor(label_x_anchor)
                .y_anchor(label_y_anchor)
                .font(
                    plotly::common::Font::new()
                        .family(theme::TITLE_FONT)
                        .size(22)
                        .color(segment_color),
                )
                .show_arrow(false),
        );

        // Overlay minor-SNV variants and indels for this segment.
        add_variant_indel_traces(
            &mut plot,
            &segment_name,
            segment_color,
            &variants_data,
            &insertions_data,
            &deletions_data,
            &xaxis,
            &yaxis,
            &segment_name,
        );
    }

    // Configure subplot layout
    // Create a base layout first
    let mut layout = Layout::new()
        .template(theme::cdc_template())
        .grid(
            LayoutGrid::new()
                .rows(rows)
                .columns(cols)
                .pattern(GridPattern::Independent),
        )
        .title(format!(
            "Segment Coverage | {}",
            input_directory
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("Unknown")
        ));

    // Add annotations to layout
    layout = layout.annotations(annotations);

    plot.set_layout(layout);

    // Apply configuration to plot
    plot.set_configuration(
        plotly::Configuration::new()
            .responsive(true)
            .display_logo(false)
            .fill_frame(true)
            .to_image_button_options(
                ToImageButtonOptions::new()
                    .format(ImageButtonFormats::Svg)
                    .filename("coverage_plot"),
            ),
    );

    Ok(plot)
}

// TO DO: fix colors for Sankey diagram, abstract parts of this
#[allow(clippy::too_many_lines)]
pub fn generate_sankey_plot(input_directory: &Path) -> Result<Plot, Box<dyn Error>> {
    // Path to READ_COUNTS.txt
    let read_counts_path = input_directory.join("tables").join("READ_COUNTS.txt");

    // Check if file exists
    if !read_counts_path.exists() {
        return Err(format!(
            "READ_COUNTS.txt not found at {}",
            read_counts_path.display()
        )
        .into());
    }

    // Open and read the file
    let file = File::open(read_counts_path)?;
    let reader = BufReader::new(file);

    // Data structures for Sankey diagram
    let mut node_labels = Vec::new();
    let mut node_colors = Vec::new();
    let mut source_indices = Vec::new();
    let mut target_indices = Vec::new();
    let mut values = Vec::new();
    let mut node_map = HashMap::new();

    // Skip the header line
    let mut lines = reader.lines();
    if let Some(Ok(_)) = lines.next() {
        // Header skipped
    }

    // Process data and build node map first
    let mut records = Vec::new();
    for line in lines.map_while(Result::ok) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let record = parts[0];
            let reads: u32 = parts[1].parse().unwrap_or(0);

            // Skip "NA" values and 0 reads
            if parts[1] != "NA" && reads > 0 {
                records.push((record.to_string(), reads));
            }
        }
    }

    // Add initial nodes
    add_node(
        "Initial Reads",
        &mut node_labels,
        &mut node_map,
        &mut node_colors,
        "#3366CC",
    );
    add_node(
        "Pass QC",
        &mut node_labels,
        &mut node_map,
        &mut node_colors,
        "#109618",
    ); // green
    add_node(
        "Fail QC",
        &mut node_labels,
        &mut node_map,
        &mut node_colors,
        "#990099",
    ); // purple
    add_node(
        "No Match",
        &mut node_labels,
        &mut node_map,
        &mut node_colors,
        "#3B3EAC",
    ); // indigo
    add_node(
        "Alt Match",
        &mut node_labels,
        &mut node_map,
        &mut node_colors,
        "#0099C6",
    ); // cyan
    // Process records to create nodes and links
    let mut _initial_reads = 0;
    let mut pass_qc = 0;
    let mut fail_qc = 0;
    let mut no_match = 0;
    let mut chi_alt_reads = 0;
    let mut primary_match_sum = 0;
    let mut four_segments: Vec<(String, u32)> = Vec::new();

    for (record, reads) in &records {
        match record.as_str() {
            "1-initial" => _initial_reads = *reads,
            "2-failQC" => fail_qc = *reads,
            "2-passQC" => pass_qc = *reads,
            "3-nomatch" => no_match = *reads,
            "3-chimeric" | "3-altmatch" => chi_alt_reads += *reads,
            _ => {
                if let Some(stripped) = record.strip_prefix("4-") {
                    primary_match_sum += *reads;
                    let segment = stripped.to_string();
                    four_segments.push((segment, *reads));
                }
            }
        }
    }

    // Add Primary Match node if needed
    if primary_match_sum > 0 {
        add_node(
            "Primary Match",
            &mut node_labels,
            &mut node_map,
            &mut node_colors,
            "#66AA00", // lime
        );
        // Link from Pass QC to Primary Match
        source_indices.push(node_map["Pass QC"]);
        target_indices.push(node_map["Primary Match"]);
        values.push(primary_match_sum);
    }

    // Now add 4- segment nodes and links from Primary Match
    for (segment, reads) in four_segments {
        let segment_color = get_segment_color(&segment);
        add_node(
            &segment,
            &mut node_labels,
            &mut node_map,
            &mut node_colors,
            segment_color,
        );
        // Link from Primary Match to this segment
        source_indices.push(node_map["Primary Match"]);
        target_indices.push(node_map[&segment]);
        values.push(reads);
    }

    // Now process 5- records as before
    for (record, reads) in &records {
        if let Some(stripped) = record.strip_prefix("5-") {
            let segment = stripped.to_string();
            let segment_color = get_segment_color(&segment);
            add_node(
                &segment,
                &mut node_labels,
                &mut node_map,
                &mut node_colors,
                segment_color,
            );
            // Link from Alt Match to this segment
            source_indices.push(node_map["Alt Match"]);
            target_indices.push(node_map[&segment]);
            values.push(*reads);
        }
    }

    // Link: Initial -> Fail QC
    if fail_qc > 0 {
        source_indices.push(node_map["Initial Reads"]);
        target_indices.push(node_map["Fail QC"]);
        values.push(fail_qc);
    }
    // Link: Initial -> Pass QC
    if pass_qc > 0 {
        source_indices.push(node_map["Initial Reads"]);
        target_indices.push(node_map["Pass QC"]);
        values.push(pass_qc);
    }

    // Link: Pass QC -> alt match
    if chi_alt_reads > 0 {
        source_indices.push(node_map["Pass QC"]);
        target_indices.push(node_map["Alt Match"]);
        values.push(chi_alt_reads);
    }
    // Link: Pass QC -> No Match
    if no_match > 0 {
        source_indices.push(node_map["Pass QC"]);
        target_indices.push(node_map["No Match"]);
        values.push(no_match);
    }

    // Prepare Sankey plot
    let mut plot = Plot::new();

    // Node colors: CDC blues for the read-funnel stages, segment/virus colors
    // (shared with the coverage plots) for the assignment nodes.
    let node_colors: Vec<&'static str> = node_labels
        .iter()
        .map(|label| match label.as_str() {
            "Initial Reads" => "#87B5E3",   // light blue
            "Pass QC" => theme::CDC_BLUE_1, // #3382CF
            "Alt Match" => theme::CDC_TEAL,
            "Primary Match" => theme::CDC_BLUE, // #0057B7
            "Fail QC" | "No Match" => theme::GRAY,
            _ => get_segment_color(label),
        })
        .collect();

    // Let Snap auto-lay out the columns from the link topology: sink nodes
    // (Fail QC, No Match and every segment) align on the same right-hand x,
    // Pass QC sits left of the match nodes, and nodes stay draggable. Height
    // scales with the sink column so tall diagrams still fit.
    let source_set: std::collections::HashSet<usize> = source_indices.iter().copied().collect();
    let sink_count = (0..node_labels.len())
        .filter(|i| !source_set.contains(i))
        .count()
        .max(6);
    let sankey_height = (sink_count * 70).clamp(500, 1400);

    // Outgoing reads per node (also the denominator for a child's "% of parent").
    let mut parent_totals: HashMap<usize, u32> = HashMap::new();
    for (s, v) in source_indices.iter().zip(values.iter()) {
        *parent_totals.entry(*s).or_insert(0) += *v;
    }

    // Incoming reads and parents per node, for the label read count and %.
    let mut node_in: HashMap<usize, u32> = HashMap::new();
    let mut node_parents: HashMap<usize, std::collections::HashSet<usize>> = HashMap::new();
    for ((s, t), v) in source_indices
        .iter()
        .zip(target_indices.iter())
        .zip(values.iter())
    {
        *node_in.entry(*t).or_insert(0) += *v;
        node_parents.entry(*t).or_default().insert(*s);
    }

    // Thousands-separated read count.
    let commafy = |n: u32| -> String {
        let s = n.to_string();
        let len = s.len();
        s.chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if i > 0 && (len - i) % 3 == 0 {
                    vec![',', c]
                } else {
                    vec![c]
                }
            })
            .collect()
    };

    // Node labels show name, read count, and % of parent (root has no parent).
    let node_display: Vec<String> = node_labels
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let incoming = node_in.get(&i).copied().unwrap_or(0);
            let outgoing = parent_totals.get(&i).copied().unwrap_or(0);
            let count = incoming.max(outgoing);
            let denom: u32 = node_parents
                .get(&i)
                .map(|ps| {
                    ps.iter()
                        .map(|s| parent_totals.get(s).copied().unwrap_or(0))
                        .sum()
                })
                .unwrap_or(0);
            if denom > 0 {
                format!(
                    "{name} ({} reads, {:.1}%)",
                    commafy(count),
                    f64::from(incoming) / f64::from(denom) * 100.0
                )
            } else {
                format!("{name} ({} reads)", commafy(count))
            }
        })
        .collect();

    // Manual columns so Fail QC shares Pass QC's x, and No Match shares the
    // Primary/Alt Match x. Segments occupy the rightmost column.
    let col_of = |label: &str| -> usize {
        match label {
            "Initial Reads" => 0,
            "Pass QC" | "Fail QC" => 1,
            "Primary Match" | "Alt Match" | "No Match" => 2,
            _ => 3,
        }
    };
    let max_col = 3usize;
    let cols: Vec<usize> = node_labels.iter().map(|l| col_of(l)).collect();
    let mut col_counts = vec![0usize; max_col + 1];
    for &c in &cols {
        col_counts[c] += 1;
    }
    // Even vertical spread within each column seeds the snap layout ordering.
    let mut col_seen = vec![0usize; max_col + 1];
    let mut node_x = Vec::with_capacity(node_labels.len());
    let mut node_y = Vec::with_capacity(node_labels.len());
    for &c in &cols {
        let x = if c == 0 {
            0.001
        } else if c == max_col {
            0.999
        } else {
            c as f64 / max_col as f64
        };
        node_x.push(x);
        let j = col_seen[c];
        col_seen[c] += 1;
        node_y.push((j as f64 + 1.0) / (col_counts[c] as f64 + 1.0));
    }

    // Built as raw JSON to control the node/link fields directly.
    let sankey_json = serde_json::json!({
        "type": "sankey",
        "arrangement": "snap",
        "node": {
            "label": node_display,
            "color": node_colors,
            "x": node_x,
            "y": node_y,
            "pad": 15,
            "thickness": 20,
            "line": { "color": "black" },
            "hovertemplate": "<b>%{label}</b><extra></extra>"
        },
        "link": {
            "source": source_indices,
            "target": target_indices,
            "value": values,
            "hoverinfo": "skip"
        }
    });

    plot.add_trace(Box::new(RawTrace(sankey_json)));

    // Set layout
    let layout = Layout::new()
        .template(theme::cdc_template())
        .title(format!(
            "Read Assignment | {}",
            input_directory
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("Unknown")
        ))
        .height(sankey_height)
        .auto_size(true);

    plot.set_layout(layout);

    // Apply configuration
    plot.set_configuration(
        plotly::Configuration::new()
            .responsive(true)
            .display_logo(false)
            .fill_frame(true)
            .to_image_button_options(
                ToImageButtonOptions::new()
                    .format(ImageButtonFormats::Svg)
                    .filename(&format!(
                        "{}_read_flow",
                        input_directory
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or("Unknown")
                    )),
            ),
    );

    Ok(plot)
}

// Helper function to add node and maintain the node map
#[allow(clippy::implicit_hasher)]
pub fn add_node(
    name: &str,
    labels: &mut Vec<String>,
    node_map: &mut HashMap<String, usize>,
    colors: &mut Vec<String>,
    color: &str,
) {
    if !node_map.contains_key(name) {
        let idx = labels.len();
        node_map.insert(name.to_string(), idx);
        labels.push(name.to_string());
        colors.push(color.to_string());
    }
}

pub fn plotter_process(args: PlotterArgs) -> Result<(), Box<dyn Error>> {
    // Check for correct number of arguments
    //let args = PlotterArgs::parse();

    // Get the input directory and output file path from the command line arguments
    let input_directory = args.irma_dir;
    let output_html_file = args.output;

    // Generate coverage plot if specified
    if args.coverage {
        let plot = generate_plot_coverage(&input_directory)?;

        // Save the plot as an HTML file if output path is provided
        if let Some(optional_file) = &output_html_file {
            plot.write_html(optional_file);
        }

        // Show the plot if specified
        if args.display {
            plot.show();
        }

        // If inline HTML is requested, print the HTML to stdout
        if args.inline_html {
            println!("{}", plot.to_inline_html(None));
        }
    }

    // Generate segmented coverage subplots if specified
    if args.coverage_seg {
        let plot = generate_plot_coverage_seg(&input_directory)?;

        // Save the plot as an HTML file if output path is provided
        if let Some(optional_file) = &output_html_file {
            // Add "_seg" suffix to the filename to distinguish from regular coverage plot
            let seg_file = optional_file.with_file_name(format!(
                "{}_seg{}",
                optional_file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy(),
                optional_file
                    .extension()
                    .map_or_else(String::new, |ext| format!(".{}", ext.to_string_lossy()))
            ));
            plot.write_html(seg_file);
        }

        // Show the plot if specified
        if args.display {
            plot.show();
        }
        // If inline HTML is requested, print the HTML to stdout
        if args.inline_html {
            println!("{}", plot.to_inline_html(None));
        }
    }

    // Generate read flow sankey diagram if specified
    if args.read_flow {
        let plot = generate_sankey_plot(&input_directory)?;

        // Save the plot as an HTML file if output path is provided
        if let Some(optional_file) = &output_html_file {
            let flow_file = optional_file.with_file_name(format!(
                "{}_read_assignment{}",
                optional_file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy(),
                optional_file
                    .extension()
                    .map_or_else(String::new, |ext| format!(".{}", ext.to_string_lossy()))
            ));
            plot.write_html(flow_file);
        }

        // Show the plot if specified
        if args.display {
            plot.show();
        }
        // If inline HTML is requested, print the HTML to stdout
        if args.inline_html {
            println!("{}", plot.to_inline_html(None));
        }
    }

    Ok(())
}
