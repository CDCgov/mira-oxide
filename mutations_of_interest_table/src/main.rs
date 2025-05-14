//#[allow(unused_variables)]
use clap::Parser;

use either::Either;
use serde::{self, Deserialize};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Stdin, Write, stdin, stdout},
    path::PathBuf,
};
use zoe::alignment::sw::sw_scalar_alignment;
use zoe::alignment::{ScalarProfile, pairwise_align_with_cigar};
use zoe::data::{ByteIndexMap, WeightMatrix};

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

#[derive(Deserialize)]
pub struct DaisInput {
    sample_id: String,
    subtype: String,
    ref_strain: String,
    protein: String,
    #[serde(skip)]
    nt_hash: String,
    #[serde(skip)]
    query_aa_seq: String,
    query_aa_aln_seq: String,
    #[serde(skip)]
    cds_id: String,
    #[serde(skip)]
    insertion: bool,
    #[serde(skip)]
    inert_shift: bool,
    #[serde(skip)]
    cds_seq: String,
    #[serde(skip)]
    cds_aln: String,
    #[serde(skip)]
    query_nt_coordinates: String,
    #[serde(skip)]
    cds_nt_coordinates: String,
}

#[derive(Deserialize)]
pub struct RefInput {
    sample_id: String,
    subtype: String,
    ref_strain: String,
    protein: String,
    #[serde(skip)]
    nt_hash: String,
    query_aa_aln_seq: String,
    #[serde(skip)]
    cds_aln: String,
}

#[derive(Deserialize)]
pub struct MutsOfInterestInput {
    protein: String,
    aa_position: String,
    aa: String,
    description: String,
}

pub struct Entry {
    sample_id: String,
    ref_strain: String,
    gisaid_accession: String,
    subtype: String,
    dais_ref: String,
    protein: String,
    aa_ref: char,
    position: usize,
    aa_mut: char,
    phenotypic_consequences: String,
}

impl Entry {
    fn header(delim: &str) -> String {
        [
            "sample",
            "ref_strain",
            "gisaid_accession",
            "subtype",
            "dais_reference",
            "protein",
            "aa_reference",
            "aa_position",
            "aa_mutation",
            "phenotypic_consequences",
        ]
        .join(delim)
    }

    fn to_delimited(&self, delim: &str) -> String {
        [
            self.sample_id.as_str(),
            self.ref_strain.as_str(),
            self.gisaid_accession.as_str(),
            self.subtype.as_str(),
            self.dais_ref.as_str(),
            self.protein.as_str(),
            &self.aa_ref.to_string(),
            &self.position.to_string(),
            &self.aa_mut.to_string(),
            self.phenotypic_consequences.as_str(),
        ]
        .join(delim)
    }

    fn update_entry_from_alignment(
        &mut self,
        aa_1: u8,
        aa_2: u8,
        muts_columns: &[Vec<String>],
    ) -> bool {
        self.aa_mut = aa_2 as char;
        self.aa_ref = aa_1 as char;
        let hold_aa_mut = self.aa_mut.to_string();
        //aa differences that are also in our "mutations of interest" list are written to file
        for k in 0..muts_columns[2].len() {
            if self.protein == muts_columns[0][k] && self.position.to_string() == muts_columns[1][k]
            {
                if hold_aa_mut == muts_columns[2][k] {
                    self.phenotypic_consequences = muts_columns[3][k].to_string();
                    return true;
                }
                //aa that are missing and also in our "mutations of interest" list are written to file
                else if hold_aa_mut == "-" {
                    self.phenotypic_consequences = String::from("amino acid information missing");
                    return true;
                }
            }
        }
        false
    }
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

pub fn align_sequences<'a>(query: &'a [u8], reference: &'a [u8]) -> (Vec<u8>, Vec<u8>) {
    const MAPPING: ByteIndexMap<22> = ByteIndexMap::new(*b"ACDEFGHIKLMNPQRSTVWXY*", b'X');
    const WEIGHTS: WeightMatrix<i8, 22> = WeightMatrix::new(&MAPPING, 1, 0, Some(b'X'));
    const GAP_OPEN: i8 = -1;
    const GAP_EXTEND: i8 = 0;

    let profile =
        ScalarProfile::<22>::new(query, WEIGHTS, GAP_OPEN, GAP_EXTEND).expect("Ya beefed it");
    let alignment = sw_scalar_alignment(reference, &profile);

    pairwise_align_with_cigar(
        reference,
        query,
        &alignment.cigar,
        alignment.ref_range.start,
    )
}

fn main() -> std::io::Result<()> {
    let args = APDArgs::parse();
    // let delim = args.output_delimiter.clone();

    //read in input file (dais results)
    let reader = create_reader(args.input_file)?;

    // Initialize vectors to store columns for the input file (dais results)
    // TODO convert to bytes fr fr ong ong
    let dais_columns = lines_to_vec(reader)?;

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
    writeln!(&mut writer, "{}", Entry::header(&args.output_delimiter))?;

    //Finding reference sequences in the same coordinate space to compare with
    for i in 0..dais_columns[1].len() {
        for j in 0..ref_columns[1].len() {
            if dais_columns[1][i] == ref_columns[5][j]
                && dais_columns[2][i] == ref_columns[6][j]
                && dais_columns[3][i] == ref_columns[7][j]
            {
                let aa_seq1 = ref_columns[8][j].as_bytes();
                let aa_seq2 = dais_columns[6][i].as_bytes();
                //If aa seq are the same length start seqe comparison
                //aa seqs that are the same length will be aligned already and saves time to not align
                if aa_seq1.len() == aa_seq2.len() {
                    let mut entry = Entry {
                        sample_id: dais_columns[0][i].to_string(),
                        ref_strain: ref_columns[1][j].to_string(),
                        gisaid_accession: ref_columns[0][j].to_string(),
                        subtype: dais_columns[1][i].to_string(),
                        dais_ref: dais_columns[2][i].to_string(),
                        protein: dais_columns[3][i].to_string(),
                        position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        phenotypic_consequences: String::new(),
                    };
                    for aa in 0..aa_seq1.len() {
                        entry.position += 1;

                        if aa_seq1[aa] == aa_seq2[aa] {
                        } else {
                            //aa difference moved foraward in process
                            entry.aa_mut = aa_seq2[aa] as char;
                            entry.aa_ref = aa_seq1[aa] as char;
                            if entry.update_entry_from_alignment(
                                aa_seq1[aa],
                                aa_seq2[aa],
                                &muts_columns,
                            ) {
                                writeln!(
                                    &mut writer,
                                    "{}",
                                    entry.to_delimited(&args.output_delimiter)
                                )?;
                            }
                        }
                    }
                } else {
                    //If aa seq are not the same length perform alignment to get them into the same coordinate space
                    //Using Zoe for alignment
                    let query = dais_columns[6][i].as_bytes();
                    let reference = ref_columns[8][j].as_bytes();
                    let (aligned_1, aligned_2) = align_sequences(query, reference);

                    let mut entry = Entry {
                        sample_id: dais_columns[0][i].to_string(),
                        ref_strain: ref_columns[1][j].to_string(),
                        gisaid_accession: ref_columns[0][j].to_string(),
                        subtype: dais_columns[1][i].to_string(),
                        dais_ref: dais_columns[2][i].to_string(),
                        protein: dais_columns[3][i].to_string(),
                        position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        phenotypic_consequences: String::new(),
                    };

                    for aa in 0..aligned_1.len() {
                        entry.position += 1;

                        if aligned_1[aa] == aligned_2[aa] {
                        } else {
                            //aa difference moved foraward in process
                            entry.aa_mut = aligned_2[aa] as char;
                            entry.aa_ref = aligned_1[aa] as char;
                            if entry.update_entry_from_alignment(
                                aligned_1[aa],
                                aligned_2[aa],
                                &muts_columns,
                            ) {
                                writeln!(
                                    &mut writer,
                                    "{}",
                                    entry.to_delimited(&args.output_delimiter)
                                )?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
