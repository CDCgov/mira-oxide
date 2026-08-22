# Variants

Tool for observing nucleotide, codon and amino acid differences. Comparisons can be made between query samples and references using DAIS-ribsome outputs or minor variants from IRMA can be annotated.

## Overview

This tool compares aligned nucleotide sequences between query samples and reference strains or between consensus and minor alleles, reporting differences at the nucleotide, codon, and amino acid level. It supports three mutually exclusive modes of operation, with some modes allowing for optional cross-referencing with known variants of interest and annotation with minor variant data.

## Modes

Exactly one mode must be specified:

### `--all-diffs`
Reports every nucleotide difference between each query and its matching reference, without regard to positions of interest.

- **Requires:** `--query-dais-file`, `--query-insertion-file`, `--query-deletion-file`, `--ref-dais-file`, `--ref-insertion-file`, `--ref-deletion-file`
- **Optional:** `--variants-of-interest` (adds `variant_of_interest`/`position_of_interest` columns), `--minor-variants` (adds minor-variant columns)

### `--positions-of-interest`
Produces a variants inormation report restricted to positions listed in the variants-of-interest file.

- **Requires:** `--query-dais-file`, `--query-insertion-file`, `--query-deletion-file`, `--ref-dais-file`, `--ref-insertion-file`, `--ref-deletion-file`, `--variants-of-interest`
- **Optional:** `--minor-variants` (adds minor-variant columns)

### `--annotate-minor-variants`
Annotates the minor-variants file with codon/amino-acid context derived from the query DAIS data. Does not require any reference dais/insertion/deletion files.

- **Requires:** `--query-dais-file`, `--query-insertion-file`, `--query-deletion-file`, `--minor-variants`

## Command-Line Flags

| Flag | Description |
|---|---|
| `--query-dais-file` | Input dais-ribosome file (required) |
| `--ref-dais-file` | Reference dais-ribosome file |
| `--variants-of-interest` | Variants-of-interest (mutations) file |
| `--query-insertion-file` | Query insertion (.ins) file (required) |
| `--query-deletion-file` | Query deletion (.del) file (required) |
| `--ref-insertion-file` | Reference insertion (.ins) file |
| `--ref-deletion-file` | Reference deletion (.del) file |
| `--minor-variants` | Minor variants (.csv, with headers) file |
| `--output-xsv` | Output delimited file; prints to stdout if omitted |
| `--output-delimiter` | Delimiter for output fields (default: `,`) |
| `--all-positions` | Print all positions in the positions-of-interest report, not just those where query and reference nucleotides differ. Has no effect in `--annotate-minor-variants` mode or when `--all-diffs` is used |
| `--all-diffs` | Mode selector (see above) |
| `--positions-of-interest` | Mode selector (see above) |
| `--annotate-minor-variants` | Mode selector (see above) |

## Input File Formats

All DAIS-ribosome, insertion, deletion, and mutations-of-interest files are **tab-separated** and have **no headers**. The minor-variants file is **comma-separated** (CSV) **with headers**.

### Query DAIS-ribosome file
Columns: `sample_id`, `ctype`, `dais_ref_id`, `protein`, `nt_hash`, `query_aa_seq`, `query_aa_aln_seq`, `cds_id`, `insertion`, `inert_shift`, `query_cds_seq`, `query_cds_aln`, `query_nt_coordinates`, `cds_nt_coordinates`

### Reference DAIS-ribosome file
Columns: `ref_id`, `ctype`, `dais_ref_id`, `protein`, `nt_hash`, `ref_aa_seq`, `ref_aa_aln_seq`, `cds_id`, `insertion`, `inert_shift`, `ref_cds_seq`, `ref_cds_aln`, `ref_nt_coordinates`, `ref_cds_nt_coordinates`

### Mutations-of-interest file
Columns: `subtype`, `protein`, `aa_position`, `aa`, `description`

### Insertion file
Columns: `query_id`, `ctype`, `reference_id`, `product_name`, `upstream_aa_pos`, `inserted_nt`, `inserted_aa`, `upstream_nt_pos`, `codon_shift`

### Deletion file
Columns: `query_id`, `ctype`, `reference_id`, `product_name`, `variant_hash`, `del_aa_start`, `del_aa_end`, `del_aa_len`, `in_frame`, `cds_id`, `del_cds_start`, `del_cds_end`, `del_cds_len`

### Minor variants file
Columns: `sample`, `reference`, `sample_position`, `depth`, `consensus_allele`, `minority_allele`, `consensus_count`, `minority_count`, `minority_frequency`, `run_id`, `instrument`

## How It Works

1. **Alignment matching**: Query and reference entries are matched on `ctype`, `dais_ref_id`, and `protein`. All DAIS-ribosome outputs from v2.1.0 and higher should be the same length if sharing `dais_ref_id`. Pairs with mismatched aligned sequence lengths are skipped with a warning printed to stdout.
2. **Codon translation**: Aligned sequences are split into codons and translated to amino acids using the standard genetic code. A trailing partial codon (if any) is handled separately and marked with `~`.
3. **Position adjustment**: Raw alignment positions are adjusted for insertions and deletions on both the query and reference side to provide actual nucleotide and amino acid positions with the query (`query_nt_position` and `query_aa_position`) and the reference (`ref_nt_position` and `ref_aa_position`).
4. **Minor variant lookup**: When a minor-variants file is supplied, minor alleles at matching positions are substituted into the query codon to compute an alternate codon/amino acid.

## Output

Output is written as delimited text (default comma-separated) either to a file (`--output-xsv`) or to stdout. Column sets vary by mode and by which optional inputs are supplied — see the mode descriptions above for details on which extra columns are appended.

## Example Usage

```bash
# Full diff report against a reference
variants --query-dais-file query.tsv \
  --query-insertion-file query.ins \
  --query-deletion-file query.del \
  --ref-dais-file ref.tsv \
  --ref-insertion-file ref.ins \
  --ref-deletion-file ref.del \
  --all-diffs \
  --output-xsv all_diffs.csv

# Positions-of-interest report with minor variants
variants --query-dais-file query.tsv \
  --query-insertion-file query.ins \
  --query-deletion-file query.del \
  --ref-dais-file ref.tsv \
  --ref-insertion-file ref.ins \
  --ref-deletion-file ref.del \
  --variants-of-interest muts.tsv \
  --minor-variants minor_variants.csv \
  --positions-of-interest \
  --output-xsv poi_report.csv

# Annotate minor variants only
variants --query-dais-file query.tsv \
  --query-insertion-file query.ins \
  --query-deletion-file query.del \
  --minor-variants minor_variants.csv \
  --annotate-minor-variants \
  --output-xsv annotated_minor_variants.csv
```

> **Note:** `variants` is used as a placeholder binary name in the examples above since the actual crate/binary name isn't shown in the source — replace it with whatever is defined in your `Cargo.toml`.