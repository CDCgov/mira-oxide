# Variants

Tool for observing nucleotide, codon and amino acid differences. Comparisons can be made between query samples and references using DAIS-ribsome outputs or minor variants output from Mira can be annotated.

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

### Query DAIS-ribosome file (headerless)
Columns: `sample_id`, `ctype`, `dais_ref_id`, `protein`, `nt_hash`, `query_aa_seq`, `query_aa_aln_seq`, `cds_id`, `insertion`, `inert_shift`, `query_cds_seq`, `query_cds_aln`, `query_nt_coordinates`, `cds_nt_coordinates`

### Reference DAIS-ribosome file (headerless)
Columns: `ref_id`, `ctype`, `dais_ref_id`, `protein`, `nt_hash`, `ref_aa_seq`, `ref_aa_aln_seq`, `cds_id`, `insertion`, `inert_shift`, `ref_cds_seq`, `ref_cds_aln`, `ref_nt_coordinates`, `ref_cds_nt_coordinates`

### Insertion file (headerless)
Columns: `query_id`, `ctype`, `reference_id`, `product_name`, `upstream_aa_pos`, `inserted_nt`, `inserted_aa`, `upstream_nt_pos`, `codon_shift`

### Deletion file (headerless)
Columns: `query_id`, `ctype`, `reference_id`, `product_name`, `variant_hash`, `del_aa_start`, `del_aa_end`, `del_aa_len`, `in_frame`, `cds_id`, `del_cds_start`, `del_cds_end`, `del_cds_len`

### Minor variants file (headers)
Columns: `sample`, `reference`, `sample_position`, `depth`, `consensus_allele`, `minority_allele`, `consensus_count`, `minority_count`, `minority_frequency`, `run_id`, `instrument`

### Variants-of-interest file (headers)
This should be the only file that the user is creating themselves.

Columns: `subtype`, `protein`, `positionS`, `amino_acid`

| Column | Description |
|---|---|
| `subtype` | This will be the subtype the aa variant of interest is typically seen in. If in more than one use `ALL` and it will check all subtypes |
| `protein` | The protein that the amino acid variant should correspond to. |
| `positionS` | This is the amino acid position within the protein where the variant of interest occurs. |
| `amino_acid` | This is the variant amino acid that the user is interested in detecting. |

Example:
```
subtype	protein	positions	amino_acid
H1N1	NA	275	Y
ALL	PA	38	F
ALL	PA	38	M
H3N2	NA	119	V
H3N2	NA	292	K
BVIC	NA	197	N
```

## How It Works

1. **Alignment matching**: Query and reference entries are matched on `ctype`, `dais_ref_id`, and `protein`. All DAIS-ribosome outputs from v2.1.0 and higher should be the same length if sharing `dais_ref_id`. Pairs with mismatched aligned sequence lengths are skipped with a warning printed to stdout.
2. **Codon translation**: Aligned sequences are split into codons and translated to amino acids using the standard genetic code. A trailing partial codon (if any) is handled separately and marked with `~`.
3. **Position adjustment**: Raw alignment positions are adjusted for insertions and deletions on both the query and reference side to provide actual nucleotide and amino acid positions with the query (`query_nt_position` and `query_aa_position`) and the reference (`ref_nt_position` and `ref_aa_position`).
4. **Minor variant lookup**: When a minor-variants file is supplied, minor alleles at matching positions are substituted into the query codon to compute an alternate codon/amino acid.


## Example Usage

```bash
# Full diff report against a reference
variants --query-dais-file query.tsv \
  --query-insertion-file query.ins \
  --query-deletion-file query.del \
  --ref-dais-file ref.tsv \
  --ref-insertion-file ref.ins \
  --ref-deletion-file ref.del \
  --variants-of-interest variants.tsv (optional)\
  --minor-variants minor_variants.csv (optional) \
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
  --minor-variants minor_variants.csv (optional) \
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

## Output

Output is written as delimited text (default comma-separated) either to a file (`--output-xsv`) or to stdout. Column sets vary by mode and by which optional inputs are supplied — see the mode descriptions above for details on which extra columns are appended.

## Example Outputs

The examples below show representative rows for each mode (with all optional inputs supplied), based on the column orders produced by the tool.

### `--all-diffs` output

Base columns, plus `variant_of_interest,position_of_interest` (if `--variants-of-interest` given) and minor-variant columns (if `--minor-variants` given):

```
query_name,ref_name,ctype,dais_reference,protein,aln_nt_position,ref_nt_position,query_nt_position,ref_nt,query_nt,position_in_codon,ref_codon,query_codon,aln_aa_position,ref_aa_position,query_aa_position,aa_mutation,variant_of_interest,position_of_interest,depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,minor_variant_codon,consensus_aa,minor_variant_aa
sample_1,ref_A,H3N2,ref_A_id,HA,148,148,148,A,G,1,ACC,GCC,50,50,50,T:50:A,true,true,1000,A,G,950,50,0.05,ACC,GCC,T,A
sample_1,ref_A,H3N2,ref_A_id,HA,225,225,222,T,C,3,AAT,AAC,75,75,74,N:74:N,false,true,,,,,,,,,,
```

- Every nucleotide difference between query and reference is reported, regardless of whether it falls in a position of interest.
- `variant_of_interest` is only `true` when the observed amino acid matches a listed mutation of interest at that position; `position_of_interest` is `true` whenever the position is *listed*, whether or not the amino acid matches.
- Rows with no matching minor-variant data have empty trailing fields (as in the second row above).

### `--all-diffs` columns

| Column | Description |
|---|---|
| `query_name` | Sample ID of the query sequence (`sample_id`) |
| `ref_name` | ID of the matched reference strain (`ref_id`) |
| `ctype` | Sequence/segment type shared by query and reference |
| `dais_reference` | DAIS reference ID used to match query and reference entries |
| `protein` | Protein name |
| `aln_nt_position` | Nucleotide position within the aligned sequence (1-indexed, unadjusted for indels) |
| `ref_nt_position` | Reference-side nucleotide position, adjusted for reference insertions/deletions |
| `query_nt_position` | Query-side nucleotide position, adjusted for query insertions/deletions |
| `ref_nt` | Reference nucleotide at this position in dais alignment space |
| `query_nt` | Query nucleotide at this position in dais alignment space |
| `position_in_codon` | Position within the codon (1, 2, or 3) where the difference occurs |
| `ref_codon` | Full reference codon containing this nucleotide in dais alignment space |
| `query_codon` | Full query codon containing this nucleotide in dais alignment space |
| `aln_aa_position` | Amino acid position within the aligned sequence (unadjusted for indels) |
| `ref_aa_position` | Reference-side amino acid position, adjusted for reference insertions/deletions |
| `query_aa_position` | Query-side amino acid position, adjusted for query insertions/deletions |
| `ref_aa_vs_query_aa` | Amino acid change in dais alignment space, formatted as `ref:position:query` (e.g. `T:50:A`) |
| `variant_of_interest`* | `true` if the observed amino acid matches a listed variant of interest at this position |
| `position_of_interest`* | `true` if this position is listed in the variant-of-interest file, regardless of which amino acid is observed |
| `depth`† | Sequencing depth at this position from the minor-variants file |
| `consensus_allele`† | Consensus (majority) nucleotide allele reported in the minor-variants file |
| `minority_allele`† | Minority (sub-consensus) nucleotide allele reported in the minor-variants file |
| `consensus_count`† | Read count supporting the consensus allele |
| `minority_count`† | Read count supporting the minority allele |
| `minority_frequency`† | Frequency of the minority allele (minority_count / depth) |
| `consensus_codon`† | Query codon as originally observed (before minority allele substitution) |
| `minor_variant_codon`† | Query codon with the minority allele substituted in at `position_in_codon` in dais alignment space|
| `consensus_aa`† | Amino acid translated from `consensus_codon` in dais alignment space |
| `minor_variant_aa`† | Amino acid translated from `minor_variant_codon` in dais alignment space |

\* Present only when `--variants-of-interest` is supplied.
† Present only when `--minor-variants` is supplied. If no minor variant matches this position, these fields are emitted empty.

### `--positions-of-interest` output

Base columns, plus minor-variant columns (if `--minor-variants` given):

```
query_name,ref_name,ctype,dais_reference,protein,aln_nt_position,ref_nt_position,query_nt_position,query_nt,ref_nt,position_in_codon,query_codon,ref_codon,aln_aa_position,ref_aa_position,query_aa_position,aa_mutation,variant_of_interest,depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,consensus_aa,minor_variant_codon,minor_variant_aa
sample_1,ref_A,H3N2,ref_A_id,HA,148,148,148,G,A,1,GCC,ACC,T:50:A,50,50,50,true,1000,A,G,950,50,0.05,ACC,T,GCC,A
```

- Only rows matching an entry in the mutations-of-interest file are emitted (unless `--all-positions` is set, in which case all listed positions are shown even where query and reference agree).
- This mode always reflects `variant_of_interest`; there's no separate `position_of_interest` column here since every row is already restricted to listed positions.

### `--positions-of-interest` columns

| Column | Description |
|---|---|
| `query_name` | Sample ID of the query sequence (`sample_id`) |
| `ref_name` | ID of the matched reference strain (`ref_id`) |
| `ctype` | Sequence/segment type shared by query and reference |
| `dais_reference` | DAIS reference ID used to match query and reference entries |
| `protein` | Protein/product name |
| `aln_nt_position` | Nucleotide position within the aligned sequence (1-indexed, unadjusted for indels) |
| `ref_nt_position` | Reference-side nucleotide position, adjusted for reference insertions/deletions |
| `query_nt_position` | Query-side nucleotide position, adjusted for query insertions/deletions |
| `query_nt` | Query nucleotide at this position in dais alignment space |
| `ref_nt` | Reference nucleotide at this position in dais alignment space |
| `position_in_codon` | Position within the codon (1, 2, or 3) for this nucleotide |
| `query_codon` | Full query codon containing this nucleotide in dais alignment space |
| `ref_codon` | Full reference codon containing this nucleotide in dais alignment space |
| `aln_aa_position` | Amino acid position within the aligned sequence (unadjusted for indels) |
| `ref_aa_position` | Reference-side amino acid position, adjusted for reference insertions/deletions |
| `query_aa_position` | Query-side amino acid position, adjusted for query insertions/deletions |
| `ref_aa_vs_query_aa` | Amino acid change in dais alignment space, formatted as `ref:position:query` (e.g. `T:50:A`) |
| `variant_of_interest` | `true` if the observed amino acid matches the listed mutation of interest at this position |
| `depth`† | Sequencing depth at this position from the minor-variants file |
| `consensus_allele`† | Consensus (majority) nucleotide allele reported in the minor-variants file |
| `minority_allele`† | Minority (sub-consensus) nucleotide allele reported in the minor-variants file |
| `consensus_count`† | Read count supporting the consensus allele |
| `minority_count`† | Read count supporting the minority allele |
| `minority_frequency`† | Frequency of the minority allele (minority_count / depth) |
| `consensus_codon`† | Query codon as originally observed (before minority allele substitution) |
| `consensus_aa`† | Amino acid translated from `consensus_codon` |
| `minor_variant_codon`† | Query codon with the minority allele substituted in at `position_in_codon` in dais alignment space|
| `minor_variant_aa`† | Amino acid translated from `minor_variant_codon` in dais alignment space|

† Present only when `--minor-variants` is supplied. If no minor variant matches this position, these fields are emitted empty.

### `--annotate-minor-variants` output

```
sample,reference,dais_reference,dais_ref_position,sample_position,depth,consensus_allele,minority_allele,consensus_count,minority_count,minority_frequency,consensus_codon,minor_variant_codon,consensus_aa,minor_variant_aa,major_aa_vs_minor_aa,run_id,instrument
sample_1,H3N2,ref_A_id,148,148,1000,A,G,950,50,0.05,ACC,GCC,T,A,T:148:A,run_2024_08,MiSeq
sample_2,H1N1,ref_B_id,,200,500,C,T,480,20,0.04,,,,,: :,run_2024_08,MiSeq
```

- Each row of the input minor-variants CSV is annotated with the codon and amino acid context derived from the matching query DAIS entry.
- If no matching DAIS entry or raw position mapping is found (as in the second example row), the codon/amino-acid fields are left blank and `major_aa_vs_minor_aa` shows empty values around the position.

### `--annotate-minor-variants` columns

| Column | Description |
|---|---|
| `sample` | Sample name from the minor-variants file |
| `reference` | Reference/ctype from the minor-variants file |
| `dais_reference` | DAIS reference ID of the matching query entry (longest `query_cds_aln` match on sample/ctype) |
| `dais_ref_position` | Raw (pre-indel-adjustment) alignment position in the query sequence that maps to `sample_position` |
| `sample_position` | Original sample position from the minor-variants file |
| `depth` | Sequencing depth at this position |
| `consensus_allele` | Consensus (majority) nucleotide allele |
| `minority_allele` | Minority (sub-consensus) nucleotide allele |
| `consensus_count` | Read count supporting the consensus allele |
| `minority_count` | Read count supporting the minority allele |
| `minority_frequency` | Frequency of the minority allele (minority_count / depth) |
| `consensus_codon` | Codon at `dais_ref_position` as observed in the query sequence |
| `minor_variant_codon` | Codon with the minority allele substituted in |
| `consensus_aa` | Amino acid translated from `consensus_codon` |
| `minor_variant_aa` | Amino acid translated from `minor_variant_codon` |
| `major_aa_vs_minor_aa` | Formatted comparison of consensus vs. minor amino acid, as `consensus_aa:dais_ref_position:minor_variant_aa` |
| `run_id` | Sequencing run ID from the minor-variants file |
| `instrument` | Sequencing instrument from the minor-variants file |

If no matching query DAIS entry is found, or no raw position maps to `sample_position`, the codon/amino-acid fields are left blank.