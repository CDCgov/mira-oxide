# cdcgov/mira-oxide: Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2025-12-15

- [Amanda Sullivan](https://github.com/mandysulli)
- [Sam Shepard](https://github.com/sammysheep)
- [Sam Wiley](https://github.com/samcwiley)
- [William Chettleburgh](https://github.com/willchet)

### `Added`
- [PR #40](https://github.com/CDCgov/mira-oxide/pull/40) - Added `prepare_mira_reports.rs` to proccesses and supporting files to src/io, src/utils and src/constants. Made to replace prepareIRMAjson.py, irma2pandas.py, dais2pandas.py and parquet_maker.py within MIRA-NF.
- [PR #52](https://github.com/CDCgov/mira-oxide/pull/52) - Added additional logic to `prepare_mira_reports.rs` to replace static_reports.py within MIRA-NF


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
