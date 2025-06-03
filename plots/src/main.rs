use csv::ReaderBuilder;
use glob::glob;
use plotly::common::Mode;
use plotly::{Layout, Plot, Scatter};
use std::env;
use std::error::Error;
use std::fs::File;

fn main() -> Result<(), Box<dyn Error>> {
    // Check for correct number of arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_directory> <output_html_file>", args[0]);
        std::process::exit(1);
    }

    // Get the input directory and output file path from the command line arguments
    let input_directory = &args[1];
    let output_html_file = &args[2];

    // Create a Plotly plot
    let mut plot = Plot::new();

    // Iterate over all CSV files in the input directory
    for entry in glob(&format!("{input_directory}/*coverage.txt",))? {
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
                    .name(path.file_name().unwrap().to_str().unwrap());
                plot.add_trace(trace);
            }
            Err(e) => eprintln!("Error reading file: {e}",),
        }
    }

    // Set the figure title
    let layout = Layout::new().title(format!("Coverage | {input_directory}"));
    plot.set_layout(layout);

    // Save the plot as an HTML file
    plot.write_html(output_html_file);
    plot.show();

    Ok(())
}
