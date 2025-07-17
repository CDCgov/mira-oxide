use clap::Parser;
use either::Either;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Stdin, Write, stdin, stdout},
    path::PathBuf,
};
use zoe::{data::fasta::FastaNT, distance::dna::NucleotidesDistance, prelude::*};

#[derive(Debug, Parser)]
#[command(
    about = "Tool for calculating hamming distances between all samples within a given fasta file"
)]
pub struct HammingArgs {
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

pub fn all_sample_hd_process(args: HammingArgs) -> std::io::Result<BufReader<Either<File, Stdin>>> {
    //let args = APDArgs::parse();
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

    let n = all_sequences.len();
    let mut matrix_cache = Vec::with_capacity(n * (n + 1) / 2);
    for (r, sequence) in all_sequences.iter().map(|v| &v.sequence).enumerate() {
        for (c, seq2) in all_sequences.iter().map(|v| &v.sequence).enumerate() {
            if r <= c {
                matrix_cache.push(sequence.distance_hamming(seq2));
            }
        }
    }

    for (r, sequence_name) in all_sequences.iter().map(|v| &v.name).enumerate() {
        write!(&mut writer, "{sequence_name}")?;
        for c in 0..n {
            let index = if r <= c {
                r * n - r * (r + 1) / 2 + c
            } else {
                c * n - c * (c + 1) / 2 + r
            };
            // This space was in the original
            write!(&mut writer, "{delim} {dist}", dist = matrix_cache[index])?;
        }
        writeln!(&mut writer)?;
    }

    Ok(())
}
