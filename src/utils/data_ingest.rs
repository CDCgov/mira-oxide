use csv::ReaderBuilder;
use either::Either;
use glob::glob;
use serde::{self, Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Stdin},
    path::{Path, PathBuf},
};

/////////////// Structs to hold IRMA data ///////////////
/// Coverage struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoverageData {
    #[serde(rename = "Sample")]
    pub sample_id: Option<String>,
    #[serde(rename = "Reference_Name")]
    pub reference_name: String,
    #[serde(rename = "Position")]
    pub position: i32,
    #[serde(rename = "Coverage Depth")]
    pub coverage_depth: i32,
    #[serde(rename = "Consensus")]
    pub consensus: String,
    #[serde(rename = "Deletions")]
    pub deletions: i32,
    #[serde(rename = "Ambiguous")]
    pub ambiguous: i32,
    #[serde(rename = "Consensus_Count")]
    pub consensus_count: i32,
    #[serde(rename = "Consensus_Average_Quality")]
    pub consensus_avg_quality: f64,
    #[serde(rename = "Run_ID")]
    pub run_id: Option<String>,
    #[serde(rename = "Instrument")]
    pub instrument: Option<String>,
}

/// Reads struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadsData {
    #[serde(rename = "Sample")]
    pub sample_id: Option<String>,
    #[serde(rename = "Record")]
    pub record: String,
    #[serde(rename = "Reads")]
    pub reads: i32,
    #[serde(rename = "Patterns")]
    pub patterns: String,
    #[serde(rename = "PairsAndWidows")]
    pub pairs_and_windows: String,
    #[serde(rename = "Stage")]
    pub stage: Option<String>,
    #[serde(rename = "Run_ID")]
    pub run_id: Option<String>,
    #[serde(rename = "Instrument")]
    pub instrument: Option<String>,
    #[serde(rename = "Percent Mapping")]
    pub percent_mapping: Option<f32>,
}

/// vtype struct
#[derive(Serialize, Debug, Clone)]
pub struct ProcessedRecord {
    pub sample_id: Option<String>,
    pub vtype: String,
    pub ref_type: String,
    pub subtype: String,
}

/// Alleles struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllelesData {
    #[serde(rename = "Sample")]
    pub sample_id: Option<String>,
    #[serde(rename = "Reference_Name")]
    pub reference: String,
    #[serde(rename = "HMM_Position")]
    pub reference_position: Option<i32>,
    #[serde(rename = "Position")]
    pub sample_position: i32,
    #[serde(rename = "Total")]
    pub coverage: i32,
    #[serde(rename = "Consensus_Allele")]
    pub consensus_allele: String,
    #[serde(rename = "Minority_Allele")]
    pub minority_allele: String,
    #[serde(rename = "Consensus_Count")]
    pub consensus_count: i32,
    #[serde(rename = "Minority_Count")]
    pub minority_count: i32,
    #[serde(rename = "Minority_Frequency")]
    pub minority_frequency: f64,
    #[serde(rename = "Run_ID")]
    pub run_id: Option<String>,
    #[serde(rename = "Instrument")]
    pub instrument: Option<String>,
}

/// Indel struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndelsData {
    #[serde(rename = "Sample")]
    pub sample_id: Option<String>,
    #[serde(rename = "Reference_Name")]
    pub reference_name: String,
    #[serde(rename = "HMM_Position")]
    pub reference_upstream_position: Option<String>,
    #[serde(rename = "Upstream_Position")]
    pub sample_upstream_position: Option<String>,
    #[serde(rename = "Insert")]
    pub insert: Option<String>,
    #[serde(rename = "Length")]
    pub length: Option<String>,
    #[serde(rename = "Context")]
    pub context: String,
    #[serde(rename = "Called")]
    pub called: String,
    #[serde(rename = "Count")]
    pub count: i32,
    #[serde(rename = "Total")]
    pub total: String,
    #[serde(rename = "Frequency")]
    pub frequency: String,
    #[serde(rename = "Average_Quality")]
    pub average_quality: Option<String>,
    #[serde(rename = "ConfidenceNotMacErr")]
    pub confidence_not_mac_err: Option<String>,
    #[serde(rename = "PairedUB")]
    pub paired_ub: String,
    #[serde(rename = "QualityUB")]
    pub quality_ub: Option<String>,
    #[serde(rename = "Run_ID")]
    pub run_id: Option<String>,
    #[serde(rename = "Instrument")]
    pub instrument: Option<String>,
}

#[derive(Debug)]
pub struct SeqData {
    name: String,
    sequence: String,
}

/////////////// Structs to hold dais-ribosome data ///////////////
/// Insertion Data
#[derive(Serialize, Deserialize, Debug)]
pub struct InsertionData {
    #[serde(rename = "ID")]
    pub sample_id: Option<String>,
    #[serde(rename = "C_type")]
    pub ctype: Option<String>,
    #[serde(rename = "Ref_ID")]
    pub reference: String,
    #[serde(rename = "Protein")]
    pub protein: String,
    #[serde(rename = "Upstream_aa")]
    pub upstream_aa_position: String,
    #[serde(rename = "Inserted_nucleotides")]
    pub inserted_nucleotides: String,
    #[serde(rename = "Inserted_residues")]
    pub inserted_residues: String,
    #[serde(rename = "Upstream_nt")]
    pub upstream_nt: String,
    #[serde(rename = "Codon_shift")]
    pub in_frame: String,
}

/// Deletions Data
#[derive(Serialize, Deserialize, Debug)]
pub struct DeletionsData {
    #[serde(rename = "ID")]
    pub sample_id: Option<String>,
    #[serde(rename = "C_type")]
    pub ctype: Option<String>,
    #[serde(rename = "Ref_ID")]
    pub reference: String,
    #[serde(rename = "Protein")]
    pub protein: String,
    #[serde(rename = "VH")]
    pub vh: Option<String>,
    #[serde(rename = "Del_AA_start")]
    pub del_start_aa_position: Option<String>,
    #[serde(rename = "Del_AA_end")]
    pub del_end_aa_position: Option<String>,
    #[serde(rename = "Del_AA_len")]
    pub del_aa_length: String,
    #[serde(rename = "In_frame")]
    pub in_frame: String,
    #[serde(rename = "CDS_ID")]
    pub cds_id: Option<String>,
    #[serde(rename = "Del_CDS_start")]
    pub del_start_cds_position: String,
    #[serde(rename = "Del_CDS_end")]
    pub del_end_cds_position: String,
    #[serde(rename = "Del_CDS_len")]
    pub del_cds_length: Option<String>,
}

/// Dais Sequence Data
#[derive(Serialize, Deserialize, Debug)]
pub struct DaisSeqData {
    #[serde(rename = "ID")]
    pub sample_id: Option<String>,
    #[serde(rename = "C_type")]
    pub ctype: Option<String>,
    #[serde(rename = "Ref_ID")]
    pub reference: String,
    #[serde(rename = "Protein")]
    pub protein: String,
    #[serde(rename = "VH")]
    pub vh: Option<String>,
    #[serde(rename = "AA_seq")]
    pub aa_seq: String,
    #[serde(rename = "AA_aln")]
    pub aa_aln: String,
    #[serde(rename = "CDS_ID")]
    pub cds_id: Option<String>,
    #[serde(rename = "Insertion")]
    pub insertion: String,
    #[serde(rename = "Shift_Insert")]
    pub insertions_shift_frame: String,
    #[serde(rename = "CDS_seq")]
    pub cds_sequence: String,
    #[serde(rename = "CDS_aln")]
    pub aligned_cds_sequence: String,
    #[serde(rename = "Query_nt_coordinates")]
    pub reference_nt_positions: String,
    #[serde(rename = "CDS_nt_coordinates")]
    pub sample_nt_positions: String,
}

/////////////// Imp for the process_txt_with_sample_function ///////////////
/// Define a trait for structs that have a `sample_id` field
trait GetSampleId {
    fn set_sample_id(&mut self, sample_id: String);
}

// Implement the trait for CoverageData
impl GetSampleId for CoverageData {
    fn set_sample_id(&mut self, sample_id: String) {
        self.sample_id = Some(sample_id);
    }
}

// Implement the trait for ReadsData
impl GetSampleId for ReadsData {
    fn set_sample_id(&mut self, sample_id: String) {
        self.sample_id = Some(sample_id);
    }
}

// Implement the trait for AllelesData
impl GetSampleId for AllelesData {
    fn set_sample_id(&mut self, sample_id: String) {
        self.sample_id = Some(sample_id);
    }
}

// Implement the trait for IndelsData
impl GetSampleId for IndelsData {
    fn set_sample_id(&mut self, sample_id: String) {
        self.sample_id = Some(sample_id);
    }
}

/////////////// Data reading functions for IRMA///////////////
/// Creating a reader for processing files
pub fn create_reader(path: PathBuf) -> io::Result<BufReader<Either<File, Stdin>>> {
    if path.to_str() == Some("-") {
        // If the path is "-", use stdin
        Ok(BufReader::new(Either::Right(io::stdin())))
    } else {
        // Otherwise, open the file at the given path
        let file = OpenOptions::new().read(true).open(path)?;
        Ok(BufReader::new(Either::Left(file)))
    }
}

/// Reads in csv file - currently only used for samplesheet
pub fn read_csv<T: DeserializeOwned, R: std::io::Read>(
    reader: R,
    has_headers: bool,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b',')
        .from_reader(reader);

    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let record: T = result?;
        records.push(record);
    }

    Ok(records)
}

/// Extract the sample name from the file path
fn extract_sample_name(path: &Path) -> Result<String, Box<dyn Error>> {
    let parent_dir = path.parent().and_then(|p| p.parent());
    if let Some(parent_dir) = parent_dir {
        let sample = parent_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        Ok(sample)
    } else {
        Err("Failed to extract sample name from path.".into())
    }
}

/// Read tab-delimited data and include the sample name
fn process_txt_with_sample<R, T>(
    reader: R,
    has_headers: bool,
    sample_id: String,
) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    R: Read,
    T: for<'de> Deserialize<'de> + GetSampleId,
{
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b'\t')
        .from_reader(reader);

    let mut records: Vec<T> = Vec::new();
    for result in rdr.deserialize() {
        let mut record: T = result?;
        record.set_sample_id(sample_id.clone());
        records.push(record);
    }

    Ok(records)
}

/// Read in the coverage files made by IRMA and save to a vector of CoverageData
pub fn coverage_data_collection(
    irma_path: impl AsRef<Path>,
    platform: &str,
    runid: &str,
) -> Result<Vec<CoverageData>, Box<dyn std::error::Error>> {
    let pattern = format!(
        "{}/*/IRMA/*/tables/*coverage.txt",
        irma_path.as_ref().display()
    );

    let mut cov_data: Vec<CoverageData> = Vec::new();

    // Iterate over all files matching the pattern and get the sample name from file
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<CoverageData> = process_txt_with_sample(reader, true, sample)?;
                for line in &mut records {
                    line.run_id = Some(runid.to_string());
                    line.instrument = Some(platform.to_string());
                }
                cov_data.append(&mut records);
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(cov_data)
}

///  Collect read data created by IRMA and save to vector of ReadsData
pub fn reads_data_collection(
    irma_path: impl AsRef<Path>,
    platform: &str,
    runid: &str,
) -> Result<Vec<ReadsData>, Box<dyn std::error::Error>> {
    let pattern = format!(
        "{}/*/IRMA/*/tables/READ_COUNTS.txt",
        irma_path.as_ref().display()
    );

    let mut reads_data: Vec<ReadsData> = Vec::new();

    // Iterate over all files matching the pattern and get the sample name from file
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<ReadsData> = process_txt_with_sample(reader, true, sample)?;
                for line in &mut records {
                    line.run_id = Some(runid.to_string());
                    line.instrument = Some(platform.to_string());
                    if let Some(first_char) = line.record.chars().next() {
                        line.stage = Some(first_char.to_string());
                    }
                }

                reads_data.append(&mut records);
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(reads_data)
}

/// Collecting allele data created by IRMA and and save to vector of AllelesData
pub fn allele_data_collection(
    irma_path: &Path,
) -> Result<Vec<AllelesData>, Box<dyn std::error::Error>> {
    let pattern = format!(
        "{}/*/IRMA/*/tables/*variants.txt",
        irma_path.to_string_lossy()
    );

    let mut alleles_data: Vec<AllelesData> = Vec::new();

    // Iterate over all files matching the pattern and get the sample name from file
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<AllelesData> = process_txt_with_sample(reader, true, sample)?;
                records.retain(|record| record.minority_frequency >= 0.05);
                alleles_data.append(&mut records);
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(alleles_data)
}

/// Collect indel data and save to vector of IndelsData
/// Note that insertions and deletions are being added  to the same Vec<Indelsdata>
pub fn indels_data_collection(
    irma_path: impl AsRef<Path>,
) -> Result<Vec<IndelsData>, Box<dyn std::error::Error>> {
    let pattern1 = format!(
        "{}/*/IRMA/*/tables/*insertions.txt",
        irma_path.as_ref().display()
    );
    let pattern2 = format!(
        "{}/*/IRMA/*/tables/*deletions.txt",
        irma_path.as_ref().display()
    );

    let mut indels_data: Vec<IndelsData> = Vec::new();

    // Iterate over all files matching the pattern1 (Insertions) and get the sample name from file
    for entry in glob(&pattern1).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<IndelsData> = process_txt_with_sample(reader, true, sample)?;
                indels_data.append(&mut records);
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }

    // Iterate over all files matching the pattern2 (Deletions) and get the sample name from file
    for entry in glob(&pattern2).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<IndelsData> = process_txt_with_sample(reader, true, sample)?;
                indels_data.append(&mut records);
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(indels_data)
}

/// Read in IRMA amended consensus fasta files to SeqData struct
pub fn amended_consensus_data_collection(
    irma_path: impl AsRef<Path>,
    organism: &str,
) -> Result<Vec<SeqData>, Box<dyn std::error::Error>> {
    // Determine the glob pattern based on the organism
    let pattern = if organism == "flu" {
        format!(
            "{}/*/IRMA/*/amended_consensus/*fa",
            irma_path.as_ref().display()
        )
    } else {
        format!(
            "{}/*/IRMA/*/amended_consensus/*pad.fa",
            irma_path.as_ref().display()
        )
    };

    let mut seq_data: Vec<SeqData> = Vec::new();

    // Iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Parse the file line by line (assuming FASTA format)
                let mut current_name = String::new();
                let mut current_sequence = String::new();

                for line in reader.lines() {
                    let line = line?;
                    if line.starts_with('>') {
                        // If there's an existing sequence, save it
                        if !current_name.is_empty() {
                            seq_data.push(SeqData {
                                name: current_name.clone(),
                                sequence: current_sequence.clone(),
                            });
                        }
                        // Start a new sequence
                        current_name = line[1..].to_string();
                        current_sequence.clear();
                    } else {
                        // Append to the current sequence
                        current_sequence.push_str(&line);
                    }
                }

                // Save the last sequence
                if !current_name.is_empty() {
                    seq_data.push(SeqData {
                        name: current_name,
                        sequence: current_sequence,
                    });
                }
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }

    Ok(seq_data)
}

/////////////// Functions for manipulating IRMA data ///////////////
/// Breaking up the records column into three string for the create_vtype_data function
fn read_record2type(record: &str) -> (String, String, String) {
    let parts: Vec<&str> = record.split('_').collect();
    if parts.len() >= 2 {
        let vtype = parts[0][2..].to_string();
        let ref_type = parts[1].to_string();
        let subtype = if ref_type == "HA" || ref_type == "NA" {
            parts.last().unwrap_or(&"").to_string()
        } else {
            "".to_string()
        };
        (vtype, ref_type, subtype)
    } else {
        let fallback = record[2..].to_string();
        (fallback.clone(), fallback.clone(), fallback.clone())
    }
}

/// Converting info for read data into vtype
pub fn create_vtype_data(reads_data: &Vec<ReadsData>) -> Vec<ProcessedRecord> {
    let mut processed_records = Vec::new();

    for data in reads_data.iter() {
        // Filter records where the first character of 'record' is '4'
        if data.record.starts_with('4') {
            let (vtype, ref_type, subtype) = read_record2type(&data.record);
            let processed_record = ProcessedRecord {
                sample_id: data.sample_id.clone(),
                vtype,
                ref_type,
                subtype,
            };
            processed_records.push(processed_record);
        }
    }

    processed_records
}

// Function to collect reference lengths from IRMA outputs
pub fn get_reference_lens(
    irma_path: impl AsRef<Path>,
) -> Result<HashMap<String, usize>, Box<dyn std::error::Error>> {
    let pattern = format!(
        "{}/*/IRMA/*/intermediate/0-ITERATIVE-REFERENCES/R0*ref",
        irma_path.as_ref().display()
    );

    let mut ref_len_map: HashMap<String, usize> = HashMap::new();

    for entry in glob(&pattern)? {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                let mut ref_name = String::new();
                let mut current_sequence = String::new();

                for line in reader.lines() {
                    let line = line?;
                    if line.starts_with('>') {
                        if !ref_name.is_empty() {
                            // Remove "{S1}" suffix if present - sc2 situations
                            if ref_name.ends_with("{S1}") {
                                ref_name.truncate(ref_name.len() - 4);
                            }
                            ref_len_map.insert(ref_name.clone(), current_sequence.len());
                        }
                        ref_name = line[1..].to_string();
                        current_sequence.clear();
                    } else {
                        current_sequence.push_str(&line);
                    }
                }

                if !ref_name.is_empty() {
                    // Remove "{S1}" suffix if present - sc2 situations
                    if ref_name.ends_with("{S1}") {
                        ref_name.truncate(ref_name.len() - 4);
                    }
                    ref_len_map.insert(ref_name, current_sequence.len());
                }
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }

    Ok(ref_len_map)
}

/////////////// Data reading functions for DAIS-ribosome ///////////////
/// Read tab-delimited data a withouot including sample name
pub fn process_txt<R, T>(reader: R, has_headers: bool) -> Result<Vec<T>, Box<dyn std::error::Error>>
where
    R: Read,
    T: for<'de> Deserialize<'de>,
{
    let mut rdr = ReaderBuilder::new()
        .has_headers(has_headers)
        .delimiter(b'\t')
        .from_reader(reader);

    let mut records: Vec<T> = Vec::new();
    for result in rdr.deserialize() {
        let record: T = result?;
        records.push(record);
    }

    Ok(records)
}

/// Read in dais-ribosome ins file fto InsertionData struct
pub fn dais_insertion_data_collection(
    dais_path: impl AsRef<Path>,
) -> Result<Vec<InsertionData>, Box<dyn std::error::Error>> {
    // Construct the glob pattern for matching files
    //If using * situation, you will have to use glob
    let pattern = format!(
        "{}/aggregate_outputs/dais-ribosome/DAIS_ribosome.ins",
        dais_path.as_ref().display()
    );

    let mut dais_ins_data: Vec<InsertionData> = Vec::new();

    // Use the glob crate to find all matching files
    for entry in glob(&pattern)? {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let mut records: Vec<InsertionData> = process_txt(reader, false)?;
                dais_ins_data.append(&mut records);
            }
            Err(e) => {
                eprintln!("Error processing file: {e}");
            }
        }
    }

    Ok(dais_ins_data)
}

/// Read in dais-ribosome ins file fto DeletionsData struct
pub fn dias_deletion_data_collection(
    dais_path: impl AsRef<Path>,
) -> Result<Vec<DeletionsData>, Box<dyn std::error::Error>> {
    // Construct the glob pattern for matching files
    //If using * situation, you will have to use glob
    let pattern = format!(
        "{}/aggregate_outputs/dais-ribosome/DAIS_ribosome.del",
        dais_path.as_ref().display()
    );

    let mut dais_del_data: Vec<DeletionsData> = Vec::new();

    // Use the glob crate to find all matching files
    for entry in glob(&pattern)? {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let mut records: Vec<DeletionsData> = process_txt(reader, false)?;
                dais_del_data.append(&mut records);
            }
            Err(e) => {
                eprintln!("Error processing file: {e}");
            }
        }
    }
    Ok(dais_del_data)
}

/// Read in dais-ribosome ins file fto DaisSeqData struct
pub fn dias_sequence_data_collection(
    dais_path: impl AsRef<Path>,
) -> Result<Vec<DaisSeqData>, Box<dyn std::error::Error>> {
    // Construct the glob pattern for matching files
    //If using * situation, you will have to use glob
    let pattern = format!(
        "{}/aggregate_outputs/dais-ribosome/DAIS_ribosome.seq",
        dais_path.as_ref().display()
    );

    let mut dais_seq_data: Vec<DaisSeqData> = Vec::new();

    // Use the glob crate to find all matching files
    for entry in glob(&pattern)? {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let mut records: Vec<DaisSeqData> = process_txt(reader, false)?;
                dais_seq_data.append(&mut records);
            }
            Err(e) => {
                eprintln!("Error processing file: {e}");
            }
        }
    }

    Ok(dais_seq_data)
}

/// Read in dais-ribosome ins file fto DaisSeqData struct
pub fn dais_ref_seq_data_collection(
    dais_path: impl AsRef<Path>,
    organism: &str,
) -> Result<Vec<DaisSeqData>, Box<dyn std::error::Error>> {
    // Construct the glob pattern for matching files
    //If using * situation, you will have to use glob
    let pattern = format!(
        "{}/data/references/*{}.seq",
        dais_path.as_ref().display(),
        organism
    );

    let mut dais_seq_data: Vec<DaisSeqData> = Vec::new();

    // Use the glob crate to find all matching files
    for entry in glob(&pattern)? {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                let mut records: Vec<DaisSeqData> = process_txt(reader, false)?;
                dais_seq_data.append(&mut records);
            }
            Err(e) => {
                eprintln!("Error processing file: {e}");
            }
        }
    }

    Ok(dais_seq_data)
}
