#![allow(dead_code, unused_imports)]
use std::{collections::HashMap, error::Error, path::PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::io::{
    data_ingest::{NextcladeData, create_reader, nextclade_data_collection, read_csv},
    write_csv_files::write_out_updated_summary_csv,
    write_parquet_files::write_updated_irma_summary_to_parquet,
};

#[derive(Debug, Parser)]
#[command(
    about = "Package for adding the nextclade clade results back into the summary.csv and sumary.parq files created by MIRA"
)]
pub struct SummaryUpdateArgs {
    #[arg(short = 'i', long)]
    /// The file path to the samples folders with nextclade outputs.
    nextclade_path: PathBuf,

    #[arg(short = 'o', long)]
    /// The file path where the `prepare_mira_report` outputs will be saved.
    output_path: PathBuf,

    #[arg(short = 's', long)]
    /// The filepath to the input summary csv
    summary_csv: PathBuf,

    #[arg(short = 'p', long)]
    /// The platform used to generate the data.
    /// Options: illumina or ont
    platform: String,

    #[arg(short = 'v', long)]
    /// The virus the the data was generated from.
    /// Options: flu, sc2-wgs, sc2-spike or rsv
    virus: String,

    #[arg(short = 'r', long)]
    /// The run id. Used to create custom file names associated with `run_id`.
    runid: String,

    #[arg(short = 'w', long)]
    /// The file path to the user's cloned MIRA-NF repo.
    workdir_path: PathBuf,

    #[arg(short = 'f', long)]
    /// (Optional) A flag to indicate whether to create parquet files.
    parq: bool,
}

/// Summary struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatedIRMASummary {
    pub sample_id: Option<String>,
    pub total_reads: Option<i32>,
    pub pass_qc: Option<i32>,
    pub reads_mapped: Option<i32>,
    pub reference: Option<String>,
    pub percent_reference_coverage: Option<f64>,
    pub median_coverage: Option<i32>,
    pub count_minor_snv: Option<i32>,
    pub count_minor_indel: Option<i32>,
    pub spike_percent_coverage: Option<f64>,
    pub spike_median_coverage: Option<i32>,
    pub pass_fail_reason: Option<String>,
    pub subtype: Option<String>,
    pub mira_module: Option<String>,
    pub runid: Option<String>,
    pub instrument: Option<String>,
    pub nextclade_field_1: Option<String>,
    pub nextclade_field_2: Option<String>,
    pub nextclade_field_3: Option<String>,
}

fn normalize_nextclade_field(field: &mut Option<String>) {
    let is_empty = field.as_ref().is_none_or(|s| s.trim().is_empty());

    if is_empty {
        *field = Some("Undetermined".to_string());
    }

    // replace "na" (any case) with empty string
    if let Some(val) = field.as_ref() {
        if val.eq_ignore_ascii_case("na") {
            *field = Some(String::new());
        }
    }
}

pub fn summary_report_update_process(args: &SummaryUpdateArgs) -> Result<(), Box<dyn Error>> {
    let summary_path = create_reader(&args.summary_csv)?;
    let mut summary_data: Vec<UpdatedIRMASummary> = read_csv(summary_path, true)?;

    let nextclade_data = nextclade_data_collection(&args.workdir_path, &args.virus)?;

    // Build lookup table: sample_id -> NextcladeData
    let nextclade_map: HashMap<String, NextcladeData> = nextclade_data
        .into_iter()
        .filter_map(|n| n.sample_id.clone().map(|id| (id, n)))
        .collect();

    // Merge nextclade data into summary
    for summary in &mut summary_data {
        let Some(sample_id) = &summary.sample_id else {
            continue;
        };

        if let Some(nc) = nextclade_map.get(sample_id) {
            match args.virus.as_str() {
                "flu" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                    summary.nextclade_field_2 = nc.short_clade.clone();
                    summary.nextclade_field_3 = nc.subclade.clone();

                    // If short_clade is missing → "na"
                    if summary
                        .nextclade_field_2
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())
                    {
                        summary.nextclade_field_2 = Some("na".to_string());
                    }
                }
                "sc2-wgs" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                    summary.nextclade_field_2 = nc.clade_who.clone();
                    summary.nextclade_field_3 = nc.nextclade_pango.clone();
                }
                "rsv" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                    summary.nextclade_field_2 = nc.g_clade.clone();

                    // If g_clade is missing → "na"
                    if summary
                        .nextclade_field_2
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())
                    {
                        summary.nextclade_field_2 = Some("na".to_string());
                    }
                }
                _ => {}
            }
        }

        // NIf there is no nextclade results for a sample, set fields to "Undetermined"
        normalize_nextclade_field(&mut summary.nextclade_field_1);
        normalize_nextclade_field(&mut summary.nextclade_field_2);
        normalize_nextclade_field(&mut summary.nextclade_field_3);
    }

    write_out_updated_summary_csv(&summary_data, &args.virus, &args.runid, &args.output_path)?;

    if args.parq {
        println!("Writing PARQUET files");
        write_updated_irma_summary_to_parquet(
            &summary_data,
            &args.virus,
            &format!(
                "{}/mira_{}_summary.parq",
                &args.output_path.display(),
                args.runid
            ),
        )?;
    }

    Ok(())
}
