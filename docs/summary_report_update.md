# MIRA SUmmary Report Update Package

This code defines a command-line tool to update MIRA-generated summary files (summary.csv and optionally .parq) with clade information from Nextclade outputs for viral sequencing data.

## Commands
-i, --irma-path <PathBuf>
    The file path to the samples folders with nextclade outputs.

-o, --output-path <PathBuf>
    The file path where the `summary_report_updates` outputs will be saved.

-s, --samplesheet <PathBuf>
    The file path to the input summary csv.

-v, --virus <String>
    The virus the data was generated from (e.g., flu, sc2-wgs, sc2-spike or rsv).

-r, --runid <String>
   The run id. Used to create custom file names associated with run_id.

-f, --parq
    (Optional) A flag to indicate whether to create Parquet files.

## How to Run
After cloning the mira-oxide repo, execute this command to create a mutations of interest table for the samples:

```bash
cargo run -- summary-report-update -s <PATH>/summary.csv -i ~<PATH_TO_NEXTCLADE_TSV_FILES> -o <OUTDIR> -w <PATH>/MIRA-NF -r <RUN_ID> -v <VIRUS> -f (optional)
```

### Files Outputs
### Where parquet files only generated when -f flag invoked

```
Starting data ingestion...
Finished ingesting data.
 -> CSV written to test//mira_flu_sum_up_summary.csv
Writing PARQUET files
 -> PARQUET written to test//mira_flu_sum_up_summary.parq
```

## Finding your way to the bugs
### Main Process
`src/processes/summary_report_update.rs`
This the top layer where a lot of the summary update logic exists.

### Ingesting Data
`src/io/data_ingest.rs`
contains `nextclade_data_collection` function that looks for nextclade tsv files and reads them into a struct based on the virus given. 
Holds NextcladeData struct.


### Writing to Files
`src/io/write_csv_files.rs ` -> `write_out_updated_summary_csv`
`src/io/write_parquet_files.rs` -> `write_updated_irma_summary_to_parquet`
Each script has the indicated functions tat write out the file type indicated