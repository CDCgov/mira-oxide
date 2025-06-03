#![allow(dead_code, unused_imports)]
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
use serde::{self, Deserialize, de::DeserializeOwned};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Stdin, Write, stdin, stdout},
    path::{Path, PathBuf},
};
use zoe::data::{ByteIndexMap, StdGeneticCode, WeightMatrix};
use zoe::{alignment::sw::sw_scalar_alignment, prelude::Nucleotides};
use zoe::{
    alignment::{ScalarProfile, pairwise_align_with_cigar},
    data::nucleotides::GetCodons,
};

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
    output_delimiter: Option<String>,
}

// input files *must* be tab-separated
fn read_tsv<T: DeserializeOwned, R: std::io::Read>(
    reader: R,
    has_headers: bool,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b'\t')
        .from_reader(reader);

    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let record: T = result?;
        records.push(record);
    }

    Ok(records)
}

#[derive(Deserialize, Debug)]
pub struct DaisInput {
    sample_id: String,
    subtype: String,
    ref_strain: String,
    protein: String,
    nt_hash: String,
    query_nt_seq: String,
    query_aa_aln_seq: String,
    cds_id: String,
    insertion: String,
    inert_shift: String,
    cds_seq: String,
    cds_aln: String,
    query_nt_coordinates: String,
    cds_nt_coordinates: String,
}

#[derive(Deserialize, Debug)]
pub struct RefInput {
    isolate_id: String,
    isolate_name: String,
    subtype: String,
    passage_history: String,
    nt_id: String,
    ctype: String,
    reference_id: String,
    protein: String,
    aa_aln: String,
    cds_aln: String,
}

#[derive(Deserialize, Debug)]
pub struct MutsOfInterestInput {
    protein: String,
    aa_position: String,
    aa: String,
    description: String,
}

pub struct Entry<'a> {
    sample_id: &'a str,
    ref_strain: &'a str,
    gisaid_accession: &'a str,
    subtype: &'a str,
    dais_ref: &'a str,
    protein: &'a str,
    nt_ref: char,
    nt_position: usize,
    nt_mut: char,
    aa_ref: char,
    aa_position: usize,
    aa_mut: char,
    phenotypic_consequences: String,
}

impl Entry<'_> {
    fn update_entry_from_alignment(
        &mut self,
        aa_1: u8,
        aa_2: u8,
        muts_columns: &Vec<MutsOfInterestInput>,
    ) -> bool {
        self.aa_mut = aa_2 as char;
        self.aa_ref = aa_1 as char;
        let hold_aa_mut = self.aa_mut.to_string();
        //aa differences that are also in our "mutations of interest" list are written to file
        for muts_entry in muts_columns {
            if self.protein == muts_entry.protein
                && self.aa_position.to_string() == muts_entry.aa_position
            {
                if hold_aa_mut == muts_entry.aa {
                    self.phenotypic_consequences = muts_entry.description.clone();
                    return true;
                }
                //aa that are missing and also in our "mutations of interest" list are written to file
                else if hold_aa_mut == "." {
                    self.phenotypic_consequences = String::from("amino acid information missing");
                    return true;
                } else if hold_aa_mut == "~" {
                    self.phenotypic_consequences = String::from("partial amino acid");
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
    const MAPPING: ByteIndexMap<6> = ByteIndexMap::new(*b"ACGTN*", b'N');
    const WEIGHTS: WeightMatrix<i8, 6> = WeightMatrix::new(&MAPPING, 1, 0, Some(b'N'));
    const GAP_OPEN: i8 = -1;
    const GAP_EXTEND: i8 = 0;

    let profile = ScalarProfile::<6>::new(query, WEIGHTS, GAP_OPEN, GAP_EXTEND)
        .expect("Alignment profile failed");
    let alignment = sw_scalar_alignment(reference, &profile);

    pairwise_align_with_cigar(
        reference,
        query,
        &alignment.cigar,
        alignment.ref_range.start,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = APDArgs::parse();
    let delim = args.output_delimiter.unwrap_or(",".to_owned());

    //read in input file (dais input, ref input, muts input)
    let muts_reader = create_reader(args.muts_file)?;
    let muts_interest: Vec<MutsOfInterestInput> = read_tsv(muts_reader, false)?;

    let dais_reader = create_reader(args.input_file)?;
    let dais: Vec<DaisInput> = read_tsv(dais_reader, false)?;

    let ref_reader = create_reader(args.ref_file)?;
    let refs: Vec<RefInput> = read_tsv(ref_reader, true)?;

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
        "sample, reference_strain,gisaid_accession,ctype,dais_reference,protein,nt_mutation,aa_mutation,phenotypic_consequence",
    )?;

    //Finding reference sequences in the same coordinate space to compare with
    for dais_entry in &dais {
        for ref_entry in &refs {
            if dais_entry.subtype == ref_entry.ctype
                && dais_entry.ref_strain == ref_entry.reference_id
                && dais_entry.protein == ref_entry.protein
            {
                let nt_seq1 = ref_entry.cds_aln.as_bytes();
                let nt_seq2 = dais_entry.cds_aln.as_bytes();
                //If nt seq are the same length start seq comparison
                //nt seqs that are the same length will be aligned already and saves time to not align

                if nt_seq1.len() == nt_seq2.len() {
                    let mut entry = Entry {
                        sample_id: &dais_entry.sample_id,
                        ref_strain: &ref_entry.isolate_name,
                        gisaid_accession: &ref_entry.isolate_id,
                        subtype: &dais_entry.subtype,
                        dais_ref: &dais_entry.ref_strain,
                        protein: &dais_entry.protein,
                        nt_position: 0,
                        nt_ref: 'N',
                        nt_mut: 'N',
                        aa_position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        phenotypic_consequences: String::new(),
                    };

                    let compare_seqs: Vec<(usize, &[u8], &[u8])> = nt_seq1
                        .chunks(3)
                        .zip(nt_seq2.chunks(3))
                        .enumerate()
                        .filter(|(_, (ref_chunk, query_chunk))| ref_chunk != query_chunk)
                        .map(|(index, (ref_chunk, query_chunk))| (index, ref_chunk, query_chunk))
                        .collect();

                    for (index, ref_chunk, query_chunk) in compare_seqs {
                        let mut chunk_position = 0;
                        for nt in 0..ref_chunk.len() {
                            chunk_position += 1;
                            let aa_index = index + 1;

                            let ref_aa;
                            if ref_chunk.len() == 3 {
                                ref_aa = StdGeneticCode::translate_codon(ref_chunk);
                            } else {
                                ref_aa = b'~';
                            }

                            let query_aa;
                            if query_chunk.len() == 3 {
                                query_aa = StdGeneticCode::translate_codon(query_chunk);
                            } else {
                                query_aa = b'~';
                            }

                            if ref_chunk[nt] != query_chunk[nt] {
                                let nt_idex = ((index + 1) * 3) - (3 - chunk_position);
                                entry.nt_position = nt_idex;
                                entry.nt_ref = ref_chunk[nt] as char;
                                entry.nt_mut = query_chunk[nt] as char;
                                entry.aa_position = aa_index;
                                entry.aa_ref = ref_aa as char;
                                entry.aa_mut = query_aa as char;

                                //aa difference moved forward in process;
                                if entry.update_entry_from_alignment(
                                    ref_aa,
                                    query_aa,
                                    &muts_interest,
                                ) {
                                    writeln!(
                                        &mut writer,
                                        "{}{}{}{}{}{}{}{}{}{}{}{}{}:{}:{}{}{}:{}:{}{}{}",
                                        entry.sample_id,
                                        delim,
                                        entry.ref_strain,
                                        delim,
                                        entry.gisaid_accession,
                                        delim,
                                        entry.subtype,
                                        delim,
                                        entry.dais_ref,
                                        delim,
                                        entry.protein,
                                        delim,
                                        entry.nt_ref.to_string(),
                                        entry.nt_position.to_string(),
                                        entry.nt_mut.to_string(),
                                        delim,
                                        entry.aa_ref.to_string(),
                                        entry.aa_position.to_string(),
                                        entry.aa_mut.to_string(),
                                        delim,
                                        entry.phenotypic_consequences.as_str(),
                                    )?;
                                }
                            }
                        }
                    }
                } else {
                    //If aa seq are not the same length perform alignment to get them into the same coordinate space
                    //Using Zoe for alignment
                    let query = dais_entry.cds_aln.as_bytes();
                    let reference = ref_entry.cds_aln.as_bytes();
                    let (aligned_1, aligned_2) = {
                        let (a1, a2) = align_sequences(query, reference);
                        (Nucleotides::from(a1), Nucleotides::from(a2))
                    };

                    let mut entry = Entry {
                        sample_id: &dais_entry.sample_id,
                        ref_strain: &ref_entry.isolate_name,
                        gisaid_accession: &ref_entry.isolate_id,
                        subtype: &dais_entry.subtype,
                        dais_ref: &dais_entry.ref_strain,
                        protein: &dais_entry.protein,
                        nt_position: 0,
                        nt_ref: 'N',
                        nt_mut: 'N',
                        aa_position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        phenotypic_consequences: String::new(),
                    };

                    let (codons1, _tail1) = aligned_1.as_codons(); // TODO: fix tails
                    let (codons2, _tail2) = aligned_2.as_codons();

                    for (index, (ref_codon, query_codon)) in codons1
                        .iter()
                        .zip(codons2.iter())
                        .enumerate()
                        .filter(|(_, (ref_chunk, query_chunk))| ref_chunk != query_chunk)
                    {
                        let aa_index = index + 1;
                        let ref_aa = StdGeneticCode::get(ref_codon).unwrap_or(b'X');
                        let query_aa = StdGeneticCode::translate_codon(query_codon);

                        let mut chunk_position: usize = 0;
                        for nt in 0..ref_codon.len() {
                            chunk_position += 1;

                            if ref_codon[nt] != query_codon[nt] {
                                let nt_idex = ((index + 1) * 3) - (3 - chunk_position);
                                entry.nt_position = nt_idex;
                                entry.nt_ref = ref_codon[nt] as char;
                                entry.nt_mut = query_codon[nt] as char;
                                entry.aa_position = aa_index;
                                entry.aa_ref = ref_aa as char;
                                entry.aa_mut = query_aa as char;

                                //aa difference moved forward in process;

                                if entry.update_entry_from_alignment(
                                    ref_aa,
                                    query_aa,
                                    &muts_interest,
                                ) {
                                    let Entry {
                                        sample_id,
                                        ref_strain,
                                        gisaid_accession,
                                        subtype,
                                        dais_ref,
                                        protein,
                                        nt_ref,
                                        nt_position,
                                        nt_mut,
                                        aa_ref,
                                        aa_position,
                                        aa_mut,
                                        phenotypic_consequences,
                                    } = &entry;
                                    let d = &delim;

                                    writeln!(
                                        &mut writer,
                                        "{sample_id}{d}{ref_strain}{d}{gisaid_accession}{d}\
                                        {subtype}{d}{dais_ref}{d}{protein}{d}\
                                        {nt_ref}:{nt_position}:{nt_mut}{d}\
                                        {aa_ref}:{aa_position}:{aa_mut}{d}\
                                        {phenotypic_consequences}",
                                    )?;
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
