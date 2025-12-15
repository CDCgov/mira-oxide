use crate::utils::alignment::align_sequences;
use clap::Parser;
use csv::ReaderBuilder;
use either::Either;
use serde::{self, Deserialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Stdin, Write, stdin, stdout},
    path::PathBuf,
};
use zoe::{
    data::{StdGeneticCode, nucleotides::GetCodons},
    prelude::{Len, Nucleotides},
};

#[derive(Debug, Parser)]
#[command(about = "Tool for calculating amino acid difference tables")]
pub struct VariantsArgs {
    #[arg(short = 'i', long)]
    /// Input dais file
    input_file: PathBuf,

    #[arg(short = 'r', long)]
    /// Reference strains file
    ref_file: PathBuf,

    #[arg(short = 'm', long)]
    /// Variants of interest file
    muts_file: PathBuf,

    #[arg(short = 'v', long)]
    /// virus that is being analyzed
    virus: String,

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

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct DaisInput {
    sample_id: String,
    ctype: String,
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

#[allow(dead_code)]
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
pub struct SampleSubtpyes {
    sample_id: String,
    sample_name: String,
    segment_number: String,
    subtype: String,
}

#[derive(Deserialize, Debug)]
pub struct MutsOfInterestInput {
    subtype: String,
    protein: String,
    aa_position: String,
    aa: String,
    description: String,
}

#[derive(Clone, Debug)]
pub struct Entry<'a> {
    sample_id: &'a str,
    ref_strain: &'a str,
    gisaid_accession: &'a str,
    subtype: &'a str,
    ctype: &'a str,
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

        for muts_entry in muts_columns {
            // Check if the mutation matches the entry
            if subtype == muts_entry.subtype
                && self.protein == muts_entry.protein
                && self.aa_position.to_string() == muts_entry.aa_position
            {
                // Use match for cleaner handling of `hold_aa_mut` cases
                self.phenotypic_consequences = match hold_aa_mut.as_str() {
                    "~" => "partial amino acid".to_string(),
                    "-" => "amino acid covered".to_string(),
                    "X" | "." => "amino acid information missing".to_string(),
                    aa if aa == muts_entry.aa => muts_entry.description.clone(),
                    _ => String::new(),
                };

                return true;
            }
        }

        false
    }
}

impl Entry<'_> {
    // Helper function to compare two entries ignoring `ref_strain`
    fn is_same_except_ref_strain(&self, other: &Entry) -> bool {
        self.sample_id == other.sample_id
            && self.ctype == other.ctype
            && self.dais_ref == other.dais_ref
            && self.protein == other.protein
            && self.ref_codon == other.ref_codon
            && self.mut_codon == other.mut_codon
            && self.aa_ref == other.aa_ref
            && self.aa_position == other.aa_position
            && self.aa_mut == other.aa_mut
            && self.phenotypic_consequences == other.phenotypic_consequences
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

#[must_use]
pub fn extract_unique_samples(inputs: &Vec<DaisInput>) -> Vec<SampleSubtpyes> {
    let mut unique_sample_ids = HashSet::new();
    let mut sample_list = Vec::new();
    let mut sample_key: Vec<SampleSubtpyes> = Vec::new();

    // Collect unique sample IDs
    for entry in inputs {
        if unique_sample_ids.insert(entry.sample_id.clone()) {
            sample_list.push(entry.sample_id.clone());
        }
    }

    // Create SampleSubtypes structs
    for sample in sample_list {
        let parts: Vec<&str> = sample.rsplitn(2, '_').collect();
        if parts.len() != 2 {
            continue;
        }

        let segment_number = parts[0];
        let sample_name = parts[1].to_string();

        sample_key.push(SampleSubtpyes {
            sample_id: sample.clone(),
            sample_name,
            segment_number: segment_number.to_owned(),
            subtype: String::new(),
        });
    }
    #[allow(clippy::assigning_clones)]
    // Update subtype field based on matching sample_id and subtype containing "NA"
    for entry in inputs {
        if entry.ctype.contains("NA") {
            for sample in &mut sample_key {
                if sample.sample_id == entry.sample_id {
                    sample.subtype = entry.ctype.clone();
                }
            }
        }
    }
    // Find the subtype for segment_number 6 for each sample_name
    let mut segment_6_subtypes: HashMap<String, String> = HashMap::new();
    for sample in &sample_key {
        if sample.segment_number == "6" {
            segment_6_subtypes.insert(sample.sample_name.clone(), sample.subtype.clone());
        }
    }

    // Update all entries to use the subtype from segment_number 6 or "unknown" if not found
    for sample in &mut sample_key {
        if let Some(subtype) = segment_6_subtypes.get(&sample.sample_name) {
            sample.subtype = subtype.clone();
        } else {
            sample.subtype = "unknown".to_string();
        }
    }

    sample_key
}

fn find_duplicate_aa_entries_with_diff_strain<'a>(
    entries: &Vec<Entry<'a>>,
    sample_subtypes: &'a [SampleSubtpyes],
) -> Vec<Entry<'a>> {
    let mut result = Vec::new();

    for (i, entry1) in entries.iter().enumerate() {
        let mut has_match = false;

        // Skip entry1 if its sample_id already exists in the result
        if result
            .iter()
            .any(|e: &Entry| e.sample_id == entry1.sample_id)
        {
            continue;
        }

        for entry2 in entries.iter().skip(i + 1) {
            // Skip entry2 if its sample_id already exists in the result
            if result
                .iter()
                .any(|e: &Entry| e.sample_id == entry2.sample_id)
            {
                continue;
            }

            if entry1.is_same_except_ref_strain(entry2) {
                has_match = true;
                for sample in sample_subtypes {
                    if entry1.sample_id == sample.sample_id {
                        match sample.subtype.as_str() {
                            "A_NA_N1" => {
                                if entry1.subtype.contains("H1N1") {
                                    result.push(entry1.clone());
                                } else if entry2.subtype.contains("H1N1") {
                                    result.push(entry2.clone());
                                }
                            }
                            "A_NA_N2" => {
                                if entry1.subtype.contains("H3N2") {
                                    result.push(entry1.clone());
                                } else if entry2.subtype.contains("H3N2") {
                                    result.push(entry2.clone());
                                }
                            }
                            _ => {
                                result.push(entry1.clone());
                            }
                        }
                    }
                }
            }
        }

        // If no match was found for entry1, push it to the result
        if !has_match {
            result.push(entry1.clone());
        }
    }

    result
}

#[allow(clippy::too_many_lines)]
pub fn variants_of_interest_process(args: VariantsArgs) -> Result<(), Box<dyn Error>> {
    let delim = args.output_delimiter;

    let muts_reader = create_reader(Some(&args.muts_file))?;
    let muts_interest: Vec<MutsOfInterestInput> = read_tsv(muts_reader, false)?;

    let dais_reader = create_reader(Some(&args.input_file))?;
    let dais: Vec<DaisInput> = read_tsv(dais_reader, false)?;

    let ref_reader = create_reader(Some(&args.ref_file))?;
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

    // Write the header
    writeln!(
        &mut writer,
        "sample,reference_strain,gisaid_accession,ctype,dais_reference,protein,sample_codon,reference_codon,aa_mutation,phenotypic_consequence",
    )?;

    let mut mutations_vec: Vec<Entry> = Vec::new();

    for dais_entry in &dais {
        for ref_entry in &refs {
            if dais_entry.ctype == ref_entry.ctype
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
                        subtype: &ref_entry.subtype,
                        ctype: &dais_entry.ctype,
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

                        if ref_codon != query_codon {
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
                                mutations_vec.push(entry.clone()); // Save the entry to mutations_vec
                            }
                        }
                    }

                    if tail1 != tail2 {
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
                            mutations_vec.push(entry.clone()); // Save the entry to mutations_vec
                        }
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
                        subtype: &ref_entry.subtype,
                        ctype: &dais_entry.ctype,
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

                        if ref_codon != query_codon {
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
                                mutations_vec.push(entry.clone()); // Save the entry to mutations_vec
                            }
                        }
                    }

                    if !tail1.is_empty() && tail1 != tail2 {
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
                            mutations_vec.push(entry.clone()); // Save the entry to mutations_vec
                        }
                    }
                }
            }
        }
    }

    if &args.virus == "INFLUENZA" {
        let sample_subtypes = extract_unique_samples(&dais);

        let mutations_vec =
            find_duplicate_aa_entries_with_diff_strain(&mutations_vec, &sample_subtypes);

        // Write all entries from mutations_vec at the end
        for entry in &mutations_vec {
            let Entry {
                sample_id,
                ref_strain,
                gisaid_accession,
                subtype: _,
                ctype,
                dais_ref,
                protein,
                ref_codon,
                mut_codon,
                aa_ref,
                aa_position,
                aa_mut,
                phenotypic_consequences,
            } = entry;
            let d = &delim;

            writeln!(
                &mut writer,
                "{sample_id}{d}{ref_strain}{d}{gisaid_accession}{d}\
                {ctype}{d}{dais_ref}{d}{protein}{d}\
                {ref_codon}{d}{mut_codon}{d}\
                {aa_ref}:{aa_position}:{aa_mut}{d}\
                {phenotypic_consequences}",
            )?;
        }
    } else {
        // Write all entries from mutations_vec at the end
        for entry in &mutations_vec {
            let Entry {
                sample_id,
                ref_strain,
                gisaid_accession,
                subtype: _,
                ctype,
                dais_ref,
                protein,
                ref_codon,
                mut_codon,
                aa_ref,
                aa_position,
                aa_mut,
                phenotypic_consequences,
            } = entry;
            let d = &delim;

            writeln!(
                &mut writer,
                "{sample_id}{d}{ref_strain}{d}{gisaid_accession}{d}\
                {ctype}{d}{dais_ref}{d}{protein}{d}\
                {ref_codon}{d}{mut_codon}{d}\
                {aa_ref}:{aa_position}:{aa_mut}{d}\
                {phenotypic_consequences}",
            )?;
        }
    }

    Ok(())
}
