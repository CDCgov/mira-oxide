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
#[command(about = "Tool for observing nucleotide, codon and amino acid differences.")]
#[allow(clippy::struct_excessive_bools)]
pub struct VariantsArgs {
    #[arg(short = 'q', long)]
    /// Input dais-ribosome file
    query_dais_file: PathBuf,

    #[arg(short = 'r', long)]
    /// Reference dais-ribosome file.
    ref_dais_file: Option<PathBuf>,

    #[arg(short = 'v', long)]
    /// Variants-of-interest (mutations) file.
    variants_of_interest: Option<PathBuf>,

    #[arg(short = 'i', long)]
    /// Query insertion (.ins) file
    query_insertion_file: PathBuf,

    #[arg(short = 'd', long)]
    /// Query deletion (.del) file
    query_deletion_file: PathBuf,

    #[arg(short = 'j', long)]
    /// Reference insertion (.ins) file.
    ref_insertion_file: Option<PathBuf>,

    #[arg(short = 'e', long)]
    /// Reference deletion (.del) file.
    ref_deletion_file: Option<PathBuf>,

    #[arg(short = 'm', long)]
    /// Minor variants (.csv, with headers) file.
    minor_variants: Option<PathBuf>,

    #[arg(short = 'o', long)]
    /// Optional output delimited file. If not provided printes to screen
    output_xsv: Option<PathBuf>,

    #[arg(short = 's', long, default_value = ",")]
    /// Use the provider delimiter for separating fields. Default is ','
    output_delimiter: String,

    #[arg(short = 'a', long)]
    /// Print all positions in the positions-of-interest report, not just those where the
    /// query and reference nucleotides differ. Has no effect in --annotate-minor-variants mode
    /// or when --all-diffs is used.
    all_positions: bool,

    #[arg(long = "all-diffs")]
    /// Mode selector. Reports variant information at every nucleotide difference between each
    // query and its matching reference without restricting to specific positions.
    all_diffs: bool,

    #[arg(long = "positions-of-interest")]
    /// Mode selector. reports variant information when nucleotide differences are found within protein positions listed in a variants-of-interest file
    positions_of_interest_mode: bool,

    #[arg(long)]
    /// Mode selector. Annotate the file given by --minor-variants (required in this mode) with
    /// codon/amino-acid context derived from the query dais data. Does not require any
    /// reference dais/insertion/deletion files. Exactly one of --all-diffs,
    /// --positions-of-interest, or --annotate-minor-variants must be given.
    annotate_minor_variants: bool,
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
pub struct VarsOfInterestInput {
    subtype: String,
    protein: String,
    aa_position: String,
    aa: String,
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

/// Minor variants file
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
        muts_columns: &[VarsOfInterestInput],
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

    /// Returns true if this entry's (dais_ref_id, protein, aa_position) matches any row in
    /// `muts_columns`, regardless of the amino acid listed there. Used for the `all-diffs`
    /// report's `position_of_interest` column, which flags whether an nt diff falls within a
    /// codon that is a position of interest at all -- independent of whether the observed
    /// amino acid is the flagged one (that's what `variant_of_interest` checks).
    fn is_position_of_interest(
        &self,
        dais_ref_id: &str,
        muts_columns: &[VarsOfInterestInput],
    ) -> bool {
        muts_columns.iter().any(|muts_entry| {
            let assigned_ref = assign_dais_refs(&muts_entry.subtype, &muts_entry.protein);
            (assigned_ref == Some(dais_ref_id) || muts_entry.subtype.to_lowercase() == "all")
                && self.protein == muts_entry.protein
                && self.aa_position.to_string() == muts_entry.aa_position
        })
    }
}

/// Computes an insertion/deletion-adjusted nucleotide position on the query side.
/// Insertions applied first (adding the length of each `inserted_nt` occurring upstream of the raw position), then deletions
/// (subtracting `del_cds_len` for each deletion whose deleted region ends at or before the already-adjusted position).
fn calc_query_nt_position(
    raw_position: usize,
    sample_id: &str,
    ctype: &str,
    dais_ref_id: &str,
    protein: &str,
    insertions: &[InsertionInput],
    deletions: &[DeletionInput],
) -> usize {
    let mut position = i64::try_from(raw_position).unwrap_or(i64::MAX);

    for ins in insertions {
        if ins.query_id == sample_id
            && ins.ctype == ctype
            && ins.reference_id == dais_ref_id
            && ins.product_name == protein
            && ins.upstream_nt_pos < raw_position.try_into().unwrap()
        {
            position += i64::try_from(ins.inserted_nt.len()).unwrap_or(i64::MAX);
        }
    }

    for del in deletions {
        if del.query_id == sample_id
            && del.ctype == ctype
            && del.reference_id == dais_ref_id
            && del.product_name == protein
            && (del.del_cds_start + del.del_cds_len) <= position
        {
            position -= del.del_cds_len;
        }
    }

    usize::try_from(position.max(0)).unwrap_or(usize::MAX)
}

/// Computes an insertion/deletion-adjusted nucleotide position on the reference side.
/// Insertion applied first, then deletions, mirroring `calc_query_nt_position` but against the reference-side insertion/deletion files.
fn calc_ref_nt_position(
    raw_position: usize,
    ref_strain: &str,
    ctype: &str,
    dais_ref_id: &str,
    protein: &str,
    ref_insertions: &[InsertionInput],
    ref_deletions: &[DeletionInput],
) -> usize {
    let raw_position_i64 = i64::try_from(raw_position).unwrap_or(i64::MAX);
    let mut position = raw_position_i64;

    for ins in ref_insertions {
        if ins.query_id == ref_strain
            && ins.ctype == ctype
            && ins.reference_id == dais_ref_id
            && ins.product_name == protein
            && ins.upstream_nt_pos < raw_position_i64
        {
            position += i64::try_from(ins.inserted_nt.len()).unwrap_or(i64::MAX);
        }
    }

    for del in ref_deletions {
        if del.query_id == ref_strain
            && del.ctype == ctype
            && del.reference_id == dais_ref_id
            && del.product_name == protein
            && (del.del_cds_start + del.del_cds_len) <= position
        {
            position -= del.del_cds_len;
        }
    }

    usize::try_from(position.max(0)).unwrap_or(usize::MAX)
}

/// Computes an insertion/deletion-adjusted amino acid position on the query side.
/// Insertion applied first (adding the length of each `inserted_aa` occurring upstream of the raw position), then deletions
/// (subtracting `del_aa_len` for each deletion whose deleted region ends at or before the already-adjusted position).
fn calc_query_aa_position(
    raw_position: usize,
    sample_id: &str,
    ctype: &str,
    dais_ref_id: &str,
    protein: &str,
    insertions: &[InsertionInput],
    deletions: &[DeletionInput],
) -> usize {
    let raw_position_i64 = i64::try_from(raw_position).unwrap_or(i64::MAX);
    let mut position = raw_position_i64;

    for ins in insertions {
        if ins.query_id == sample_id
            && ins.ctype == ctype
            && ins.reference_id == dais_ref_id
            && ins.product_name == protein
            && ins.upstream_aa_pos < raw_position_i64
        {
            position =
                position.saturating_add(i64::try_from(ins.inserted_aa.len()).unwrap_or(i64::MAX));
        }
    }

    for del in deletions {
        if del.query_id == sample_id
            && del.ctype == ctype
            && del.reference_id == dais_ref_id
            && del.product_name == protein
            && (del.del_aa_start + del.del_aa_len) <= position
        {
            position -= del.del_aa_len;
        }
    }

    usize::try_from(position.max(0)).unwrap_or(usize::MAX)
}

/// Computes an insertion/deletion-adjusted amino acid position on the reference side.
/// Insertion  applied first, then deletion, mirroring `calc_query_aa_position` but against the reference-side insertion/deletion files.
fn calc_ref_aa_position(
    raw_position: usize,
    ref_strain: &str,
    ctype: &str,
    dais_ref_id: &str,
    protein: &str,
    ref_insertions: &[InsertionInput],
    ref_deletions: &[DeletionInput],
) -> usize {
    let raw_position_i64 = i64::try_from(raw_position).unwrap_or(i64::MAX);
    let mut position = raw_position_i64;

    for ins in ref_insertions {
        if ins.query_id == ref_strain
            && ins.ctype == ctype
            && ins.reference_id == dais_ref_id
            && ins.product_name == protein
            && ins.upstream_aa_pos < raw_position_i64
        {
            position += i64::try_from(ins.inserted_aa.len()).unwrap_or(i64::MAX);
        }
    }

    for del in ref_deletions {
        if del.query_id == ref_strain
            && del.ctype == ctype
            && del.reference_id == dais_ref_id
            && del.product_name == protein
            && (del.del_aa_start + del.del_aa_len) <= position
        {
            position -= del.del_aa_len;
        }
    }

    usize::try_from(position.max(0)).unwrap_or(usize::MAX)
}

/// Looks up all minor variant rows matching the given sample, ctype, and query nt position.
/// Matches on reference==ctype, `sample_position`==`query_nt_position`, and `sample_id` containing
/// sample as a substring (e.g. sample `sample_1` should match `sample_id` "`sample_1_4`).
/// Returns every matching row, since a single position can have more than one minor variant
/// (e.g. two different minority alleles reported at the same position).
fn find_minor_variants<'a>(
    minor_variants: &'a [MinorVariantInput],
    sample_id: &str,
    ctype: &str,
    query_nt_position: usize,
) -> Vec<&'a MinorVariantInput> {
    minor_variants
        .iter()
        .filter(|mv| {
            sample_id.contains(mv.sample.as_str())
                && mv.reference == ctype
                && usize::try_from(mv.sample_position).ok() == Some(query_nt_position)
        })
        .collect()
}

/// Substitutes a minor variant's `minority_allele` into the query codon at the given
/// 1-indexed `position_in_codon`, returning the resulting codon and its translated amino acid.
/// Returns None if the codon isn't exactly 3 bytes or `position_in_codon` is out of range.
fn build_minor_variant_codon(
    query_codon: &str,
    position_in_codon: usize,
    minority_allele: &str,
) -> Option<(String, char)> {
    let mut codon_bytes = query_codon.as_bytes().to_vec();
    if codon_bytes.len() != 3 || position_in_codon == 0 || position_in_codon > 3 {
        return None;
    }
    let allele_byte = *minority_allele.as_bytes().first()?;
    codon_bytes[position_in_codon - 1] = allele_byte;
    let aa = StdGeneticCode::translate_codon(&codon_bytes) as char;
    let codon_str = std::str::from_utf8(&codon_bytes).ok()?.to_string();
    Some((codon_str, aa))
}

/// Finds the raw (aln) nucleotide position within `query_cds_aln` that maps to the given
/// adjusted `query_nt_position`, by searching raw positions 1..=len and adjusting each with
/// `calc_query_nt_position` until one matches. Returns None if no raw position maps to it.
/// Does not filter insertions/deletions by protein, since minor-variants-only mode has no
/// protein context to match against.
fn find_raw_query_position(
    aln_len: usize,
    target_query_nt_position: usize,
    sample_id: &str,
    ctype: &str,
    insertions: &[InsertionInput],
    deletions: &[DeletionInput],
) -> Option<usize> {
    (1..=aln_len).find(|&raw_pos| {
        let raw_pos_i64 = i64::try_from(raw_pos).unwrap_or(i64::MAX);
        let mut position = raw_pos_i64;

        for ins in insertions {
            if ins.query_id == sample_id && ins.ctype == ctype && ins.upstream_nt_pos < raw_pos_i64
            {
                position += i64::try_from(ins.inserted_nt.len()).unwrap_or(i64::MAX);
            }
        }

        for del in deletions {
            if del.query_id == sample_id
                && del.ctype == ctype
                && (del.del_cds_start + del.del_cds_len) <= position
            {
                position -= del.del_cds_len;
            }
        }

        usize::try_from(position.max(0)).unwrap_or(usize::MAX) == target_query_nt_position
    })
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
pub fn variants_process(args: VariantsArgs) -> Result<(), Box<dyn Error>> {
    let mode_count = [
        args.all_diffs,
        args.positions_of_interest_mode,
        args.annotate_minor_variants,
    ]
    .iter()
    .filter(|&&b| b)
    .count();

    if mode_count != 1 {
        return Err(
            "Exactly one of --all-diffs, --positions-of-interest, or --annotate-minor-variants must be provided."
                .into(),
        );
    }

    if args.positions_of_interest_mode && args.variants_of_interest.is_none() {
        return Err(
            "--variants-of-interest is required when --positions-of-interest mode is used.".into(),
        );
    }

    if args.annotate_minor_variants && args.minor_variants.is_none() {
        return Err(
            "--minor-variants is required when --annotate-minor-variants mode is used.".into(),
        );
    }

    let delim = args.output_delimiter;
    let all_positions = args.all_positions;
    let all_diffs = args.all_diffs;

    // Query dais-ribosome file is always required.
    let dais_reader = create_reader(Some(&args.query_dais_file))?;
    let dais: Vec<QueryInput> = read_tsv(dais_reader, false)?;
    println!(
        "Read {} entries from the input dais-ribosome file.",
        dais.len()
    );

    // Query insertions/deletions are always required.
    let ins_reader = create_reader(Some(&args.query_insertion_file))?;
    let insertions: Vec<InsertionInput> = read_tsv(ins_reader, false)?;
    println!(
        "Read {} entries from the insertion file: {:?}",
        insertions.len(),
        &args.query_insertion_file.display()
    );

    let del_reader = create_reader(Some(&args.query_deletion_file))?;
    let deletions: Vec<DeletionInput> = read_tsv(del_reader, false)?;
    println!(
        "Read {} entries from the deletion file: {:?}",
        deletions.len(),
        &args.query_deletion_file.display()
    );

    // Optional: minor variants (.csv, with headers)
    let minor_variants: Vec<MinorVariantInput> = if let Some(mv_path) = &args.minor_variants {
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
    let include_minor_variants = args.minor_variants.is_some();

    if all_diffs {
        // --all-diffs mode: requires ref dais/insertion/deletion files, but positions-of-interest
        // is optional (only affects whether variant_of_interest is computed/printed).
        let ref_dais_file = args
            .ref_dais_file
            .ok_or("--ref-dais-file is required when --all-diffs is used.")?;
        let ref_insertion_file = args
            .ref_insertion_file
            .ok_or("--ref-insertion-file is required when --all-diffs is used.")?;
        let ref_deletion_file = args
            .ref_deletion_file
            .ok_or("--ref-deletion-file is required when --all-diffs is used.")?;

        println!(
            "Processing all nucleotide differences for input file: {:?} and reference file: {:?}",
            &args.query_dais_file.display(),
            &ref_dais_file.display()
        );

        // positions-of-interest is optional in this mode.
        let muts_interest: Vec<VarsOfInterestInput> =
            if let Some(muts_path) = &args.variants_of_interest {
                let muts_reader = create_reader(Some(muts_path))?;
                let parsed: Vec<VarsOfInterestInput> = read_tsv(muts_reader, false)?;
                println!(
                    "Read {} entries from the variants of interest file.",
                    parsed.len()
                );
                parsed
            } else {
                Vec::new()
            };
        let include_variant_of_interest = args.variants_of_interest.is_some();

        let ref_reader = create_reader(Some(&ref_dais_file))?;
        let refs: Vec<RefDaisInput> = read_tsv(ref_reader, false)?;
        println!(
            "Read {} entries from the reference dais-ribosome file.",
            refs.len()
        );

        let ref_ins_reader = create_reader(Some(&ref_insertion_file))?;
        let ref_insertions: Vec<InsertionInput> = read_tsv(ref_ins_reader, false)?;
        println!(
            "Read {} entries from the reference insertion file: {:?}",
            ref_insertions.len(),
            &ref_insertion_file.display()
        );

        let ref_del_reader = create_reader(Some(&ref_deletion_file))?;
        let ref_deletions: Vec<DeletionInput> = read_tsv(ref_del_reader, false)?;
        println!(
            "Read {} entries from the reference deletion file: {:?}",
            ref_deletions.len(),
            &ref_deletion_file.display()
        );

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

        let mut header = String::from(
            "query_name,ref_name,ctype,dais_reference,protein,aln_nt_position,ref_nt_position,query_nt_position,ref_nt,query_nt,position_in_codon,ref_codon,query_codon,aln_aa_position,ref_aa_position,query_aa_position,aa_mutation",
        );
        if include_variant_of_interest {
            header.push_str(",variant_of_interest,position_of_interest");
        }
        if include_minor_variants {
            header.push_str(",depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,minor_variant_codon,consensus_aa,minor_variant_aa");
        }
        writeln!(&mut writer, "{header}")?;

        for dais_entry in &dais {
            for ref_entry in &refs {
                if dais_entry.ctype == ref_entry.ctype
                    && dais_entry.dais_ref_id == ref_entry.dais_ref_id
                    && dais_entry.protein == ref_entry.protein
                {
                    let nt_seq1: Nucleotides = ref_entry.ref_cds_aln.clone().into();
                    let nt_seq2: Nucleotides = dais_entry.query_cds_aln.clone().into();

                    if nt_seq1.len() != nt_seq2.len() {
                        println!(
                            "Warning: Aligned sequences for sample {} (length {}) and reference {} (length {}) have different lengths. Skipping this pair.",
                            dais_entry.sample_id,
                            nt_seq1.len(),
                            ref_entry.ref_id,
                            nt_seq2.len()
                        );
                        continue;
                    }

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

                        // Only used to populate variant_of_interest when requested; the row
                        // itself is still emitted regardless of whether this matches.
                        let matched_poi = if include_variant_of_interest {
                            entry.update_entry(
                                &ref_entry.dais_ref_id,
                                ref_aa,
                                query_aa,
                                &muts_interest,
                            )
                        } else {
                            false
                        };
                        let _ = matched_poi;

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
                        let aln_aa_position = *aa_position;
                        let query_aa_position = calc_query_aa_position(
                            aln_aa_position,
                            sample_id,
                            ctype,
                            dais_ref,
                            protein,
                            &insertions,
                            &deletions,
                        );
                        let ref_aa_position = calc_ref_aa_position(
                            aln_aa_position,
                            ref_strain,
                            ctype,
                            dais_ref,
                            protein,
                            &ref_insertions,
                            &ref_deletions,
                        );
                        for (offset, (ref_nt, query_nt)) in
                            ref_codon.bytes().zip(mut_codon.bytes()).enumerate()
                        {
                            if ref_nt == query_nt {
                                continue;
                            }

                            let nt_position = codon_nt_start + offset + 1;
                            let position_in_codon = offset + 1;
                            let query_nt_position = calc_query_nt_position(
                                nt_position,
                                sample_id,
                                ctype,
                                dais_ref,
                                protein,
                                &insertions,
                                &deletions,
                            );
                            let ref_nt_position = calc_ref_nt_position(
                                nt_position,
                                ref_strain,
                                ctype,
                                dais_ref,
                                protein,
                                &ref_insertions,
                                &ref_deletions,
                            );

                            let mut row = format!(
                                "{sample_id}{d}{ref_strain}{d}\
                                        {ctype}{d}{dais_ref}{d}{protein}{d}\
                                        {nt_position}{d}{ref_nt_position}{d}{query_nt_position}{d}{}{d}{}{d}\
                                        {position_in_codon}{d}\
                                        {ref_codon}{d}{mut_codon}{d}\
                                        {aln_aa_position}{d}{ref_aa_position}{d}{query_aa_position}{d}\
                                        {aa_ref}:{aa_position}:{aa_mut}",
                                ref_nt as char, query_nt as char,
                            );
                            if include_variant_of_interest {
                                let poi = entry.is_position_of_interest(dais_ref, &muts_interest);
                                row.push_str(&format!("{d}{variant_of_interest}{d}{poi}"));
                            }

                            if include_minor_variants {
                                let mv_matches = find_minor_variants(
                                    &minor_variants,
                                    sample_id,
                                    ctype,
                                    query_nt_position,
                                );
                                if mv_matches.is_empty() {
                                    row.push_str(&format!("{d}{d}{d}{d}{d}{d}{d}{d}"));
                                    writeln!(&mut writer, "{row}")?;
                                } else {
                                    for mv in &mv_matches {
                                        let (mv_codon, mv_aa) = match build_minor_variant_codon(
                                            mut_codon,
                                            position_in_codon,
                                            &mv.minority_allele,
                                        ) {
                                            Some((codon, aa)) => (codon, aa.to_string()),
                                            None => (String::new(), String::new()),
                                        };
                                        let mv_suffix = format!(
                                            "{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}",
                                            mv.depth,
                                            mv.consensus_allele,
                                            mv.minority_allele,
                                            mv.consensus_count,
                                            mv.minority_count,
                                            mv.minority_frequency,
                                            mv_aa,
                                            mv_codon,
                                            mv_aa
                                        );
                                        writeln!(&mut writer, "{row}{mv_suffix}")?;
                                    }
                                }
                            } else {
                                writeln!(&mut writer, "{row}")?;
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

                    if include_variant_of_interest {
                        entry.update_entry(
                            &ref_entry.dais_ref_id,
                            partial_codon,
                            partial_codon,
                            &muts_interest,
                        );
                    }

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
                    let aln_aa_position = *aa_position;
                    let query_aa_position = calc_query_aa_position(
                        aln_aa_position,
                        sample_id,
                        ctype,
                        dais_ref,
                        protein,
                        &insertions,
                        &deletions,
                    );
                    let ref_aa_position = calc_ref_aa_position(
                        aln_aa_position,
                        ref_strain,
                        ctype,
                        dais_ref,
                        protein,
                        &ref_insertions,
                        &ref_deletions,
                    );
                    for (offset, (ref_nt, query_nt)) in tail1.iter().zip(tail2.iter()).enumerate() {
                        if ref_nt == query_nt {
                            continue;
                        }

                        let nt_position = codon_nt_start + offset + 1;
                        let position_in_codon = offset + 1;
                        let query_nt_position = calc_query_nt_position(
                            nt_position,
                            sample_id,
                            ctype,
                            dais_ref,
                            protein,
                            &insertions,
                            &deletions,
                        );
                        let ref_nt_position = calc_ref_nt_position(
                            nt_position,
                            ref_strain,
                            ctype,
                            dais_ref,
                            protein,
                            &ref_insertions,
                            &ref_deletions,
                        );

                        let mut row = format!(
                            "{sample_id}{d}{ref_strain}{d}\
                                    {ctype}{d}{dais_ref}{d}{protein}{d}\
                                    {nt_position}{d}{ref_nt_position}{d}{query_nt_position}{d}{}{d}{}{d}\
                                    {position_in_codon}{d}\
                                    {mut_codon}{d}{ref_codon}{d}\
                                    {aa_ref}:{aa_position}:{aa_mut}{d}\
                                    {aln_aa_position}{d}{ref_aa_position}{d}{query_aa_position}",
                            *query_nt as char, *ref_nt as char,
                        );
                        if include_variant_of_interest {
                            let poi = entry.is_position_of_interest(dais_ref, &muts_interest);
                            row.push_str(&format!("{d}{variant_of_interest}{d}{poi}"));
                        }

                        if include_minor_variants {
                            let mv_matches = find_minor_variants(
                                &minor_variants,
                                sample_id,
                                ctype,
                                query_nt_position,
                            );
                            if mv_matches.is_empty() {
                                row.push_str(&format!("{d}{d}{d}{d}{d}{d}{d}{d}"));
                                writeln!(&mut writer, "{row}")?;
                            } else {
                                for mv in &mv_matches {
                                    let (mv_codon, mv_aa) = match build_minor_variant_codon(
                                        mut_codon,
                                        position_in_codon,
                                        &mv.minority_allele,
                                    ) {
                                        Some((codon, aa)) => (codon, aa.to_string()),
                                        None => (String::new(), String::new()),
                                    };
                                    let mv_suffix = format!(
                                        "{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}",
                                        mv.depth,
                                        mv.consensus_allele,
                                        mv.minority_allele,
                                        mv.consensus_count,
                                        mv.minority_count,
                                        mv.minority_frequency,
                                        mv_codon,
                                        mv_aa
                                    );
                                    writeln!(&mut writer, "{row}{mv_suffix}")?;
                                }
                            }
                        } else {
                            writeln!(&mut writer, "{row}")?;
                        }
                    }
                }
            }
        }
    } else if args.positions_of_interest_mode {
        let muts_path = args
            .variants_of_interest
            .as_ref()
            .expect("validated above: --variants-of-interest is required in this mode");
        // Full positions-of-interest mode: requires ref dais/insertion/deletion files.
        let ref_dais_file = args
            .ref_dais_file
            .ok_or("--ref-dais-file is required when --positions-of-interest is used.")?;
        let ref_insertion_file = args
            .ref_insertion_file
            .ok_or("--ref-insertion-file is required when --positions-of-interest is used.")?;
        let ref_deletion_file = args
            .ref_deletion_file
            .ok_or("--ref-deletion-file is required when --positions-of-interest is used.")?;

        println!(
            "Processing positions of interest for input file: {:?}, reference file: {:?}, and variants file: {:?}",
            &args.query_dais_file.display(),
            &ref_dais_file.display(),
            &muts_path.display()
        );

        let muts_reader = create_reader(Some(muts_path))?;
        let muts_interest: Vec<VarsOfInterestInput> = read_tsv(muts_reader, false)?;
        println!(
            "Read {} entries from the variants of interest file.",
            muts_interest.len()
        );

        let ref_reader = create_reader(Some(&ref_dais_file))?;
        let refs: Vec<RefDaisInput> = read_tsv(ref_reader, false)?;
        println!(
            "Read {} entries from the reference dais-ribosome file.",
            refs.len()
        );

        let ref_ins_reader = create_reader(Some(&ref_insertion_file))?;
        let ref_insertions: Vec<InsertionInput> = read_tsv(ref_ins_reader, false)?;
        println!(
            "Read {} entries from the reference insertion file: {:?}",
            ref_insertions.len(),
            &ref_insertion_file.display()
        );

        let ref_del_reader = create_reader(Some(&ref_deletion_file))?;
        let ref_deletions: Vec<DeletionInput> = read_tsv(ref_del_reader, false)?;
        println!(
            "Read {} entries from the reference deletion file: {:?}",
            ref_deletions.len(),
            &ref_deletion_file.display()
        );

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
        let mut header = String::from(
            "query_name,ref_name,ctype,dais_reference,protein,aln_nt_position,ref_nt_position,query_nt_position,query_nt,ref_nt,position_in_codon,query_codon,ref_codon,aa_mutation,aln_aa_position,ref_aa_position,query_aa_position,variant_of_interest",
        );
        if include_minor_variants {
            header.push_str(",depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,consensus_aa,minor_variant_codon,minor_variant_aa");
        }
        writeln!(&mut writer, "{header}")?;

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
                                let aln_aa_position = *aa_position;
                                let query_aa_position = calc_query_aa_position(
                                    aln_aa_position,
                                    sample_id,
                                    ctype,
                                    dais_ref,
                                    protein,
                                    &insertions,
                                    &deletions,
                                );
                                let ref_aa_position = calc_ref_aa_position(
                                    aln_aa_position,
                                    ref_strain,
                                    ctype,
                                    dais_ref,
                                    protein,
                                    &ref_insertions,
                                    &ref_deletions,
                                );
                                for (offset, (ref_nt, query_nt)) in
                                    ref_codon.bytes().zip(mut_codon.bytes()).enumerate()
                                {
                                    let nt_position = codon_nt_start + offset + 1;
                                    let position_in_codon = offset + 1;
                                    let query_nt_position = calc_query_nt_position(
                                        nt_position,
                                        sample_id,
                                        ctype,
                                        dais_ref,
                                        protein,
                                        &insertions,
                                        &deletions,
                                    );
                                    let ref_nt_position = calc_ref_nt_position(
                                        nt_position,
                                        ref_strain,
                                        ctype,
                                        dais_ref,
                                        protein,
                                        &ref_insertions,
                                        &ref_deletions,
                                    );

                                    // Only emit this position if the caller asked for all
                                    // positions, or the query/reference nucleotides actually differ.
                                    if !all_positions && query_nt == ref_nt {
                                        continue;
                                    }

                                    let mv_matches = find_minor_variants(
                                        &minor_variants,
                                        sample_id,
                                        ctype,
                                        query_nt_position,
                                    );
                                    let mv_suffixes: Vec<String> = if include_minor_variants {
                                        if mv_matches.is_empty() {
                                            vec![format!("{d}{d}{d}{d}{d}{d}{d}{d}")]
                                        } else {
                                            mv_matches
                                                .iter()
                                                .map(|mv| {
                                                    let (mv_codon, mv_aa) =
                                                        match build_minor_variant_codon(
                                                            mut_codon,
                                                            position_in_codon,
                                                            &mv.minority_allele,
                                                        ) {
                                                            Some((codon, aa)) => {
                                                                (codon, aa.to_string())
                                                            }
                                                            None => (String::new(), String::new()),
                                                        };
                                                    format!(
                                                        "{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}",
                                                        mv.depth,
                                                        mv.consensus_allele,
                                                        mv.minority_allele,
                                                        mv.consensus_count,
                                                        mv.minority_count,
                                                        mv.minority_frequency,
                                                        mv_codon,
                                                        mv_aa
                                                    )
                                                })
                                                .collect()
                                        }
                                    } else {
                                        vec![String::new()]
                                    };
                                    for mv_suffix in &mv_suffixes {
                                        writeln!(
                                            &mut writer,
                                            "{sample_id}{d}{ref_strain}{d}\
                                                    {ctype}{d}{dais_ref}{d}{protein}{d}\
                                                    {nt_position}{d}{ref_nt_position}{d}{query_nt_position}{d}{}{d}{}{d}\
                                                    {position_in_codon}{d}\
                                                    {mut_codon}{d}{ref_codon}{d}\
                                                    {aa_ref}:{aa_position}:{aa_mut}{d}\
                                                    {aln_aa_position}{d}{ref_aa_position}{d}{query_aa_position}{d}\
                                                    {variant_of_interest}{mv_suffix}",
                                            query_nt as char, ref_nt as char,
                                        )?;
                                    }
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
                            let aln_aa_position = *aa_position;
                            let query_aa_position = calc_query_aa_position(
                                aln_aa_position,
                                sample_id,
                                ctype,
                                dais_ref,
                                protein,
                                &insertions,
                                &deletions,
                            );
                            let ref_aa_position = calc_ref_aa_position(
                                aln_aa_position,
                                ref_strain,
                                ctype,
                                dais_ref,
                                protein,
                                &ref_insertions,
                                &ref_deletions,
                            );
                            for (offset, (ref_nt, query_nt)) in
                                tail1.iter().zip(tail2.iter()).enumerate()
                            {
                                let nt_position = codon_nt_start + offset + 1;
                                let position_in_codon = offset + 1;
                                let query_nt_position = calc_query_nt_position(
                                    nt_position,
                                    sample_id,
                                    ctype,
                                    dais_ref,
                                    protein,
                                    &insertions,
                                    &deletions,
                                );
                                let ref_nt_position = calc_ref_nt_position(
                                    nt_position,
                                    ref_strain,
                                    ctype,
                                    dais_ref,
                                    protein,
                                    &ref_insertions,
                                    &ref_deletions,
                                );

                                // Only emit this position if the caller asked for all
                                // positions, or the query/reference nucleotides actually differ.
                                if !all_positions && query_nt == ref_nt {
                                    continue;
                                }

                                let mv_matches = find_minor_variants(
                                    &minor_variants,
                                    sample_id,
                                    ctype,
                                    query_nt_position,
                                );
                                let mv_suffixes: Vec<String> = if include_minor_variants {
                                    if mv_matches.is_empty() {
                                        vec![format!("{d}{d}{d}{d}{d}{d}{d}{d}{d}{d}")]
                                    } else {
                                        mv_matches
                                            .iter()
                                            .map(|mv| {
                                                let (consensus_codon, consensus_aa) =
                                                    match build_minor_variant_codon(
                                                        mut_codon,
                                                        position_in_codon,
                                                        &mv.consensus_allele,
                                                    ) {
                                                        Some((codon, aa)) => {
                                                            (codon, aa.to_string())
                                                        }
                                                        None => (String::new(), String::new()),
                                                    };
                                                let (mv_codon, mv_aa) =
                                                    match build_minor_variant_codon(
                                                        mut_codon,
                                                        position_in_codon,
                                                        &mv.minority_allele,
                                                    ) {
                                                        Some((codon, aa)) => {
                                                            (codon, aa.to_string())
                                                        }
                                                        None => (String::new(), String::new()),
                                                    };
                                                format!(
                                                    "{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}{d}{}",
                                                    mv.depth,
                                                    mv.consensus_allele,
                                                    mv.minority_allele,
                                                    mv.consensus_count,
                                                    mv.minority_count,
                                                    mv.minority_frequency,
                                                    consensus_codon,
                                                    consensus_aa,
                                                    mv_codon,
                                                    mv_aa
                                                )
                                            })
                                            .collect()
                                    }
                                } else {
                                    vec![String::new()]
                                };
                                for mv_suffix in &mv_suffixes {
                                    writeln!(
                                        &mut writer,
                                        "{sample_id}{d}{ref_strain}{d}\
                                                {ctype}{d}{dais_ref}{d}{protein}{d}\
                                                {nt_position}{d}{ref_nt_position}{d}{query_nt_position}{d}{}{d}{}{d}\
                                                {position_in_codon}{d}\
                                                {mut_codon}{d}{ref_codon}{d}\
                                                {aa_ref}:{aa_position}:{aa_mut}{d}\
                                                {aln_aa_position}{d}{ref_aa_position}{d}{query_aa_position}{d}\
                                                {variant_of_interest}{mv_suffix}",
                                        *query_nt as char, *ref_nt as char,
                                    )?;
                                }
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
    } else {
        // annotate-minor-variants mode (validated above: exactly one mode flag is set, and
        // --minor-variants is required here).
        debug_assert!(args.annotate_minor_variants);

        /// Finds the raw (aln) nucleotide position within `query_cds_aln` that maps to the given
        /// adjusted `query_nt_position`, by searching raw positions 1..=len and adjusting each with
        /// `calc_query_nt_position` until one matches. Returns None if no raw position maps to it.
        /// Filters insertions/deletions by `reference_id` and `product_name` (in addition to `sample_id`
        /// and ctype) so that samples with multiple products/references sharing a ctype don't have
        /// unrelated indels applied.
        #[allow(clippy::too_many_arguments)]
        fn find_raw_query_position(
            aln_len: usize,
            target_query_nt_position: usize,
            sample_id: &str,
            ctype: &str,
            reference_id: &str,
            product_name: &str,
            insertions: &[InsertionInput],
            deletions: &[DeletionInput],
        ) -> Option<usize> {
            (1..=aln_len).find(|&raw_pos| {
                let mut position = i64::try_from(raw_pos).unwrap_or(i64::MAX);

                for ins in insertions {
                    if ins.query_id == sample_id
                        && ins.ctype == ctype
                        && ins.reference_id == reference_id
                        && ins.product_name == product_name
                        && ins.upstream_nt_pos < i64::try_from(raw_pos).unwrap_or(i64::MAX)
                    {
                        position += i64::try_from(ins.inserted_nt.len()).unwrap_or(i64::MAX);
                    }
                }

                for del in deletions {
                    if del.query_id == sample_id
                        && del.ctype == ctype
                        && del.reference_id == reference_id
                        && del.product_name == product_name
                        && (del.del_cds_start + del.del_cds_len) <= position
                    {
                        position -= del.del_cds_len;
                    }
                }

                usize::try_from(position).is_ok_and(|position| position == target_query_nt_position)
            })
        }

        // Minor-variants only will annotate the minor variants CSV with minor_variant_codon
        // and minor_variant_aa columns, computed from the query dais data.
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
            "sample{delim}reference{delim}dais_reference{delim}dais_ref_position{delim}sample_position{delim}depth{delim}consensus_allele{delim}minority_allele{delim}consensus_count{delim}minority_count{delim}minority_frequency{delim}consensus_codon{delim}minor_variant_codon{delim}consensus_aa{delim}minor_variant_aa{delim}major_aa_vs_minor_aa{delim}run_id{delim}instrument"
        )?;

        for mv in &minor_variants {
            // Find the query dais row matching this minor variant: sample_id contains sample,
            // ctype == reference. Multiple product rows (e.g. HA-signal, HA, HA1) can share the
            // same sample_id/ctype, so prefer the one with the longest query_cds_aln, since
            // shorter fragments (like signal peptides) can't contain large sample_positions.
            let matching_dais_entry = dais
                .iter()
                .filter(|d| d.sample_id.contains(mv.sample.as_str()) && d.ctype == mv.reference)
                .max_by_key(|d| d.query_cds_aln.len());

            let (dais_reference, dais_ref_position, consensus_codon, consensus_aa, mv_codon, mv_aa) =
                if let Some(dais_entry) = matching_dais_entry {
                    let dais_reference = dais_entry.dais_ref_id.clone();
                    let nt_seq: Nucleotides = dais_entry.query_cds_aln.clone().into();
                    let aln_len = nt_seq.len();
                    let raw_pos = find_raw_query_position(
                        aln_len,
                        usize::try_from(mv.sample_position).unwrap_or(0),
                        &dais_entry.sample_id,
                        &dais_entry.ctype,
                        &dais_entry.dais_ref_id,
                        &dais_entry.protein,
                        &insertions,
                        &deletions,
                    );

                    match raw_pos {
                        Some(raw_pos) => {
                            // dais_ref_position is the raw (pre-indel-adjustment) position; the codon
                            // and amino acid are derived directly from this position.
                            let dais_ref_position = raw_pos;
                            let (codons, tail) = nt_seq.as_codons();
                            let codon_index = (dais_ref_position - 1) / 3;
                            let position_in_codon = ((dais_ref_position - 1) % 3) + 1;
                            let codon_bytes: Option<&[u8]> = if codon_index < codons.len() {
                                Some(&codons[codon_index])
                            } else if codon_index == codons.len() && !tail.is_empty() {
                                Some(tail)
                            } else {
                                None
                            };
                            match codon_bytes {
                                Some(codon_bytes) => {
                                    let codon_str = std::str::from_utf8(codon_bytes)
                                        .unwrap_or_default()
                                        .to_string();
                                    // consensus_codon/consensus_aa reflect the reference codon as-is
                                    // only the minority allele is substituted in to build the minor variant codon/aa.
                                    let (consensus_codon, consensus_aa) = if codon_str.len() == 3 {
                                        let aa =
                                            StdGeneticCode::translate_codon(codon_str.as_bytes())
                                                as char;
                                        (codon_str.clone(), aa.to_string())
                                    } else {
                                        (String::new(), String::new())
                                    };
                                    let (mv_codon, mv_aa) = match build_minor_variant_codon(
                                        &codon_str,
                                        position_in_codon,
                                        &mv.minority_allele,
                                    ) {
                                        Some((codon, aa)) => (codon, aa.to_string()),
                                        None => (String::new(), String::new()),
                                    };
                                    (
                                        dais_reference,
                                        dais_ref_position.to_string(),
                                        consensus_codon,
                                        consensus_aa,
                                        mv_codon,
                                        mv_aa,
                                    )
                                }
                                None => (
                                    dais_reference,
                                    dais_ref_position.to_string(),
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                    String::new(),
                                ),
                            }
                        }
                        None => (
                            dais_reference,
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                        ),
                    }
                } else {
                    (
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    )
                };

            let major_aa_vs_minor_aa = format!("{consensus_aa}:{dais_ref_position}:{mv_aa}");

            writeln!(
                &mut writer,
                "{}{delim}{}{delim}{dais_reference}{delim}{dais_ref_position}{delim}{}{delim}{}{delim}{}{delim}{}{delim}{}{delim}{}{delim}{consensus_codon}{delim}{mv_codon}{delim}{consensus_aa}{delim}{mv_aa}{delim}{major_aa_vs_minor_aa}{delim}{}{delim}{}{delim}{}",
                mv.sample,
                mv.reference,
                mv.sample_position,
                mv.depth,
                mv.consensus_allele,
                mv.minority_allele,
                mv.consensus_count,
                mv.minority_count,
                mv.minority_frequency,
                mv.run_id,
                mv.instrument,
            )?;
        }

        println!(
            "Wrote {} annotated minor variant entries.",
            minor_variants.len()
        );
    }
    Ok(())
}
