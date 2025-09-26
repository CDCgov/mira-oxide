# Prepare MIRA Reports Package

Still under construction

This script is a comprehensive data aggregation and processing tool designed to streamline the analysis of outputs from the IRMA (Iterative Refinement Meta-Assembler) and DAIS-ribosome pipelines. It begins by ingesting various input files, including samplesheets, QC configuration files, and IRMA/DAIS outputs, to extract key data such as coverage, reads, alleles, indels, and sequence information. The script processes this data to compute metrics like coverage percentages, median coverage, and amino acid variants, while also extracting subtype information for viruses such as influenza, RSV, and SARS-CoV-2. Quality control checks are applied to determine pass/fail statuses for samples and sequences based on predefined criteria. The processed data is then organized into structured outputs, including nucleotide and amino acid sequences categorized as passing or failing QC. Finally, the script generates multiple output formats, including FASTA, CSV, JSON, and optionally Parquet files, to facilitate downstream analysis and reporting. This tool provides a robust and automated workflow for aggregating, transforming, and summarizing bioinformatics data, enabling efficient and standardized reporting of viral sequencing results.

## Commands
-i, --irma-path <PathBuf>
    The file path to the samples folders with IRMA outputs.

-o, --output-path <PathBuf>
    The file path where the `prepare_mira_report` outputs will be saved.

-s, --samplesheet <PathBuf>
    The file path to the input samplesheet.

-q, --qc-yaml <PathBuf>
    The file path to the QC YAML file.

-p, --platform <String>
    The platform used to generate the data (e.g., illumina or ont).

-v, --virus <String>
    The virus the data was generated from (e.g., flu, sc2-wgs, sc2-spike or rsv).

-r, --runid <String>
   The run id. Used to create custom file names associated with run_id.

-w, --workdir-path <PathBuf>
    The file path to the user's cloned MIRA-NF repo.

-f, --parq
    (Optional) A flag to indicate whether to create Parquet files.

-c, --irma-config <String> (default: "default-config")
    (Optional) The IRMA configuration used for processing.

## How to Run
After cloning the mira-oxide repo, execute this command to create a mutations of interest table for the samples:

```bash
cargo run -- prepare-mira-reports -s <PATH>/samplesheet.csv -i ~<PATH_TO_MIRA_NF_OUTPUTS. -o <OUTDIR> -q <PATH>/qc_test.yaml -p <PLATFORM> -w <PATH>/MIRA-NF -r <RUN_ID> -v <VIRUS> 
```

### Files Outputs

```
FASTA written to ./test/flu_run_id_test_amended_consensus_summary.fasta
FASTA written to ./test/flu_run_id_test_failed_amended_consensus_summary.fasta
FASTA written to ./test/flu_run_id_test_amino_acid_consensus_summary.fasta
FASTA written to ./test/flu_run_id_test_failed_amino_acid_consensus_summary.fasta
CSV written to ./test/flu_run_id_test_coverage.csv
CSV written to ./test/flu_run_id_test_reads.csv
CSV written to ./test/flu_run_id_test_alleles.csv
CSV written to ./test/flu_run_id_test_indels.csv
CSV written to ./test/flu_run_id_test_variants.csv
CSV written to ./test/flu_run_id_test_summary.csv
CSV written to ./test/flu_run_id_test_amended_consensus.csv
CSV written to ./test/flu_run_id_test_amino_acid_consensus.csv
CSV written to ./test/flu_run_id_test_irma_config.csv
Split-oriented JSON written to ./test/coverage.json
Split-oriented JSON written to ./test/reads.json
Split-oriented JSON written to ./test/vtype.json
Split-oriented JSON written to ./test/alleles.json
Split-oriented JSON written to ./test/indels.json
Data written to ref_data.json
Split-oriented JSON written to ./test/dais_vars.json
JSON written to ./test/qc_statement.json
Split-oriented JSON written to ./test/irma_summary.json
Split-oriented JSON written to ./test/pass_fail_qc.json
Split-oriented JSON written to ./test/nt_sequences.json
PARQUET written to ./test/flu_run_id_test_coverage.parq
PARQUET written to ./test/flu_run_id_test_reads.parq
PARQUET written to ./test/flu_run_id_test_alleles.parq
PARQUET written to ./test/flu_run_id_test_indels.parq
PARQUET written to ./test/flu_run_id_test_variants.parq
PARQUET written to ./test/flu_run_id_test_summary.parq
PARQUET written to ./test/flu_run_id_test_amended_consensus.parq
PARQUET written to ./test/flu_run_id_test_amino_acid_consensus.parq
PARQUET written to ./test/flu_run_id_test_irma_config.parq
```

