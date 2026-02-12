use clap::Parser;
use csv::Reader;
use glob::glob;
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    thread,
    time::Duration,
};

#[derive(Debug, Parser)]
#[command(about = "Generate Nextflow-compatible samplesheet")]
pub struct SamplesheetArgs {
    /// Samplesheet CSV file
    #[arg(short = 's', long)]
    pub samplesheet: PathBuf,

    /// Run directory path to fastq files
    #[arg(short = 'r', long)]
    pub runid: PathBuf,

    /// Experiment type
    #[arg(short = 'e', long)]
    pub experiment_type: String,
}

#[derive(Debug, Deserialize)]
struct SampleRow {
    #[serde(rename = "sample_id")]
    sample_id: String,

    #[serde(rename = "sample_type")]
    sample_type: String,

    #[serde(rename = "barcode", default)]
    barcode: String,
}

pub fn find_fastq(patterns: &[String]) -> Option<PathBuf> {
    for pattern in patterns {
        if let Ok(mut paths) = glob(pattern)
            && let Some(path) = paths.find_map(Result::ok)
        {
            return Some(path);
        }
    }
    None
}

#[allow(clippy::format_push_string)]
pub fn create_nextflow_samplesheet(args: &SamplesheetArgs) -> io::Result<()> {
    thread::sleep(Duration::from_mins(1));

    let mut rdr = Reader::from_path(&args.samplesheet)?;
    let mut output = String::new();

    // (sample_id, reason)
    let mut missing_samples: Vec<(String, String)> = Vec::new();

    let experiment = args.experiment_type.to_ascii_lowercase();
    let runpath = args.runid.to_string_lossy();

    let is_ont = experiment.contains("ont");

    if is_ont {
        output.push_str("sample,barcodes,fastq_1,fastq_2,sample_type\n");
    } else {
        output.push_str("sample,fastq_1,fastq_2,sample_type\n");
    }

    for result in rdr.deserialize() {
        let record: SampleRow = result?;
        let id = record.sample_id;

        if is_ont {
            let pattern = format!("{runpath}/fastq_pass/cat_fastqs/{id}_nf_combined.fastq*");

            let fastq_1 = glob(&pattern).ok().and_then(|mut g| g.find_map(Result::ok));

            let fastq_1 = match fastq_1 {
                Some(fq) => fq,
                None => {
                    missing_samples.push((id, "Missing FASTQ file".to_string()));
                    continue;
                }
            };

            if fs::metadata(&fastq_1)?.len() == 0 {
                missing_samples.push((id, "Empty FASTQ".to_string()));
                continue;
            }

            output.push_str(&format!(
                "{},{},{},,{}\n",
                id,
                record.barcode,
                fastq_1.display(),
                record.sample_type
            ));
        } else {
            let r1_patterns = vec![
                format!("{runpath}/{id}_R1*.fastq*"),
                format!("{runpath}/{id}_R1*.fq*"),
            ];

            let r2_patterns = vec![
                format!("{runpath}/{id}_R2*.fastq*"),
                format!("{runpath}/{id}_R2*.fq*"),
            ];

            let r1 = find_fastq(&r1_patterns);
            let r2 = find_fastq(&r2_patterns);

            let (r1, r2) = match (r1, r2) {
                (Some(r1), Some(r2)) => (r1, r2),
                (None, Some(_)) => {
                    missing_samples.push((id, "Missing R1 FASTQ".to_string()));
                    continue;
                }
                (Some(_), None) => {
                    missing_samples.push((id, "Missing R2 FASTQ".to_string()));
                    continue;
                }
                (None, None) => {
                    missing_samples.push((id, "Missing R1 and R2 FASTQ".to_string()));
                    continue;
                }
            };

            // 🔍 Check both files before deciding
            let r1_empty = fs::metadata(&r1)?.len() == 0;
            let r2_empty = fs::metadata(&r2)?.len() == 0;

            if r1_empty && r2_empty {
                missing_samples.push((id, "Empty R1 and R2 FASTQ".to_string()));
                continue;
            } else if r1_empty {
                missing_samples.push((id, "Empty R1 FASTQ".to_string()));
                continue;
            } else if r2_empty {
                missing_samples.push((id, "Empty R2 FASTQ".to_string()));
                continue;
            }

            output.push_str(&format!(
                "{},{},{},{}\n",
                id,
                r1.display(),
                r2.display(),
                record.sample_type
            ));
        }
    }

    let mut file = File::create("nextflow_samplesheet.csv")?;
    file.write_all(output.as_bytes())?;

    if !missing_samples.is_empty() {
        let mut bad_samples = File::create("bad_samples.tsv")?;
        writeln!(bad_samples, "sample_id\treason")?;

        for (sample_id, reason) in missing_samples {
            writeln!(bad_samples, "{sample_id}\t{reason}")?;
        }
    }

    Ok(())
}
