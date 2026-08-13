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
use zoe::{
    alignment::{ScalarProfile, sw::sw_scalar_align},
    data::{
        WeightMatrix,
        mappings::{ByteIndexMap, StdGeneticCode},
        nucleotides::GetCodons,
    },
    prelude::{Len, Nucleotides},
};

use crate::io::data_ingest::read_csv;
use crate::utils::get_dais_refs::assign_dais_refs;

#[derive(Debug, Parser)]
#[command(about = "Tool for observing codon and amino acid differences at a given poistion")]
pub struct PositionsArgs {
    #[arg(short = 'q', long)]
    /// Input dais-ribosome file
    query_dais_file: PathBuf,

    #[arg(short = 'r', long)]
    /// Reference dais-ribosome file
    ref_dais_file: PathBuf,

    #[arg(short = 'v', long)]
    /// Optional input fasta
    muts_file: PathBuf,

    #[arg(short = 'i', long)]
    /// Insertion (.ins) file
    query_insertion_file: PathBuf,

    #[arg(short = 'd', long)]
    /// Deletion (.del) file
    query_deletion_file: PathBuf,

    #[arg(short = 'j', long)]
    /// Reference insertion (.ins) file
    ref_insertion_file: PathBuf,

    #[arg(short = 'e', long)]
    /// Reference deletion (.del) file
    ref_deletion_file: PathBuf,

    #[arg(short = 'm', long)]
    /// Minor variants (.csv, with headers) file
    minor_variants_file: Option<PathBuf>,

    #[arg(short = 'o', long)]
    /// Optional output delimited file
    output_xsv: Option<PathBuf>,

    #[arg(short = 's', long, default_value = ",")]
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
pub struct QueryInput {
    sample_id: String,
    ctype: String,
    dais_ref_id: String,
    protein: String,
    nt_hash: String,
    query_aa_seq: String,
    query_aa_aln_seq: String,
    cds_id: String,
    insertion: String,
    inert_shift: String,
    query_cds_seq: String,
    query_cds_aln: String,
    query_nt_coordinates: String,
    cds_nt_coordinates: String,
}

#[derive(Deserialize, Debug)]
pub struct RefDaisInput {
    ref_id: String,
    ctype: String,
    dais_ref_id: String,
    protein: String,
    nt_hash: String,
    ref_aa_seq: String,
    ref_aa_aln_seq: String,
    cds_id: String,
    insertion: String,
    inert_shift: String,
    ref_cds_seq: String,
    ref_cds_aln: String,
    ref_nt_coordinates: String,
    ref_cds_nt_coordinates: String,
}

#[derive(Deserialize, Debug)]
pub struct UpdatedRefInput {
    ref_id: String,
    dais_ref_id: String,
    protein: String,
    ref_cds_aln: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MutsOfInterestInput {
    subtype: String,
    protein: String,
    aa_position: String,
    aa: String,
    description: String,
}

/// Insertion file.
#[derive(Deserialize, Debug, Clone)]
pub struct InsertionInput {
    query_id: String,
    ctype: String,
    reference_id: String,
    product_name: String,
    upstream_aa_pos: i64,
    inserted_nt: String,
    inserted_aa: String,
    upstream_nt_pos: i64,
    codon_shift: i64,
}

// Deletion file.
#[derive(Deserialize, Debug, Clone)]
pub struct DeletionInput {
    query_id: String,
    ctype: String,
    reference_id: String,
    product_name: String,
    variant_hash: String,
    del_aa_start: i64,
    del_aa_end: i64,
    del_aa_len: i64,
    in_frame: bool,
    cds_id: String,
    del_cds_start: i64,
    del_cds_end: i64,
    del_cds_len: i64,
}

/// Minor_variants,csv
#[derive(Deserialize, Debug, Clone)]
pub struct MinorVariantInput {
    sample: String,
    reference: String,
    sample_position: i64,
    depth: i64,
    consensus_allele: String,
    minority_allele: String,
    consensus_count: i64,
    minority_count: i64,
    minority_frequency: f64,
    run_id: String,
    instrument: String,
}

pub struct Entry<'a> {
    sample_id: &'a str,
    ref_strain: &'a str,
    dais_ref_id: &'a str,
    protein: &'a str,
    position_in_codon: usize,
    ref_codon: String,
    mut_codon: String,
    aa_ref: char,
    aa_position: usize,
    aa_mut: char,
    variant_of_interest: bool,
}

impl Entry<'_> {
    fn update_entry(
        &mut self,
        dais_ref_id: &str,
        aa_1: u8,
        aa_2: u8,
        muts_columns: &[MutsOfInterestInput],
    ) -> bool {
        self.aa_mut = aa_2 as char;
        self.aa_ref = aa_1 as char;
        let hold_aa_mut = self.aa_mut.to_string();

        for muts_entry in muts_columns {
            // get subtype and protein, then compare against the passed-in dais_ref_id.
            let assigned_ref = assign_dais_refs(&muts_entry.subtype, &muts_entry.protein);

            if (assigned_ref == Some(dais_ref_id) || muts_entry.subtype.to_lowercase() == "all")
                && self.protein == muts_entry.protein
                && self.aa_position.to_string() == muts_entry.aa_position
            {
                self.variant_of_interest = hold_aa_mut == muts_entry.aa;

                return true;
            }
        }

        false
    }
}

fn create_reader(path: Option<&PathBuf>) -> std::io::Result<BufReader<Either<File, Stdin>>> {
    let reader = if let Some(ref file_path) = path {
        let file = OpenOptions::new().read(true).open(file_path)?;
        BufReader::new(Either::Left(file))
    } else {
        BufReader::new(Either::Right(stdin()))
    };

    Ok(reader)
}

#[allow(clippy::too_many_lines)]
pub fn positions_of_interest_process(args: PositionsArgs) -> Result<(), Box<dyn Error>> {
    let delim = args.output_delimiter;

    println!(
        "Processing positions of interest for input file: {:?}, reference file: {:?}, and mutations file: {:?}",
        &args.query_dais_file.display(),
        &args.ref_dais_file.display(),
        &args.muts_file.display()
    );

    let muts_reader = create_reader(Some(&args.muts_file))?;
    let muts_interest: Vec<MutsOfInterestInput> = read_tsv(muts_reader, false)?;

    println!(
        "Read {} entries from the mutations of interest file.",
        muts_interest.len()
    );

    let dais_reader = create_reader(Some(&args.query_dais_file))?;
    let dais: Vec<QueryInput> = read_tsv(dais_reader, false)?;
    println!(
        "Read {} entries from the input dais-ribosome file.",
        dais.len()
    );

    let ref_reader = create_reader(Some(&args.ref_dais_file))?;
    let refs: Vec<RefDaisInput> = read_tsv(ref_reader, false)?;
    println!(
        "Read {} entries from the reference dais-ribosome file.",
        refs.len()
    );

    // Insertions (.ins)
    let ins_reader = create_reader(Some(&args.query_insertion_file))?;
    let insertions: Vec<InsertionInput> = read_tsv(ins_reader, false)?;
    println!(
        "Read {} entries from the insertion file: {:?}",
        insertions.len(),
        &args.query_insertion_file.display()
    );

    // Deletions (.del)
    let del_reader = create_reader(Some(&args.query_deletion_file))?;
    let deletions: Vec<DeletionInput> = read_tsv(del_reader, false)?;
    println!(
        "Read {} entries from the deletion file: {:?}",
        deletions.len(),
        &args.query_deletion_file.display()
    );

    // Reference insertions (.ins)
    let ref_ins_reader = create_reader(Some(&args.ref_insertion_file))?;
    let ref_insertions: Vec<InsertionInput> = read_tsv(ref_ins_reader, false)?;
    println!(
        "Read {} entries from the reference insertion file: {:?}",
        ref_insertions.len(),
        &args.ref_insertion_file.display()
    );

    // Reference deletions (.del)
    let ref_del_reader = create_reader(Some(&args.ref_deletion_file))?;
    let ref_deletions: Vec<DeletionInput> = read_tsv(ref_del_reader, false)?;
    println!(
        "Read {} entries from the reference deletion file: {:?}",
        ref_deletions.len(),
        &args.ref_deletion_file.display()
    );

    // Optional: minor variants (.csv, with headers)
    let minor_variants: Vec<MinorVariantInput> = if let Some(mv_path) = &args.minor_variants_file {
        let mv_reader = create_reader(Some(mv_path))?;
        let parsed: Vec<MinorVariantInput> = read_csv(mv_reader, true)?;
        println!(
            "Read {} entries from the minor variants file: {:?}",
            parsed.len(),
            mv_path.display()
        );
        parsed
    } else {
        Vec::new()
    };

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
    writeln!(
        &mut writer,
        "query_name,ref_name,ctype,dais_reference,protein,nt_position,query_nt,ref_nt,position_in_codon,query_codon,ref_codon,aa_mutation,variant_of_interest",
    )?;

    for dais_entry in &dais {
        for ref_entry in &refs {
            if dais_entry.ctype == ref_entry.ctype
                && dais_entry.dais_ref_id == ref_entry.dais_ref_id
                && dais_entry.protein == ref_entry.protein
            {
                let nt_seq1: Nucleotides = ref_entry.ref_cds_aln.clone().into();
                let nt_seq2: Nucleotides = dais_entry.query_cds_aln.clone().into();

                if nt_seq1.len() == nt_seq2.len() {
                    let mut entry = Entry {
                        sample_id: &dais_entry.sample_id,
                        ref_strain: &ref_entry.ref_id,
                        dais_ref_id: &dais_entry.dais_ref_id,
                        protein: &dais_entry.protein,
                        position_in_codon: 0,
                        ref_codon: "NNN".to_string(),
                        mut_codon: "NNN".to_string(),
                        aa_position: 0,
                        aa_ref: 'X',
                        aa_mut: 'X',
                        variant_of_interest: false,
                    };

                    let mut tail_index = 0;
                    let (codons1, tail1) = nt_seq1.as_codons();
                    let (codons2, tail2) = nt_seq2.as_codons();

                    for (index, (ref_codon, query_codon)) in
                        codons1.iter().zip(codons2.iter()).enumerate()
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

                        if entry.update_entry(
                            &ref_entry.dais_ref_id,
                            ref_aa,
                            query_aa,
                            &muts_interest,
                        ) {
                            let Entry {
                                sample_id,
                                ref_strain,
                                dais_ref_id: dais_ref,
                                protein,
                                position_in_codon: _,
                                ref_codon,
                                mut_codon,
                                aa_ref,
                                aa_position,
                                aa_mut,
                                variant_of_interest,
                            } = &entry;
                            let d = &delim;
                            let ctype = &dais_entry.ctype;

                            let codon_nt_start = (aa_index - 1) * 3;
                            for (offset, (ref_nt, query_nt)) in
                                ref_codon.bytes().zip(mut_codon.bytes()).enumerate()
                            {
                                let nt_position = codon_nt_start + offset + 1;
                                let position_in_codon = offset + 1;
                                writeln!(
                                    &mut writer,
                                    "{sample_id}{d}{ref_strain}{d}\
                                            {ctype}{d}{dais_ref}{d}{protein}{d}\
                                            {nt_position}{d}{}{d}{}{d}\
                                            {position_in_codon}{d}\
                                            {mut_codon}{d}{ref_codon}{d}\
                                            {aa_ref}:{aa_position}:{aa_mut}{d}\
                                            {variant_of_interest}",
                                    query_nt as char, ref_nt as char,
                                )?;
                            }
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
                    entry.aa_ref = '~';
                    entry.aa_mut = '~';

                    if entry.update_entry(
                        &ref_entry.dais_ref_id,
                        partial_codon,
                        partial_codon,
                        &muts_interest,
                    ) {
                        let Entry {
                            sample_id,
                            ref_strain,
                            dais_ref_id: dais_ref,
                            protein,
                            position_in_codon: _,
                            ref_codon,
                            mut_codon,
                            aa_ref,
                            aa_position,
                            aa_mut,
                            variant_of_interest,
                        } = &entry;
                        let d = &delim;
                        let ctype = &dais_entry.ctype;

                        let codon_nt_start = tail_index * 3;
                        for (offset, (ref_nt, query_nt)) in
                            tail1.iter().zip(tail2.iter()).enumerate()
                        {
                            let nt_position = codon_nt_start + offset + 1;
                            let position_in_codon = offset + 1;
                            writeln!(
                                &mut writer,
                                "{sample_id}{d}{ref_strain}{d}\
                                        {ctype}{d}{dais_ref}{d}{protein}{d}\
                                        {nt_position}{d}{}{d}{}{d}\
                                        {position_in_codon}{d}\
                                        {mut_codon}{d}{ref_codon}{d}\
                                        {aa_ref}:{aa_position}:{aa_mut}{d}\
                                        {variant_of_interest}",
                                *query_nt as char, *ref_nt as char,
                            )?;
                        }
                    }
                } else {
                    println!(
                        "Warning: Aligned sequences for sample {} (length {}) and reference {} (length {}) have different lengths. Skipping this pair.",
                        dais_entry.sample_id,
                        nt_seq1.len(),
                        ref_entry.ref_id,
                        nt_seq2.len()
                    );
                }
            }
        }
    }
    Ok(())
}
