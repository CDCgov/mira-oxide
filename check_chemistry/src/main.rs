use std::{fmt, fs::File, io::BufReader, path::PathBuf};
use zoe::prelude::*;
use clap::{builder::PossibleValue, Parser, ValueEnum};


#[derive(Debug, Parser)]
#[command(about = "Tool for calculating amino acid difference tables")]
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

#[derive(Debug)]
pub enum IrmaModule {
    FLU,
    CoV,
    RSV,
    FLUMinion,
    CoVsGene,
}

impl fmt::Display for IrmaModule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IrmaModule::FLU => write!(f, "FLU"),
            IrmaModule::CoV => write!(f, "CoV"),
            IrmaModule::RSV => write!(f, "RSV"),
            IrmaModule::FLUMinion => write!(f, "FLU-minion"),
            IrmaModule::CoVsGene => write!(f, "CoV-s-gene"),
        }
    }
}

#[derive(Debug)]
pub struct ChemistryOutput {
    pub sample: String,
    pub irma_custom: String,
    pub subsample: usize,
    pub irma_module: IrmaModule,
}

fn get_average_line_length(fastq: &PathBuf) -> Result<usize, std::io::Error> {
    let sample_size = 5;
    let file = File::open(&fastq)?;
    let buf_reader = BufReader::new(file);
    let fastq_reader = FastQReader::new(buf_reader);
    let avg_length = fastq_reader.take(sample_size).map(|fastq| {
        let fastq = fastq.expect("Could not read fastq!");
        fastq.sequence.len()
    }).sum::<usize>() as f64 / sample_size as f64;
    Ok(avg_length as usize)
}

fn parse_chemistry_args(args: &CheckChemArgs) -> Result<ChemistryOutput, std::io::Error> {
    let _line_length = get_average_line_length(&args.fastq);
    let out = ChemistryOutput {
        sample: "".to_owned(),
        irma_custom: "".to_owned(),
        subsample: 0usize,
        irma_module: IrmaModule::CoV,
    };
    Ok(out)
}

fn main() {
    let args= CheckChemArgs::parse();
    let args = parse_chemistry_args(&args).unwrap();
    println!("{:?}", args);
    // outputs: headers for a "chemistry csv"
    // sample, irma_custom_0, irma_custom_1, subsample, IRMA_module
}