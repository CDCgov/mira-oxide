use std::path::PathBuf;

use clap::{Args, ValueEnum, builder::PossibleValue};

#[derive(Args, Debug)]
pub struct CheckChemArgs {
    #[arg(short = 's', long)]
    /// Name of sample
    pub sample: String,
    
    #[arg(short = 'q', long)]
    /// Path to fastq file
    pub fastq: PathBuf,

    #[arg(short = 'r', long)]
    /// Run ID
    pub run_id: usize,

    #[arg(short = 'e', long)]
    /// Experiment type
    pub experiment: Experiment,

    #[arg(short = 'p', long)]
    /// Path to working directory
    pub wd_path: PathBuf,

    #[arg(short = 'c', long)]
    /// Read counts
    pub read_count: usize,

    #[arg(short = 'i', long)]
    /// Alternative IRMA config
    pub irma_config: Option<IRMAConfig>,

    #[arg(short = 'g', long)]
    /// Custom irma config path
    pub irma_config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Experiment {
    FluIllumina,
    SC2WholeGenomeIllumina,
    RSVIllumina,
}

impl ValueEnum for Experiment {
    #[inline]
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::FluIllumina,
            Self::SC2WholeGenomeIllumina,
            Self::RSVIllumina,
        ]
    }

    #[inline]
    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Experiment::FluIllumina => Some(PossibleValue::new("FluIllumina")),
            Experiment::SC2WholeGenomeIllumina => Some(PossibleValue::new("SC2WholeGenomeIllumina")),
            Experiment::RSVIllumina => Some(PossibleValue::new("RSVIllumina")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IRMAConfig {
    Sensitive,
    Secondary,
    UTR,
    Custom,
}

impl ValueEnum for IRMAConfig {
    #[inline]
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Sensitive,
            Self::Secondary,
            Self::UTR,
            Self::Custom,
        ]
    }

    #[inline]
    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            IRMAConfig::Sensitive => Some(PossibleValue::new("Sensitive")),
            IRMAConfig::Secondary => Some(PossibleValue::new("Secondary")),
            IRMAConfig::UTR => Some(PossibleValue::new("UTR")),
            IRMAConfig::Custom => Some(PossibleValue::new("Custom")),
        }
    }
}

fn main() {
    todo!()
    // outputs: headers for a "chemistry csv"
    // sample, irma_custom_0, irma_custom_1, subsample_ IRMA_module
}