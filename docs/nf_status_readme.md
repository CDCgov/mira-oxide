# NFStatus - Nextflow Pipeline Status Monitor

## Overview

`nf-status` is a command-line tool that monitors and visualizes the progress of MIRA-NF (Nextflow) pipeline runs. It parses Nextflow log files and samplesheets to generate comprehensive status reports in both detailed table and visual dashboard formats.

## Features

- **Real-time Status Tracking**: Monitor process completion, running tasks, errors, and staged processes
- **Multi-format Output**: Generate detailed HTML tables and visual progress dashboards
- **Runtime Analytics**: Track individual process runtimes and total workflow duration
- **Sample-level Granularity**: View status for each sample across all pipeline processes
- **Interactive Visualizations**: Hover over cells to see detailed runtime information
- **Flexible Output Options**: Write to files or pipe to stdout

## Usage

### Basic Command

```bash
mira-oxide nf-status -s <samplesheet.csv> -l <nextflow.log> [OPTIONS]
```

### Required Arguments

- `-s, --samplesheet <PATH>`: Path to the samplesheet CSV file containing sample IDs
- `-l, --log <PATH>`: Path to the Nextflow log file (typically `.nextflow.log`)

### Optional Arguments

- `-o, --output <PATH>`: Output HTML file path (default: `./nf_status_table.html`)
- `--table`: Generate the detailed status table HTML
- `--progress`: Generate the progress dashboard HTML
- `--inline-table`: Output table HTML to stdout instead of writing to file
- `--inline-progress`: Output progress HTML to stdout instead of writing to file

## Examples

### Generate Both HTML Outputs

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  --table --progress
```

This creates:
- `nf_status_table.html` - Detailed status table
- `nf_status_table_progress.html` - Visual progress dashboard

### Generate Only the Table

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  --table
```

### Generate Only the Progress Dashboard

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  --progress
```

### Custom Output Path

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  -o /path/to/output/status.html \
  --table --progress
```

Creates:
- `/path/to/output/status.html`
- `/path/to/output/status_progress.html`

### Pipe to Stdout

```bash
# Pipe table to stdout
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  --table --inline-table > my_table.html

# Pipe progress dashboard to stdout
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  --progress --inline-progress > my_dashboard.html
```

## Output Formats

### 1. Detailed Status Table

A comprehensive table showing the status of every sample across all pipeline processes.

**Features:**
- Vertical headers to maximize screen space
- Summary row showing completion statistics per process
- Color-coded status indicators:
  - ✅ Completed
  - ⏱️ Running (with elapsed time: HH:MM:SS)
  - ⁉️ Failed
  - 🛄 Staged (not yet started)
- Hover tooltips showing runtime information
- Responsive design with scrolling for large datasets

**Color Scheme:**
- Light blue backgrounds for easy reading
- Alternating row colors for clarity
- Highlighted summary row

### 2. Progress Dashboard

A high-level visual dashboard for quick status assessment.

**Features:**
- Overall workflow statistics:
  - Total number of samples
  - Total number of pipeline processes
  - Total workflow runtime (HH:MM:SS)
- Per-process progress bars showing:
  - Completed samples (teal gradient)
  - Running samples (blue gradient)
  - Failed samples (red gradient)
  - Staged samples (purple gradient)
- Detailed statistics grid for each process
- Modern, card-based layout
- Hover effects and tooltips
- Color-coded legend

**Color Scheme:**
- Purple gradient background
- Clean white card containers
- Status-specific color coding (see [Color Palette](color-palette.md))

## Status Indicators

### Completed (✅)
Process has successfully finished for this sample. Hover to see total runtime.

### Running (HH:MM:SS)
Process is currently executing. Displays elapsed time in hours:minutes:seconds format.

### Failed (⁉️)
Process encountered an error and did not complete successfully. Hover to see how long it ran before failing.

### Staged (🛄)
Process has not yet started for this sample (waiting in queue or dependency not met).

## Input File Formats

### Samplesheet CSV

Must contain a `Sample ID` column header. Example:

```csv
Sample ID,R1,R2,experiment_type
SAMPLE001,reads_R1.fastq.gz,reads_R2.fastq.gz,sc2
SAMPLE002,reads_R1.fastq.gz,reads_R2.fastq.gz,flu
```

The tool will:
1. Search for the `Sample ID` column (case-sensitive)
2. Extract all sample IDs from subsequent rows
3. Track status for each sample across all processes

### Nextflow Log

Standard Nextflow log file (`.nextflow.log`) containing:
- Process submission events: `Submitted process > <NAME> (<SAMPLE>)`
- Process completion events: `Task completed > TaskHandler[...name: <NAME> (<SAMPLE>); status: <STATUS>;`
- Process start events: `Starting process > <NAME>`

The tool extracts:
- Process names and execution order
- Sample-specific task assignments
- Start and end timestamps
- Success/failure status

## Process Detection

The tool automatically detects all processes from the Nextflow log by:
1. Identifying "Starting process" events
2. Extracting process names (using the last component after `:`)
3. Maintaining execution order as processes are encountered
4. Filtering out special processes (e.g., `PASSFAILED`)

If `CHECKMIRAVERSION` process is detected, it's automatically moved to the first column after the sample ID for better visibility.

## Runtime Calculation

### Individual Process Runtime
- **Start time**: Extracted from "Submitted process" log entries
- **End time**: Extracted from "Task completed" log entries
- **Format**: HH:MM:SS (hours:minutes:seconds)

### Total Workflow Runtime
- **Start time**: Earliest process submission across all samples
- **End time**: Latest process completion across all samples
- **Format**: HH:MM:SS

### Running Process Elapsed Time
For processes still in progress, the elapsed time is calculated from the submission time to the current time.

## Global vs. Sample-Specific Processes

Some Nextflow processes operate globally (once per workflow) rather than per sample:
- Detected when completion log shows sample ID as "1" or empty
- Marked as completed for all samples
- Examples: validation, configuration, environment setup processes

## Tips and Best Practices

### Monitoring Active Pipelines

Run `nf-status` periodically to track progress:

```bash
# Run every 5 minutes
watch -n 300 'mira-oxide nf-status -s samplesheet.csv -l .nextflow.log --table --progress'
```

### Checking for Failures

Look for the ⁉️ emoji in the table or red segments in the progress bars. Hover over failed cells to see how long the process ran before failing.

### Performance Considerations

- Large datasets (100+ samples, 20+ processes) generate large HTML tables
- Progress dashboard is more lightweight for quick checks
- Consider generating only the visualization you need (`--table` or `--progress`)

### Troubleshooting

**No processes shown:**
- Verify the Nextflow log file exists and is readable
- Check that processes have started (look for "Starting process" entries in the log)
- Ensure the log file path is correct

**Missing samples:**
- Verify `Sample ID` column exists in samplesheet (case-sensitive)
- Check for CSV formatting issues (commas, quotes)
- Ensure sample IDs in samplesheet match those in the Nextflow log

**Runtime showing "N/A":**
- Workflow hasn't started yet (no submission timestamps)
- Log file doesn't contain timestamp information
- Timestamps couldn't be parsed (check log format)

## Integration with MIRA-NF

This tool is designed to work seamlessly with the MIRA-NF pipeline:
- Automatically handles standard MIRA-NF process naming
- Recognizes common experiment types (SC2, Flu, RSV)
- Compatible with MIRA-NF samplesheet format
- Understands MIRA-NF's multi-pathogen workflow structure

## Output Files

When generating files (not using `--inline-*` options):

### Default Naming
- Table: `nf_status_table.html`
- Progress: `nf_status_table_progress.html`

### Custom Naming (with `-o` flag)
If you specify `-o my_report.html`:
- Table: `my_report.html`
- Progress: `my_report_progress.html`

If you specify `-o my_report` (without .html):
- Table: `my_report.html`
- Progress: `my_report_progress.html`

## Browser Compatibility

Both HTML outputs are compatible with:
- Chrome/Chromium (recommended)
- Firefox
- Safari
- Edge
- Mobile browsers

No JavaScript required - pure HTML/CSS for maximum compatibility and performance.

## Color Palette

All colors used in the visualizations follow the official mira-oxide color palette. See [Color Palette Documentation](color-palette.md) for details.

## Related Commands

- `mira-oxide variants-of-interest`: Generate variants of interest tables
- `mira-oxide positions-of-interest`: Analyze positions of interest
- `mira-oxide plotter`: Create visualization plots

## Version History

See [CHANGELOG.md](../CHANGELOG.md) for version history and updates.

## Support

For issues, questions, or feature requests, please open an issue on the [GitHub repository](https://github.com/CDCgov/mira-oxide).
