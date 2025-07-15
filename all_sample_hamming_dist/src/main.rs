use clap::Parser;
use either::Either;
use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Write, stdin, stdout},
    path::PathBuf,
};
use zoe::{data::fasta::FastaNT, distance::dna::NucleotidesDistance, prelude::*};

#[derive(Debug, Parser)]
#[command(
    about = "Tool for calculating hamming distances between all samples within a given fasta file"
)]
pub struct APDArgs {
    #[arg(short = 'i', long)]
    /// Input fasta
    input_fasta: Option<PathBuf>,

    #[arg(short = 'o', long)]
    /// Optional output delimited file
    output_xsv: Option<PathBuf>,

    #[arg(short = 'd', long)]
    /// Use the provider delimiter for separating fields. Default is ','
    output_delimiter: Option<char>,
}

#[derive(Debug)]
struct ValidSeq {
    name: String,
    sequence: Nucleotides,
}

fn main() -> Result<(), std::io::Error> {
    let args = APDArgs::parse();
    let delim = args.output_delimiter.unwrap_or(',');

    //read in fasta file
    let reader = if let Some(ref file_path) = args.input_fasta {
        FastaReader::new(BufReader::new(Either::Left(
            OpenOptions::new()
                .read(true)
                .open(file_path)
                .expect("File opening error"),
        )))
    } else {
        FastaReader::new(BufReader::new(Either::Right(stdin())))
    };

    //output
    let mut writer = if let Some(ref file_path) = args.output_xsv {
        BufWriter::new(Either::Left(
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(file_path)
                .expect("File write error"),
        ))
    } else {
        BufWriter::new(Either::Right(stdout()))
    };

    let all_sequences = reader
        .map(|record|
            // TODO: don't translate, instead defer until later
            record.map(|r| {
                let FastaNT { name, sequence } = r.recode_to_dna();
                ValidSeq {
                    name,
                    sequence,
                }
              }))
        .collect::<Result<Vec<_>, _>>()?;

    write!(&mut writer, "sequences")?;
    for query_header in all_sequences.iter().map(|f| f.name.as_str()) {
        write!(&mut writer, "{delim}{query_header}")?;
    }
    writeln!(&mut writer)?;

    let mut row = vec![0; all_sequences.len()];
    // This can be made more efficient by caching the matrix
    for ValidSeq { name, sequence } in all_sequences.iter() {
        for (i, seq2) in all_sequences.iter().map(|v| &v.sequence).enumerate() {
            row[i] = sequence.distance_hamming(seq2);
        }
        write!(&mut writer, "{name}",)?;
        for r in row.iter() {
            // There were spaces in the original
            write!(&mut writer, "{delim} {r}")?;
        }
        writeln!(&mut writer)?;
    }

    Ok(())
}
