#![feature(let_chains)]

// TODO: replace extra work with csv + Serde and define structs to deserialization
// Create structs
// Derserialize to them
// Add helper methods as need directly to the struct
// Consider unifying errors in some less manual way

use clap::Parser;
use either::Either;
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

pub struct Strain {
    sample_id:               String,
    ref_strain:              String,
    gisaid_accession:        String,
    subtype:                 String,
    dais_ref:                String,
    protein:                 String,
    aa_ref:                  char,
    position:                usize,
    aa_mut:                  char,
    phenotypic_consequences: String,
}

impl Strain {
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

    fn update_entry_from_alignment(&mut self, aa_1: u8, aa_2: u8, muts_columns: &[Vec<String>]) -> bool {
        self.aa_mut = aa_2 as char;
        self.aa_ref = aa_1 as char;
        let hold_aa_mut = self.aa_mut.to_string();
        //aa differences that are also in our "mutations of interest" list are written to file
        for k in 0..muts_columns[2].len() {
            if self.protein == muts_columns[0][k]
                && hold_aa_mut == muts_columns[2][k]
                && self.position.to_string() == muts_columns[1][k]
            {
                self.phenotypic_consequences = muts_columns[3][k].to_string();
                return true;
            //aa that are missing and also in our "mutations of interest" list are written to file
            } else if self.protein == muts_columns[0][k]
                && hold_aa_mut == "-"
                && self.position.to_string() == muts_columns[1][k]
            {
                self.phenotypic_consequences = String::from("amino acid information missing");
                return true;
            }
        }
        false
    }
}

fn update_entry_from_alignment<'a>(
    query_aa: char, query_pos: usize, protein: &str, mutations: &'a [MutationOfInterest],
) -> Option<&'a str> {
    for mutation in mutations {
        if protein == mutation.protein && query_pos == mutation.position {
            if query_aa == mutation.wildtype_aa {
                return Some(mutation.description.as_str());
            } else if query_aa == '-' {
                return Some("amino acid information missing");
            }
        }
    }
    None
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

pub fn lines_to_vec<R: BufRead>(reader: R) -> Result<Vec<Vec<String>>, ParsingErrors> {
    let mut columns: Vec<Vec<String>> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result.map_err(|_| ParsingErrors::MissingLine)?;
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

    let profile = ScalarProfile::<22>::new(query, WEIGHTS, GAP_OPEN, GAP_EXTEND).expect("Ya beefed it");
    let alignment = sw_scalar_alignment(reference, &profile);

    pairwise_align_with_cigar(reference, query, &alignment.cigar, alignment.ref_range.start)
}

#[derive(Debug)]
enum ParsingErrors {
    BadPosition,
    MissingField,
    MissingLine,
}

impl std::fmt::Display for ParsingErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsingErrors::BadPosition => write!(f, "Failed to parse position as a number"),
            ParsingErrors::MissingField => write!(f, "Missing required field in input"),
            ParsingErrors::MissingLine => write!(f, "Missing line in file"),
        }
    }
}

impl std::error::Error for ParsingErrors {}

// Define a Result type alias for parsing operations
type ParseIntoResult<T> = Result<T, ParsingErrors>;

impl From<std::num::ParseIntError> for ParsingErrors {
    fn from(_: std::num::ParseIntError) -> Self {
        ParsingErrors::BadPosition
    }
}

struct MutationOfInterest {
    protein:     String,
    position:    usize,
    wildtype_aa: char,
    description: String,
}

impl TryFrom<&str> for MutationOfInterest {
    type Error = ParsingErrors;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split = value.split('\t');

        let protein = split.next().ok_or(ParsingErrors::MissingField)?.to_string();
        let position = split.next().ok_or(ParsingErrors::MissingField)?.parse::<usize>()?;
        let wildtype_aa = split
            .next()
            .ok_or(ParsingErrors::MissingField)?
            .chars()
            .next()
            .ok_or(ParsingErrors::MissingField)?;
        let description = split.next().ok_or(ParsingErrors::MissingField)?.to_string();

        Ok(MutationOfInterest {
            protein,
            position,
            wildtype_aa,
            description,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = APDArgs::parse();
    // let delim = args.output_delimiter.clone();

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
    let mut muts_reader = create_reader(args.muts_file)?;

    let mut mutations = Vec::new();
    // Initialize vectors to store columns (mutations of interest)
    for line in muts_reader.lines() {
        let line = line?;
        let record: MutationOfInterest = line.as_str().try_into()?;
        mutations.push(record);
    }

    let mut writer = if let Some(ref file_path) = args.output_xsv {
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(file_path)?;
        BufWriter::new(Either::Left(file))
    } else {
        BufWriter::new(Either::Right(stdout()))
    };

    //header of output file
    writeln!(&mut writer, "{}", Strain::header(&args.output_delimiter))?;

    // TODO: fix me
    let dlm = '\t';

    //Finding reference sequences in the same coordinate space to compare with
    for i in 0..columns[1].len() {
        for j in 0..ref_columns[1].len() {
            if columns[1][i] == ref_columns[5][j] && columns[2][i] == ref_columns[6][j] && columns[3][i] == ref_columns[7][j]
            {
                let aa_seq1 = ref_columns[8][j].as_bytes();
                let aa_seq2 = columns[6][i].as_bytes();
                //If aa seq are the same length start seqe comparison
                //aa seqs that are the same length will be aligned already and saves time to not align
                if aa_seq1.len() == aa_seq2.len() {
                    let (sample_id, ref_strain, gisaid_accession, subtype, dais_ref, protein) = (
                        columns[0][i].as_str(),
                        ref_columns[1][j].as_str(),
                        ref_columns[0][j].as_str(),
                        columns[1][i].as_str(),
                        columns[2][i].as_str(),
                        columns[3][i].as_str(),
                    );

                    for (pos, ref_aa, query_aa) in aa_seq1
                        .iter()
                        .zip(aa_seq2)
                        .enumerate()
                        .map(|(i, (&r, &q))| (i + 1, r as char, q as char))
                    {
                        if ref_aa != query_aa
                            && let Some(description) = update_entry_from_alignment(query_aa, pos, protein, &mutations)
                        {
                            writeln!(
                                &mut writer,
                                "{sample_id}{dlm}{ref_strain}{dlm}{gisaid_accession}{dlm}{subtype}{dlm}\
                                 {dais_ref}{dlm}{protein}{dlm}{pos}{dlm}{ref_aa}{dlm}{query_aa}{dlm}{description}"
                            )?;
                        }
                    }
                } else {
                    //If aa seq are not the same length perform alignment to get them into the same coordinate space
                    //Using Zoe for alignment
                    let query = columns[6][i].as_bytes();
                    let reference = ref_columns[8][j].as_bytes();
                    let (aligned_1, aligned_2) = align_sequences(query, reference);

                    let mut entry = Strain {
                        sample_id:               columns[0][i].to_string(),
                        ref_strain:              ref_columns[1][j].to_string(),
                        gisaid_accession:        ref_columns[0][j].to_string(),
                        subtype:                 columns[1][i].to_string(),
                        dais_ref:                columns[2][i].to_string(),
                        protein:                 columns[3][i].to_string(),
                        position:                0,
                        aa_ref:                  'X',
                        aa_mut:                  'X',
                        phenotypic_consequences: String::new(),
                    };

                    for aa in 0..aligned_1.len() {
                        entry.position += 1;

                        if aligned_1[aa] == aligned_2[aa] {
                        } else {
                            //aa difference moved foraward in process
                            entry.aa_mut = aligned_2[aa] as char;
                            entry.aa_ref = aligned_1[aa] as char;

                            todo!()
                            /*if entry.update_entry_from_alignment(aligned_1[aa], aligned_2[aa], &mutations) {
                                writeln!(&mut writer, "{}", entry.to_delimited(&args.output_delimiter))?;
                            }*/
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
