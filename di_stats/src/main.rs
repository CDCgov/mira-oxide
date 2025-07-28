// filepath: /home/qgx6/dev/mira-oxide/di_stats/src/main.rs
use clap::Parser;
use glob::glob;
use regex::Regex;
use serde::Deserialize;
use std::error::Error;
use std::path::{Path, PathBuf};

/// A Rust utility for calculating DI metric statistics from IRMA output
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the IRMA assembly directory
    #[arg(required = true)]
    assembly_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CoverageRecord {
    #[serde(rename = "Coverage Depth")]
    coverage_depth: f64,
}
/// Given a <seg>-coverage.txt file from IRMA's output,
/// this function returns a tuple with two ratios, one for the 5'
/// end and one for the 3' end. The ratio is calculated as the mean
/// coverage for [length] bases at either end / the coverage for [length]
/// bases in the center of the segment.
fn di_stat(cov_file: &Path, length: usize) -> Result<(f64, f64), Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(cov_file)?;

    let data: Vec<f64> = rdr
        .deserialize::<CoverageRecord>()
        .filter_map(|result| result.ok().map(|record| record.coverage_depth))
        .collect();

    if data.len() < length * 2 {
        return Err(format!(
            "Not enough data in {:?} for calculation ({} points < {} required)",
            cov_file,
            data.len(),
            length * 2
        )
        .into());
    }
    let mid = data.len() / 2;
    let mid_start = mid.saturating_sub(length / 2);
    let mid_end = mid + (length / 2);

    let mid_slice = &data[mid_start..mid_end];
    let mid_mean = mid_slice.iter().sum::<f64>() / mid_slice.len() as f64;

    if mid_mean == 0.0 {
        return Ok((0.0, 0.0)); // Avoid division by zero
    }

    let prime5_slice = &data[..length];
    let prime5_mean = prime5_slice.iter().sum::<f64>() / prime5_slice.len() as f64;

    let prime3_slice = &data[data.len() - length..];
    let prime3_mean = prime3_slice.iter().sum::<f64>() / prime3_slice.len() as f64;

    // Round to 3 decimal places
    let prime5_ratio = (prime5_mean / mid_mean * 1000.0).round() / 1000.0;
    let prime3_ratio = (prime3_mean / mid_mean * 1000.0).round() / 1000.0;

    Ok((prime5_ratio, prime3_ratio))
}

/// Calculate 5p and 3p DI stats for an entire assembly directory.
fn di_stat_assembly(assembly_dir: &Path) -> Result<(), Box<dyn Error>> {
    let run_id_pattern = Regex::new(r"/(?P<id>[0-9]{6}_[A-Za-z0-9]{6}_.+)/assemblies")?;
    let seg_pattern = Regex::new(r"tables/(?P<seg>.+)-")?;

    let run_id = run_id_pattern
        .captures(assembly_dir.to_str().unwrap_or_default())
        .and_then(|caps| caps.name("id"))
        .map_or("null", |m| m.as_str());

    let path_pattern = format!("{}/*", assembly_dir.to_str().unwrap_or_default());

    for entry in glob(&path_pattern)?.filter_map(Result::ok) {
        if entry.is_dir() {
            let sample_id = entry.file_name().unwrap_or_default().to_str().unwrap_or_default();
            let cov_pattern = format!("{}/tables/*coverage.txt", entry.to_str().unwrap_or_default());

            for cov_path in glob(&cov_pattern)?.filter_map(Result::ok) {
                let cov_str = cov_path.to_str().unwrap_or_default();
                if let Some(seg_match) = seg_pattern.captures(cov_str) {
                    let seg = &seg_match["seg"];
                    match di_stat(&cov_path, 300) {

                        Ok((prime5, prime3)) => {
                            println!("{}\t{}\t{}\t{}\t{}", run_id, sample_id, seg, prime5, prime3);
                        }
                        Err(e) => eprintln!("Could not process file {:?}: {}", cov_path, e),
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() {
        let args = Args::parse();
    if let Err(e) = di_stat_assembly(&args.assembly_dir) {
        eprintln!("Application error: {}", e);
    }
}