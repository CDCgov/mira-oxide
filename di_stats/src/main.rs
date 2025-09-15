use clap::Parser;
use glob::glob;
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
    let run_id = assembly_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map_or("null", |id_str| {
            let parts: Vec<&str> = id_str.split('_').collect();
            if parts.len() != 4 {
                return "null";
            }

            let final_parts: Vec<&str> = parts[3].split('-').collect();
            if final_parts.len() != 2 {
                return "null";
            }

            let is_valid = parts[0].len() == 6 && parts[0].chars().all(char::is_numeric) &&
                           parts[1].len() == 6 && parts[1].chars().all(char::is_alphanumeric) &&
                           parts[2].len() == 4 && parts[2].chars().all(char::is_numeric) &&
                           final_parts[0].len() == 9 && final_parts[0].chars().all(char::is_numeric) &&
                           final_parts[1].len() == 5 && final_parts[1].chars().all(char::is_alphabetic);

            if is_valid {
                id_str
            } else {
                "null"
            }
        });

    let path_pattern = format!("{}/*", assembly_dir.to_str().unwrap_or_default());

    for entry in glob(&path_pattern)?.filter_map(Result::ok) {
        if entry.is_dir() {
            let sample_id = entry.file_name().unwrap_or_default().to_str().unwrap_or_default();
            let cov_pattern = format!("{}/tables/*coverage.txt", entry.to_str().unwrap_or_default());

            for cov_path in glob(&cov_pattern)?.filter_map(Result::ok) {
                let cov_str = cov_path.to_str().unwrap_or_default();
                if let Some(seg) = cov_str.split("tables/").nth(1).and_then(|s| s.split('-').next()) {
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