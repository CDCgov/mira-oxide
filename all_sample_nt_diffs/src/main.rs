use clap::Parser;
use either::Either;
use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Write, stdin, stdout},
    path::PathBuf,
};
use zoe::{
    data::fasta::FastaNT,
    prelude::*,
};

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

fn main() {
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
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_die("Could not process other data.");
    
        writeln!(
            &mut writer,
            "sequence_1{}sequence_2{}nt_sequence_1{}position{}nt_sequence_2",
            delim, delim, delim, delim
        ).unwrap();

        all_sequences.iter().for_each(|f| {
            let name_1 = &f.name;
            let seq1 = &f.sequence;
            all_sequences.iter().for_each(|f| {
                let name_2 = &f.name;
                let seq2 = &f.sequence;
                for (i, (nt1, nt2)) in seq1.iter().zip(seq2.iter()).enumerate() {
                    if nt1 != nt2 {
                        let nucleotide1 = char::from(*nt1);
                        let nucleotide2 = char::from(*nt2);
                        writeln!(
                            &mut writer,
                            "{}{}{}{}{}{}{}{}{}",
                            name_1, delim, name_2, delim, nucleotide1, delim, i, delim, nucleotide2
                        )
                        .unwrap();
                    }
                }
            });
        });
        

}
