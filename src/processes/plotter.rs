use crate::utils::dataframes::coverage_df;
use clap::Parser;
use plotly::common::{Mode, Title};
use plotly::configuration::{ImageButtonFormats, ToImageButtonOptions};
use plotly::layout::{Axis, GridPattern, LayoutGrid};
use plotly::{Layout, Plot, Sankey, Scatter};
use polars::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

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

// Add this function to generate consistent colors for segment names
fn get_segment_color(segment_name: &str) -> &'static str {
    // This ensures the same segment always gets the same color across all plots
    // Check if segment_name contains any of our known segment identifiers
    if segment_name.contains("PB2") {
        "#3366CC" // blue
    } else if segment_name.contains("PB1") {
        "#DC3912" // red
    } else if segment_name.contains("PA") {
        "#FF9900" // orange
    } else if segment_name.contains("HA") {
        "#109618" // green
    } else if segment_name.contains("NP") {
        "#990099" // purple
    } else if segment_name.contains("NA") {
        "#3B3EAC" // indigo
    } else if segment_name.contains("MP") {
        "#0099C6" // cyan
    } else if segment_name.contains("NS") {
        "#DD4477" // pink
    } else {
        // For any other segments, use a hash of the segment name to pick a color
        let hash = segment_name
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        match hash % 10 {
            0 => "#3366CC", // blue
            1 => "#DC3912", // red
            2 => "#FF9900", // orange
            3 => "#109618", // green
            4 => "#990099", // purple
            5 => "#3B3EAC", // indigo
            6 => "#0099C6", // cyan
            7 => "#DD4477", // pink
            8 => "#66AA00", // lime
            _ => "#B82E2E", // dark red
        }
    }
}

fn generate_plot_coverage(input_directory: &PathBuf) -> Result<Plot, Box<dyn Error>> {
    // Use coverage_df to read all coverage data into a DataFrame
    let df = coverage_df(input_directory)?;

    // Create a Plotly plot
    let mut plot = Plot::new();

    // Helper to check for column existence by &str
    let colnames = df.get_column_names();
    let ref_col = if colnames.iter().any(|c| *c == "Reference_Name") {
        "Reference_Name"
    } else {
        "Reference"
    };
    let pos_col = if colnames.iter().any(|c| *c == "HMM_Position") {
        "HMM_Position"
    } else {
        "Position"
    };
    let cov_col = if colnames.iter().any(|c| *c == "Coverage_Depth") {
        "Coverage_Depth"
    } else {
        "Coverage Depth"
    };

    // Get unique segment/reference names
    let segments: std::collections::HashSet<&str> =
        df.column(ref_col)?.utf8()?.into_no_null_iter().collect();

    for segment in segments {
        // Filter DataFrame for this segment
        let mask = df
            .column(ref_col)?
            .utf8()?
            .into_iter()
            .map(|v| v == Some(segment))
            .collect::<ChunkedArray<BooleanType>>();
        let seg_df = df.filter(&mask)?;
        let x_values = seg_df
            .column(pos_col)?
            .u32()?
            .into_no_null_iter()
            .collect::<Vec<_>>();
        let y_values = seg_df
            .column(cov_col)?
            .u32()?
            .into_no_null_iter()
            .collect::<Vec<_>>();
        let segment_color = get_segment_color(segment);
        let trace = Scatter::new(x_values, y_values)
            .mode(Mode::Lines)
            .name(segment)
            .line(plotly::common::Line::new().color(segment_color));
        plot.add_trace(trace);
    }

    // Set the figure title
    let layout = Layout::new()
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

fn generate_plot_coverage_seg(input_directory: &PathBuf) -> Result<Plot, Box<dyn Error>> {
    // Use coverage_df to read all coverage data into a DataFrame
    let df = coverage_df(input_directory)?;

    // Create a Plotly plot
    let mut plot = Plot::new();

    // Helper to check for column existence by &str
    let colnames = df.get_column_names();
    let ref_col = if colnames.iter().any(|c| *c == "Reference_Name") {
        "Reference_Name"
    } else {
        "Reference"
    };
    let pos_col = if colnames.iter().any(|c| *c == "HMM_Position") {
        "HMM_Position"
    } else {
        "Position"
    };
    let cov_col = if colnames.iter().any(|c| *c == "Coverage_Depth") {
        "Coverage_Depth"
    } else {
        "Coverage Depth"
    };

    // Get unique segment/reference names
    let segments: Vec<&str> = df.column(ref_col)?.utf8()?.into_no_null_iter().collect();
    let n_segments = segments.len();
    let rows = 4;
    let cols = 2;

    // Load variant data into a HashMap keyed by segment name
    // TODO: consider a struct with named fields
    let mut variants_data: HashMap<String, Vec<(u32, String, String, u32, u32, f32)>> = HashMap::new();

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

    for (idx, segment) in segments.iter().enumerate() {
        // Filter DataFrame for this segment
        let mask = df
            .column(ref_col)?
            .utf8()?
            .into_iter()
            .map(|v| v == Some(*segment))
            .collect::<ChunkedArray<BooleanType>>();
        let seg_df = df.filter(&mask)?;
        let x_values = seg_df
            .column(pos_col)?
            .u32()?
            .into_no_null_iter()
            .collect::<Vec<_>>();
        let y_values = seg_df
            .column(cov_col)?
            .u32()?
            .into_no_null_iter()
            .collect::<Vec<_>>();
        let segment_color = get_segment_color(segment);
        let trace = Scatter::new(x_values, y_values.clone())
            .mode(Mode::Lines)
            .name(segment)
            .line(plotly::common::Line::new().color(segment_color))
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
        plot.add_trace(trace);

        // Add variant data as scatter traces if we have data for this segment
        if let Some(variants) = variants_data.get(*segment) {
            let mut variant_positions: Vec<u32> = Vec::new();
            let mut consensus_values: Vec<u32> = Vec::new();
            let mut minority_values: Vec<u32> = Vec::new();
            let mut hover_texts: Vec<String> = Vec::new();
            for &(
                position,
                ref consensus_allele,
                ref minority_allele,
                consensus_count,
                minority_count,
                minority_frequency,
            ) in variants
            {
                variant_positions.push(position);
                consensus_values.push(consensus_count + minority_count);
                minority_values.push(minority_count);
                hover_texts.push(format!(
                    "<b>Position:</b> {}<br><br><b>Consensus Allele:</b> {}<br><b>Consensus Count:</b> {}<br><br><b>Minority Allele:</b> {}<br><b>Minority Count:</b> {}<br><b>Minority Frequency:</b> {:.2}%<br><br><b>Total:</b> {}",
                    position, consensus_allele, consensus_count, minority_allele, minority_count, minority_frequency * 100.0, consensus_count + minority_count
                ));
            }
            let minority_trace = Scatter::new(variant_positions, minority_values)
                .mode(Mode::Markers)
                .name(segment)
                .marker(
                    plotly::common::Marker::new()
                        .color(segment_color)
                        .opacity(0.5)
                        .size(15)
                        .symbol(plotly::common::MarkerSymbol::TriangleUp),
                )
                .text_array(hover_texts)
                .x_axis(&xaxis)
                .y_axis(&yaxis)
                .show_legend(false);
            plot.add_trace(minority_trace);
        }
    }

    // Configure subplot layout
    let mut layout = Layout::new()
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

    // Add annotations for each segment title
    let mut annotations = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        annotations.push(
            plotly::layout::Annotation::new()
                .text(segment)
                .x_ref("paper")
                .y_ref("paper")
                .x(match col {
                    0 => 0.2,
                    1 => 0.8,
                    _ => (col as f64 + 0.5) / cols as f64,
                })
                .y(match row {
                    0 => 1.0,
                    1 => 0.78,
                    2 => 0.48,
                    3 => 0.18,
                    _ => (row as f64 + 0.5) / rows as f64,
                })
                .font(
                    plotly::common::Font::new()
                        .size(22)
                        .color(get_segment_color(segment)),
                )
                .show_arrow(false),
        );
    }
    layout = layout.annotations(annotations);
    plot.set_layout(layout);
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

// TO DO: fix colors for Sankey diagram
fn generate_sankey_plot(input_directory: &PathBuf) -> Result<Plot, Box<dyn Error>> {
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
                if record.starts_with("4-") {
                    primary_match_sum += *reads;
                    let segment = record[2..].to_string();
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
        if record.starts_with("5-") {
            let segment = record[2..].to_string();
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

    // Create Sankey trace
    let node_labels_refs: Vec<&str> = node_labels.iter().map(|s| s.as_str()).collect();

    // Explicitly define x and y positions for each node
    let n = node_labels.len();
    let mut x = vec![0.0; n];
    let mut y = vec![0.0; n];
    // Assign positions for the first five nodes (Initial Reads, Pass QC, Fail QC, No Match, Alt Match)
    // Remaining nodes (segments) are stacked vertically in the last column
    let mut seg_idx = 0;
    for (i, label) in node_labels.iter().enumerate() {
        match label.as_str() {
            "Initial Reads" => {
                x[i] = 0.0;
                y[i] = 0.5;
            }
            "Pass QC" => {
                x[i] = 0.2;
                y[i] = 0.2;
            }
            "Fail QC" => {
                x[i] = 0.2;
                y[i] = 0.1;
            }
            "No Match" => {
                x[i] = 0.4;
                y[i] = 0.2;
            }
            "Alt Match" => {
                x[i] = 0.4;
                y[i] = 0.8;
            }
            "Primary Match" => {
                x[i] = 0.4;
                y[i] = 0.5;
            }
            _ => {
                // Segment nodes: stack vertically in last column
                x[i] = 0.7;
                y[i] = 0.1 + 0.8 * (seg_idx as f64) / ((n - 5).max(1) as f64);
                seg_idx += 1;
            }
        }
    }

    let sankey = Sankey::new()
        .node(
            plotly::sankey::Node::new()
                .label(node_labels_refs)
                .x(x)
                .y(y)
                .pad(15)
                .thickness(20)
                .line(plotly::sankey::Line::new().color("black"))
                .hover_template("<b>%{label}</b><br>%{value} reads")
                .hover_info(plotly::common::HoverInfo::Name),
        )
        .link(
            plotly::sankey::Link::new()
                .source(source_indices)
                .target(target_indices)
                .value(values)
                //.color(vec!["rgba(0,0,0,0.2)"; values.len()])
                //.hover_info("all")
                .hover_info(plotly::common::HoverInfo::None),
        )
        .arrangement(plotly::sankey::Arrangement::Snap);

    plot.add_trace(sankey);

    // Set layout
    let layout = Layout::new()
        .title(format!(
            "Read Assignment | {}",
            input_directory
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("Unknown")
        ))
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
fn add_node(
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
