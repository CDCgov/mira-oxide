# Variants

Tool for observing codon and amino acid differences between query samples (dais-ribosome
output) and their matching references, optionally filtered to positions of interest and/or
annotated with minor variant calls.

## Overview

The tool operates in three mutually distinct output modes, chosen by which flags you pass:

1. **All-diffs mode** (`-x/--all-diffs`) — reports every nucleotide difference between each
   query and its matching reference, across all codons, without restricting to specific
   positions. Positions-of-interest and minor-variants annotation are both optional add-ons
   in this mode.
2. **Positions-of-interest mode** (`-p/--positions-of-interest`) — reports codon/amino-acid
   differences only at protein positions listed in a positions-of-interest file, flagging
   whether the observed amino acid matches a specific variant of concern.
3. **Minor-variants-only mode** (`-m/--minor-variants-file` alone, no `-p` or `-x`) — annotates
   an input minor-variants CSV with codon/amino-acid context derived from the query dais data,
   without needing any reference data at all.

At least one of `-p`, `-x`, or `-m` must be provided.

## Inputs

| File | Flag | Format | Required when |
|---|---|---|---|
| Query dais-ribosome file | `-q/--query-dais-file` | headerless TSV | always |
| Reference dais-ribosome file | `-r/--ref-dais-file` | headerless TSV | `-p` or `-x` |
| Query insertion file | `-i/--query-insertion-file` | headerless TSV | always |
| Query deletion file | `-d/--query-deletion-file` | headerless TSV | always |
| Reference insertion file | `-j/--ref-insertion-file` | headerless TSV | `-p` or `-x` |
| Reference deletion file | `-e/--ref-deletion-file` | headerless TSV | `-p` or `-x` |
| Positions-of-interest file | `-p/--positions-of-interest` | headerless TSV | optional; enables POI mode or POI annotation in all-diffs mode |
| Minor variants file | `-m/--minor-variants-file` | CSV with header | optional; enables minor-variant annotation, or triggers minor-variants-only mode if `-p`/`-x` absent |

### Query dais-ribosome file columns (headerless TSV)

```
sample_id, ctype, dais_ref_id, protein, nt_hash, query_aa_seq, query_aa_aln_seq,
cds_id, insertion, inert_shift, query_cds_seq, query_cds_aln,
query_nt_coordinates, cds_nt_coordinates
```

- `dais_ref_id` here is the reference strain's short name (e.g. `BRISBANE60`), matching the
  reference dais file's `dais_ref_id` column — this is the join key between query and ref rows.
- `query_aa_aln_seq` / `query_cds_aln` may be padded with `.` and a leading `~` to represent
  partial/incomplete codons at sequence boundaries.

### Reference dais-ribosome file columns (headerless TSV)

```
ref_id, ctype, dais_ref_id, protein, nt_hash, ref_aa_seq, ref_aa_aln_seq,
cds_id, insertion, inert_shift, ref_cds_seq, ref_cds_aln,
ref_nt_coordinates, ref_cds_nt_coordinates
```

- `ref_id` is a compound strain identifier (e.g. `BRISBANE60_B_HA_295337I3`) used to match
  insertion/deletion file rows on the reference side.
- `dais_ref_id` here is the short strain name (e.g. `BRISBANE60`).

Query and reference rows are paired for comparison when `ctype`, `dais_ref_id`, and `protein`
all match, and their aligned CDS sequences (`query_cds_aln` / `ref_cds_aln`) are the same
length. Mismatched-length pairs are skipped with a warning.

### Insertion file columns (headerless TSV)

```
query_id, ctype, reference_id, product_name, upstream_aa_pos, inserted_nt,
inserted_aa, upstream_nt_pos, codon_shift
```

- For the **query** insertion file, `query_id` is the sample ID (e.g. `sample_1_4`).
- For the **reference** insertion file, `query_id` is the compound reference strain ID
  (e.g. `BRISBANE60_B_NS_115367`) — despite the field name, it plays the same structural role.

### Deletion file columns (headerless TSV)

```
query_id, ctype, reference_id, product_name, variant_hash, del_aa_start, del_aa_end,
del_aa_len, in_frame, cds_id, del_cds_start, del_cds_end, del_cds_len
```

Same `query_id` convention as the insertion file (sample ID for query-side, compound strain
ID for reference-side).

### Positions-of-interest file columns (headerless TSV)

```
subtype, protein, aa_position, aa, description
```

- `subtype` may be a specific subtype/lineage code (matched against the dais reference's
  resolved subtype) or the literal `all` (case-insensitive) to apply to every subtype.
- A row matches a given codon when `protein` and `aa_position` match, and (`subtype` resolves
  to the same reference as the sample's `dais_ref_id`, or `subtype` is `all`).
- `aa` is the specific amino acid that, if observed at that position, marks the row as a
  variant of interest.

### Minor variants file columns (CSV with header)

```
sample, reference, sample_position, depth, consensus_allele, minority_allele,
consensus_count, minority_count, minority_frequency, run_id, instrument
```

- `sample` is matched against query dais `sample_id` by **substring containment** (e.g.
  `sample_1` matches `sample_id` `sample_1_4`), and `reference` is matched against `ctype`.
- `sample_position` is expected to be in the same coordinate space as the tool's computed
  `query_nt_position` (indel-adjusted), for positions-of-interest/all-diffs modes. In
  minor-variants-only mode it is instead resolved back to a raw alignment position.

## Modes and outputs

### 1. All-diffs mode (`-x/--all-diffs`)

Requires `-r`, `-j`, `-e` in addition to the always-required files. `-p` is **optional** in
this mode.

Walks every codon of every matching query/reference pair (regardless of positions of interest)
and emits one row per nucleotide position where the query and reference differ.
`-a/--all-positions` has no effect in this mode.

If `-p` is supplied, two extra columns are included:
- `variant_of_interest` — true if the codon's observed amino acid matches a positions-of-interest
  entry's flagged `aa` (same semantics as in positions-of-interest mode).
- `position_of_interest` — true if the codon's `(protein, aa_position)` matches **any**
  positions-of-interest entry, regardless of which amino acid is listed there.

If `-m` is also supplied, the same minor-variant columns as positions-of-interest mode are
appended.

**Output columns:**
```
query_name, ref_name, ctype, dais_reference, protein, aln_nt_position, ref_nt_position,
query_nt_position, ref_nt, query_nt, position_in_codon, ref_codon, query_codon,
aln_aa_position, ref_aa_position, query_aa_position, aa_mutation
[, variant_of_interest, position_of_interest]
[, depth, consensus_allele, minority_allele, consensus_count, minority_count,
   minority_frequency, consensus_codon, minor_variant_codon, consensus_aa, minor_variant_aa]
```

**Example (diffs only, no positions-of-interest or minor variants):**
```sh
variants \
  -q query.dais -r ref.dais \
  -i query.ins -d query.del \
  -j ref.ins -e ref.del \
  -x \
  -o output.csv
```

**Example (diffs with positions-of-interest and minor-variant annotation):**
```sh
variants \
  -q query.dais -r ref.dais \
  -i query.ins -d query.del \
  -j ref.ins -e ref.del \
  -x \
  -p positions_of_interest.tsv \
  -m minor_variants.csv \
  -o output.csv
```

### 2. Positions-of-interest mode (`-p`)

Requires `-r`, `-j`, `-e` in addition to the always-required files.

Emits one row per nucleotide position that falls within a codon matching a positions-of-interest
entry — by default, only rows where the query and reference nucleotides actually differ. Pass
`-a/--all-positions` to emit every nucleotide in a matching codon regardless of whether it
differs.

If `-m` is also supplied, minor-variant columns are appended, with one row emitted per matching
minor variant when more than one exists at the same position (rows are otherwise identical
except for the minor-variant columns).

**Output columns:**
```
query_name, ref_name, ctype, dais_reference, protein, aln_nt_position, ref_nt_position,
query_nt_position, ref_nt, query_nt, position_in_codon, ref_codon, query_codon,
aln_aa_position, ref_aa_position, query_aa_position, aa_mutation, variant_of_interest
[, depth, consensus_allele, minority_allele, consensus_count, minority_count,
   minority_frequency, consensus_codon, minor_variant_codon, consensus_aa, minor_variant_aa]
```
(The bracketed columns only appear when `-m` is used.)

**Example:**
```sh
variants \
  -q query.dais \
  -r ref.dais \
  -i query.ins \
  -d query.del \
  -j ref.ins \
  -e ref.del \
  -p positions_of_interest.tsv \
  -o output.csv
```

With minor variants included:
```sh
variants \
  -q query.dais -r ref.dais \
  -i query.ins -d query.del \
  -j ref.ins -e ref.del \
  -p positions_of_interest.tsv \
  -m minor_variants.csv \
  -o output.csv
```

Add `-a` to emit every position in every flagged codon, not just differences:
```sh
variants \
  -q query.dais -r ref.dais \
  -i query.ins -d query.del \
  -j ref.ins -e ref.del \
  -p positions_of_interest.tsv \
  -a \
  -o output.csv
```

### 3. Minor-variants-only mode (`-m` alone)

Used when neither `-p` nor `-x` is given. Only the always-required files plus `-m` are needed
— no reference dais/insertion/deletion files.

Annotates each row of the input minor-variants CSV with the matching query dais context: the
raw (pre-indel-adjustment) alignment position, the consensus codon/amino acid at that position,
and the minor-variant codon/amino acid produced by substituting in the minority allele.

When multiple query dais rows match a minor variant's `sample`/`reference` (e.g. HA-signal, HA,
and HA1 all sharing the same `sample_id`/`ctype`), the row with the longest `query_cds_aln` is
preferred, since shorter fragments can't contain larger `sample_position` values.

**Output columns:**
```
sample, reference, dais_reference, dais_ref_position, sample_position, depth,
consensus_allele, minority_allele, consensus_count, minority_count, minority_frequency,
consensus_codon, minor_variant_codon, consensus_aa, minor_variant_aa,
major_aa_v_minor_aa, run_id, instrument
```

- `dais_reference` is the matched query dais row's `dais_ref_id`.
- `major_aa_v_minor_aa` is `consensus_aa:dais_ref_position:minor_variant_aa` concatenated
  (e.g. `Q:598:K`).
- Fields derived from a lookup are left empty when no matching dais row is found, or when the
  minor variant's position doesn't resolve to a valid raw alignment position.

**Example:**
```sh
variants \
  -q query.dais \
  -i query.ins \
  -d query.del \
  -m minor_variants.csv \
  -o output.csv
```

## Other options

| Flag | Description |
|---|---|
| `-o/--output-xsv <path>` | Write output to a file instead of stdout. |
| `-s/--output-delimiter <char>` | Field delimiter for output (default `,`). |
| `-a/--all-positions` | Positions-of-interest mode only (`-p` without `-x`): emit every position in a flagged codon, not just differences. No effect in all-diffs mode or minor-variants-only mode. |

## Notes

- All coordinate values in the codon-diff outputs (`aln_nt_position`, `ref_nt_position`,
  `query_nt_position`, `aln_aa_position`, `ref_aa_position`, `query_aa_position`) are
  1-indexed.
- `aln_nt_position`/`aln_aa_position` refer to the position within the aligned CDS sequence
  (before any indel adjustment); `query_nt_position`/`query_aa_position` and
  `ref_nt_position`/`ref_aa_position` are adjusted for insertions and deletions on the
  respective side.
- A trailing partial codon (marked with `~` in the aligned sequences) is handled separately
  from the main reading frame and is included in all modes' output using the same column
  layout as regular codons.