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

fn hamm(seq1: &Nucleotides, seq2: &Nucleotides) -> usize {
    seq1.into_iter()
        .zip(seq2)
        .filter(|(c1, c2)| c1 != c2)
        .count()
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
    
    //write out header
    let mut buffer = format!("seqeunces");
    let mut header_hold: Vec<String> = Vec::new();
    for query_header in all_sequences.iter().map(|f| f.name.as_str()) {
        buffer.push(delim);
        buffer.push_str(query_header);
        header_hold.push(query_header.to_string());
    }
    writeln!(&mut writer, "{buffer}").unwrap_or_fail();

    //Initialize vectors to store headers and sequences
    all_sequences.iter().for_each(|f| {
        let mut distances = format!("{seq_name}", seq_name = &f.name);
        let seqs_1 = &f.sequence;
        for seqs_2 in all_sequences.iter().map(|f| &f.sequence) {
            let total_mismatch = hamm(seqs_1, seqs_2);
            distances.push(delim);
            distances.push_str(&format!(" {}", total_mismatch));
            
        }
        writeln!(
            &mut writer,
            "{distances}",
        )
        .unwrap_or_fail();
    }); 
        

}
