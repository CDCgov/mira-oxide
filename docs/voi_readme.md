# VOI (Reference-vs-Query Positions of Interest) Package

The `voi` package compares "query" samples against a set of "reference" strains using
DAIS-ribosome protein-level output, and reports nucleotide / amino-acid / codon differences,
flagging those that fall on user-supplied positions of interest.

Both the reference and query inputs are DAIS-ribosome output. Because the DAIS `cds_aln`
column is already expressed in reference-coordinate space (deletions appear as `-`, padding
as `.`, and insertions are stored separately in the `.ins` file), the comparison is performed
position-by-position in that shared coordinate space. Insertions from the `.ins` files are
spliced back in to recover the true, indel-adjusted isolate positions reported in the output.

## Inputs

### 1. Reference and query DAIS-ribosome output (protein-level)

Each side (reference and query) is a set of three tab-delimited DAIS files. Genome-level
(`.gen_*`) files are not used.

`*.seq.txt` (SEQUENCE_OUTPUT) — 14 columns:

```text
query_id  ctype  reference_id  protein  aa_id  aa_seq  aa_aln  cds_id  has_insertion  has_shift_indel  cds_seq  cds_aln  query_coordinates  cds_coordinates
```

`*.ins.txt` (INSERTION_OUTPUT) — 9 columns:

```text
query_id  ctype  reference_id  protein  upstream_aa_pos  inserted_nt  inserted_aa  upstream_nt_pos  codon_shift
```

`*.del.txt` (DELETION_OUTPUT) — 13 columns:

```text
query_id  ctype  reference_id  protein  aa_id  del_aa_start  del_aa_end  del_aa_len  in_frame  cds_id  del_cds_start  del_cds_end  del_cds_len
```

Deletions are already encoded as gaps in `cds_aln`, so the `.del` files are parsed for
format validation but are not required to reconstruct sequences.

### 2. Positions-of-interest file (tab-delimited, no header)

```text
ref-name  segment  amino-acid-position  amino-acid-of-interest
```

- `ref-name` matches the sample name in column 1 (`query_id`) of the reference DAIS files.
- `segment` is the second underscore-delimited field of `ctype` (e.g. `A_NA_N1` -> `NA`,
  `A_HA_H1` -> `HA`, `A_MP` -> `MP`).
- `amino-acid-position` is the reference strain's own (indel-adjusted) amino-acid position,
  i.e. it is matched against the `ref-aa-position` column of the output.
- The named reference record supplies a `reference_id` + protein; query records that align to the
  same `reference_id` + protein are compared against it.

Example (using a real reference sample name from column 1 of the reference `.seq` file):

```text
CY009630	NA	275	Y
CY009630	NA	119	V
CY009630	NA	294	S
```

## Usage

```bash
cargo run -p mira-oxide -- voi \
  --ref-seq   <ref.seq.txt>   --ref-ins   <ref.ins.txt>   --ref-del   <ref.del.txt> \
  --query-seq <qry.seq.txt>   --query-ins <qry.ins.txt>   --query-del <qry.del.txt> \
  --positions <positions_of_interest.txt> \
  [--minor-variants <minor_variants.csv>] \
  [--filter all-diffs|poi|poi-match]  [-o <output>]  [-d <delimiter>]
```

- `--minor-variants` (optional): a MIRA minor-variant CSV (`sample, reference, sample_position,
  depth, consensus_allele, minority_allele, consensus_count, minority_count, minority_frequency,
  ...`). When supplied, six annotation columns are appended to the output. Rows are joined by
  sample (the CSV `sample` matches the VOI `query-name` with its trailing `_<segment>` stripped),
  `reference` (the full DAIS `ctype`), and `sample_position` (matched to `query-nt-position`).

- `--filter` (default `all-diffs`):
  - `all-diffs`: every nucleotide/amino-acid/codon difference.
  - `poi`: only differences whose reference amino-acid position is a position of interest.
  - `poi-match`: only positions-of-interest differences where the query amino acid equals the
    `amino-acid-of-interest` for that entry.
- `-d`/`--delimiter`: single-character output delimiter (default TAB).
- `-o`/`--output`: output file (defaults to stdout).

## Output

Delimited table with header:

```text
query-name  segment  protein  ref-name  ref-nt-position  query-nt-position  ref-nt  query-nt  ref-aa-position  query-aa-position  ref-codon  query-codon  codon-position  ref-aa  query-aa  position-of-interest
```

- `ref-nt-position` / `query-nt-position`: indel-adjusted isolate nucleotide positions
  (a value of `0` indicates a deletion or padding on that side).
- `ref-aa-position` / `query-aa-position`: indel-adjusted isolate amino-acid positions.
- `ref-codon` / `query-codon`: the reference-coordinate codon (may contain `-` for deletions).
- `codon-position`: the position (1, 2, or 3) of the differing nucleotide within its codon.
- `ref-aa` / `query-aa`: the amino acid translated from the respective codon (`X` for
  codons containing gaps/ambiguities).
- `position-of-interest`: `true` when the reference amino-acid position (`ref-aa-position`)
  is listed for that `ref-name`/segment in the positions file.

When `--minor-variants` is supplied, six more columns are appended (blank when no minor-variant
call matches that query position):

- `minor-nt`: the CSV `minority_allele`.
- `minor-codon`: the `query-codon` with the minor allele substituted at its codon position.
- `minor-aa`: the amino acid translated from `minor-codon`.
- `freq-minor`: the CSV `minority_frequency`.
- `freq-major`: `consensus_count` / `depth`.
- `total-depth`: the CSV `depth`.

Example (deletion carrier `MW170131` shows the query nt position offset by an upstream deletion):

```text
query-name  segment  protein  ref-name  ref-nt-position  query-nt-position  ref-nt  query-nt  ref-aa-position  query-aa-position  ref-codon  query-codon  codon-position  ref-aa  query-aa  position-of-interest
CY019973    NA       NA       CY009630  825              825                C       T         275              275                CAC        CAT          3               H       H         true
MW170131    NA       NA       CY009630  357              354                A       G         119              118                GAA        GAG          3               E       E         true
```
