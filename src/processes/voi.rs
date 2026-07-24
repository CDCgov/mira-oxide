//! `voi` — reference-vs-query positions-of-interest comparison.
//!
//! Reconstructs full-length sequences from DAIS-ribosome protein-level output
//! (`.seq` / `.ins` / `.del`) for a set of "reference" strains and a set of "query"
//! samples, then compares each query against its matching reference in the shared
//! DAIS reference-coordinate space, reporting nucleotide / amino-acid / codon
//! differences and flagging those that land on a user-supplied position of interest.
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
#[command(about = "Compare query DAIS output against reference at positions of interest")]
pub struct VoiArgs {
    /// Reference `SEQUENCE_OUTPUT` (`.seq`) file
    #[arg(long)]
    ref_seq: PathBuf,
    /// Reference `INSERTION_OUTPUT` (`.ins`) file
    #[arg(long)]
    ref_ins: PathBuf,
    /// Reference `DELETION_OUTPUT` (`.del`) file
    #[arg(long)]
    ref_del: PathBuf,
    /// Query `SEQUENCE_OUTPUT` (`.seq`) file
    #[arg(long)]
    query_seq: PathBuf,
    /// Query `INSERTION_OUTPUT` (`.ins`) file
    #[arg(long)]
    query_ins: PathBuf,
    /// Query `DELETION_OUTPUT` (`.del`) file
    #[arg(long)]
    query_del: PathBuf,
    /// Positions-of-interest file (ref-name, segment, aa-position, aa-of-interest)
    #[arg(long)]
    positions: PathBuf,
    /// Optional MIRA minor-variant CSV; when supplied, adds minor-allele annotation columns
    #[arg(long)]
    minor_variants: Option<PathBuf>,
    /// Row selection mode
    #[arg(long, value_enum, default_value_t = FilterMode::AllDiffs)]
    filter: FilterMode,
    /// Optional output file (defaults to stdout)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
    /// Single-character output delimiter (defaults to TAB)
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
    let file = OpenOptions::new().read(true).open(path)?;
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
    let contents = std::fs::read_to_string(path)?;
    parse_positions_str(&contents)
}

/// Read a MIRA minor-variant CSV into an index keyed by (sample, reference, position).
fn read_minor_variants(path: &PathBuf) -> Result<MinorIndex, Box<dyn Error>> {
    let file = OpenOptions::new().read(true).open(path)?;
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
pub fn voi_process(args: VoiArgs) -> Result<(), Box<dyn Error>> {
    let VoiArgs {
        ref_seq: ref_seq_path,
        ref_ins: ref_ins_path,
        ref_del: ref_del_path,
        query_seq: query_seq_path,
        query_ins: query_ins_path,
        query_del: query_del_path,
        positions: positions_path,
        minor_variants: minor_variants_path,
        filter,
        output,
        delimiter,
    } = args;

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
    writeln!(&mut writer, "{}", header.join(&delim_str))?;

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
                writeln!(&mut writer, "{}", row.to_delimited(delim, include_minor))?;
            }
        }
    }

    writer.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_extraction() {
        assert_eq!(segment_of("A_HA_H1"), Some("HA"));
        assert_eq!(segment_of("A_MP"), Some("MP"));
        assert_eq!(segment_of("B_NA"), Some("NA"));
        assert_eq!(segment_of(""), None);
    }

    #[test]
    fn positions_parse_tolerates_whitespace_and_optional_column() {
        // Mixed tab/space separators, an optional missing 4th column, blanks, and comments.
        let contents = "CY019971\tNA\t275\tY\n\
                        CY019971    HA  14  A\n\
                        # a comment\n\
                        \n\
                        CY019971\tPB1\t100\n";
        let recs = parse_positions_str(contents).expect("should parse");
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].ref_name, "CY019971");
        assert_eq!(recs[0].segment, "NA");
        assert_eq!(recs[0].aa_position, 275);
        assert_eq!(recs[0].aa_of_interest, "Y");
        // Space-separated row.
        assert_eq!(recs[1].segment, "HA");
        assert_eq!(recs[1].aa_of_interest, "A");
        // Row without the optional amino-acid-of-interest column.
        assert_eq!(recs[2].segment, "PB1");
        assert_eq!(recs[2].aa_position, 100);
        assert_eq!(recs[2].aa_of_interest, "");
    }

    #[test]
    fn positions_parse_rejects_short_rows() {
        assert!(parse_positions_str("CY019971\tNA\n").is_err());
        assert!(parse_positions_str("CY019971 NA notanumber\n").is_err());
    }

    #[test]
    fn translate_handles_gaps_and_bases() {
        assert_eq!(translate(b"ATG") as char, 'M');
        assert_eq!(translate(b"AAA") as char, 'K');
        assert_eq!(translate(b"---") as char, 'X');
        assert_eq!(translate(b"A.G") as char, 'X');
        assert_eq!(translate(b"AT") as char, 'X');
    }

    #[test]
    fn calls_without_indels_map_one_to_one() {
        // No insertions, no gaps: dais and isolate positions match.
        let calls = build_calls(b"ATGAAA", &[]);
        assert_eq!(calls.len(), 6);
        for (i, c) in calls.iter().enumerate() {
            assert_eq!(c.dais_nt_pos, i + 1);
            assert_eq!(c.ref_nt_pos, i + 1);
        }
    }

    #[test]
    fn deletion_gap_holds_isolate_position() {
        // A 3-nt deletion (gap) at reference positions 4-6.
        let calls = build_calls(b"ATG---AAA", &[]);
        // Reference positions 1-3 -> isolate 1-3.
        assert_eq!(calls[2].ref_nt_pos, 3);
        // Gap positions have isolate position 0.
        assert_eq!(calls[3].ref_nt_pos, 0);
        assert_eq!(calls[4].ref_nt_pos, 0);
        assert_eq!(calls[5].ref_nt_pos, 0);
        // Reference position 7 resumes isolate numbering at 4.
        assert_eq!(calls[6].ref_nt_pos, 4);
        assert_eq!(calls[6].dais_nt_pos, 7);
    }

    #[test]
    fn insertion_advances_isolate_position() {
        // Insert 3 nt after reference position 3.
        let calls = build_calls(b"ATGAAA", &[(3, "CCC".to_string())]);
        // Positions 1-3 unchanged.
        assert_eq!(calls[2].ref_nt_pos, 3);
        // Position 4 (reference) is now isolate position 7 (3 inserted bases skipped).
        assert_eq!(calls[3].ref_nt_pos, 7);
        assert_eq!(calls[3].dais_nt_pos, 4);
    }

    #[test]
    fn point_mutation_is_reported_and_flagged() {
        // Reference codon 2 = AAA (K); query codon 2 = GAA (E) at reference nt 4.
        let mut poi = HashMap::new();
        poi.insert(2usize, "E".to_string());

        let rows = compare_record(
            "sampleQ",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"ATGGAA",
            &[],
            &poi,
            None,
            FilterMode::AllDiffs,
        );
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.ref_nt, 'A');
        assert_eq!(r.query_nt, 'G');
        assert_eq!(r.ref_nt_pos, 4);
        assert_eq!(r.query_nt_pos, 4);
        assert_eq!(r.ref_aa_pos, 2);
        assert_eq!(r.query_aa_pos, 2);
        assert_eq!(r.ref_codon, "AAA");
        assert_eq!(r.query_codon, "GAA");
        assert_eq!(r.codon_position, 1);
        assert_eq!(r.ref_aa, 'K');
        assert_eq!(r.query_aa, 'E');
        assert!(r.poi);
    }

    #[test]
    fn minor_variant_annotation_is_joined_and_translated() {
        // Query codon 2 = GAA (E) at query nt 4. A minor allele C at that position
        // (codon position 1) yields CAA (Q).
        let poi = HashMap::new();
        let mut minor: MinorIndex = HashMap::new();
        minor.insert(
            ("sampleQ".to_string(), "A_NA_N1".to_string(), 4usize),
            MinorRaw {
                minority_allele: b'C',
                depth: 100,
                consensus_count: 70,
                minority_frequency: 0.30,
            },
        );

        // query_name carries a segment suffix that must be stripped to match the CSV sample.
        let rows = compare_record(
            "sampleQ_4",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"ATGGAA",
            &[],
            &poi,
            Some(&minor),
            FilterMode::AllDiffs,
        );
        assert_eq!(rows.len(), 1);
        let m = rows[0].minor.as_ref().expect("minor info present");
        assert_eq!(m.minor_nt, 'C');
        assert_eq!(m.minor_codon, "CAA");
        assert_eq!(m.minor_aa, 'Q');
        assert!((m.freq_minor - 0.30).abs() < 1e-9);
        assert!((m.freq_major - 0.70).abs() < 1e-9);
        assert_eq!(m.total_depth, 100);
    }

    #[test]
    fn sample_base_strips_segment_suffix() {
        assert_eq!(sample_base("046435d3_4"), "046435d3");
        assert_eq!(sample_base("CY019971"), "CY019971");
        assert_eq!(sample_base("weird_name"), "weird_name");
    }

    #[test]
    fn poi_match_filters_on_amino_acid() {
        // Query codon 2 translates to E. poi-match requires query aa == target.
        let mut poi = HashMap::new();
        poi.insert(2usize, "E".to_string());

        let matching = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"ATGGAA",
            &[],
            &poi,
            None,
            FilterMode::PoiMatch,
        );
        assert_eq!(matching.len(), 1);

        // Different target amino acid -> filtered out.
        let mut poi2 = HashMap::new();
        poi2.insert(2usize, "Y".to_string());
        let non_matching = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"ATGGAA",
            &[],
            &poi2,
            None,
            FilterMode::PoiMatch,
        );
        assert!(non_matching.is_empty());
    }

    #[test]
    fn poi_filter_drops_non_poi_positions() {
        // Difference at aa position 1, but poi only lists position 2.
        let mut poi = HashMap::new();
        poi.insert(2usize, "K".to_string());
        let rows = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"GTGAAA",
            &[],
            &poi,
            None,
            FilterMode::Poi,
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn poi_matches_reference_aa_position_not_dais_coordinate() {
        // Reference has 3 nt of leading padding, so its isolate amino-acid numbering
        // is offset from the DAIS reference coordinate. A difference at DAIS nt 7
        // (DAIS aa 3) is reference aa position 2.
        let ref_aln = b"...ATGAAA";
        let query_aln = b"...ATGGAA";

        // Keyed on the reference aa position (2) -> matches.
        let mut poi_ref = HashMap::new();
        poi_ref.insert(2usize, "E".to_string());
        let rows = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "strain",
            ref_aln,
            &[],
            query_aln,
            &[],
            &poi_ref,
            None,
            FilterMode::Poi,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ref_aa_pos, 2);
        assert!(rows[0].poi);

        // Keyed on the DAIS coordinate (3) -> no longer matches.
        let mut poi_dais = HashMap::new();
        poi_dais.insert(3usize, "E".to_string());
        let rows = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "strain",
            ref_aln,
            &[],
            query_aln,
            &[],
            &poi_dais,
            None,
            FilterMode::Poi,
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn padding_is_not_a_difference() {
        let rows = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"..GAAA",
            &[],
            &HashMap::new(),
            None,
            FilterMode::AllDiffs,
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn deletion_is_reported_as_difference() {
        // Query has a single-base deletion at reference position 4.
        let rows = compare_record(
            "q",
            "A_NA_N1",
            "NA",
            "NA",
            "CALI07",
            b"ATGAAA",
            &[],
            b"ATG-AA",
            &[],
            &HashMap::new(),
            None,
            FilterMode::AllDiffs,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].query_nt, '-');
        assert_eq!(rows[0].query_nt_pos, 0);
    }
}
