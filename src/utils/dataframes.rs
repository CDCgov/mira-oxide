use glob::glob;
use polars::prelude::*;
use std::error::Error;
use std::path::PathBuf;

pub fn coverage_df(irma_path: &PathBuf) -> Result<DataFrame, Box<dyn Error>> {
    // Define the pattern to match text files
    let pattern = format!(
        "{}/*/IRMA/*/tables/*coverage.txt",
        irma_path.to_string_lossy()
    );
    println!("{}", pattern);

    // Initialize an empty DataFrame to hold the combined data
    let mut combined_cov_df: Option<DataFrame> = None;

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                // Read the CSV file into a DataFrame
                let file_path = path.to_str().unwrap();
                println!("Reading file: {}", file_path);

                let df = CsvReader::from_path(file_path)?.has_header(true).finish()?;

                // Combine the DataFrame with the existing one
                combined_cov_df = match combined_cov_df {
                    Some(existing_df) => Some(existing_df.vstack(&df)?),
                    None => Some(df),
                };
            }
            Err(e) => println!("Error reading file: {}", e),
        }
    }

    // Return the combined DataFrame or an error if no data was found
    if let Some(df) = combined_cov_df {
        Ok(df)
    } else {
        Err("No files found or no data to combine.".into())
    }
}
