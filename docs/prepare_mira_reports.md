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
    (Optional) The name of the IRMA configuration that was used for running IRMA.

## How to Run
After cloning the mira-oxide repo, execute this command to create a mutations of interest table for the samples:

```bash
cargo run -- prepare-mira-reports -s <PATH>/samplesheet.csv -i ~<PATH_TO_MIRA_NF_OUTPUTS. -o <OUTDIR> -q <PATH>/qc_test.yaml -p <PLATFORM> -w <PATH>/MIRA-NF -r <RUN_ID> -v <VIRUS> -f (optional) -n (optional) -c <CONFIG> (optional)
```

**NOTE: This script expects you to have the DAIS_ribosome.seq file to be in the location that you are deploying the command for MIRA-NF compatibility**

### Files Outputs with no -f flag invoked

```
Starting data ingestion...
Finished ingesting data.
Writing Output Files...
Writing FASTA files
 -> FASTA written to ./test/mira_run_id_test_amended_consensus.fasta
 -> FASTA written to ./test/mira_run_id_test_failed_amended_consensus.fasta
 -> FASTA written to ./test/mira_run_id_test_amino_acid_consensus.fasta
 -> FASTA written to ./test/mira_run_id_test_failed_amino_acid_consensus.fasta
 **NEXTCLADE FASTA FILES**
Writing CSV files
 -> CSV written to ./test/mira_run_id_test_coverage.csv
 -> CSV written to ./test/mira_run_id_test_reads.csv
 -> CSV written to ./test/mira_run_id_test_all_alleles.csv
 -> CSV written to ./test/mira_run_id_test_indels.csv
 -> CSV written to ./test/mira_run_id_test_filtered_variants.csv
 -> CSV written to ./test/mira_run_id_test_aavars.csv
 -> CSV written to ./test/mira_run_id_test_summary.csv
 -> CSV written to ./test/mira_run_id_test_amended_consensus.csv
 -> CSV written to ./test/mira_run_id_test_amino_acid_consensus.csv
 -> CSV written to ./test/mira_run_id_test_irma_config.csv
Writing JSON files
 -> JSON written to ./test/coverage.json
 -> JSON written to ./test/reads.json
 -> JSON written to ./test/vtype.json
 -> JSON written to ./test/alleles.json
 -> JSON written to ./test/indels.json
 -> JSON written to ./test/dais_vars.json
 -> JSON written to ./test/qc_statement.json
 -> JSON written to ./test/irma_summary.json
 -> JSON written to ./test/pass_fail_qc.json
 -> JSON written to ./test/nt_sequences.json
Building coverage plots for 2 samples as JSONs
  -> saved ./test/coveragefig_s3_linear.json
  -> saved ./test/coveragefig_s1_linear.json
Building read sankey plots as JSON
  -> read sankey plot json saved to ./test/readsfig_s2.json
  -> read sankey plot json saved to ./test/readsfig_s1.json
  -> read sankey plot json saved to ./test/readsfig_s3.json
Building coverage heatmap as JSON
  -> coverage heatmap json saved to ./test/heatmap.json
Building pass_fail_heatmap as JSON
  -> pass_fail heatmap json saved to ./test/pass_fail_heatmap.json
Building barcode distribution pie figure as JSON
  -> barcode distribution pie figure saved to ./test/barcode_distribution.json
Building static HTML file
  -> static HTML saved to "./test/mira_run_id_test_summary.html"
```


### Files Outputs when -f flag invoked

```
Starting data ingestion...
Finished ingesting data.
Writing Output Files...
Writing FASTA files
 -> FASTA written to ./test/mira_run_id_test_amended_consensus.fasta
 -> FASTA written to ./test/mira_run_id_test_failed_amended_consensus.fasta
 -> FASTA written to ./test/mira_run_id_test_amino_acid_consensus.fasta
 -> FASTA written to ./test/mira_run_id_test_failed_amino_acid_consensus.fasta
  **NEXTCLADE FASTA FILES**
Writing CSV files
 -> CSV written to ./test/mira_run_id_test_coverage.csv
 -> CSV written to ./test/mira_run_id_test_reads.csv
 -> CSV written to ./test/mira_run_id_test_all_alleles.csv
 -> CSV written to ./test/mira_run_id_test_indels.csv
 -> CSV written to ./test/mira_run_id_test_filtered_variants.csv
 -> CSV written to ./test/mira_run_id_test_aavars.csv
 -> CSV written to ./test/mira_run_id_test_summary.csv
 -> CSV written to ./test/mira_run_id_test_amended_consensus.csv
 -> CSV written to ./test/mira_run_id_test_amino_acid_consensus.csv
 -> CSV written to ./test/mira_run_id_test_irma_config.csv
Writing JSON files
 -> JSON written to ./test/coverage.json
 -> JSON written to ./test/reads.json
 -> JSON written to ./test/vtype.json
 -> JSON written to ./test/alleles.json
 -> JSON written to ./test/indels.json
 -> JSON written to ./test/dais_vars.json
 -> JSON written to ./test/qc_statement.json
 -> JSON written to ./test/irma_summary.json
 -> JSON written to ./test/pass_fail_qc.json
 -> JSON written to ./test/nt_sequences.json
Writing PARQUET files
 -> PARQUET written to ./test/mira_run_id_test_coverage.parq
 -> PARQUET written to ./test/mira_run_id_test_reads.parq
 -> PARQUET written to ./test/mira_run_id_test_all_alleles.parq
 -> PARQUET written to ./test/mira_run_id_test_indels.parq
 -> PARQUET written to ./test/mira_run_id_test_minor_variants.parq
 -> PARQUET written to ./test/mira_run_id_test_summary.parq
 -> PARQUET written to ./test/mira_run_id_test_amended_consensus.parq
 -> PARQUET written to ./test/mira_run_id_test_amino_acid_consensus.parq
 -> PARQUET written to ./test/mira_run_id_test_irma_config.parq
 -> PARQUET written to ./test/mira_run_id_test_samplesheet.parq
Building coverage plots for 2 samples as JSONs
  -> saved ./test/coveragefig_s3_linear.json
  -> saved ./test/coveragefig_s1_linear.json
Building read sankey plots as JSON
  -> read sankey plot json saved to ./test/readsfig_s2.json
  -> read sankey plot json saved to ./test/readsfig_s1.json
  -> read sankey plot json saved to ./test/readsfig_s3.json
Building coverage heatmap as JSON
  -> coverage heatmap json saved to ./test/heatmap.json
Building pass_fail_heatmap as JSON
  -> pass_fail heatmap json saved to ./test/pass_fail_heatmap.json
Building barcode distribution pie figure as JSON
  -> barcode distribution pie figure saved to ./test/barcode_distribution.json
Building static HTML file
  -> static HTML saved to "./test/mira_run_id_test_summary.html"
```

### Potential FASTA files created by the nextclade flag
```
nextclade_<runid>_influenza-a-h3n2-ha.fasta
nextclade_<runid>_influenza-a-h1n1pdm-ha.fasta
nextclade_<runid>_influenza-b-victoria-ha.fasta
nextclade_<runid>_influenza-a-h1n1pdm-na.fasta
nextclade_<runid>_influenza-a-h3n2-na.fasta
nextclade_<runid>_influenza-b-victoria-na.fasta
nextclade_<runid>_rsv-a.fasta
nextclade_<runid>_rsv-b.fasta
nextclade_<runid>_sars-cov-2.fasta
```

## Notes
This ingest error can be ignored (will occur with IRMA veresions prior to v1.3.1):
```
Warning: Failed to deserialize record: CSV error: record 19 (line: 20, byte: 769): found record with 2 fields, but the previous record has 3 fields
Warning: Failed to deserialize record: CSV error: record 47 (line: 48, byte: 1960): found record with 2 fields, but the previous record has 3 fields
Warning: Failed to deserialize record: CSV error: record 51 (line: 52, byte: 2178): found record with 1 fields, but the previous record has 3 fields
Warning: Failed to deserialize record: CSV error: record 61 (line: 62, byte: 2619): found record with 2 fields, but the previous record has 3 fields
Warning: Failed to deserialize record: CSV error: record 64 (line: 65, byte: 2739): found record with 1 fields, but the previous record has 3 fields
```

**creating the samplesheet.parq is on my to do list**

## Finding your way to the bugs
### Main Process
`src/processes/prepare_mira_reports.rs`
This the top layer where a lot of the decision logic exists. Divided into 3 sections: data ingest, data processing and writing files out.

### Ingesting Data
`src/io/data_ingest.rs`
contains data struct definitions, file collection and parsing to structs, adds metadata to ingested data when needed (i.e. sample_id) and helper functions to make these things happen. 

### Data Processing
`src/utils/data_processing.rs`
contains data struct definitions to transformed data, performs data transformation, quality filtering and sequence processing. Also contains the helper function to make this happen

### Writing to Files
`src/io/write_fasta_files.rs`
`src/io/write_csv_files.rs `
`src/io/write_json_files.rs `
`src/io/write_parquet_files.rs`
Each script writes out the file type indicated

### Figures to JSONs
`src/io/coverage_json_per_sample.rs`
`src/io/coverage_to_heatmap.rs `
`src/io/create_passfail_heatmap.rs`
`src/io/reads_to_piechart.rs`
`src/io/reads_to_sankey_json.rs`
Each script writes out the file type indicated

### Creating the Static HTML Files
`src/io/create_statichtml.rs`
Uses json's created above to create a static HTML