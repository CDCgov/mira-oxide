#![allow(unused_variables)]
use clap::Parser;
use either::Either;
use std::{
    fs::{File, OpenOptions},
    io::{stdin, stdout, BufRead, BufReader, BufWriter, Stdin, Write},
    path::PathBuf,
};
use zoe::alignment::sw::sw_scalar_alignment;
use zoe::alignment::{ScalarProfile, pairwise_align_with_cigar};
use zoe::data::{ByteIndexMap, WeightMatrix};
use zoe::prelude::*;

#[derive(Debug, Parser)]
#[command(about = "Tool for calculating amino acid difference tables")]
pub struct APDArgs {
    #[arg(short = 'i', long)]
    /// Optional input fasta
    input_file: Option<PathBuf>,

    #[arg(short = 'r', long)]
    /// Optional input fasta
    ref_file: Option<PathBuf>,

    #[arg(short = 'm', long)]
    /// Optional input fasta
    muts_file: Option<PathBuf>,

    #[arg(short = 'o', long)]
    /// Optional output delimited file
    output_xsv: Option<PathBuf>,

    #[arg(short = 'd', long, default_value = ",")]
    /// Use the provider delimiter for separating fields. Default is ','
    output_delimiter: String,
}

fn create_reader(path: Option<PathBuf>) -> std::io::Result<BufReader<Either<File, Stdin>>> {
    let reader = if let Some(ref file_path) = path {
        let file = OpenOptions::new().read(true).open(file_path)?;
        BufReader::new(Either::Left(file))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    Ok(reader)
}

pub fn lines_to_vec<R: BufRead>(reader: R) -> std::io::Result<Vec<Vec<String>>> {
    let mut columns: Vec<Vec<String>> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let values: Vec<_> = line.split('\t').map(str::to_owned).collect();

        if columns.is_empty() {
            columns.resize_with(values.len(), Vec::new);
        }

        for (col, val) in columns.iter_mut().zip(values) {
            col.push(val);
        }
    }

    Ok(columns)
}

fn main() -> std::io::Result<()> {
    let args = APDArgs::parse();
    let delim = args.output_delimiter;

    //read in input file (dais results)
    let reader = create_reader(args.input_file)?;

    // Initialize vectors to store columns for the input file (dais results)
    // TODO convert to bytes fr fr ong ong
    let columns = lines_to_vec(reader)?;

    //read in reference file (reference cvv and zoonotic strains)
    let ref_reader = create_reader(args.ref_file)?;

    // Initialize vectors to store columns (reference cvv and zoonotic strains)
    let ref_columns: Vec<Vec<String>> = lines_to_vec(ref_reader)?;

    //read in mutations file (mutations of interest)
    let muts_reader = create_reader(args.muts_file)?;

    // Initialize vectors to store columns (mutations of interest)
    let muts_columns: Vec<Vec<String>> = lines_to_vec(muts_reader)?;

    let mut writer = if let Some(ref file_path) = args.output_xsv {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)?;
        BufWriter::new(Either::Left(file))
    } else {
        BufWriter::new(Either::Right(stdout()))
    };

    //header of output file
    writeln!(
        &mut writer,
        "sample{delim}ref_strain{delim}gisaid_accession{delim}subtype{delim}dais_reference{delim}protein{delim}aa_reference{delim}aa_postion{delim}aa_mutation{delim}phenotypic_consequences"
    )
    .unwrap_or_fail();

    //Finding reference sequences in the same coordinate space to compare with
    for i in 0..columns[1].len() {
        for j in 0..ref_columns[1].len() {
            if columns[1][i] == ref_columns[5][j]
                && columns[2][i] == ref_columns[6][j]
                && columns[3][i] == ref_columns[7][j]
            {
                let aa_seq1 = ref_columns[8][j].as_bytes();
                let aa_seq2 = columns[6][i].as_bytes();
                //If aa seq are the same length start seqe comparison
                //aa seqs that are the same length will be aligned already and saves time to not align
                if aa_seq1.len() == aa_seq2.len() {
                    let mut position = 0;
                    let sample_id = columns[0][i].to_string();
                    let ref_strain = ref_columns[1][j].to_string();
                    let gisaid_accession = ref_columns[0][j].to_string();
                    let subtype = columns[1][i].to_string();
                    let dais_ref = columns[2][i].to_string();
                    let protein = columns[3][i].to_string();
                    for aa in 0..aa_seq1.len() {
                        position = position + 1;

                        if aa_seq1[aa] == aa_seq2[aa] {
                        } else {
                            //aa difference moved foraward in process
                            let aa_mut = aa_seq2[aa] as char;
                            let aa_ref = aa_seq1[aa] as char;
                            let hold_aa_mut = aa_mut.to_string();
                            //aa differences that are also in our "mutations of interest" list are written to file
                            for k in 0..muts_columns[2].len() {
                                if protein == muts_columns[0][k]
                                    && hold_aa_mut == muts_columns[2][k]
                                    && position.to_string() == muts_columns[1][k]
                                {
                                    let phenotypic_consequences = muts_columns[3][k].to_string();
                                    writeln!(
                                &mut writer,
                                "{sample_id}{delim}{ref_strain}{delim}{gisaid_accession}{delim}{subtype}{delim}{dais_ref}{delim}{protein}{delim}{aa_ref}{delim}{position}{delim}{aa_mut}{delim}{phenotypic_consequences}"
                                )
                                .unwrap_or_fail();
                                //aa that are missing and also in our "mutations of interest" list are written to file
                                } else if protein == muts_columns[0][k]
                                    && hold_aa_mut == "-"
                                    && position.to_string() == muts_columns[1][k]
                                {
                                    let phenotypic_consequences = "amino acid information missing";
                                    writeln!(
                                &mut writer,
                                "{sample_id}{delim}{ref_strain}{delim}{gisaid_accession}{delim}{subtype}{delim}{dais_ref}{delim}{protein}{delim}{aa_ref}{delim}{position}{delim}{aa_mut}{delim}{phenotypic_consequences}"
                                )
                                .unwrap_or_fail();
                                }
                            }
                        }
                    }
                } else {
                    //If aa seq are not the same length perform alignment to get them into the same coordinate space
                    //Using Zoe for alignment
                    let query = columns[6][i].as_bytes();
                    let reference = ref_columns[8][j].as_bytes();
                    const MAPPING: ByteIndexMap<22> =
                        ByteIndexMap::new(*b"ACDEFGHIKLMNPQRSTVWXY*", b'X');
                    //These weight work because previous alignment has taken place.
                    const WEIGHTS: WeightMatrix<i8, 22> =
                        WeightMatrix::new(&MAPPING, 1, 0, Some(b'X'));
                    const GAP_OPEN: i8 = -1;
                    const GAP_EXTEND: i8 = 0;

                    let profile = ScalarProfile::<22>::new(query, WEIGHTS, GAP_OPEN, GAP_EXTEND)
                        .expect("Ya beefed it");
                    let alignment = sw_scalar_alignment(&reference, &profile);

                    //aligned_1 -> referece, aligned_2 -> query.
                    let (aligned_1, aligned_2) = pairwise_align_with_cigar(
                        reference,
                        query,
                        &alignment.cigar,
                        alignment.ref_range.start,
                    );
                    let mut position = 0;
                    let sample_id = columns[0][i].to_string();
                    let ref_strain = ref_columns[1][j].to_string();
                    let gisaid_accession = ref_columns[0][j].to_string();
                    let subtype = columns[1][i].to_string();
                    let dais_ref = columns[2][i].to_string();
                    let protein = columns[3][i].to_string();

                    for aa in 0..aligned_1.len() {
                        position = position + 1;

                        if aligned_1[aa] == aligned_2[aa] {
                        } else {
                            //aa difference moved foraward in process
                            let aa_mut = aligned_2[aa] as char;
                            let aa_ref = aligned_1[aa] as char;
                            let hold_aa_mut = aa_mut.to_string();
                            for k in 0..muts_columns[2].len() {
                                //aa differences that are also in our "mutations of interest" list are written to file
                                if protein == muts_columns[0][k]
                                    && hold_aa_mut == muts_columns[2][k]
                                    && position.to_string() == muts_columns[1][k]
                                {
                                    let phenotypic_consequences = muts_columns[3][k].to_string();
                                    writeln!(
                                &mut writer,
                                "{sample_id}{delim}{ref_strain}{delim}{gisaid_accession}{delim}{subtype}{delim}{dais_ref}{delim}{protein}{delim}{aa_ref}{delim}{position}{delim}{aa_mut}{delim}{phenotypic_consequences}"
                                )
                                .unwrap_or_fail();
                                //aa calls that are missing and also in our "mutations of interest" list are written to file
                                } else if protein == muts_columns[0][k]
                                    && hold_aa_mut == "-"
                                    && position.to_string() == muts_columns[1][k]
                                {
                                    let phenotypic_consequences = "amino acid information missing";
                                    writeln!(
                                &mut writer,
                                "{sample_id}{delim}{ref_strain}{delim}{gisaid_accession}{delim}{subtype}{delim}{dais_ref}{delim}{protein}{delim}{aa_ref}{delim}{position}{delim}{aa_mut}{delim}{phenotypic_consequences}"
                                )
                                .unwrap_or_fail();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
