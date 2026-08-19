# variants

A command-line tool for identifying codon and amino-acid differences between a query sample
and a reference, at specific positions of interest, and/or annotating minor (sub-consensus)
variants with their codon/amino-acid context.

## Overview

This tool consumes DAIS-ribosome output (query and, optionally, reference alignments) along
with insertion/deletion records, and produces a delimited report. It supports two modes,
which can be run independently or together:

1. **Positions-of-interest mode** — compares a query sample against a reference sequence,
   codon by codon, and reports every position that matches a caller-supplied list of
   mutations of interest (`--positions-of-interest`). Optionally annotates each reported
   position with any overlapping minor variants.
2. **Minor-variants-only mode** — takes a minor variants CSV (sub-consensus allele calls,
   e.g. from deep sequencing) and annotates each row with the consensus and minor-variant
   codon/amino acid at that position, without requiring a reference or a mutations-of-interest
   list.

At least one of `--positions-of-interest` or `--minor-variants-file` must be supplied.

## Usage

```
variants \
  -q <query_dais_file> \
  -i <query_insertion_file> \
  -d <query_deletion_file> \
  [-r <ref_dais_file>] \
  [-j <ref_insertion_file>] \
  [-e <ref_deletion_file>] \
  [-v <positions_of_interest_file>] \
  [-m <minor_variants_file>] \
  [-o <output_file>] \
  [-s <output_delimiter>]
```

### Arguments

| Flag | Long form | Required | Description |
|---|---|---|---|
| `-q` | `--query-dais-file` | Yes | DAIS-ribosome output for the query sample(s). Tab-separated, no header. |
| `-i` | `--query-insertion-file` | Yes | Insertion (`.ins`) file for the query sample(s). Tab-separated, no header. |
| `-d` | `--query-deletion-file` | Yes | Deletion (`.del`) file for the query sample(s). Tab-separated, no header. |
| `-r` | `--ref-dais-file` | Only if `-v` is used | DAIS-ribosome output for the reference. |
| `-j` | `--ref-insertion-file` | Only if `-v` is used | Insertion file for the reference. |
| `-e` | `--ref-deletion-file` | Only if `-v` is used | Deletion file for the reference. |
| `-p` | `--positions-of-interest` | No* | Tab-separated, no-header file of mutations of interest. Triggers full positions-of-interest mode. |
| `-m` | `--minor-variants-file` | No* | CSV (with header) of minor/sub-consensus variant calls. |
| `-o` | `--output-xsv` | No | Output file path. Defaults to stdout. |
| `-s` | `--output-delimiter` | No | Delimiter for the output file. Defaults to `,`. |

\* At least one of `-v` or `-m` must be provided.

### Mode selection

- **`-v` provided** → full positions-of-interest mode runs. `-r`, `-j`, and `-e` become
  required in this case. If `-m` is *also* provided, each reported position is further
  annotated with any matching minor variant columns.
- **`-v` omitted, `-m` provided** → minor-variants-only mode runs instead. No reference
  files are needed.

## Input file formats

### Query/reference DAIS file (`-q` / `-r`)

Tab-separated, no header. Query and reference files use slightly different column sets
(the reference file has a `ref_id` instead of `sample_id`, `ref_aa_seq` instead of
`query_aa_seq`, etc.), but both describe, per sample/reference and per protein:

- Sample/reference ID, ctype (e.g. `B_HA`, `A_PB2`), DAIS reference ID, protein name
- Amino acid sequence and its aligned form
- CDS nucleotide sequence and its aligned form
- Nucleotide and CDS coordinate ranges

### Insertion file (`-i` / `-j`)

Tab-separated, no header. One row per insertion event:

```
query_id  ctype  reference_id  product_name  upstream_aa_pos  inserted_nt  inserted_aa  upstream_nt_pos  codon_shift
```

### Deletion file (`-d` / `-e`)

Tab-separated, no header. One row per deletion event:

```
query_id  ctype  reference_id  product_name  variant_hash  del_aa_start  del_aa_end  del_aa_len  in_frame  cds_id  del_cds_start  del_cds_end  del_cds_len
```

### Positions-of-interest file (`-v`)

Tab-separated, no header:

```
subtype  protein  aa_position  aa  description
```

`subtype` may be `all` to match any DAIS reference. Matching is done by resolving
`(subtype, protein)` to a DAIS reference ID (via `assign_dais_refs`) and comparing against
the sample's own `dais_ref_id`.

### Minor variants file (`-m`)

CSV with header:

```
sample,reference,sample_position,depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,run_id,instrument
```

`sample_position` is the **query nucleotide position after insertion/deletion adjustment**
(i.e. the position as called against the query's own indel-adjusted coordinate space, not
the raw alignment).

## How position adjustment works

DAIS alignments are gapped to a common coordinate system, but real-world positions (as used
in variant calling) are relative to the sample's own sequence, accounting for insertions and
deletions relative to that alignment. The tool moves between these coordinate spaces in both
directions:

- **`calc_query_nt_position` / `calc_ref_nt_position`** — take a raw (aligned) nucleotide
  position and produce the indel-adjusted query- or reference-side position, by adding
  upstream insertion lengths and subtracting upstream deletion lengths. Used in
  positions-of-interest mode to report `query_nt_position` / `ref_nt_position` alongside
  the raw `aln_nt_position`.
- **`calc_query_aa_position` / `calc_ref_aa_position`** — the amino-acid analogues of the
  above.
- **`find_raw_query_position`** — the inverse operation, used in minor-variants-only mode.
  Given an indel-adjusted query nucleotide position (as supplied in the minor variants file),
  it searches the raw alignment for the position that adjusts to that value. This raw
  position is reported as `dais_ref_position`, and is also what's used to locate the
  correct codon for translation.

All four forward functions and `find_raw_query_position` filter candidate insertion/deletion
rows by `sample_id`/`ref_strain`, `ctype`, `reference_id`, and `product_name`, so that indels
belonging to a different product or reference sharing the same ctype are not incorrectly
applied.

## Output formats

### Positions-of-interest mode output

CSV (or custom delimiter) with header:

```
query_name,ref_name,ctype,dais_reference,protein,aln_nt_position,ref_nt_position,query_nt_position,query_nt,ref_nt,position_in_codon,query_codon,ref_codon,aa_mutation,aln_aa_position,ref_aa_position,query_aa_position,variant_of_interest
```

If `-m` is also supplied, ten additional columns are appended:

```
depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,consensus_aa,minor_variant_codon,minor_variant_aa
```

One row is written per nucleotide position within a codon that matches a position of
interest. If multiple minor variants exist at the same query nucleotide position, one row
is written per minor variant (all other columns repeated); if none exist, the minor-variant
columns are left blank.

A final partial ("tail") codon — any leftover nucleotides not forming a complete codon — is
handled separately and marked with `~` for its reference/query amino acids.

### Minor-variants-only mode output

CSV (or custom delimiter) with header:

```
sample,reference,sample_position,dais_ref_position,depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,consensus_aa,minor_variant_codon,minor_variant_aa,run_id,instrument
```

- `dais_ref_position` — the raw (pre-indel-adjustment) nucleotide position corresponding to
  `sample_position`, as found by `find_raw_query_position`.
- `consensus_codon` / `consensus_aa` — the reference/consensus codon at that position,
  translated as-is (no allele substitution).
- `minor_variant_codon` / `minor_variant_aa` — the same codon with the minority allele
  substituted in at the appropriate base, then translated.

For each minor variant row, the tool finds the matching query DAIS entry by checking that
`sample_id` contains the `sample` value and that `ctype` matches `reference`. If multiple
product rows match (e.g. `HA`, `HA1`, `HA-signal` for the same sample/ctype), the one with
the longest `query_cds_aln` is preferred, since shorter fragments may not contain the
position being queried.

If no matching DAIS entry is found, or no raw position can be resolved, the codon/amino-acid
columns (and `dais_ref_position`, in the latter case) are left blank.

## Notes and caveats

- All positions in output columns are 1-indexed.
- `-o`/`--output-xsv` writes to a file; omitting it writes to stdout.
- The `-s`/`--output-delimiter` flag lets you produce TSV or other delimited output instead
  of CSV; it does not affect how input files are parsed (query/reference DAIS, insertion,
  and deletion files are always tab-separated; the minor variants file is always
  comma-separated).
- Query and reference DAIS entries are paired by matching `ctype`, `dais_ref_id`, and
  `protein`; pairs whose aligned nucleotide sequences differ in length are skipped with a
  warning, since codon-by-codon comparison requires equal-length alignments.