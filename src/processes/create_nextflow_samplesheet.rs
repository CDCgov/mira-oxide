use clap::Parser;
use csv::Reader;
use glob::glob;
use serde::Deserialize;
use std::{
    fs::File,
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
    #[serde(rename = "Sample ID")]
    sample_id: String,

    #[serde(rename = "Sample Type")]
    sample_type: String,

    #[serde(rename = "Barcode #", default)]
    barcode: String,
}

#[allow(clippy::format_push_string)]
pub fn create_nextflow_samplesheet(args: &SamplesheetArgs) -> io::Result<()> {
    // Match Python's time.sleep(60)
    thread::sleep(Duration::from_mins(1));

    let mut rdr = Reader::from_path(&args.samplesheet)?;
    let mut output = String::new();

    let experiment = args.experiment_type.as_str().to_ascii_lowercase();
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

            let fastq_1 = glob(&pattern)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
                .find_map(Result::ok)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Fastq not found for sample {id}"),
                    )
                })?;

            output.push_str(&format!(
                "{},{},{},,{}\n",
                id,
                record.barcode,
                fastq_1.display(),
                record.sample_type
            ));
        } else {
            let r1_pattern = format!("{runpath}/{id}*R1*fastq*");
            let r2_pattern = format!("{runpath}/{id}*R2*fastq*");

            let r1 = glob(&r1_pattern)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
                .find_map(Result::ok)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("R1 fastq not found for sample {id}"),
                    )
                })?;

            let r2 = glob(&r2_pattern)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
                .find_map(Result::ok)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("R2 fastq not found for sample {id}"),
                    )
                })?;

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

    Ok(())
}
