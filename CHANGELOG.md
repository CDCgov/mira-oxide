# cdcgov/mira-oxide: Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.0] - 2025-02-12

- [Amanda Sullivan](https://github.com/mandysulli)

### `Added`
- [PR #64](https://github.com/CDCgov/mira-oxide/pull/64) - Added create_nextflow_samplesheet.rs to replace create_nextflow_samplesheet_i.py and create_nextflow_samplesheet+o.py in MIRA-NF. Takes the input samplesheet and fastq files and puts them into the nextflow sampleseet format.
- [PR #64](https://github.com/CDCgov/mira-oxide/pull/64) - Added added functionality to create_nextflow_samplesheet.rs that tells user when fastq files are missing or empty
- [PR #65](https://github.com/CDCgov/mira-oxide/pull/65) - replaced alleles.json with minor_variants.json - may break MIRA GUI

### `Fixed`
- [PR #64](https://github.com/CDCgov/mira-oxide/pull/64) - Zoe alignment syntax to be compatible with update
- [PR #65](https://github.com/CDCgov/mira-oxide/pull/65) - reading in the allAlleles.txt files for the all_alleles.parq now - may break schemas

### `Deprecated`
- [PR #65](https://github.com/CDCgov/mira-oxide/pull/65) - No longer making alleles.json or all_alleles.csv
- [PR #65](https://github.com/CDCgov/mira-oxide/pull/65) - minor_variants.csv and minor_variants.parq no longer filtered to frequency of 0.05
- [PR #65](https://github.com/CDCgov/mira-oxide/pull/65) - the minor_indel_count column has been removed from summary.csv, summary.json and summary.parq - may break schemas

## [1.3.2] - 2025-02-02

- [Amanda Sullivan](https://github.com/mandysulli)

### `Fixed`
- [PR #63](https://github.com/CDCgov/mira-oxide/pull/63) - Fix low median coverage flag call and made BYAM subtype call stricter (need 100% of HA segment now).


## [1.3.1] - 2025-01-13

- [Amanda Sullivan](https://github.com/mandysulli)

### `Fixed`
- [PR #62](https://github.com/CDCgov/mira-oxide/pull/62) - Fix compatibility with MIRA-NF.

## [1.3.0] - 2025-01-13

- [Amanda Sullivan](https://github.com/mandysulli)

### `Added`
- [PR #60](https://github.com/CDCgov/mira-oxide/pull/60) - Added divide_nt_into_nextclade_vec and write_out_nextclade_fasta_files funcitons to write out nextclade fastsa files that are divided by subtype( and segment for flu).
- [PR #60](https://github.com/CDCgov/mira-oxide/pull/60) - Added summary_report_update function to add the nextclade clade results back into the summary.csv and sumary.parq files.

## [1.2.3] - 2025-01-05

- [Amanda Sullivan](https://github.com/mandysulli)
- [Kristine Lacek](https://github.com/kristinelacek)

### `Fixed`
- [PR #58](https://github.com/CDCgov/mira-oxide/pull/58) - Add assembly_time coumn to the run_info.parq

## [1.2.2] - 2025-12-23

- [Amanda Sullivan](https://github.com/mandysulli)
- [Kristine Lacek](https://github.com/kristinelacek)

### `Fixed`
- [PR #57](https://github.com/CDCgov/mira-oxide/pull/57) - Fix samplesheet.parq schema fix for handling Illumina.

## [1.2.1] - 2025-12-18

- [Amanda Sullivan](https://github.com/mandysulli)

### `Fixed`
- [PR #55](https://github.com/CDCgov/mira-oxide/pull/55) - Fix filepath, subtype and spike coverage logic in prepare-mira-reports subrocess to be compatible with MIRA-MF

## [1.2.0] - 2025-12-15

- [Amanda Sullivan](https://github.com/mandysulli)
- [Sam Shepard](https://github.com/sammysheep)
- [Sam Wiley](https://github.com/samcwiley)
- [William Chettleburgh](https://github.com/willchet)

### `Added`
- [PR #40](https://github.com/CDCgov/mira-oxide/pull/40) - Added `prepare_mira_reports.rs` to proccesses and supporting files to src/io, src/utils and src/constants. Made to replace prepareIRMAjson.py, irma2pandas.py, dais2pandas.py and parquet_maker.py within MIRA-NF.
- [PR #52](https://github.com/CDCgov/mira-oxide/pull/52) - Added additional logic to `prepare_mira_reports.rs` to replace static_reports.py within MIRA-NF

### `Fixed`
- [PR #53](https://github.com/CDCgov/mira-oxide/pull/53) - Fix github trigger to run upon tagging. Not merging from dev.

## [1.1.3] - 2025-11-26

- [Sam Wiley](https://github.com/samcwiley)

### `Added`

- [PR #50](https://github.com/CDCgov/mira-oxide/pull/50) - Added `check_mira_versions.rs` to proccesses. Made to replace checkmiraversion.py within MIRA-NF

## [1.1.2] - 2025-09-19

- [Amanda Sullivan](https://github.com/mandysulli)

### `Added`

- [PR #44](https://github.com/CDCgov/mira-oxide/pull/44) - Added filtering to a single subtype for the variants_of_interest outputs if the virus flu (as "INFLUENZA") is passed to the program.
- [PR #46](https://github.com/CDCgov/mira-oxide/pull/46) - Added filtering to a single subtype for the positions_of_interest outputs if the virus flu (as "INFLUENZA") is passed to the program.

## [1.1.1] - 2025-09-11

- [Sam Wiley](https://github.com/samcwiley)

### `Fixed`

- [PR #38](https://github.com/CDCgov/mira-oxide/pull/38) - Fixed issue with `find_chemistry.rs` not being able to handle compressed files

### `Added`

- [PR #38](https://github.com/CDCgov/mira-oxide/pull/38) - Adds `utils/read_fastq.rs` with functionality for handling gzipped fastq files

<!-- Versions -->

[1.1.1]: https://github.com/CDCgov/mira-oxide/compare/f824940...v1.1.1
