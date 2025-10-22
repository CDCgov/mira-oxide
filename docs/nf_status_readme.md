# NFStatus - Nextflow Pipeline Status Monitor

## Overview

`nf-status` is a command-line tool that monitors and visualizes the progress of MIRA-NF (Nextflow) pipeline runs. It parses Nextflow log files and samplesheets to generate an interactive visual progress dashboard.

## Features

- **Real-time Status Tracking**: Monitor process completion, running tasks, errors, and staged processes
- **Visual Progress Dashboard**: Interactive HTML dashboard with collapsible process cards
- **Pipeline Overview**: High-level statistics showing total samples, completion status, and runtime
- **Sample-level Granularity**: View detailed status for each sample across all pipeline processes
- **Runtime Analytics**: Track individual process runtimes and total workflow duration
- **Auto-collapse Completed Processes**: Processes that are 100% completed automatically collapse for cleaner view
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

- `-o, --output <PATH>`: Output HTML file path (default: `./nf_status_progress.html`)
- `--inline`: Output HTML to stdout instead of writing to file

## Examples

### Generate Progress Dashboard

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log
```

This creates `nf_status_progress.html` in the current directory.

### Custom Output Path

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  -o /path/to/output/my_dashboard.html
```

Creates `/path/to/output/my_dashboard.html`

### Pipe to Stdout

```bash
mira-oxide nf-status \
  -s samplesheet.csv \
  -l .nextflow.log \
  --inline > my_dashboard.html
```

## Output Format

### Progress Dashboard

An interactive visual dashboard for comprehensive pipeline monitoring.

**Features:**

#### Pipeline Overview Card
A unified statistics card at the top showing:
- **Total Samples**: Total number of samples in the pipeline
- **Samples Completed**: Number of samples with all processes completed (✅)
- **Samples In Progress**: Number of samples with any running or failed processes
- **Total Runtime**: Total workflow runtime from first process start to last process completion (HH:MM:SS)

The overview card uses color-coded visual indicators:
- Teal border/highlight for completed samples
- Red border/highlight for in-progress samples
- Clean, unified design with hover effects

#### Process Cards
Individual collapsible cards for each pipeline process showing:
- **Progress Bar**: Visual representation of sample status distribution
  - Completed samples (teal gradient)
  - Running samples (blue gradient)
  - Failed samples (red gradient)
  - Staged samples (purple gradient)
- **Statistics Grid**: Detailed breakdown of sample counts and percentages
- **Sample Lists**: Expandable sections showing which specific samples are in each state
- **Auto-collapse**: Processes that are 100% completed automatically collapse for cleaner view

**Interactive Features:**
- Click process headers to expand/collapse details
- Click "Show samples" links to view sample lists
- Hover over elements for visual feedback
- Smooth animations for all interactions

**Color Scheme:**
- Purple gradient background (#47264F to #722161)
- Clean white card containers with rounded corners
- Status-specific color coding (see [Color Palette](color-palette.md))
- Light blue card backgrounds (#F4FCFC) with subtle borders

## Status Indicators

### Completed (✅)
Process has successfully finished for this sample. Shown in teal-colored progress bars and sample lists.

### Running (HH:MM:SS)
Process is currently executing. Displays elapsed time in hours:minutes:seconds format. Shown in blue-colored progress bars.

### Failed (⁉️)
Process encountered an error and did not complete successfully. Shown in red-colored progress bars and sample lists.

### Staged (🛄)
Process has not yet started for this sample (waiting in queue or dependency not met). Shown in purple-colored progress bars.

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
watch -n 300 'mira-oxide nf-status -s samplesheet.csv -l .nextflow.log'
```

Or use a simple loop:

```bash
while true; do
  mira-oxide nf-status -s samplesheet.csv -l .nextflow.log
  sleep 300  # Wait 5 minutes
done
```

### Checking for Failures

Look for red segments in the progress bars. Click on "Show samples" in the Error section of any process card to see which specific samples failed.

### Understanding the Pipeline Overview

The Pipeline Overview card gives you instant insight into your pipeline's health:
- **All samples completed?** Total Samples = Samples Completed
- **Pipeline still running?** Samples In Progress > 0
- **How long has it been running?** Check Total Runtime

### Using Auto-collapse

Processes that have completed for all samples (100% completion) automatically collapse to reduce clutter. You can:
- Click the collapsed process header to expand and view details
- Focus on processes that still have work in progress
- Quickly scan for any non-collapsed processes that need attention

### Performance Considerations

- The progress dashboard efficiently handles large datasets (100+ samples, 20+ processes)
- Auto-collapse feature reduces visual clutter for long-running pipelines
- Single HTML file output makes it easy to share with team members

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

**Pipeline Overview shows 0 samples:**
- Check that the samplesheet has data rows (not just headers)
- Verify sample IDs are not empty
- Ensure CSV file is properly formatted

## Integration with MIRA-NF

This tool is designed to work seamlessly with the MIRA-NF pipeline:
- Automatically handles standard MIRA-NF process naming
- Recognizes common experiment types (SC2, Flu, RSV)
- Compatible with MIRA-NF samplesheet format
- Understands MIRA-NF's multi-pathogen workflow structure

## Output Files

When generating files (not using `--inline` option):

### Default Naming
- Progress Dashboard: `nf_status_progress.html`

### Custom Naming (with `-o` flag)
If you specify `-o my_report.html`:
- Progress Dashboard: `my_report.html`

If you specify `-o my_report` (without .html):
- Progress Dashboard: `my_report.html`

## Browser Compatibility

The HTML dashboard is compatible with:
- Chrome/Chromium (recommended)
- Firefox
- Safari
- Edge
- Mobile browsers (responsive design)

Requires JavaScript for interactive features (collapsing/expanding, sample list toggling). Falls back gracefully if JavaScript is disabled.

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
