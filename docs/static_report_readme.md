# Static Report - MIRA HTML Summary Generator

## Overview

`static-report` is a command-line tool that generates a comprehensive HTML summary report from MIRA-NF pipeline output data. It creates an interactive HTML report with embedded visualizations, downloadable data tables, and links to detailed per-sample results.

## Features

- **Base64-encoded images**: Embeds logos and icons directly in the HTML for portability
- **Interactive Plotly visualizations**: Displays barcode distribution, pass/fail heatmaps, and coverage plots
- **Data tables**: Generates downloadable Excel files for summary tables, AA variants, minor variants, and indels
- **Per-sample coverage reports**: Creates individual HTML pages for each sample's coverage and read mapping
- **FASTA downloads**: Provides links to download assembled sequences
- **Self-contained output**: Single HTML file with all resources embedded (except external downloads)

## Usage

### Basic Command

```bash
mira-oxide static-report -d <data-path> [OPTIONS]
```

### Required Arguments

- `-d, --data-path <PATH>`: Path to the directory containing JSON data files from MIRA-NF pipeline

### Optional Arguments

- `-r, --run-path <PATH>`: Path to the run directory (defaults to data_path). Used to extract run name.
- `-l, --logo-path <PATH>`: Path to the assets folder containing logos (mira-logo, favicon, Excel icon)
- `-o, --output <PATH>`: Output HTML file path (default: `MIRA-summary-{run}.html`)

## Examples

### Generate Report from Data Directory

```bash
mira-oxide static-report -d /path/to/mira/output
```

This creates `MIRA-summary-{run}.html` in the current directory.

### Specify Run Name and Logo Path

```bash
mira-oxide static-report \
  -d /path/to/mira/output \
  -r /path/to/run/directory \
  -l /path/to/mira-nf/assets
```

### Custom Output Path

```bash
mira-oxide static-report \
  -d /path/to/mira/output \
  -o my-custom-report.html
```

## Input Files

The tool expects the following JSON files in the data directory:

### Required JSON Files

- `barcode_distribution.json` - Plotly figure showing read distribution across barcodes
- `pass_fail_heatmap.json` - Plotly heatmap of QC pass/fail results
- `heatmap.json` - Coverage summary heatmap
- `irma_summary.json` - Summary statistics table (orient="split" format)
- `dais_vars.json` - Amino acid variants table
- `alleles.json` - Minor variants table
- `indels.json` - Minor indels table

### Coverage Figures (per sample)

- `coveragefig_{sample}_linear.json` - Coverage plot for each sample
- `readsfig_{sample}.json` - Read mapping Sankey diagram for each sample

### FASTA Files

- `*.fasta` - Any FASTA files in the data directory will be copied and linked

### Asset Images

- `assets/mira-logo-midjourney_20230526_rmbkgnd.png` - MIRA logo
- `assets/favicon.ico` - Browser favicon
- `assets/Microsoft_Excel-Logo.png` - Excel download icon

## Output Files

### Main Report

- `MIRA-summary-{run}.html` - Main HTML summary report with all visualizations

### Generated Files

The tool creates several additional files in the current directory:

**Excel Tables:**
- `MIRA_{run}_summary.xlsx` - Summary statistics
- `MIRA_{run}_aavars.xlsx` - Amino acid variants
- `MIRA_{run}_minorvariants.xlsx` - Minor variants
- `MIRA_{run}_minorindels.xlsx` - Minor indels

**Per-sample Coverage:**
- `MIRA_{sample}_coverage.html` - Individual coverage page for each sample

**FASTA Files:**
- `MIRA_{run}_{filename}` - Copied FASTA files with run name prefix

## Report Sections

The generated HTML report includes the following sections:

### 1. Barcode Assignment
Interactive pie chart showing read distribution across samples and controls.

### 2. Automatic Quality Control Decisions
Heatmap showing which samples passed/failed QC criteria:
- Minimum median coverage: 50x
- Minimum reference coverage: 90%
- Maximum minor variants: <10 at ≥5% frequency
- Premature stop codons flagged

### 3. Median Coverage
Heatmap summarizing mean coverage per sample per reference.

### 4. MIRA Summary Table
Comprehensive table with downloadable Excel file containing:
- Sample names
- Total reads, reads passing QC, reads mapped
- Reference information
- Coverage statistics
- Minor variant counts
- Pass/fail status

### 5. Individual Sample Coverage Figures
Links to detailed per-sample HTML pages showing:
- Read mapping Sankey diagram
- Coverage plot across reference

### 6. AA Variants Table
Table of amino acid variants with downloadable Excel file.

### 7. Minor Variants & Indels
Download links for detailed tables of:
- Minor single nucleotide variants
- Minor insertions/deletions

### 8. FASTA Downloads
Links to download assembled consensus sequences.

## Implementation Notes

### Rust vs Python Differences

This Rust implementation provides equivalent functionality to the original Python `static_report.py` with some differences:

1. **Plotly Rendering**: The Rust version embeds Plotly JSON data and uses client-side JavaScript to render plots, whereas Python uses `plotly.io` for server-side rendering.

2. **Excel Generation**: Currently returns placeholder text. Full implementation would require integrating a crate like `rust_xlsxwriter` to generate actual Excel files from the JSON data.

3. **JSON Parsing**: The current implementation reads JSON as strings. Full implementation would use `serde_json` to parse and manipulate the data structures.

### Future Enhancements

Potential improvements for full feature parity:

- [ ] Add `serde_json` for proper JSON parsing
- [ ] Integrate `rust_xlsxwriter` for Excel file generation
- [ ] Add proper error handling for missing files
- [ ] Support for different JSON data formats
- [ ] Validation of input data structure
- [ ] Progress indicators for large datasets
- [ ] Configuration file support

## Troubleshooting

**Missing files errors:**
- Ensure all required JSON files exist in the data directory
- Check that file paths are correct (absolute or relative to current directory)
- Verify logo/asset paths if using `-l` flag

**Empty visualizations:**
- Check that JSON files contain valid Plotly figure data
- Ensure Plotly CDN is accessible (requires internet connection)
- Open browser console to see JavaScript errors

**Excel files not generated:**
- Current implementation creates placeholder links
- Use Python version for full Excel export functionality
- Or contribute Excel generation code using `rust_xlsxwriter`

## Related Commands

- `mira-oxide nf-status`: Generate pipeline progress dashboard
- `mira-oxide plotter`: Create visualization plots
- `mira-oxide variants-of-interest`: Analyze variants of interest

## Version History

See [CHANGELOG.md](../CHANGELOG.md) for version history and updates.

## Support

For issues, questions, or feature requests, please open an issue on the [GitHub repository](https://github.com/CDCgov/mira-oxide).
