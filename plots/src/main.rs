use clap::Parser;
use csv::ReaderBuilder;
use glob::glob;
use plotly::common::Mode;
use plotly::{Layout, Plot, Scatter};
use plotly::layout::{LayoutGrid, GridPattern};
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about="Generate plotly plots for IRMA output")]
struct Args {
    #[arg(short='i', long)]
    // Required path to IRMA-sample directory
    irma_dir: PathBuf,

    #[arg(short='o', long)]
    // Optional output path for HTML files (default: None)
    output: Option<PathBuf>,
    
    #[arg(short='c', long, default_value_t = false)]
    // Generate coverage plot (default: false)
    coverage: bool,
    
    #[arg(short='d', long, default_value_t = false)]
    // Show plots immediately in browser (default: false)
    display: bool,
    
    #[arg(short='s', long, default_value_t = false)]
    // Generate segmented coverage subplots
    coverage_seg: bool,
}

fn generate_plot_coverage(input_directory: &PathBuf) -> Result<Plot, Box<dyn Error>> {
    // Create a Plotly plot
    let mut plot = Plot::new();

    // Iterate over all coverage files in the input directory
    for entry in glob(&format!("{}/tables/*coverage.txt", input_directory.display()))? {
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

                // Create a trace for the current CSV file
                let trace = Scatter::new(x_values, y_values)
                    .mode(Mode::Lines)
                    .name(path.file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .split('-')
                            .next()
                            .unwrap());
                plot.add_trace(trace);
            }
            Err(e) => eprintln!("Error reading file: {e}",),
        }
    }

    // Set the figure title
<<<<<<< HEAD
    let layout = Layout::new().title(format!("Coverage | {input_directory}"));
=======
    let layout = Layout::new()
                            .title(&format!("Coverage | {}", input_directory.file_name()
                                                                            .unwrap()
                                                                            .to_str()
                                                                            .unwrap()
                                                                            .split('-')
                                                                            .next()
                                                                            .unwrap()));
>>>>>>> 6c928e2 (arguments and subplots per segment)
    plot.set_layout(layout);

    Ok(plot)
}

fn generate_plot_coverage_seg(input_directory: &PathBuf) -> Result<Plot, Box<dyn Error>> {
    // Init a Plotly plot
    let mut plot = Plot::new();
    
    // Track number of files for subplot layout
    let mut file_count = 0;
    let mut file_paths = Vec::new();
    
    // First, count files and collect paths
    for entry in glob(&format!("{}/tables/*coverage.txt", input_directory.display()))? {
        if let Ok(path) = entry {
            file_count += 1;
            file_paths.push(path);
        }
    }
    
    // Calculate grid dimensions for subplots
    let rows = (file_count as f64).sqrt().ceil() as usize;
    let cols = (file_count + rows - 1) / rows; // Ceiling division
    
    // Process each file and create a subplot
    for (idx, path) in file_paths.iter().enumerate() {
        // Open the CSV file
        let file = File::open(path)?;

        // Create a CSV reader
        let mut rdr = ReaderBuilder::new()
                                        .delimiter(b'\t')
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

        // Extract file name for title
        let file_name = path.file_name()
                      .unwrap_or_default()
                      .to_str()
                      .unwrap_or("Unknown")
                      .split('-')
                      .next()
                      .unwrap_or("Unknown");

        // Create a trace for the current CSV file
        let trace = Scatter::new(x_values, y_values)
            .mode(Mode::Lines)
            .name(file_name);
            
        // Calculate row and column for this subplot (1-indexed)
        let row = idx / cols + 1;
        let col = idx % cols + 1;

        // Set xaxis and yaxis for subplot assignment
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

        let trace = trace.x_axis(xaxis).y_axis(yaxis);

        // Add trace to plot
        plot.add_trace(trace);
    }
    
    // Configure subplot layout
    let layout = Layout::new()
                            .grid(LayoutGrid::new()
                                .rows(4)
                                .columns(2)
                                .pattern(GridPattern::Independent)
                            )
                            .title(&format!("Segmented Coverage | {}", input_directory.file_name()
                                                                                    .unwrap_or_default()
                                                                                    .to_str()
                                                                                    .unwrap_or("Unknown")
                                            )
                                );
    
    plot.set_layout(layout);

    Ok(plot)
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
                optional_file.file_stem().unwrap_or_default().to_string_lossy(),
                optional_file.extension().map_or_else(String::new, |ext| format!(".{}", ext.to_string_lossy()))
            ));
            plot.write_html(seg_file);
        }
        
        // Show the plot if specified
        if args.display {
            plot.show();
        }
    }

    Ok(())
}
