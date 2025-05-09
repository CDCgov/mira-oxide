#![allow(unused_variables)]
use clap::Parser;
use either::Either;
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, BufWriter, Write, stdin, stdout},
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

    #[arg(short = 'd', long)]
    /// Use the provider delimiter for separating fields. Default is ','
    output_delimiter: Option<String>,
}

fn main() {
    let args = APDArgs::parse();
    let delim = args.output_delimiter.unwrap_or(",".to_owned());

    //read in input file (dais results)
    let reader = if let Some(ref file_path) = args.input_file {
        BufReader::new(Either::Left(
            OpenOptions::new()
                .read(true)
                .open(file_path)
                .expect("File opening error"),
        ))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    // Initialize vectors to store columns for the input file (dais results)
    //TODO convert to bytes fr fr ong ong
    let mut columns: Vec<Vec<String>> = Vec::new();

    // Read the file line by line
    for line in reader.lines() {
        let values: Vec<String> = line
            .expect("REASON")
            .split('\t')
            .map(|s| s.to_string())
            .collect();

        // Ensure the columns vector has enough vectors to store each column
        if columns.is_empty() {
            columns.resize(values.len(), Vec::new());
        }

        // Push each value into the corresponding column vector
        for (i, value) in values.iter().enumerate() {
            columns[i].push(value.clone());
        }
    }

    //read in reference file (reference cvv and zoonotic strains)
    let ref_reader = if let Some(ref ref_file_path) = args.ref_file {
        BufReader::new(Either::Left(
            OpenOptions::new()
                .read(true)
                .open(ref_file_path)
                .expect("File opening error"),
        ))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    // Initialize vectors to store columns (reference cvv and zoonotic strains)
    let mut ref_columns: Vec<Vec<String>> = Vec::new();

    // Read the file line by line
    for ref_line in ref_reader.lines() {
        let ref_values: Vec<String> = ref_line
            .expect("REASON")
            .split('\t')
            .map(|ref_s| ref_s.to_string())
            .collect();

        // Ensure the columns vector has enough vectors to store each column
        if ref_columns.is_empty() {
            ref_columns.resize(ref_values.len(), Vec::new());
        }

        // Push each value into the corresponding column vector
        for (i, ref_value) in ref_values.iter().enumerate() {
            ref_columns[i].push(ref_value.clone());
        }
    }

    //read in mutations file (mutations of interest)
    let muts_reader = if let Some(ref muts_file_path) = args.muts_file {
        BufReader::new(Either::Left(
            OpenOptions::new()
                .read(true)
                .open(muts_file_path)
                .expect("File opening error"),
        ))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    // Initialize vectors to store columns (mutations of interest)
    let mut muts_columns: Vec<Vec<String>> = Vec::new();

    // Read the file line by line (mutations of interest)
    for muts_line in muts_reader.lines() {
        let muts_values: Vec<String> = muts_line
            .expect("REASON")
            .split('\t')
            .map(|muts_s| muts_s.to_string())
            .collect();

        // Ensure the columns vector has enough vectors to store each column
        if muts_columns.is_empty() {
            muts_columns.resize(muts_values.len(), Vec::new());
        }

        // Push each value into the corresponding column vector
        for (i, muts_values) in muts_values.iter().enumerate() {
            muts_columns[i].push(muts_values.clone());
        }
    }

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
}
