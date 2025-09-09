use clap::{Parser, ValueEnum, builder::PossibleValue};
use flate2::read::MultiGzDecoder;
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use zoe::{
    define_whichever,
    prelude::{FastQReader, Len},
};

pub(crate) fn is_gz<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().extension().is_some_and(|ext| ext == "gz")
}

#[inline]
pub(crate) fn open_fastq_file<P: AsRef<Path>>(
    path: P,
) -> std::io::Result<FastQReader<ReadFileZip>> {
    let file = File::open(&path)?;

    if is_gz(&path) {
        Ok(FastQReader::from_readable(ReadFileZip::Zipped(
            MultiGzDecoder::new(file),
        ))?)
    } else {
        Ok(FastQReader::from_readable(ReadFileZip::File(file))?)
    }
}

#[derive(Debug, Parser)]
#[command(about = "Get relevant IRMA configuration and modules for the current experiment.")]
pub struct FindChemArgs {
    #[arg(short = 's', long)]
    /// Name of sample
    pub sample: String,

    #[arg(short = 'q', long)]
    /// Path to fastq file
    pub fastq: PathBuf,

    #[arg(short = 'e', long, ignore_case = true)]
    /// Experiment type
    pub experiment: Experiment,

    #[arg(short = 'p', long)]
    /// Path to working directory
    pub wd_path: PathBuf,

    #[arg(short = 'c', long)]
    /// Read counts
    pub read_count: usize,

    #[arg(short = 'i', long, ignore_case = true, default_value = "None")]
    /// Alternative IRMA config. To use Sensitive, Secondary, or UTR, the
    /// experiment type must be Flu-Illumina or Flu-ONT.
    pub irma_config: IRMAConfig,

    #[arg(short = 'g', long)]
    /// Custom irma config path
    pub irma_config_path: Option<PathBuf>,
}

impl FindChemArgs {
    /// Function for ensuring that specific IRMA configs are only used with the
    /// proper experiment. Secondary, Sensitive, and UTR must be matched with a
    /// Flu experiment.
    fn validate(&self) -> Result<(), String> {
        match self.irma_config {
            IRMAConfig::Sensitive | IRMAConfig::Secondary | IRMAConfig::UTR
                if self.experiment == Experiment::FluIllumina
                    || self.experiment == Experiment::FluONT =>
            {
                Ok(())
            }
            IRMAConfig::Custom | IRMAConfig::NoConfig => Ok(()),
            _ => Err(format!(
                "Invalid combination: {:?} cannot be used with {:?}",
                self.experiment, self.irma_config
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Enum for the possible experiment types, both Illumina and ONT
pub enum Experiment {
    FluIllumina,
    SC2WholeGenomeIllumina,
    RSVIllumina,
    FluONT,
    SC2SpikeOnlyONT,
    SC2WholeGenomeONT,
    RSVONT,
}

impl ValueEnum for Experiment {
    #[inline]
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::FluIllumina,
            Self::SC2WholeGenomeIllumina,
            Self::RSVIllumina,
            Self::FluONT,
            Self::SC2SpikeOnlyONT,
            Self::SC2WholeGenomeONT,
            Self::RSVONT,
        ]
    }

    #[inline]
    /// Provides the literal strings for the users to input to get these enum
    /// variants
    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Experiment::FluIllumina => {
                Some(PossibleValue::new("Flu-Illumina").alias("FluIllumina"))
            }
            Experiment::SC2WholeGenomeIllumina => Some(
                PossibleValue::new("SC2-Whole-Genome-Illumina").alias("SC2WholeGenomeIllumina"),
            ),
            Experiment::RSVIllumina => {
                Some(PossibleValue::new("RSV-Illumina").alias("RSVIllumina"))
            }
            Experiment::FluONT => Some(PossibleValue::new("Flu-ONT").alias("FluONT")),
            Experiment::SC2SpikeOnlyONT => {
                Some(PossibleValue::new("SC2-Spike-Only-ONT").alias("SC2SpikeOnlyONT"))
            }
            Experiment::SC2WholeGenomeONT => {
                Some(PossibleValue::new("SC2-Whole-Genome-ONT").alias("SC2WholeGenomeONT"))
            }
            Experiment::RSVONT => Some(PossibleValue::new("RSV-ONT").alias("RSVONT")),
        }
    }
}

impl Experiment {
    /// Selects the appropriate IRMA Module from the user provided experiment type
    fn get_module(self) -> IrmaModule {
        match self {
            Self::FluIllumina => IrmaModule::FLU,
            Self::RSVIllumina | Self::RSVONT => IrmaModule::RSV,
            Self::SC2WholeGenomeIllumina | Self::SC2WholeGenomeONT => IrmaModule::CoV,
            Self::FluONT => IrmaModule::FLUMinion,
            Self::SC2SpikeOnlyONT => IrmaModule::CoVsGene,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IRMAConfig {
    Sensitive,
    Secondary,
    UTR,
    Custom,
    NoConfig,
}

impl ValueEnum for IRMAConfig {
    #[inline]
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Sensitive,
            Self::Secondary,
            Self::UTR,
            Self::Custom,
            Self::NoConfig,
        ]
    }

    #[inline]
    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            IRMAConfig::Sensitive => Some(PossibleValue::new("Sensitive")),
            IRMAConfig::Secondary => Some(PossibleValue::new("Secondary")),
            IRMAConfig::UTR => Some(PossibleValue::new("UTR")),
            IRMAConfig::Custom => Some(PossibleValue::new("Custom")),
            IRMAConfig::NoConfig => Some(PossibleValue::new("NoConfig").alias("None")),
        }
    }
}

/// Selects the correct config file based on experiment, custom config path, and
/// length of sequences
fn get_config_path(args: &FindChemArgs, seq_len: Option<usize>) -> String {
    if args.irma_config == IRMAConfig::Custom {
        return args
            .irma_config_path
            .as_ref()
            .expect("Custom IRMA config specified but no path to config file was provided")
            .to_str()
            .expect("Failed to convert IRMA config path to string")
            .to_string();
    }

    let path_extension = match (args.experiment, seq_len, args.irma_config) {
        (_, None, _) => return String::new(),
        (_, _, IRMAConfig::Sensitive) => "/bin/irma_config/FLU-sensitive.sh",
        (_, _, IRMAConfig::Secondary) => "/bin/irma_config/FLU-secondary.sh",
        (_, _, IRMAConfig::UTR) => "/bin/irma_config/FLU-utr.sh",
        (_, _, IRMAConfig::Custom) => unreachable!(),
        (Experiment::FluIllumina, Some(seq_len), IRMAConfig::NoConfig) => {
            if seq_len >= 145 {
                "/bin/irma_config/FLU.sh"
            } else {
                "/bin/irma_config/FLU-2x75.sh"
            }
        }
        (Experiment::SC2WholeGenomeIllumina, Some(seq_len), IRMAConfig::NoConfig) => {
            if seq_len > 80 {
                "/bin/irma_config/CoV.sh"
            } else {
                "/bin/irma_config/SC2-2x75.sh"
            }
        }
        (Experiment::RSVIllumina, Some(seq_len), IRMAConfig::NoConfig) => {
            if seq_len > 80 {
                "/bin/irma_config/RSV.sh"
            } else {
                "/bin/irma_config/RSV-2x75.sh"
            }
        }
        (Experiment::FluONT, _, IRMAConfig::NoConfig) => "/bin/irma_config/FLU-minion-container.sh",
        (Experiment::SC2SpikeOnlyONT, _, IRMAConfig::NoConfig) => {
            "/bin/irma_config/s-gene-container.sh"
        }
        (Experiment::SC2WholeGenomeONT, _, IRMAConfig::NoConfig) => {
            "/bin/irma_config/SC2-WGS-Nanopore.sh"
        }
        (Experiment::RSVONT, _, IRMAConfig::NoConfig) => "/bin/irma_config/RSV-Nanopore.sh",
    };

    let wd_path = args
        .wd_path
        .to_str()
        .expect("Failed to convert work directory path to string");
    format!("{wd_path}{path_extension}")
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

impl fmt::Display for ChemistryOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            self.sample, self.irma_custom, self.subsample, self.irma_module
        )
    }
}

/// Averages the first five sequence lengths if possible. If the file has no
/// sequences, returns None
fn get_average_line_length<P: AsRef<Path>>(fastq_path: P) -> Result<Option<usize>, std::io::Error> {
    const SAMPLE_SIZE: usize = 5;

    let fastq_reader = open_fastq_file(fastq_path)?;

    let mut total_len = 0;
    let mut count = 0;

    for result in fastq_reader.take(SAMPLE_SIZE) {
        let record = result?;
        total_len += record.sequence.len();
        count += 1;
    }

    if count == 0 {
        Ok(None)
    } else {
        Ok(Some(total_len / count))
    }
}

/// Takes user input arguments and prepares them for output
fn parse_chemistry_args(args: &FindChemArgs) -> Result<ChemistryOutput, std::io::Error> {
    let line_length = get_average_line_length(&args.fastq)?;
    let irma_custom = get_config_path(args, line_length);
    let irma_module = args.experiment.get_module();
    let out = ChemistryOutput {
        sample: args.sample.clone(),
        irma_custom,
        subsample: args.read_count,
        irma_module,
    };
    Ok(out)
}

pub fn find_chemistry_process(args: &FindChemArgs) -> Result<(), std::io::Error> {
    //let args = CheckChemArgs::parse();
    // handle input validation to ensure valid combinations of
    if let Err(e) = args.validate() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
    // parse the arguments into output format
    let output = parse_chemistry_args(args)?;
    let filename = format!("{}_chemistry.csv", args.sample);
    let headers = "sample_ID,irma_custom,subsample,irma_module";

    let mut writer = {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)?;
        BufWriter::new(file)
    };
    writeln!(&mut writer, "{headers}")?;
    writeln!(&mut writer, "{output}")?;
    writer.flush()?;
    Ok(())
}
