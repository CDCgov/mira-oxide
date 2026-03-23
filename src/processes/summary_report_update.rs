#![allow(dead_code, unused_imports)]
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
};

use clap::{Parser, ValueHint};
use serde::{Deserialize, Serialize};

use crate::{
    io::{
        coverage_json_per_sample::SampleCoverageJson,
        create_statichtml::{
            generate_html_report, irma_summary_to_plotly_json, plotly_table_script,
            update_irma_summary_to_plotly_json, update_summary_in_html,
        },
        data_ingest::{
            IndelsData, MinorVariantsData, NextcladeData, create_reader, nextclade_data_collection,
            read_csv,
        },
        reads_to_sankey_json::SampleSankeyJson,
        write_csv_files::write_out_updated_summary_csv,
        write_parquet_files::write_updated_irma_summary_to_parquet,
    },
    utils::data_processing::{DaisVarsData, IRMASummary},
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

    #[arg(short = 't', long)]
    static_html_path: PathBuf,

    #[arg(short = 'v', long)]
    virus: String,

    #[arg(short = 'r', long)]
    runid: String,

    #[arg(short = 'n', long)]
    nextclade_version: String,

    /// One or more tuples: dataset,tag,path
    #[arg(short = 'm', long, value_parser, num_args = 1..)]
    nextclade_metadata: Vec<NextcladeMetadata>,

    #[arg(short = 'f', long)]
    parq: bool,
}

#[derive(Debug, Clone)]
pub struct NextcladeMetadata {
    pub dataset: String,
    pub tag: String,
}

impl std::str::FromStr for NextcladeMetadata {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        let parts: Vec<&str> = s.splitn(2, ',').collect();
        if parts.len() != 2 {
            return Err(format!("Expected dataset,tag got '{s}'"));
        }

        Ok(Self {
            dataset: parts[0].trim().trim_start_matches('[').to_string(),
            tag: parts[1]
                .trim()
                .trim_end_matches(',')
                .trim_end_matches(']')
                .to_string(),
        })
    }
}

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
    #[serde(rename = "mira_version;module;irma_config")]
    pub mira_module: Option<String>,
    pub runid: Option<String>,
    pub instrument: Option<String>,
    pub di_ratios_5prime_3prime: Option<String>,
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

#[allow(clippy::too_many_lines)]
pub fn summary_report_update_process(args: &SummaryUpdateArgs) -> Result<(), Box<dyn Error>> {
    println!("Starting data ingestion...");
    let summary_path = create_reader(&args.summary_csv)?;
    let mut summary_data: Vec<UpdatedIRMASummary> = read_csv(summary_path, true)?;

    let nextclade_data: Vec<NextcladeData> =
        nextclade_data_collection(&args.nextclade_path, &args.virus)?;

    println!("Finished ingesting data.");

    let nextclade_map: HashMap<String, NextcladeData> = nextclade_data
        .into_iter()
        .filter_map(|n| n.sample_id.clone().map(|id| (id, n)))
        .collect();

    let metadata_map: HashMap<&str, &NextcladeMetadata> = args
        .nextclade_metadata
        .iter()
        .map(|m| (m.dataset.as_str(), m))
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
                        summary.nextclade_field_1 = nc.short_clade.clone();
                        summary.nextclade_field_2 = nc.subclade.clone();
                        if summary
                            .nextclade_field_1
                            .as_ref()
                            .map_or(true, |s| s.trim().is_empty())
                        {
                            summary.nextclade_field_1 = Some("na".to_string());
                        }
                        if summary
                            .nextclade_field_2
                            .as_ref()
                            .map_or(true, |s| s.trim().is_empty())
                        {
                            summary.nextclade_field_2 = Some("na".to_string());
                        }
                    } else {
                        summary.nextclade_field_1 = Some("na".to_string());
                        summary.nextclade_field_2 = Some("na".to_string());
                    }
                }
                "sc2-wgs" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                    summary.nextclade_field_2 = nc.clade_who.clone();
                    summary.nextclade_field_3 = nc.nextclade_pango.clone();

                    if summary
                        .nextclade_field_1
                        .as_ref()
                        .map_or(true, |s| s.trim().is_empty())
                    {
                        summary.nextclade_field_1 = Some("na".to_string());
                    }
                    if summary
                        .nextclade_field_2
                        .as_ref()
                        .map_or(true, |s| s.trim().is_empty())
                    {
                        summary.nextclade_field_2 = Some("na".to_string());
                    }
                    if summary
                        .nextclade_field_3
                        .as_ref()
                        .map_or(true, |s| s.trim().is_empty())
                    {
                        summary.nextclade_field_3 = Some("na".to_string());
                    }
                }
                "rsv" => {
                    summary.nextclade_field_1 = nc.clade.clone();
                    if summary
                        .nextclade_field_1
                        .as_ref()
                        .map_or(true, |s| s.trim().is_empty())
                    {
                        summary.nextclade_field_1 = Some("na".to_string());
                    }
                    summary.nextclade_field_2.get_or_insert("na".to_string());
                    summary.nextclade_field_3.get_or_insert("na".to_string());
                }
                _ => {}
            }

            normalize_nextclade_field(&mut summary.nextclade_field_1);
            normalize_nextclade_field(&mut summary.nextclade_field_2);
            normalize_nextclade_field(&mut summary.nextclade_field_3);

            let field_1_is_nonempty = summary
                .nextclade_field_1
                .as_ref()
                .map_or(false, |s| !s.trim().is_empty());
            if field_1_is_nonempty {
                if let Some(nc_dataset) = &nc.dataset
                    && let Some(metadata_match) = metadata_map.get(nc_dataset.as_str())
                    && metadata_match.dataset == *nc_dataset
                {
                    summary.nextclade_info = Some(format!(
                        "{};{};{}",
                        args.nextclade_version, metadata_match.dataset, metadata_match.tag
                    ));
                } else {
                    summary.nextclade_info = Some("dataset_mismatch".to_string());
                }
            } else {
                summary.nextclade_info = Some(String::new());
            }
        } else {
            summary.nextclade_info = Some(String::new());
            summary.nextclade_field_1.get_or_insert(String::new());
            summary.nextclade_field_2.get_or_insert(String::new());
            summary.nextclade_field_3.get_or_insert(String::new());
        }
    }

    // Write CSV and PARQUET outputs
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

    // Udpating the StaticHTML
    let summary_json = update_irma_summary_to_plotly_json(&summary_data, &args.virus);

    let new_summary_html =
        plotly_table_script("irma_summary_table", &summary_json, "MIRA Summary Table");

    update_summary_in_html(&args.static_html_path, &new_summary_html)?;

    Ok(())
}
