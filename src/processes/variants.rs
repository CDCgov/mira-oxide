//! `variants` — reference-vs-query positions-of-interest comparison and/or
//! standalone MIRA minor-variant annotation.
//!
//! Two modes:
//! * **Comparison** (when `--positions` is supplied): reconstructs full-length sequences
//!   from DAIS-ribosome protein-level output (`.seq` / `.ins` / `.del`) for a set of
//!   "reference" strains and a set of "query" samples, then compares each query against
//!   its matching reference in the shared DAIS reference-coordinate space, reporting
//!   nucleotide / amino-acid / codon differences and flagging positions of interest.
//!   Optionally annotates rows with MIRA minor-variant calls (`--minor-variants`).
//! * **Annotation** (when `--minor-variants` is supplied without `--positions`): runs
//!   independently, appending `codon`, `codon-position`, `consensus-aa` and `minority-aa`
//!   to each row of the MIRA minor-variant CSV using the query CDS, written to a new file
//!   (`-o`) or in place (`--in-place`).
//!
//! The DAIS `cds_aln` column (col 12) is already expressed in reference-coordinate
//! space: deletions appear as `-`, out-of-span padding appears as `.`, and insertions
//! are stripped out (they are carried only in the `.ins` file). Reconstruction therefore
//! walks `cds_aln` and splices insertions back in to recover per-nucleotide isolate
//! positions, while comparison is performed position-by-position in that shared space.

use clap::{Parser, ValueEnum};
use csv::ReaderBuilder;
use either::Either;
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Stdout, Write, stdout},
    path::PathBuf,
};
use zoe::data::mappings::StdGeneticCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FilterMode {
    /// Emit every nucleotide/amino-acid/codon difference (default).
    AllDiffs,
    /// Emit only differences at a positions-of-interest amino-acid position.
    Poi,
    /// Emit only positions-of-interest differences where the query amino acid
    /// equals the `amino-acid-of-interest` for that entry.
    PoiMatch,
}

#[derive(Debug, Parser)]
#[command(
    about = "Compare query DAIS output against reference and/or annotate MIRA minor variants"
)]
pub struct VariantsArgs {
    /// Reference `SEQUENCE_OUTPUT` (`.seq`) file (comparison mode)
    #[arg(long)]
    ref_seq: Option<PathBuf>,
    /// Reference `INSERTION_OUTPUT` (`.ins`) file (comparison mode)
    #[arg(long)]
    ref_ins: Option<PathBuf>,
    /// Reference `DELETION_OUTPUT` (`.del`) file (comparison mode)
    #[arg(long)]
    ref_del: Option<PathBuf>,
    /// Query `SEQUENCE_OUTPUT` (`.seq`) file (required)
    #[arg(long)]
    query_seq: PathBuf,
    /// Query `INSERTION_OUTPUT` (`.ins`) file (comparison mode)
    #[arg(long)]
    query_ins: Option<PathBuf>,
    /// Query `DELETION_OUTPUT` (`.del`) file (comparison mode)
    #[arg(long)]
    query_del: Option<PathBuf>,
    /// Positions-of-interest file (ref-name, segment, aa-position, aa-of-interest).
    /// Presence selects comparison mode.
    #[arg(long)]
    positions: Option<PathBuf>,
    /// MIRA minor-variant CSV. In comparison mode it adds annotation columns; on its own
    /// (without `--positions`) it runs standalone annotation of the CSV.
    #[arg(long)]
    minor_variants: Option<PathBuf>,
    /// Row selection mode (comparison mode)
    #[arg(long, value_enum, default_value_t = FilterMode::AllDiffs)]
    filter: FilterMode,
    /// Output file (defaults to stdout; ignored when `--in-place` is set)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
    /// Annotation mode: overwrite the input `--minor-variants` file in place
    #[arg(long)]
    in_place: bool,
    /// Single-character output delimiter for comparison mode (defaults to TAB)
    #[arg(short = 'd', long)]
    delimiter: Option<String>,
}

// ---------------------------------------------------------------------------
// Input records
// ---------------------------------------------------------------------------

/// DAIS `.seq` record (14 columns).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SeqRecord {
    query_id: String,
    ctype: String,
    reference_id: String,
    protein: String,
    aa_id: String,
    aa_seq: String,
    aa_aln: String,
    cds_id: String,
    has_insertion: String,
    has_shift_indel: String,
    cds_seq: String,
    cds_aln: String,
    query_coordinates: String,
    cds_coordinates: String,
}

/// DAIS `.ins` record (9 columns).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct InsRecord {
    query_id: String,
    ctype: String,
    reference_id: String,
    protein: String,
    upstream_aa_pos: usize,
    inserted_nt: String,
    inserted_aa: String,
    upstream_nt_pos: usize,
    codon_shift: usize,
}

/// DAIS `.del` record (13 columns). Parsed for validation; deletions are already
/// encoded as gaps in `cds_aln`, so these are not needed for reconstruction.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct DelRecord {
    query_id: String,
    ctype: String,
    reference_id: String,
    protein: String,
    aa_id: String,
    del_aa_start: usize,
    del_aa_end: usize,
    del_aa_len: usize,
    in_frame: String,
    cds_id: String,
    del_cds_start: usize,
    del_cds_end: usize,
    del_cds_len: usize,
}

/// Position-of-interest record. Parsed leniently from whitespace-separated columns:
/// `ref-name  segment  amino-acid-position  [amino-acid-of-interest]`. The final column
/// is optional (defaults to empty).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PosRecord {
    ref_name: String,
    segment: String,
    aa_position: usize,
    aa_of_interest: String,
}

/// MIRA minor-variant CSV record (comma-delimited, header row). Extra columns
/// (`run_id`, `instrument`, ...) are ignored.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct MinorRecord {
    sample: String,
    reference: String,
    sample_position: usize,
    depth: usize,
    consensus_allele: String,
    minority_allele: String,
    consensus_count: usize,
    minority_count: usize,
    minority_frequency: f64,
}

/// Indexed minor-variant call, keyed by (sample, reference/ctype, `sample_position`).
#[derive(Debug, Clone, Copy)]
struct MinorRaw {
    minority_allele: u8,
    depth: usize,
    consensus_count: usize,
    minority_frequency: f64,
}

type MinorIndex = HashMap<(String, String, usize), MinorRaw>;

/// Minor-allele annotation attached to an output row.
#[derive(Debug, Clone, PartialEq)]
struct MinorInfo {
    minor_nt: char,
    minor_aa: char,
    minor_codon: String,
    freq_minor: f64,
    freq_major: f64,
    total_depth: usize,
}

// ---------------------------------------------------------------------------
// Output row
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct DiffRow {
    query_name: String,
    segment: String,
    protein: String,
    ref_name: String,
    ref_nt_pos: usize,
    query_nt_pos: usize,
    ref_nt: char,
    query_nt: char,
    ref_aa_pos: usize,
    query_aa_pos: usize,
    ref_codon: String,
    query_codon: String,
    codon_position: usize,
    ref_aa: char,
    query_aa: char,
    poi: bool,
    minor: Option<MinorInfo>,
}

impl DiffRow {
    fn to_delimited(&self, delim: char, include_minor: bool) -> String {
        let mut fields = vec![
            self.query_name.clone(),
            self.segment.clone(),
            self.protein.clone(),
            self.ref_name.clone(),
            self.ref_nt_pos.to_string(),
            self.query_nt_pos.to_string(),
            self.ref_nt.to_string(),
            self.query_nt.to_string(),
            self.ref_aa_pos.to_string(),
            self.query_aa_pos.to_string(),
            self.ref_codon.clone(),
            self.query_codon.clone(),
            self.codon_position.to_string(),
            self.ref_aa.to_string(),
            self.query_aa.to_string(),
            self.poi.to_string(),
        ];
        if include_minor {
            if let Some(m) = &self.minor {
                fields.push(m.minor_nt.to_string());
                fields.push(m.minor_aa.to_string());
                fields.push(m.minor_codon.clone());
                fields.push(format!("{:.4}", m.freq_minor));
                fields.push(format!("{:.4}", m.freq_major));
                fields.push(m.total_depth.to_string());
            } else {
                // No matching minor-variant call: emit blank cells.
                for _ in 0..MINOR_HEADER.len() {
                    fields.push(String::new());
                }
            }
        }
        fields.join(&delim.to_string())
    }
}

const HEADER: [&str; 16] = [
    "query-name",
    "segment",
    "protein",
    "ref-name",
    "ref-nt-position",
    "query-nt-position",
    "ref-nt",
    "query-nt",
    "ref-aa-position",
    "query-aa-position",
    "ref-codon",
    "query-codon",
    "codon-position",
    "ref-aa",
    "query-aa",
    "position-of-interest",
];

const MINOR_HEADER: [&str; 6] = [
    "minor-nt",
    "minor-aa",
    "minor-codon",
    "freq-minor",
    "freq-major",
    "total-depth",
];

// ---------------------------------------------------------------------------
// Reconstruction primitives
// ---------------------------------------------------------------------------

/// One nucleotide call in reference-coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Call {
    /// 1-based reference-coordinate (DAIS) nucleotide position.
    dais_nt_pos: usize,
    /// 1-based indel-adjusted isolate nucleotide position (0 for deletion/padding).
    ref_nt_pos: usize,
    /// Nucleotide at this reference coordinate (`-` = deletion, `.` = padding).
    nt: u8,
}

/// The segment label is the second underscore-delimited field of the DAIS `ctype`
/// (e.g. `A_HA_H1` -> `HA`, `A_MP` -> `MP`).
fn segment_of(ctype: &str) -> Option<&str> {
    ctype.split('_').nth(1)
}

/// Build per-nucleotide calls in reference-coordinate space from a `cds_aln` string,
/// splicing insertions back in so that `ref_nt_pos` reflects the true isolate position.
fn build_calls(cds_aln: &[u8], insertions: &[(usize, String)]) -> Vec<Call> {
    // Total inserted-base count keyed by the upstream reference nt position.
    let mut ins_after: HashMap<usize, usize> = HashMap::new();
    for (upstream_nt_pos, seq) in insertions {
        *ins_after.entry(*upstream_nt_pos).or_insert(0) += seq.len();
    }

    let mut isolate_pos = 0usize;
    let mut calls = Vec::with_capacity(cds_aln.len());
    for (idx, &b) in cds_aln.iter().enumerate() {
        let i = idx + 1;
        let is_base = b != b'-' && b != b'.';
        let ref_nt_pos = if is_base {
            isolate_pos += 1;
            isolate_pos
        } else {
            0
        };
        calls.push(Call {
            dais_nt_pos: i,
            ref_nt_pos,
            nt: b,
        });
        if let Some(cnt) = ins_after.get(&i) {
            isolate_pos += cnt;
        }
    }
    calls
}

/// Extract the reference-coordinate codon (3 bytes) containing reference aa position `aa`.
fn codon_at(cds_aln: &[u8], aa: usize) -> Vec<u8> {
    let start = (aa - 1) * 3;
    let mut codon = Vec::with_capacity(3);
    for offset in 0..3 {
        codon.push(cds_aln.get(start + offset).copied().unwrap_or(b'.'));
    }
    codon
}

/// Translate a codon, returning `X` for any codon containing non-ACGTU characters.
fn translate(codon: &[u8]) -> u8 {
    let clean = codon.len() == 3
        && codon
            .iter()
            .all(|c| matches!(c.to_ascii_uppercase(), b'A' | b'C' | b'G' | b'T' | b'U'));
    if clean {
        StdGeneticCode::translate_codon(codon)
    } else {
        b'X'
    }
}

/// Isolate amino-acid position from an isolate nucleotide position (0 -> 0).
fn aa_pos_of(nt_pos: usize) -> usize {
    if nt_pos == 0 { 0 } else { (nt_pos - 1) / 3 + 1 }
}

/// The MIRA minor-variant `sample` column omits the DAIS segment suffix, so strip a
/// trailing `_<digits>` from the DAIS `query_id` (e.g. `046435d3_4` -> `046435d3`).
fn sample_base(query_id: &str) -> &str {
    if let Some((base, seg)) = query_id.rsplit_once('_')
        && !seg.is_empty()
        && seg.bytes().all(|b| b.is_ascii_digit())
    {
        return base;
    }
    query_id
}

/// Compare a single reference record against a single query record in reference space.
#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
fn compare_record(
    query_name: &str,
    query_ctype: &str,
    segment: &str,
    protein: &str,
    ref_name: &str,
    ref_aln: &[u8],
    ref_ins: &[(usize, String)],
    query_aln: &[u8],
    query_ins: &[(usize, String)],
    poi_positions: &HashMap<usize, String>,
    minor_index: Option<&MinorIndex>,
    filter: FilterMode,
) -> Vec<DiffRow> {
    let ref_calls = build_calls(ref_aln, ref_ins);
    let query_calls = build_calls(query_aln, query_ins);
    let n = ref_calls.len().min(query_calls.len());

    let mut rows = Vec::new();
    for idx in 0..n {
        let rc = ref_calls[idx];
        let qc = query_calls[idx];
        let rb = rc.nt;
        let qb = qc.nt;

        // Padding means the isolate does not span this coordinate; not a real diff.
        if rb == b'.' || qb == b'.' {
            continue;
        }
        if rb.eq_ignore_ascii_case(&qb) {
            continue;
        }

        let aa = rc.dais_nt_pos.div_ceil(3);
        let ref_codon = codon_at(ref_aln, aa);
        let query_codon = codon_at(query_aln, aa);
        let ref_aa = translate(&ref_codon) as char;
        let query_aa = translate(&query_codon) as char;
        let ref_aa_pos = aa_pos_of(rc.ref_nt_pos);
        let query_aa_pos = aa_pos_of(qc.ref_nt_pos);
        // Codon position (1/2/3) of this nucleotide within its reference-coordinate codon.
        let codon_pos0 = (rc.dais_nt_pos - 1) % 3;
        let codon_position = codon_pos0 + 1;
        // Positions of interest are numbered against the reference strain's own
        // (indel-adjusted) amino-acid positions, i.e. `ref-aa-position`.
        let poi = poi_positions.contains_key(&ref_aa_pos);

        match filter {
            FilterMode::AllDiffs => {}
            FilterMode::Poi => {
                if !poi {
                    continue;
                }
            }
            FilterMode::PoiMatch => match poi_positions.get(&ref_aa_pos) {
                Some(target) => {
                    if query_aa.to_string() != target.trim() {
                        continue;
                    }
                }
                None => continue,
            },
        }

        // Annotate with the MIRA minor-variant call at this query nucleotide position.
        let minor = minor_index.and_then(|idx| {
            let key = (
                sample_base(query_name).to_string(),
                query_ctype.to_string(),
                qc.ref_nt_pos,
            );
            idx.get(&key).map(|m| {
                let codon_pos = codon_pos0;
                let mut minor_codon = query_codon.clone();
                if codon_pos < minor_codon.len() {
                    minor_codon[codon_pos] = m.minority_allele;
                }
                let minor_aa = translate(&minor_codon) as char;
                let freq_major = if m.depth > 0 {
                    m.consensus_count as f64 / m.depth as f64
                } else {
                    0.0
                };
                MinorInfo {
                    minor_nt: m.minority_allele as char,
                    minor_aa,
                    minor_codon: String::from_utf8_lossy(&minor_codon).to_string(),
                    freq_minor: m.minority_frequency,
                    freq_major,
                    total_depth: m.depth,
                }
            })
        });

        rows.push(DiffRow {
            query_name: query_name.to_string(),
            segment: segment.to_string(),
            protein: protein.to_string(),
            ref_name: ref_name.to_string(),
            ref_nt_pos: rc.ref_nt_pos,
            query_nt_pos: qc.ref_nt_pos,
            ref_nt: rb as char,
            query_nt: qb as char,
            ref_aa_pos,
            query_aa_pos,
            ref_codon: String::from_utf8_lossy(&ref_codon).to_string(),
            query_codon: String::from_utf8_lossy(&query_codon).to_string(),
            codon_position,
            ref_aa,
            query_aa,
            poi,
            minor,
        });
    }
    rows
}

// ---------------------------------------------------------------------------
// I/O helpers
// ---------------------------------------------------------------------------

fn read_tsv<T: DeserializeOwned>(path: &PathBuf) -> Result<Vec<T>, Box<dyn Error>> {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let reader = BufReader::new(file);
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(b'\t')
        .from_reader(reader);

    let mut records = Vec::new();
    for result in rdr.deserialize() {
        records.push(result?);
    }
    Ok(records)
}

/// A closed downstream pipe (e.g. piping into `head`) is a normal, clean exit, not an error.
fn is_broken_pipe(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::BrokenPipe
}

/// Write bytes to stdout, treating a broken pipe as a clean exit.
fn write_stdout(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    match std::io::stdout().write_all(bytes) {
        Ok(()) => Ok(()),
        Err(e) if is_broken_pipe(&e) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Build `(query_id, protein) -> Vec<(upstream_nt_pos, inserted_nt)>`.
fn index_insertions(records: &[InsRecord]) -> HashMap<(String, String), Vec<(usize, String)>> {
    let mut map: HashMap<(String, String), Vec<(usize, String)>> = HashMap::new();
    for r in records {
        map.entry((r.query_id.clone(), r.protein.clone()))
            .or_default()
            .push((r.upstream_nt_pos, r.inserted_nt.clone()));
    }
    map
}

/// Parse a positions-of-interest table from raw text. Rows may be separated by any
/// whitespace (tabs and/or spaces); blank lines and `#` comments are skipped. The
/// amino-acid-of-interest column is optional.
fn parse_positions_str(contents: &str) -> Result<Vec<PosRecord>, Box<dyn Error>> {
    let mut records = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        if fields.len() < 3 {
            return Err(format!(
                "positions file line {line_no}: expected at least 3 fields \
                 (ref-name segment amino-acid-position [amino-acid-of-interest]), found {}",
                fields.len()
            )
            .into());
        }
        let aa_position: usize = fields[2].parse().map_err(|_| {
            format!(
                "positions file line {line_no}: amino-acid-position '{}' is not a non-negative integer",
                fields[2]
            )
        })?;
        records.push(PosRecord {
            ref_name: fields[0].to_string(),
            segment: fields[1].to_string(),
            aa_position,
            aa_of_interest: fields.get(3).copied().unwrap_or_default().to_string(),
        });
    }
    Ok(records)
}

/// Read a positions-of-interest file (whitespace-separated, optional 4th column).
fn read_positions(path: &PathBuf) -> Result<Vec<PosRecord>, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    parse_positions_str(&contents)
}

/// Read a MIRA minor-variant CSV into an index keyed by (sample, reference, position).
fn read_minor_variants(path: &PathBuf) -> Result<MinorIndex, Box<dyn Error>> {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let reader = BufReader::new(file);
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .delimiter(b',')
        .from_reader(reader);

    let mut index = MinorIndex::new();
    for result in rdr.deserialize() {
        let r: MinorRecord = result?;
        let minority_allele = r.minority_allele.bytes().next().unwrap_or(b'N');
        index.insert(
            (r.sample, r.reference, r.sample_position),
            MinorRaw {
                minority_allele,
                depth: r.depth,
                consensus_count: r.consensus_count,
                minority_frequency: r.minority_frequency,
            },
        );
    }
    Ok(index)
}

/// Whether an IUPAC nucleotide `code` (possibly ambiguous) can represent `base`.
fn iupac_contains(code: u8, base: u8) -> bool {
    let base = base.to_ascii_uppercase();
    let set: &[u8] = match code.to_ascii_uppercase() {
        b'A' => b"A",
        b'C' => b"C",
        b'G' => b"G",
        b'T' | b'U' => b"T",
        b'R' => b"AG",
        b'Y' => b"CT",
        b'S' => b"GC",
        b'W' => b"AT",
        b'K' => b"GT",
        b'M' => b"AC",
        b'B' => b"CGT",
        b'D' => b"AGT",
        b'H' => b"ACT",
        b'V' => b"ACG",
        b'N' => b"ACGT",
        _ => b"",
    };
    set.contains(&base)
}

/// Map a 1-based position in the sample's assembled sequence to its 1-based CDS index,
/// using the DAIS `query_coordinates` (the ordered assembled position ranges that form
/// `cds_seq`). Returns `None` if the position is not covered (e.g. UTR or an intron).
fn map_assembled_to_cds(query_coordinates: &str, p: usize) -> Option<usize> {
    let mut cds_offset = 0usize;
    for token in query_coordinates.split(';') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let (start, end) = if let Some((s, e)) = token.split_once("..") {
            (
                s.trim().parse::<usize>().ok()?,
                e.trim().parse::<usize>().ok()?,
            )
        } else {
            let v = token.parse::<usize>().ok()?;
            (v, v)
        };
        if end < start {
            return None;
        }
        if p >= start && p <= end {
            return Some(cds_offset + (p - start) + 1);
        }
        cds_offset += end - start + 1;
    }
    None
}

/// Standalone annotation: append `codon`, `codon-position`, `consensus-aa` and `minority-aa`
/// to each row of a MIRA minor-variant CSV, using the query CDS to resolve the codon.
///
/// `sample_position` is a position in the sample's assembled sequence; it is mapped to the CDS
/// index via the record's `query_coordinates`. When a segment has multiple protein products, the
/// candidate whose CDS base is consistent with `consensus_allele` is preferred (falling back to
/// the longest CDS that spans the position).
#[allow(clippy::too_many_lines)]
fn run_annotation(
    query_seq_path: &PathBuf,
    minor_path: &PathBuf,
    output: Option<&PathBuf>,
    in_place: bool,
) -> Result<(), Box<dyn Error>> {
    let query_seq: Vec<SeqRecord> = read_tsv(query_seq_path)?;

    // (sample, ctype) -> query records (per protein), longest CDS first.
    let mut by_sample_ctype: HashMap<(String, String), Vec<&SeqRecord>> = HashMap::new();
    for r in &query_seq {
        by_sample_ctype
            .entry((sample_base(&r.query_id).to_string(), r.ctype.clone()))
            .or_default()
            .push(r);
    }
    for candidates in by_sample_ctype.values_mut() {
        candidates.sort_by_key(|c| std::cmp::Reverse(c.cds_seq.len()));
    }

    let file = OpenOptions::new().read(true).open(minor_path)?;
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .delimiter(b',')
        .from_reader(BufReader::new(file));

    let headers = rdr.headers()?.clone();
    let col = |name: &str| headers.iter().position(|h| h == name);
    let sample_i = col("sample").ok_or("minor-variants CSV missing 'sample' column")?;
    let ref_i = col("reference").ok_or("minor-variants CSV missing 'reference' column")?;
    let pos_i =
        col("sample_position").ok_or("minor-variants CSV missing 'sample_position' column")?;
    let cons_i =
        col("consensus_allele").ok_or("minor-variants CSV missing 'consensus_allele' column")?;
    let minor_i =
        col("minority_allele").ok_or("minor-variants CSV missing 'minority_allele' column")?;

    let mut out_lines: Vec<String> = Vec::new();
    let mut header: Vec<String> = headers.iter().map(str::to_string).collect();
    header.extend(
        ["codon", "codon-position", "consensus-aa", "minority-aa"]
            .iter()
            .map(|s| (*s).to_string()),
    );
    out_lines.push(header.join(","));

    for result in rdr.records() {
        let rec = result?;
        let mut fields: Vec<String> = rec.iter().map(str::to_string).collect();

        let sample = rec.get(sample_i).unwrap_or_default();
        let reference = rec.get(ref_i).unwrap_or_default();
        let pos: Option<usize> = rec.get(pos_i).and_then(|s| s.parse().ok());
        let consensus = rec.get(cons_i).and_then(|s| s.bytes().next());
        let minor_nt = rec
            .get(minor_i)
            .and_then(|s| s.bytes().next())
            .unwrap_or(b'N');

        let annotation = pos.and_then(|p| {
            let candidates = by_sample_ctype.get(&(sample.to_string(), reference.to_string()))?;

            // `sample_position` is a position in the sample's assembled sequence. Map it to the
            // CDS index via the record's `query_coordinates` (the assembled positions, in order,
            // that form `cds_seq`). This handles UTR offset, splicing, and insertions.
            //
            // Prefer a CDS whose base at the mapped position is consistent with the consensus
            // allele; otherwise fall back to the longest CDS that spans the position.
            let mut fallback: Option<(&SeqRecord, usize)> = None;
            let mut chosen: Option<(&SeqRecord, usize)> = None;
            for &c in candidates {
                let Some(cds_index) = map_assembled_to_cds(&c.query_coordinates, p) else {
                    continue;
                };
                let cds = c.cds_seq.as_bytes();
                let start = (cds_index - 1) / 3 * 3;
                if start + 3 > cds.len() {
                    continue;
                }
                if fallback.is_none() {
                    fallback = Some((c, cds_index));
                }
                let codon_pos0 = (cds_index - 1) % 3;
                if consensus.is_some_and(|cons| iupac_contains(cds[start + codon_pos0], cons)) {
                    chosen = Some((c, cds_index));
                    break;
                }
            }

            let (record, cds_index) = chosen.or(fallback)?;
            let cds = record.cds_seq.as_bytes();
            let start = (cds_index - 1) / 3 * 3;
            let codon_pos = (cds_index - 1) % 3 + 1;
            let raw = &cds[start..start + 3];

            // Resolve the codon to the consensus allele at the variant site (the CDS may carry an
            // IUPAC het code there), then translate the consensus and minor-allele codons.
            let mut consensus_codon = raw.to_vec();
            if let Some(cons) = consensus {
                consensus_codon[codon_pos - 1] = cons;
            }
            let consensus_aa = translate(&consensus_codon) as char;
            let mut minor_codon = raw.to_vec();
            minor_codon[codon_pos - 1] = minor_nt;
            let minority_aa = translate(&minor_codon) as char;
            Some((
                String::from_utf8_lossy(&consensus_codon).to_string(),
                codon_pos.to_string(),
                consensus_aa.to_string(),
                minority_aa.to_string(),
            ))
        });

        let (codon, codon_pos, consensus_aa, minority_aa) = annotation.unwrap_or_default();
        fields.push(codon);
        fields.push(codon_pos);
        fields.push(consensus_aa);
        fields.push(minority_aa);
        out_lines.push(fields.join(","));
    }

    let content = format!("{}\n", out_lines.join("\n"));
    if in_place {
        std::fs::write(minor_path, content)?;
    } else if let Some(path) = output {
        std::fs::write(path, content)?;
    } else {
        write_stdout(content.as_bytes())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// A reference record a query should be compared against, plus its POI context.
struct Target<'a> {
    ref_name: String,
    segment: String,
    rrec: &'a SeqRecord,
    poi: &'a HashMap<usize, String>,
}

#[allow(clippy::too_many_lines)]
pub fn variants_process(args: VariantsArgs) -> Result<(), Box<dyn Error>> {
    let VariantsArgs {
        ref_seq,
        ref_ins,
        ref_del,
        query_seq: query_seq_path,
        query_ins,
        query_del,
        positions,
        minor_variants: minor_variants_path,
        filter,
        output,
        in_place,
        delimiter,
    } = args;

    // Standalone annotation mode: annotate the MIRA minor-variant CSV using the query CDS.
    if positions.is_none() {
        let Some(minor_path) = minor_variants_path else {
            return Err(
                "provide --positions for comparison mode, or --minor-variants for annotation mode"
                    .into(),
            );
        };
        return run_annotation(&query_seq_path, &minor_path, output.as_ref(), in_place);
    }

    // Comparison mode requires the reference set and the query indel files.
    let positions_path = positions.expect("checked above");
    let ref_seq_path = ref_seq.ok_or("comparison mode (--positions) requires --ref-seq")?;
    let ref_ins_path = ref_ins.ok_or("comparison mode (--positions) requires --ref-ins")?;
    let ref_del_path = ref_del.ok_or("comparison mode (--positions) requires --ref-del")?;
    let query_ins_path = query_ins.ok_or("comparison mode (--positions) requires --query-ins")?;
    let query_del_path = query_del.ok_or("comparison mode (--positions) requires --query-del")?;

    let delim = delimiter
        .as_deref()
        .and_then(|s| s.chars().next())
        .unwrap_or('\t');

    // Reference inputs.
    let ref_seq: Vec<SeqRecord> = read_tsv(&ref_seq_path)?;
    let ref_ins: Vec<InsRecord> = read_tsv(&ref_ins_path)?;
    // Deletions are already encoded as gaps in cds_aln; parse to validate format only.
    let _: Vec<DelRecord> = read_tsv(&ref_del_path)?;

    // Query inputs.
    let query_seq: Vec<SeqRecord> = read_tsv(&query_seq_path)?;
    let query_ins: Vec<InsRecord> = read_tsv(&query_ins_path)?;
    let _: Vec<DelRecord> = read_tsv(&query_del_path)?;

    // Positions of interest.
    let positions: Vec<PosRecord> = read_positions(&positions_path)?;

    // Optional MIRA minor-variant annotation.
    let minor_index = match minor_variants_path {
        Some(ref path) => Some(read_minor_variants(path)?),
        None => None,
    };
    let include_minor = minor_index.is_some();

    // (reference sample name, segment) -> { aa_position -> aa_of_interest }.
    // `ref_name` is a real sample name found in column 1 (query_id) of the reference files.
    let mut poi_groups: HashMap<(String, String), HashMap<usize, String>> = HashMap::new();
    for p in &positions {
        poi_groups
            .entry((p.ref_name.clone(), p.segment.clone()))
            .or_default()
            .insert(p.aa_position, p.aa_of_interest.clone());
    }

    // (reference sample name, segment) -> reference SeqRecords (one per protein).
    let mut ref_by_name_seg: HashMap<(String, String), Vec<&SeqRecord>> = HashMap::new();
    for r in &ref_seq {
        if let Some(seg) = segment_of(&r.ctype) {
            ref_by_name_seg
                .entry((r.query_id.clone(), seg.to_string()))
                .or_default()
                .push(r);
        }
    }

    // A reference record a query should be compared against, plus its POI context.
    // Query records share coordinate space with a reference record when they align to the
    // same DAIS `reference_id` and protein, so index targets by that pair.
    let mut targets: HashMap<(String, String), Vec<Target>> = HashMap::new();
    for ((ref_name, segment), poi_positions) in &poi_groups {
        let Some(recs) = ref_by_name_seg.get(&(ref_name.clone(), segment.clone())) else {
            continue;
        };
        for &rrec in recs {
            targets
                .entry((rrec.reference_id.clone(), rrec.protein.clone()))
                .or_default()
                .push(Target {
                    ref_name: ref_name.clone(),
                    segment: segment.clone(),
                    rrec,
                    poi: poi_positions,
                });
        }
    }

    let ref_ins_index = index_insertions(&ref_ins);
    let query_ins_index = index_insertions(&query_ins);

    let mut writer = if let Some(ref path) = output {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        BufWriter::new(Either::Left(file))
    } else {
        BufWriter::new(Either::Right::<File, Stdout>(stdout()))
    };

    let delim_str = delim.to_string();
    let mut header: Vec<&str> = HEADER.to_vec();
    if include_minor {
        header.extend_from_slice(&MINOR_HEADER);
    }
    if let Err(e) = writeln!(&mut writer, "{}", header.join(&delim_str)) {
        if is_broken_pipe(&e) {
            return Ok(());
        }
        return Err(e.into());
    }

    for q in &query_seq {
        let Some(tgts) = targets.get(&(q.reference_id.clone(), q.protein.clone())) else {
            continue;
        };

        for t in tgts {
            let ref_ins_for = ref_ins_index
                .get(&(t.rrec.query_id.clone(), t.rrec.protein.clone()))
                .cloned()
                .unwrap_or_default();
            let query_ins_for = query_ins_index
                .get(&(q.query_id.clone(), q.protein.clone()))
                .cloned()
                .unwrap_or_default();

            let rows = compare_record(
                &q.query_id,
                &q.ctype,
                &t.segment,
                &q.protein,
                &t.ref_name,
                t.rrec.cds_aln.as_bytes(),
                &ref_ins_for,
                q.cds_aln.as_bytes(),
                &query_ins_for,
                t.poi,
                minor_index.as_ref(),
                filter,
            );

            for row in rows {
                if let Err(e) = writeln!(&mut writer, "{}", row.to_delimited(delim, include_minor))
                {
                    if is_broken_pipe(&e) {
                        return Ok(());
                    }
                    return Err(e.into());
                }
            }
        }
    }

    if let Err(e) = writer.flush() {
        if is_broken_pipe(&e) {
            return Ok(());
        }
        return Err(e.into());
    }
    Ok(())
}
