use clap::Parser;
use csv::ReaderBuilder;
use glob::glob;
use plotly::common::{Mode, Title};
use plotly::configuration::{ImageButtonFormats, ToImageButtonOptions};
use plotly::layout::{Axis, GridPattern, LayoutGrid};
use plotly::{Layout, Plot, Sankey, Scatter};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

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

#[derive(Parser)]
#[command(version, about = "Generate plotly plots for IRMA output")]
struct Args {
    #[arg(short = 'i', long)]
    // Required path to IRMA-sample directory
    irma_dir: PathBuf,

    #[arg(short = 'o', long)]
    // Optional output path for HTML files (default: None)
    output: Option<PathBuf>,

    #[arg(short = 'c', long, default_value_t = false)]
    // Generate coverage plot (default: false)
    coverage: bool,

    #[arg(short = 'd', long, default_value_t = false)]
    // Show plots immediately in browser (default: false)
    display: bool,

    #[arg(short = 's', long, default_value_t = false)]
    // Generate segmented coverage subplots
    coverage_seg: bool,

    #[arg(short = 'r', long, default_value_t = false)]
    // Generate read flow sankey diagram
    read_flow: bool,
}

fn generate_plot_coverage(input_directory: &PathBuf) -> Result<Plot, Box<dyn Error>> {
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
                    .line(plotly::common::Line::new().color(segment_color));

                plot.add_trace(trace);
            }
            Err(e) => eprintln!("Error reading file: {}", e),
        }
    }

    // Set the figure title
    let layout = Layout::new()
        .title(&format!(
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
    // Init a Plotly plot
    let mut plot = Plot::new();

    // Track number of files for subplot layout
    let mut file_count = 0;
    let mut file_paths = Vec::new();

    // First, count files and collect paths
    for entry in glob(&format!(
        "{}/tables/*coverage.txt",
        input_directory.display()
    ))? {
        if let Ok(path) = entry {
            file_count += 1;
            file_paths.push(path);
        }
    }

    // Calculate grid dimensions for subplots
    let rows = (file_count as f64).sqrt().ceil() as usize;
    let cols = (file_count + rows - 1) / rows; // Ceiling division

    // Load variant data into a HashMap keyed by segment name
    let mut variants_data: HashMap<String, Vec<(u32, String, String, u32, u32, f32)>> =
        HashMap::new();

    // Look for variant files with matching prefixes in the directory
    for entry in glob(&format!(
        "{}/tables/*variants.txt",
        input_directory.display()
    ))? {
        if let Ok(variant_path) = entry {
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

                    variants_data
                        .entry(segment_name)
                        .or_insert_with(Vec::new)
                        .push((
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
    }

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

        // Create a trace for the current CSV file with consistent color
        let trace = Scatter::new(x_values, y_values.clone())
            .mode(Mode::Lines)
            .name(&segment_name)
            .line(plotly::common::Line::new().color(segment_color))
            .hover_template("<b>Position:</b> %{x}<br><b>Coverage:</b> %{y}<br>");

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

        // Add variant data as scatter traces if we have data for this segment
        if let Some(variants) = variants_data.get(&segment_name) {
            // Collect positions and values for consensus and minority traces
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
                consensus_values.push(consensus_count + minority_count); // Total height
                minority_values.push(minority_count);
                hover_texts.push(format!(
                    "Position: {}<br>Consensus Allele: {}<br>Consensus Count: {}<br>Minority Allele: {}<br>Minority Count: {}<br>Frequency: {:.2}%<br>Total: {}",
                    position, consensus_allele, consensus_count, minority_allele, minority_count, minority_frequency * 100.0, consensus_count + minority_count
                ));
            }

            // Create trace for minority values with consistent color (but with transparency)
            let minority_trace = Scatter::new(variant_positions, minority_values)
                .mode(Mode::Markers)
                .name(&format!("{}", segment_name))
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

            // Add variant traces to plot
            plot.add_trace(minority_trace);
        }
    }

    // Configure subplot layout
    let layout = Layout::new()
        .grid(
            LayoutGrid::new()
                .rows(4)
                .columns(2)
                .pattern(GridPattern::Independent),
        )
        .title(&format!(
            "Segment Coverage | {}",
            input_directory
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("Unknown")
        ));

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
    for line in lines {
        if let Ok(line) = line {
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
    let mut initial_reads = 0;
    let mut pass_qc = 0;
    let mut fail_qc = 0;
    let mut no_match = 0;
    let mut chi_alt_reads = 0;

    for (record, reads) in records {
        match record.as_str() {
            "1-initial" => initial_reads = reads,
            "2-failQC" => fail_qc = reads,
            "2-passQC" => pass_qc = reads,
            "3-nomatch" => no_match = reads,
            "3-chimeric" | "3-altmatch" => chi_alt_reads += reads,
            _ => {
                if record.starts_with("4-") || record.starts_with("5-") {
                    // Extract segment name
                    let segment = record[2..].to_string();

                    // Add segment as node
                    let segment_color = get_segment_color(&segment);
                    add_node(
                        &segment,
                        &mut node_labels,
                        &mut node_map,
                        &mut node_colors,
                        segment_color,
                    );

                    // Create link from matching to this segment
                    source_indices.push(node_map["Pass QC"]);
                    target_indices.push(node_map[&segment]);
                    values.push(reads);
                }
            }
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
    //let node_colors_refs: Vec<&str> = node_colors.iter().map(|s| s.as_str()).collect();
    let sankey = Sankey::new()
        .node(
            plotly::sankey::Node::new()
                .label(node_labels_refs)
                .pad(15)
                .thickness(20)
                .line(plotly::sankey::Line::new().color("black")),
        )
        .link(
            plotly::sankey::Link::new()
                .source(source_indices)
                .target(target_indices)
                .value(values)
                //.color(vec!["rgba(0,0,0,0.2)"; values.len()])
                //.hover_info("all")
                .hover_template(
                    "Source: %{source.label}<br>Target: %{target.label}<br>Value: %{value}",
                ),
        );

    plot.add_trace(sankey);

    // Set layout
    let layout = Layout::new()
        .title(&format!(
            "Read Flow | {}",
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
                    .filename("read_flow"),
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

fn main() -> Result<(), Box<dyn Error>> {
    // Check for correct number of arguments
    let args = Args::parse();

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
    }

    // Generate read flow sankey diagram if specified
    if args.read_flow {
        let plot = generate_sankey_plot(&input_directory)?;

        // Save the plot as an HTML file if output path is provided
        if let Some(optional_file) = &output_html_file {
            // Add "_flow" suffix to the filename
            let flow_file = optional_file.with_file_name(format!(
                "{}_flow{}",
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
    }

    Ok(())
}
