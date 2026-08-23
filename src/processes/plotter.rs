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
                    let y: u32 = record[2].parse()?;
                    x_values.push(x);
                    y_values.push(y);
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
                    .line(plotly::common::Line::new().color(segment_color).width(3.0));

                plot.add_trace(trace);
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

    // Load variant data into a HashMap keyed by segment name
    // TODO: consider a struct with named fields
    let mut variants_data: HashMap<String, Vec<(u32, String, String, u32, u32, f32)>> =
        HashMap::new();

    // Look for variant files with matching prefixes in the directory
    for variant_path in (glob(&format!(
        "{}/tables/*variants.txt",
        input_directory.display()
    ))?)
    .flatten()
    {
        let file = File::open(&variant_path)?;

        // Create a TSV reader
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
            let y: u32 = record[2].parse()?;
            x_values.push(x);
            y_values.push(y);
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
            .line(plotly::common::Line::new().color(segment_color).width(3.0))
            .hover_template("<b>Position:</b> %{x}<br><b>Coverage:</b> %{y}<br>")
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

        // Add variant data as vertical lines if we have data for this segment
        if let Some(variants) = variants_data.get(&segment_name) {
            for &(
                position,
                ref consensus_allele,
                ref minority_allele,
                consensus_count,
                minority_count,
                minority_frequency,
            ) in variants
            {
                let total = consensus_count + minority_count;
                let hover = format!(
                    "<b>Position:</b> {}<br><br><b>Consensus Allele:</b> {}<br><b>Consensus Count:</b> {}<br><br><b>Minority Allele:</b> {}<br><b>Minority Count:</b> {}<br><b>Minority Frequency:</b> {:.2}%<br><br><b>Total:</b> {}<extra></extra>",
                    position,
                    consensus_allele,
                    consensus_count,
                    minority_allele,
                    minority_count,
                    minority_frequency * 100.0,
                    total
                );

                // Line 1: minority allele depth, y=0 -> minority_count (solid).
                let minor_line = Scatter::new(vec![position, position], vec![0, minority_count])
                    .mode(Mode::Lines)
                    .name(&segment_name)
                    .line(plotly::common::Line::new().color(segment_color).width(4.0))
                    .hover_template(hover.clone())
                    .x_axis(&xaxis)
                    .y_axis(&yaxis)
                    .show_legend(false);
                plot.add_trace(minor_line);

                // Line 2: remainder up to total, y=minority_count -> total (dashed).
                let total_line =
                    Scatter::new(vec![position, position], vec![minority_count, total])
                        .mode(Mode::Lines)
                        .name(&segment_name)
                        .line(
                            plotly::common::Line::new()
                                .color(segment_color)
                                .width(4.0)
                                .dash(plotly::common::DashType::Dot),
                        )
                        .hover_template(hover.clone())
                        .x_axis(&xaxis)
                        .y_axis(&yaxis)
                        .show_legend(false);
                plot.add_trace(total_line);
            }
        }
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

    // Built as raw JSON to control the node/link fields directly.
    let sankey_json = serde_json::json!({
        "type": "sankey",
        "arrangement": "snap",
        "node": {
            "label": node_display,
            "color": node_colors,
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
