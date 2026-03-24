use clap::Parser;
use glob::glob;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// A Rust utility for calculating DI metric statistics from IRMA output
#[derive(Parser, Debug)]
#[command(about = "Tool for calculating DI stats")]
pub struct DIStatArgs {
    /// Path to the IRMA assembly directory
    #[arg(short = 'a', long)]
    assemblies_dir: PathBuf,

    /// Run ID to include in output
    #[arg(short = 'r', long)]
    run_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CoverageRecord {
    #[serde(rename = "Coverage Depth")]
    coverage_depth: f64,
}

/// Given a <seg>-coverage.txt file from IRMA's output,
/// this function returns a tuple with two ratios, one for the 5'
/// end and one for the 3' end.
#[allow(clippy::unnecessary_debug_formatting)]
#[allow(clippy::cast_precision_loss)]
pub fn di_stat(cov_file: &Path, length: usize) -> Result<(f64, f64), Box<dyn Error>> {
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
        return Ok((0.0, 0.0));
    }

    let prime5_slice = &data[..length];
    let prime5_mean = prime5_slice.iter().sum::<f64>() / prime5_slice.len() as f64;

    let prime3_slice = &data[data.len() - length..];
    let prime3_mean = prime3_slice.iter().sum::<f64>() / prime3_slice.len() as f64;

    let prime5_ratio = (prime5_mean / mid_mean * 1000.0).round() / 1000.0;
    let prime3_ratio = (prime3_mean / mid_mean * 1000.0).round() / 1000.0;

    Ok((prime5_ratio, prime3_ratio))
}

/// Calculate 5p and 3p DI stats for an entire assembly directory.
#[allow(clippy::unnecessary_debug_formatting)]
pub fn di_stat_assembly(
    assembly_dir: &Path,
    run_id: &str,
    writer: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let path_pattern = format!("{}/*/IRMA/*", assembly_dir.to_str().unwrap_or_default());

    for entry in glob(&path_pattern)?.filter_map(Result::ok) {
        if entry.is_dir() {
            let sample_id = entry
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            let cov_pattern = format!(
                "{}/tables/*coverage.txt",
                entry.to_str().unwrap_or_default()
            );

            for cov_path in glob(&cov_pattern)?.filter_map(Result::ok) {
                let cov_str = cov_path.to_str().unwrap_or_default();

                if let Some(seg) = cov_str
                    .split("tables/")
                    .nth(1)
                    .and_then(|s| s.split('-').next())
                {
                    match di_stat(&cov_path, 300) {
                        Ok((prime5, prime3)) => {
                            writeln!(
                                writer,
                                "{run_id}\t{sample_id}\t{seg}\t{prime5}\t{prime3}\t({prime5};{prime3})"
                            )?;
                        }
                        Err(e) => eprintln!("Could not process file {cov_path:?}: {e}"),
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn di_stats_process(args: &DIStatArgs) -> Result<(), std::io::Error> {
    let file = File::create("di_stats.txt")?;
    let mut writer = BufWriter::new(file);

    // Write header
    writeln!(
        writer,
        "run_id\tsample_id\tsegment\tprime5\tprime3\tdi_ratios_5prime_3prime"
    )?;

    if let Err(e) = di_stat_assembly(&args.assemblies_dir, &args.run_id, &mut writer) {
        eprintln!("Application error: {e}");
    }

    Ok(())
}
