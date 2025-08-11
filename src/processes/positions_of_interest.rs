#![allow(unreachable_patterns)]
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
use zoe::{alignment::sw::sw_scalar_alignment, prelude::Nucleotides};
use zoe::{
    alignment::{ScalarProfile, pairwise_align_with_cigar},
    data::nucleotides::GetCodons,
};
use zoe::{
    data::{ByteIndexMap, StdGeneticCode, WeightMatrix},
    prelude::Len,
};

#[derive(Debug, Parser)]
#[command(about = "Tool for observing codon and amino acid differences at a given poistion")]
pub struct PositionsArgs {
    #[arg(short = 'i', long)]
    /// Optional input fasta
    input_file: PathBuf,

    #[arg(short = 'r', long)]
    /// Optional input fasta
    ref_file: PathBuf,

    #[arg(short = 'm', long)]
    /// Optional input fasta
    muts_file: PathBuf,

    #[arg(short = 'o', long)]
    /// Optional output delimited file
    output_xsv: Option<PathBuf>,

    #[arg(short = 'd', long, default_value = ",")]
    /// Use the provider delimiter for separating fields. Default is ','
    output_delimiter: String,
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
    subtype: String,
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
    ref_codon: String,
    mut_codon: String,
    aa_ref: char,
    aa_position: usize,
    aa_mut: char,
    phenotypic_consequences: String,
}

impl Entry<'_> {
    fn update_entry_from_alignment(
        &mut self,
        subtype: &str,
        aa_1: u8,
        aa_2: u8,
        muts_columns: &[MutsOfInterestInput],
    ) -> bool {
        self.aa_mut = aa_2 as char;
        self.aa_ref = aa_1 as char;
        let hold_aa_mut = self.aa_mut.to_string();

        for muts_entry in muts_columns.iter() {
            // Check if the mutation matches the entry
            if subtype == muts_entry.subtype
                && self.protein == muts_entry.protein
                && self.aa_position.to_string() == muts_entry.aa_position
            {
                // Use match for cleaner handling of `hold_aa_mut` cases
                self.phenotypic_consequences = match hold_aa_mut.as_str() {
                    "." => "amino acid information missing".to_string(),
                    "~" => "partial amino acid".to_string(),
                    "-" => "amino acid covered".to_string(),
                    "X" => "amino acid information missing".to_string(),
                    aa if aa == muts_entry.aa => muts_entry.description.clone(),
                    _ => String::new(),
                };

                return true;
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

pub fn positions_of_interest_process(args: PositionsArgs) -> Result<(), Box<dyn Error>> {
    let delim = args.output_delimiter;

    let muts_reader = create_reader(Some(args.muts_file))?;
    let muts_interest: Vec<MutsOfInterestInput> = read_tsv(muts_reader, false)?;

    let dais_reader = create_reader(Some(args.input_file))?;
    let dais: Vec<DaisInput> = read_tsv(dais_reader, false)?;

    let ref_reader = create_reader(Some(args.ref_file))?;
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
    println!("YO");
    writeln!(
        &mut writer,
        "sample, reference_strain,gisaid_accession,ctype,dais_reference,protein,sample_codon,reference_codon,aa_mutation,phenotypic_consequence",
    )?;

    for dais_entry in &dais {
        for ref_entry in &refs {
            if dais_entry.subtype == ref_entry.ctype
                && dais_entry.ref_strain == ref_entry.reference_id
                && dais_entry.protein == ref_entry.protein
            {
                let nt_seq1: Nucleotides = ref_entry.cds_aln.clone().into();
                let nt_seq2: Nucleotides = dais_entry.cds_aln.clone().into();

                if nt_seq1.len() == nt_seq2.len() {
                    let mut entry = Entry {
                        sample_id: &dais_entry.sample_id,
                        ref_strain: &ref_entry.isolate_name,
                        gisaid_accession: &ref_entry.isolate_id,
                        subtype: &dais_entry.subtype,
                        dais_ref: &dais_entry.ref_strain,
                        protein: &dais_entry.protein,
                        ref_codon: "NNN".to_string(),
                        mut_codon: "NNN".to_string(),
                        aa_position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        phenotypic_consequences: String::new(),
                    };

                    let mut tail_index = 0;
                    let (codons1, tail1) = nt_seq1.as_codons();
                    let (codons2, tail2) = nt_seq2.as_codons();

                    for (index, (ref_codon, query_codon)) in codons1
                        .iter()
                        .zip(codons2.iter())
                        .enumerate()
                        .filter(|(_, (ref_chunk, query_chunk))| ref_chunk != query_chunk)
                    {
                        let aa_index = index + 1;
                        tail_index = aa_index;
                        let ref_aa = StdGeneticCode::translate_codon(ref_codon);
                        let query_aa = StdGeneticCode::translate_codon(query_codon);

                        entry.ref_codon = std::str::from_utf8(ref_codon)
                            .expect("Invalid UTF-8 sequence")
                            .to_string();
                        entry.mut_codon = std::str::from_utf8(query_codon)
                            .expect("Invalid UTF-8 sequence")
                            .to_string();
                        entry.aa_position = aa_index;
                        entry.aa_ref = ref_aa as char;
                        entry.aa_mut = query_aa as char;

                        if entry.update_entry_from_alignment(
                            &ref_entry.subtype,
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
                                ref_codon,
                                mut_codon,
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
                                        {ref_codon}{d}{mut_codon}{d}\
                                        {aa_ref}:{aa_position}:{aa_mut}{d}\
                                        {phenotypic_consequences}",
                            )?;
                        }
                    }

                    let partial_codon = b'~';
                    entry.ref_codon = std::str::from_utf8(tail1)
                        .expect("Invalid UTF-8 sequence")
                        .to_string();
                    entry.mut_codon = std::str::from_utf8(tail2)
                        .expect("Invalid UTF-8 sequence")
                        .to_string();
                    entry.aa_position = tail_index + 1;
                    entry.aa_ref = '~' as char;
                    entry.aa_mut = '~' as char;

                    if entry.update_entry_from_alignment(
                        &ref_entry.subtype,
                        partial_codon,
                        partial_codon,
                        &muts_interest,
                    ) {
                        let Entry {
                            sample_id,
                            ref_strain,
                            gisaid_accession,
                            subtype,
                            dais_ref,
                            protein,
                            ref_codon,
                            mut_codon,
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
                                    {ref_codon}{d}{mut_codon}{d}\
                                    {aa_ref}:{aa_position}:{aa_mut}{d}\
                                    {phenotypic_consequences}",
                        )?;
                    }
                } else {
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
                        ref_codon: "NNN".to_string(),
                        mut_codon: "NNN".to_string(),
                        aa_position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        phenotypic_consequences: String::new(),
                    };

                    let mut tail_index = 0;
                    let (codons1, tail1) = aligned_1.as_codons();
                    let (codons2, tail2) = aligned_2.as_codons();

                    for (index, (ref_codon, query_codon)) in codons1
                        .iter()
                        .zip(codons2.iter())
                        .enumerate()
                        .filter(|(_, (ref_chunk, query_chunk))| ref_chunk != query_chunk)
                    {
                        let aa_index = index + 1;
                        tail_index = aa_index;
                        let ref_aa = StdGeneticCode::translate_codon(ref_codon);
                        let query_aa = StdGeneticCode::translate_codon(query_codon);

                        entry.ref_codon = std::str::from_utf8(ref_codon)
                            .expect("Invalid UTF-8 sequence")
                            .to_string();
                        entry.mut_codon = std::str::from_utf8(query_codon)
                            .expect("Invalid UTF-8 sequence")
                            .to_string();
                        entry.aa_position = aa_index;
                        entry.aa_ref = ref_aa as char;
                        entry.aa_mut = query_aa as char;

                        if entry.update_entry_from_alignment(
                            &ref_entry.subtype,
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
                                ref_codon,
                                mut_codon,
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
                                        {ref_codon}{d}{mut_codon}{d}\
                                        {aa_ref}:{aa_position}:{aa_mut}{d}\
                                        {phenotypic_consequences}",
                            )?;
                        }
                    }

                    if !tail1.is_empty() {
                        let partial_codon = b'~';
                        entry.ref_codon = std::str::from_utf8(tail1)
                            .expect("Invalid UTF-8 sequence")
                            .to_string();
                        entry.mut_codon = std::str::from_utf8(tail2)
                            .expect("Invalid UTF-8 sequence")
                            .to_string();
                        entry.aa_position = tail_index + 1;
                        entry.aa_ref = '~';
                        entry.aa_mut = '~';
                        if entry.update_entry_from_alignment(
                            &ref_entry.subtype,
                            partial_codon,
                            partial_codon,
                            &muts_interest,
                        ) {
                            let Entry {
                                sample_id,
                                ref_strain,
                                gisaid_accession,
                                subtype,
                                dais_ref,
                                protein,
                                ref_codon,
                                mut_codon,
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
                                        {ref_codon}{d}{mut_codon}{d}\
                                        {aa_ref}:{aa_position}:{aa_mut}{d}\
                                        {phenotypic_consequences}",
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
