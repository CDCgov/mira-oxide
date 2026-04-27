#![allow(
    unreachable_patterns,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::upper_case_acronyms
)]
use crate::processes::{
    all_sample_hd::{HammingArgs, all_sample_hd_process},
    all_sample_nt_diffs::{NTDiffsArgs, all_sample_nt_diffs_process},
    check_mira_version::{MiraVersionArgs, check_mira_version},
    create_nextflow_samplesheet::{SamplesheetArgs, create_nextflow_samplesheet},
    di_stats::{DIStatArgs, di_stats_process},
    find_chemistry::{FindChemArgs, find_chemistry_process},
    nf_status::{NfStatusArgs, build_status_report, default_pipeline_dir, default_state_dir},
    plotter::{PlotterArgs, plotter_process},
    positions_of_interest::{PositionsArgs, positions_of_interest_process},
    prepare_mira_reports::{ReportsArgs, prepare_mira_reports_process},
    samplesheet_check::{SamplesheetCheckArgs, samplesheet_check},
    summary_report_update::{SummaryUpdateArgs, summary_report_update_process},
    variants_of_interest::{VariantsArgs, variants_of_interest_process},
};
use crate::web::{ServeArgs, render_nf_status_document, serve};
use clap::{Parser, Subcommand};
use zoe::prelude::OrFail;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Variants of Interest
    VariantsOfInterest(VariantsArgs),
    /// Positions of Interest
    PositionsOfInterest(PositionsArgs),
    /// Find Chemistry
    FindChemistry(FindChemArgs),
    /// Hamming
    Hamming(HammingArgs),
    /// NT diffs
    NTDiffs(NTDiffsArgs),
    /// Plotter
    Plotter(PlotterArgs),
    /// Check mira version
    CheckMiraVersion(MiraVersionArgs),
    /// Prepare MIRA report
    PrepareMiraReports(ReportsArgs),
    /// Summary report update
    SummaryReportUpdate(SummaryUpdateArgs),
    /// Create Nextflow samplesheet
    CreateNextflowSamplesheet(SamplesheetArgs),
    /// Samplesheet Nextflow format validation
    SamplesheetCheck(SamplesheetCheckArgs),
    /// DI Stats
    DIStats(DIStatArgs),
    /// Serve the local browser UI
    Serve(ServeArgs),
    /// Render HTML status for the latest or a selected Nextflow run
    NfStatus(NfStatusArgs),
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let module = module_path!();

    match args.command {
        Commands::VariantsOfInterest(cmd_args) => {
            variants_of_interest_process(cmd_args)
                .unwrap_or_else(|_| panic!("{module}::VariantsOfInterest"));
        }
        Commands::PositionsOfInterest(cmd_args) => {
            positions_of_interest_process(cmd_args)
                .unwrap_or_else(|_| panic!("{module}::PositionsOfInterest"));
        }
        Commands::FindChemistry(cmd_args) => {
            find_chemistry_process(&cmd_args).unwrap_or_die(&format!("{module}::FindChemistry"));
        }
        Commands::Hamming(cmd_args) => {
            all_sample_hd_process(&cmd_args).unwrap_or_die(&format!("{module}::Hamming"));
        }
        Commands::NTDiffs(cmd_args) => all_sample_nt_diffs_process(&cmd_args),
        Commands::Plotter(cmd_args) => {
            plotter_process(cmd_args).unwrap_or_else(|_| panic!("{module}::Plotter"));
        }
        Commands::CheckMiraVersion(cmd_args) => {
            check_mira_version(&cmd_args).unwrap_or_die(&format!("{module}::CheckMiraVersion"));
        }
        Commands::PrepareMiraReports(cmd_args) => {
            prepare_mira_reports_process(&cmd_args)
                .unwrap_or_else(|e| panic!("{module}::PrepareMiraReports: {e}"));
        }
        Commands::SummaryReportUpdate(cmd_args) => {
            summary_report_update_process(&cmd_args)
                .unwrap_or_else(|e| panic!("{module}::SummaryReportUpdate: {e}"));
        }
        Commands::CreateNextflowSamplesheet(cmd_args) => {
            create_nextflow_samplesheet(&cmd_args)
                .unwrap_or_die(&format!("{module}::CreateNextflowSamplesheet"));
        }
        Commands::SamplesheetCheck(cmd_args) => {
            samplesheet_check(&cmd_args).unwrap_or_die(&format!("{module}::SamplesheetCheck"));
        }
        Commands::DIStats(cmd_args) => {
            di_stats_process(&cmd_args).unwrap_or_die(&format!("{module}::DIStats"));
        }
        Commands::Serve(cmd_args) => {
            serve(cmd_args)
                .await
                .unwrap_or_else(|e| panic!("{module}::Serve: {e}"));
        }
        Commands::NfStatus(cmd_args) => {
            let state_dir = if cmd_args.state_dir.as_os_str().is_empty() {
                default_state_dir()
            } else {
                cmd_args.state_dir.clone()
            };
            let pipeline_dir = if cmd_args.pipeline_dir.as_os_str().is_empty() {
                default_pipeline_dir()
            } else {
                cmd_args.pipeline_dir.clone()
            };
            let report = build_status_report(&state_dir, &pipeline_dir, cmd_args.run_id.as_deref())
                .unwrap_or_else(|e| panic!("{module}::NfStatus: {e}"));
            print!("{}", render_nf_status_document(&report, None));
        }
    }
}

pub mod constants;
pub mod io;
pub mod processes;
pub mod utils;
pub mod web;
