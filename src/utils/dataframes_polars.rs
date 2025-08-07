use glob::glob;
use polars::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};

///reads any csv into a df
pub fn read_csv_to_dataframe(file_path: &PathBuf) -> Result<DataFrame, Box<dyn Error>> {
    // Read the CSV file into a DataFrame
    let df = CsvReader::from_path(file_path)?
        .infer_schema(None)
        .has_header(true)
        .finish()?;

    Ok(df)
}

/// Extract the sample name from the file path
fn extract_sample_name(path: &PathBuf) -> Result<String, Box<dyn Error>> {
    let parent_dir = path.parent().and_then(|p| p.parent());
    if let Some(parent_dir) = parent_dir {
        let sample = parent_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        Ok(sample)
    } else {
        Err("Failed to extract sample name from path.".into())
    }
}

///Read in the coverage files made by irma and convert to df
pub fn coverage_df(irma_path: impl AsRef<Path>) -> Result<DataFrame, Box<dyn Error>> {
    // Define the pattern to match text files
    let pattern = format!(
        "{}/*/IRMA/*/tables/*coverage.txt",
        irma_path.as_ref().to_string_lossy()
    );

    // Initialize an empty DataFrame to hold the combined data
    let mut combined_cov_df: Option<DataFrame> = None;

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file_path = path.to_str().unwrap();

                let mut df = CsvReader::from_path(file_path)?
                    .has_header(true)
                    .with_delimiter(b'\t')
                    .finish()?;

                // Add the "Sample" column to the DataFrame
                let sample_series = Series::new("Sample", vec![sample; df.height()]);
                df = df.hstack(&[sample_series])?;

                // Combine the DataFrame with the existing one
                combined_cov_df = match combined_cov_df {
                    Some(existing_df) => Some(existing_df.vstack(&df)?),
                    None => Some(df),
                };
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }

    // Return the combined DataFrame or an error if no data was found
    if let Some(df) = combined_cov_df {
        Ok(df)
    } else {
        Err("No files found or no data to combine.".into())
    }
}

///Read in the read count files made by irma and convert to df
pub fn readcount_df(irma_path: impl AsRef<Path>) -> Result<DataFrame, Box<dyn Error>> {
    // Define the pattern to match text files
    let pattern = format!(
        "{}/*/IRMA/*/tables/READ_COUNTS.txt",
        irma_path.as_ref().to_string_lossy()
    );

    // Initialize an empty DataFrame to hold the combined data
    let mut combined_reads_df: Option<DataFrame> = None;

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file_path = path.to_str().unwrap();

                let mut df = CsvReader::from_path(file_path)?
                    .has_header(true)
                    .with_delimiter(b'\t')
                    .finish()?;

                // Add the "Sample" column to the DataFrame
                let sample_series = Series::new("Sample", vec![sample; df.height()]);
                df = df.hstack(&[sample_series])?;

                // Combine the DataFrame with the existing one
                combined_reads_df = match combined_reads_df {
                    Some(existing_df) => Some(existing_df.vstack(&df)?),
                    None => Some(df),
                };
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    println!(
        "inside: {:?}",
        combined_reads_df
            .clone()
            .expect("REASON")
            .get_column_names()
    );

    // Return the combined DataFrame or an error if no data was found
    if let Some(df) = combined_reads_df {
        Ok(df)
    } else {
        Err("No files found or no data to combine.".into())
    }
}

/// Parses a record string into vtype, ref_type, and subtype.
pub fn read_record2type(record: &str) -> Vec<String> {
    let parts: Vec<&str> = record.split('_').collect();
    if parts.len() >= 2 {
        let vtype = parts[0][2..].to_string();
        let ref_type = parts[1].to_string();
        let subtype = if ref_type == "HA" || ref_type == "NA" {
            parts.last().unwrap_or(&"").to_string()
        } else {
            "".to_string()
        };
        vec![vtype, ref_type, subtype]
    } else {
        vec![record[2..].to_string(); 3]
    }
}

/// Processes the DataFrame to extract sample types based on the `Record` column.
pub fn dash_irma_sample_type(reads_df: &DataFrame) -> Result<DataFrame, PolarsError> {
    //println!("{reads_df:?}");

    // Filter rows where the first character of the 'Record' column is '4'
    let mask = reads_df
        .column("Record")?
        .utf8()?
        .into_iter()
        .map(|record| record.map(|r| r.starts_with('4')))
        .collect::<ChunkedArray<BooleanType>>();
    let type_df = reads_df.filter(&mask)?;
    // Filter the DataFrame where "Records" column contains '4' anywhere in the string

    // Create new columns: 'vtype', 'ref_type', 'subtype'
    let new_cols = ["vtype", "ref_type", "subtype"];
    let mut new_columns = Vec::new();

    for (n, col_name) in new_cols.iter().enumerate() {
        let col = type_df
            .column("Record")?
            .utf8()?
            .into_iter()
            .map(|record| {
                record.map(|r| {
                    let types = read_record2type(r);
                    types[n].clone()
                })
            })
            .collect::<Vec<Option<String>>>();
        new_columns.push(Series::new(col_name, col));
    }

    // Add the 'Reference' column
    let reference_col = type_df
        .column("Record")?
        .utf8()?
        .into_iter()
        .map(|record| {
            record.map(|r| {
                let parts: Vec<&str> = r.split('_').collect();
                parts[0][2..].to_string()
            })
        })
        .collect::<Vec<Option<String>>>();
    new_columns.push(Series::new("Reference", reference_col));

    // Create a new DataFrame with the selected columns
    let mut new_df = DataFrame::new(new_columns)?;
    //new_df = new_df.select(&["Sample", "vtype", "ref_type", "subtype"])?;
    Ok(new_df)
}
