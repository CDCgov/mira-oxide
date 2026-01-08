use csv::ReaderBuilder;
use either::Either;
use glob::glob;
use serde::{self, Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Stdin},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/////////////// Structs to hold IRMA data ///////////////
///
///QC structs
#[derive(Debug, Deserialize)]
pub struct QCSettings {
    pub med_cov: u32,
    pub minor_vars: u32,
    pub allow_stop_codons: bool,
    pub perc_ref_covered: u32,
    pub negative_control_perc: u32,
    pub negative_control_perc_exception: u32,
    pub positive_control_minimum: u32,
    pub padded_consensus: bool,
    #[serde(default)]
    pub med_spike_cov: Option<u32>,
    #[serde(default)]
    pub perc_ref_spike_covered: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct QCConfig {
    #[serde(rename = "ont-flu")]
    pub ont_flu: QCSettings,
    #[serde(rename = "ont-sc2-spike")]
    pub ont_sc2_spike: QCSettings,
    #[serde(rename = "illumina-flu")]
    pub illumina_flu: QCSettings,
    #[serde(rename = "illumina-sc2")]
    pub illumina_sc2: QCSettings,
    #[serde(rename = "ont-sc2")]
    pub ont_sc2: QCSettings,
    #[serde(rename = "illumina-rsv")]
    pub illumina_rsv: QCSettings,
    #[serde(rename = "ont-rsv")]
    pub ont_rsv: QCSettings,
}

//This function is needed to read in the NA in positions as 0 below
fn string_to_int<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    if s == "NA" {
        Ok(0)
    } else {
        s.parse::<i32>().map_err(serde::de::Error::custom)
    }
}

/// Coverage struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoverageData {
    #[serde(rename = "Sample")]
    pub sample_id: Option<String>,
    #[serde(rename = "Reference_Name")]
    pub reference_name: String,
    #[serde(rename = "Position")]
    #[serde(deserialize_with = "string_to_int")]
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
    #[serde(rename = "HMM_Position")]
    pub hmm_position: Option<i32>,
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
    pub patterns: Option<String>,
    #[serde(rename = "PairsAndWidows")]
    pub pairs_and_windows: Option<String>,
    #[serde(rename = "Stage")]
    pub stage: Option<String>,
    #[serde(rename = "Run_ID")]
    pub run_id: Option<String>,
    #[serde(rename = "Instrument")]
    pub instrument: Option<String>,
    #[serde(rename = "Percent Mapping")]
    pub percent_mapping: Option<f32>,
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

/// Struct to hold filtered and unfiltered allele data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlleleDataCollection {
    pub filtered_alleles: Vec<AllelesData>,
    pub all_alleles: Vec<AllelesData>,
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
    pub length: Option<i32>,
    #[serde(rename = "Context")]
    pub context: String,
    #[serde(rename = "Called")]
    pub called: String,
    #[serde(rename = "Count")]
    pub count: i32,
    #[serde(rename = "Total")]
    pub total: i32,
    #[serde(rename = "Frequency")]
    pub frequency: f64,
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

/// Run Info struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunInfo {
    #[serde(rename = "program_name")]
    pub program_name: Option<String>,
    #[serde(rename = "PROGRAM")]
    pub program: Option<String>,
    #[serde(rename = "Iterative Refinement Meta-Assembler (IRMA)")]
    pub irma: Option<String>,
    #[serde(rename = "Run_ID")]
    pub run_id: Option<String>,
    #[serde(rename = "Instrument")]
    pub instrument: Option<String>,
    #[serde(rename = "Timestamp")]
    pub timestamp: Option<String>,
}

#[derive(Debug)]
pub struct SeqData {
    pub name: String,
    pub sequence: String,
}

/////////////// Structs to hold dais-ribosome data ///////////////
/// Dais Sequence Data
#[derive(Serialize, Deserialize, Debug)]
pub struct DaisSeqData {
    #[serde(rename = "ID")]
    pub sample_id: Option<String>,
    #[serde(rename = "C_type")]
    pub ctype: String,
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
pub fn create_reader(path: &PathBuf) -> io::Result<BufReader<Either<File, Stdin>>> {
    if path.to_str() == Some("-") {
        // If the path is "-", use stdin
        Ok(BufReader::new(Either::Right(io::stdin())))
    } else {
        // Otherwise, open the file at the given path
        let file = OpenOptions::new().read(true).open(path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Could not open file '{}': {}", path.display(), e),
            )
        })?;
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

/// Reads in yaml file - currently only used for qc yaml
pub fn read_yaml<R: std::io::Read>(reader: R) -> Result<QCConfig, Box<dyn std::error::Error>> {
    let mut contents = String::new();
    let mut buf_reader = BufReader::new(reader);
    buf_reader.read_to_string(&mut contents)?;
    let config: QCConfig = serde_yaml_ng::from_str(&contents)?;
    Ok(config)
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
    sample_id: &str,
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
        record.set_sample_id(sample_id.to_string());
        records.push(record);
    }

    Ok(records)
}

/// Read tab-delimited data and include the sample name
fn process_txt_without_sample<R, T>(reader: R, has_headers: bool) -> Vec<T>
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
        match result {
            Ok(record) => {
                // Successfully deserialized record
                records.push(record);
            }
            Err(e) => {
                // Log a warning and skip the invalid record
                eprintln!("Warning: Failed to deserialize record: {e}");
            }
        }
    }

    records
}
/// Read in the coverage files made by IRMA and save to a vector of `CoverageData`
pub fn coverage_data_collection(
    irma_path: impl AsRef<Path>,
    platform: &str,
    runid: &str,
    virus: &str,
) -> Result<Vec<CoverageData>, Box<dyn std::error::Error>> {
    let pattern = if virus.to_lowercase() == "sc2-spike" {
        format!(
            "{}/*/IRMA/*/tables/*coverage.a2m.txt",
            irma_path.as_ref().display()
        )
    } else {
        format!(
            "{}/*/IRMA/*/tables/*coverage.txt",
            irma_path.as_ref().display()
        )
    };

    let mut cov_data: Vec<CoverageData> = Vec::new();

    // Iterate over all files matching the pattern and get the sample name from file
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<CoverageData> =
                    process_txt_with_sample(reader, true, &sample)?;

                // If virus is "sc2-spike", replace position with hmm_position
                if virus == "sc2-spike" {
                    for line in &mut records {
                        line.position = line.hmm_position.unwrap_or(0);
                    }
                }

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

///  Collect read data created by IRMA and save to vector of `ReadsData`
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
                let mut records: Vec<ReadsData> = process_txt_with_sample(reader, true, &sample)?;
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
/// Collecting allele data created by IRMA and save to two vectors of `AllelesData`
/// One vector contains filtered alleles (frequency >= 0.05), and the other contains all alleles.
pub fn allele_data_collection(
    irma_path: &Path,
    platform: &str,
    runid: &str,
) -> Result<AlleleDataCollection, Box<dyn std::error::Error>> {
    let pattern = format!(
        "{}/*/IRMA/*/tables/*variants.txt",
        irma_path.to_string_lossy()
    );

    let mut filtered_alleles: Vec<AllelesData> = Vec::new();
    let mut all_alleles: Vec<AllelesData> = Vec::new();

    // Iterate over all files matching the pattern and get the sample name from file
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let sample = extract_sample_name(&path)?;
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file and include the sample name
                let mut records: Vec<AllelesData> = process_txt_with_sample(reader, true, &sample)?;

                // Add platform and runid to each record
                for record in &mut records {
                    record.instrument = Some(platform.to_string());
                    record.run_id = Some(runid.to_string());

                    // Round minority_frequency to 3 decimal places
                    record.minority_frequency =
                        (record.minority_frequency * 1000.0).round() / 1000.0;
                }

                // Separate records into filtered and unfiltered vectors
                for record in records {
                    if record.minority_frequency >= 0.05 {
                        filtered_alleles.push(record.clone());
                    }
                    all_alleles.push(record);
                }
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }

    Ok(AlleleDataCollection {
        filtered_alleles,
        all_alleles,
    })
}

/// Collect indel data and save to vector of `IndelsData`
/// Note that insertions and deletions are being added  to the same Vec<Indelsdata>
pub fn indels_data_collection(
    irma_path: impl AsRef<Path>,
    platform: &str,
    runid: &str,
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
                let mut records: Vec<IndelsData> = process_txt_with_sample(reader, true, &sample)?;
                // Add platform and runid to each record
                for record in &mut records {
                    record.instrument = Some(platform.to_string());
                    record.run_id = Some(runid.to_string());
                }
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
                let mut records: Vec<IndelsData> = process_txt_with_sample(reader, true, &sample)?;
                // Add platform and runid to each record
                for record in &mut records {
                    record.instrument = Some(platform.to_string());
                    record.run_id = Some(runid.to_string());
                }
                indels_data.append(&mut records);
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }
    Ok(indels_data)
}

/// Read in IRMA amended consensus fasta files to `SeqData` struct
pub fn amended_consensus_data_collection(
    irma_path: impl AsRef<Path>,
    organism: &str,
) -> Result<Vec<SeqData>, Box<dyn std::error::Error>> {
    // Determine the glob pattern based on the organism
    let pattern = if organism == "flu" || organism == "sc2-spike" {
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
                    if let Some(line) = line.strip_prefix('>') {
                        // If there's an existing sequence, save it
                        if !current_name.is_empty() {
                            seq_data.push(SeqData {
                                name: current_name.clone(),
                                sequence: current_sequence.clone(),
                            });
                        }
                        // Start a new sequence
                        current_name = line.to_string();
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
                    if let Some(line) = line.strip_prefix('>') {
                        if !ref_name.is_empty() {
                            // Remove "{S1}" suffix if present - sc2 situations
                            if ref_name.ends_with("{S1}") {
                                ref_name.truncate(ref_name.len() - 4);
                            }
                            ref_len_map.insert(ref_name.clone(), current_sequence.len());
                        }
                        ref_name = line.to_string();
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

// Function to get the current timestamp in the desired format
// For irma_config file - ingest of run_info.txt
fn get_current_timestamp() -> String {
    let now = SystemTime::now();
    match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            let nanos = duration.subsec_nanos();
            format_timestamp(secs, nanos)
        }
        Err(_) => "1970-01-01 00:00:00.000000".to_string(), // Fallback in case of error
    }
}

// Function to get timestamp into correct format for CDP
// Accounts for leap years
// For irma_config file - ingest of run_info.txt
fn format_timestamp(secs: u64, nanos: u32) -> String {
    const SECONDS_IN_MINUTE: u64 = 60;
    const SECONDS_IN_HOUR: u64 = 3600;
    const SECONDS_IN_DAY: u64 = 86400;

    // Days in each month (non-leap year)
    const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    // Calculate days since UNIX epoch
    let mut days_since_epoch = secs / SECONDS_IN_DAY;

    // Calculate year
    let mut year = 1970;
    while days_since_epoch >= days_in_year(year) {
        days_since_epoch -= days_in_year(year);
        year += 1;
    }

    // Determine if current year is a leap year
    let is_leap_year = is_leap_year(year);

    // Calculate month and day
    let mut month = 0;
    let mut day = days_since_epoch + 1; // Days are 1-based
    for (i, &days_in_month) in DAYS_IN_MONTH.iter().enumerate() {
        let days_in_this_month = if i == 1 && is_leap_year {
            29
        } else {
            days_in_month
        };
        if day > u64::from(days_in_this_month) {
            day -= u64::from(days_in_this_month);
        } else {
            month = i + 1; // Months are 1-based
            break;
        }
    }

    // Calculate hours, minutes, seconds
    let remaining_secs = secs % SECONDS_IN_DAY;
    let hours = remaining_secs / SECONDS_IN_HOUR;
    let minutes = (remaining_secs % SECONDS_IN_HOUR) / SECONDS_IN_MINUTE;
    let seconds = remaining_secs % SECONDS_IN_MINUTE;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
        year,
        month,
        day,
        hours,
        minutes,
        seconds,
        nanos / 1000
    )
}

// Helper function to determine if a year is a leap year
fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// Helper function to calculate the number of days in a year
fn days_in_year(year: u64) -> u64 {
    if is_leap_year(year) { 366 } else { 365 }
}

/// Collect read info created by IRMA and save to struct of `RunInfo`
pub fn run_info_collection(
    irma_path: impl AsRef<Path>,
    platform: &str,
    runid: &str,
) -> Result<Vec<RunInfo>, Box<dyn std::error::Error>> {
    let pattern = format!(
        "{}/*/IRMA/*/logs/run_info.txt",
        irma_path.as_ref().display()
    );

    let mut run_info: Vec<RunInfo> = Vec::new();

    // Start to iterate over all files matching the pattern
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);

                // Read the data from the file
                let mut records: Vec<RunInfo> = process_txt_without_sample(reader, true);
                for line in &mut records {
                    line.run_id = Some(runid.to_string());
                    line.instrument = Some(platform.to_string());
                    line.timestamp = Some(get_current_timestamp());
                }
                run_info.extend(records);

                // Break after processing the first valid file
                break;
            }
            Err(e) => println!("Error reading file: {e}"),
        }
    }

    Ok(run_info)
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

/// Read in dais-ribosome ins file fto `DaisSeqData` struct
pub fn dais_sequence_data_collection(
    dais_path: impl AsRef<Path>,
) -> Result<Vec<DaisSeqData>, Box<dyn std::error::Error>> {
    // Construct the glob pattern for matching files
    //If using * situation, you will have to use glob
    let pattern = format!("{}DAIS_ribosome.seq", dais_path.as_ref().display());

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

/// Read in dais-ribosome ins file fto `DaisSeqData` struct
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
