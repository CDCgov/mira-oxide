# Check Chemistry

A small tool for argument parsing the user data to select the appropriate IRMA module and config filepath based on command-line arguments provided by the pipeline.
Handles both ONT and Illumina.

## How to Run
After cloning the mira-oxide repo, execute this command to create a mutations of interest table for the samples:

```bash
cargo run -- find-chemistry --sample "${sample}" --fastq "${fastq}" --experiment "${params.e}" --wd-path "${projectDir}" --read-count "${read_counts}" --irma-config "${irma_config}" --irma-config-path "${custom_irma_config}"
```

Or run the biniary (inside or outside of container):
```bash
mira-oxide find-chemistry --sample "${sample}" --fastq "${fastq}" --experiment "${params.e}" --wd-path "${projectDir}" --read-count "${read_counts}" --irma-config "${irma_config}" --irma-config-path "${custom_irma_config}"
```