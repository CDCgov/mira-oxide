use csv::ReaderBuilder;
use either::Either;
use glob::glob;
use serde::{self, Deserialize, de::DeserializeOwned};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufReader, Stdin, stdin},
    path::{Path, PathBuf},
};

// Coverage struct
#[derive(Deserialize, Debug)]
pub struct CoverageData {
    #[serde(rename = "Reference_Name")]
    reference_name: String,
    #[serde(rename = "Position")]
    position: String,
    #[serde(rename = "Coverage Depth")]
    coverage_depth: String,
    #[serde(rename = "Consensus")]
    consensus: String,
    #[serde(rename = "Deletions")]
    deletions: String,
    #[serde(rename = "Ambiguous")]
    ambiguous: String,
    #[serde(rename = "Consensus_Count")]
    consensus_count: String,
    #[serde(rename = "Consensus_Average_Quality")]
    consensus_avg_quality: String,
    sample_id: Option<String>,
}

// Reads struct
#[derive(Deserialize, Debug)]
pub struct ReadsData {
    #[serde(rename = "Record")]
    record: String,
    #[serde(rename = "Reads")]
    reads: String,
    #[serde(rename = "Patterns")]
    patterns: String,
    #[serde(rename = "PairsAndWidows")]
    pairs_and_windows: String,
    sample_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcessedRecord {
    pub sample_id: Option<String>, // Optional field
    pub vtype: String,
    pub ref_type: String,
    pub subtype: String,
}

pub fn create_reader(path: Option<PathBuf>) -> std::io::Result<BufReader<Either<File, Stdin>>> {
    let reader = if let Some(ref file_path) = path {
        let file = OpenOptions::new().read(true).open(file_path)?;
        BufReader::new(Either::Left(file))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    Ok(reader)
}

pub fn read_csv<T: DeserializeOwned, R: std::io::Read>(
    reader: R,
    has_headers: bool,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b',')
        .from_reader(reader);

    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let record: T = result?;
        records.push(record);
    }

    Ok(records)
}

/// Extract the sample name from the file path
fn extract_sample_name(path: &Path) -> Result<String, Box<dyn Error>> {
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

/// Read tab-delimited data and include the sample name
fn process_cov_txt_with_sample<R: std::io::Read>(
    reader: R,
    has_headers: bool,
    sample_id: String,
) -> Result<Vec<CoverageData>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b'\t')
        .from_reader(reader);

    let mut records: Vec<CoverageData> = Vec::new();
    for result in rdr.deserialize() {
        let mut record: CoverageData = result?;
        record.sample_id = Some(sample_id.clone()); // Add the sample_id to the record
        records.push(record);
    }

    Ok(records)
}

/// Read in the coverage files made by IRMA and convert to a vector of CoverageData
pub fn coverage_data_collection(
    irma_path: &PathBuf,
) -> Result<Vec<CoverageData>, Box<dyn std::error::Error>> {
    // Define the pattern to match text files
    let pattern = format!(
        "{}/*/IRMA/*/tables/*coverage.txt",
        irma_path.to_string_lossy()
    );

    // Initialize an empty vector to hold the combined data
    let mut cov_data: Vec<CoverageData> = Vec::new();

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records = process_cov_txt_with_sample(reader, true, sample)?;
                cov_data.append(&mut records); // Append the records to the combined data
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(cov_data)
}

/// Read tab-delimited data and include the sample name
fn process_reads_txt_with_sample<R: std::io::Read>(
    reader: R,
    has_headers: bool,
    sample_id: String,
) -> Result<Vec<ReadsData>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b'\t')
        .from_reader(reader);

    let mut records: Vec<ReadsData> = Vec::new();
    for result in rdr.deserialize() {
        let mut record: ReadsData = result?;
        record.sample_id = Some(sample_id.clone()); // Add the sample_id to the record
        records.push(record);
    }

    Ok(records)
}

pub fn reads_data_collection(
    irma_path: &PathBuf,
) -> Result<Vec<ReadsData>, Box<dyn std::error::Error>> {
    // Define the pattern to match text files
    let pattern = format!(
        "{}/*/IRMA/*/tables/READ_COUNTS.txt",
        irma_path.to_string_lossy()
    );

    // Initialize an empty vector to hold the combined data
    let mut reads_data: Vec<ReadsData> = Vec::new();

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records = process_reads_txt_with_sample(reader, true, sample)?;
                reads_data.append(&mut records); // Append the records to the combined data
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(reads_data)
}

fn read_record2type(record: &str) -> (String, String, String) {
    let parts: Vec<&str> = record.split('_').collect();
    if parts.len() >= 2 {
        let vtype = parts[0][2..].to_string(); // Remove the first two characters
        let ref_type = parts[1].to_string();
        let subtype = if ref_type == "HA" || ref_type == "NA" {
            parts.last().unwrap_or(&"").to_string()
        } else {
            "".to_string()
        };
        (vtype, ref_type, subtype)
    } else {
        let fallback = record[2..].to_string();
        (fallback.clone(), fallback.clone(), fallback.clone())
    }
}

pub fn dash_irma_sample_type(reads_data: Vec<ReadsData>) -> Vec<ProcessedRecord> {
    let mut processed_records = Vec::new();

    for data in reads_data.iter() {
        // Filter records where the first character of 'record' is '4'
        if data.record.starts_with('4') {
            let (vtype, ref_type, subtype) = read_record2type(&data.record);
            let processed_record = ProcessedRecord {
                sample_id: data.sample_id.clone(),
                vtype,
                ref_type,
                subtype,
            };
            processed_records.push(processed_record);
        }
    }

    processed_records
}
