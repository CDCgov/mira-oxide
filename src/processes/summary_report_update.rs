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
    about = "Package for adding the nextclade clade results back into the summary.csv and summary.parq files created by MIRA"
)]
pub struct SummaryUpdateArgs {
    #[arg(short = 'i', long)]
    nextclade_path: PathBuf,

    #[arg(short = 'o', long)]
    output_path: PathBuf,

    #[arg(short = 's', long)]
    summary_csv: PathBuf,

    #[arg(short = 'v', long)]
    virus: String,

    #[arg(short = 'r', long)]
    runid: String,

    #[arg(short = 'n', long)]
    nextclade_version: String,

    #[arg(short = 'd', long)]
    nextclade_dataset: String,

    #[arg(short = 't', long)]
    nextclade_tag: String,

    #[arg(short = 'f', long)]
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
    pub count_minor_snv_at_or_over_5_pct: Option<i32>,
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
    pub nextclade_info: Option<String>,
}

fn normalize_nextclade_field(field: &mut Option<String>) {
    if let Some(val) = field.as_ref()
        && val.eq_ignore_ascii_case("na")
    {
        *field = Some(String::new());
    }
}

pub fn summary_report_update_process(args: &SummaryUpdateArgs) -> Result<(), Box<dyn Error>> {
    println!("Starting data ingestion...");
    let summary_path = create_reader(&args.summary_csv)?;
    let mut summary_data: Vec<UpdatedIRMASummary> = read_csv(summary_path, true)?;
    let nextclade_data = nextclade_data_collection(&args.nextclade_path, &args.virus)?;
    println!("Finished ingesting data.");

    let nextclade_info_value = format!(
        "{};{};{}",
        args.nextclade_version, args.nextclade_dataset, args.nextclade_tag
    );

    let nextclade_map: HashMap<String, NextcladeData> = nextclade_data
        .into_iter()
        .filter_map(|n| n.sample_id.clone().map(|id| (id, n)))
        .collect();

    for summary in &mut summary_data {
        let Some(sample_id) = &summary.sample_id else {
            continue;
        };

        if let Some(nc) = nextclade_map.get(sample_id) {
            match args.virus.as_str() {
                "flu" => {
                    let has_ha = summary.reference.as_ref().is_some_and(|r| r.contains("HA"));

                    if has_ha {
                        summary.nextclade_field_1 = nc.clade.clone();
                        summary.nextclade_field_2 = nc.short_clade.clone();
                        summary.nextclade_field_3 = nc.subclade.clone();

                        if summary
                            .nextclade_field_2
                            .as_ref()
                            .is_none_or(|s| s.trim().is_empty())
                        {
                            summary.nextclade_field_2 = Some("na".to_string());
                        }
                    } else {
                        summary.nextclade_field_1 = Some("na".to_string());
                        summary.nextclade_field_2 = Some("na".to_string());
                        summary.nextclade_field_3 = Some("na".to_string());
                    }
                }

                "sc2-wgs" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                    summary.nextclade_field_2 = nc.clade_who.clone();
                    summary.nextclade_field_3 = nc.nextclade_pango.clone();
                }

                "rsv" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                }

                _ => {}
            }
        }

        // Normalize fields
        normalize_nextclade_field(&mut summary.nextclade_field_1);
        normalize_nextclade_field(&mut summary.nextclade_field_2);
        normalize_nextclade_field(&mut summary.nextclade_field_3);

        // Only print nextclade_info if nextclade_field_1 is not empty
        if summary
            .nextclade_field_1
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            summary.nextclade_info = Some(nextclade_info_value.clone());
        } else {
            summary.nextclade_info = Some("".to_string());
        }
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
