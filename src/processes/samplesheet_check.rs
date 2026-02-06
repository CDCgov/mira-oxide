use clap::Parser;
use csv::{Reader, Writer};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{self},
    path::PathBuf,
    process,
};

const VALID_FORMATS: [&str; 2] = [".fq.gz", ".fastq.gz"];

#[derive(Debug, Parser)]
#[command(about = "Validate and transform a tabular samplesheet")]
pub struct SamplesheetCheckArgs {
    /// Input samplesheet (CSV or TSV)
    #[arg(short = 'i', long)]
    pub file_in: PathBuf,

    /// Output CSV samplesheet
    #[arg(short = 'o', long)]
    pub file_out: PathBuf,
}

#[derive(Debug, Deserialize)]
struct InputRow {
    sample: String,
    fastq_1: String,

    #[serde(default)]
    fastq_2: String,
}

#[derive(Debug, Serialize)]
struct OutputRow {
    sample: String,
    single_end: bool,
    fastq_1: String,
    fastq_2: String,
}

struct RowChecker {
    seen_pairs: HashSet<(String, String)>,
    sample_counter: HashMap<String, usize>,
}

impl RowChecker {
    fn new() -> Self {
        Self {
            seen_pairs: HashSet::new(),
            sample_counter: HashMap::new(),
        }
    }

    fn validate_and_transform(&mut self, row: InputRow, line: usize) -> Result<OutputRow, String> {
        let sample = self.validate_sample(row.sample, line)?;
        self.validate_fastq(&row.fastq_1, "fastq_1", line)?;
        if !row.fastq_2.is_empty() {
            self.validate_fastq(&row.fastq_2, "fastq_2", line)?;
            self.validate_pair(&row.fastq_1, &row.fastq_2, line)?;
        }

        let pair_key = (sample.clone(), row.fastq_1.clone());
        if !self.seen_pairs.insert(pair_key) {
            return Err(format!(
                "The pair of sample name and FASTQ must be unique. On line {line}."
            ));
        }

        let counter = self.sample_counter.entry(sample.clone()).or_insert(0);
        *counter += 1;

        Ok(OutputRow {
            sample: format!("{sample}_T{counter}"),
            single_end: row.fastq_2.is_empty(),
            fastq_1: row.fastq_1,
            fastq_2: row.fastq_2,
        })
    }

    fn validate_sample(&self, sample: String, line: usize) -> Result<String, String> {
        if sample.is_empty() {
            return Err(format!("Sample input is required. On line {line}."));
        }
        Ok(sample.replace(' ', "_"))
    }

    fn validate_fastq(&self, value: &str, col: &str, line: usize) -> Result<(), String> {
        if value.is_empty() {
            return Err(format!(
                "At least the first FASTQ file is required. On line {line}."
            ));
        }

        if !VALID_FORMATS.iter().any(|ext| value.ends_with(ext)) {
            return Err(format!(
                "The FASTQ file has an unrecognized extension: {value} \
                 (column {col}). On line {line}."
            ));
        }
        Ok(())
    }

    fn validate_pair(&self, f1: &str, f2: &str, line: usize) -> Result<(), String> {
        let s1 = suffixes(f1);
        let s2 = suffixes(f2);

        if s1 != s2 {
            return Err(format!(
                "FASTQ pairs must have the same file extensions. On line {line}."
            ));
        }
        Ok(())
    }
}

fn suffixes(path: &str) -> Vec<&str> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() >= 3 {
        &parts[parts.len() - 2..]
    } else {
        &[]
    }
    .to_vec()
}

pub fn samplesheet_check(args: &SamplesheetCheckArgs) -> io::Result<()> {
    if !args.file_in.is_file() {
        eprintln!("Input file {:?} not found.", args.file_in.display());
        process::exit(2);
    }

    if let Some(parent) = args.file_out.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("{e}");
        process::exit(1);
    }

    let mut rdr = Reader::from_path(&args.file_in)?;
    let mut wtr = Writer::from_path(&args.file_out)?;

    let mut checker = RowChecker::new();

    for (idx, result) in rdr.deserialize().enumerate() {
        let row: InputRow = result.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let output = checker
            .validate_and_transform(row, idx + 2)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        wtr.serialize(output)?;
    }

    wtr.flush()?;
    Ok(())
}
