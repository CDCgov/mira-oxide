use chrono::Datelike;
use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(version, about = "Create a process status table for MIRA-NF runs.")]
pub struct NFStatusArgs {
    /// Path to samplesheet CSV file
    #[arg(short = 's', long = "samplesheet")]
    samplesheet: String,

    /// Path to nextflow log file
    #[arg(short = 'l', long = "log")]
    nextflow_log: String,

    /// Output HTML file path (optional)
    #[arg(
        short = 'o',
        long = "output",
        help = "Output HTML file path (default: ./nf_status_table.html)"
    )]
    output: Option<String>,

    /// Generate the detailed status table HTML
    #[arg(long = "table", help = "Generate the detailed status table HTML")]
    table: bool,

    /// Generate the progress dashboard HTML
    #[arg(long = "progress", help = "Generate the progress dashboard HTML")]
    progress: bool,

    /// Output table HTML to stdout instead of writing to file
    #[arg(
        long = "inline-table",
        help = "Output table HTML to stdout instead of writing to file"
    )]
    inline_table: bool,

    /// Output progress HTML to stdout instead of writing to file
    #[arg(
        long = "inline-progress",
        help = "Output progress HTML to stdout instead of writing to file"
    )]
    inline_progress: bool,
}

pub fn nf_status_process(args: NFStatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    let samplesheet_path = &args.samplesheet;
    let nextflow_log_path = &args.nextflow_log;
    let html_output_path = args.output.as_deref().unwrap_or("nf_status_table.html");

    // Determine which outputs to generate
    let generate_table = args.table;
    let generate_progress = args.progress;
    let inline_table = args.inline_table;
    let inline_progress = args.inline_progress;

    let file = File::open(samplesheet_path).expect("Could not open samplesheet");
    let reader = BufReader::new(file);
    let mut sample_ids = Vec::with_capacity(100); // Pre-allocate capacity
    let mut header_found = false;
    let mut sample_id_idx = None;
    for line in reader.lines() {
        let line = line.expect("Error reading line");
        if !header_found {
            let fields: Vec<&str> = line.split(',').collect();
            for (idx, col) in fields.iter().enumerate() {
                if col.trim() == "Sample ID" {
                    sample_id_idx = Some(idx);
                    break;
                }
            }
            header_found = true;
            continue;
        }
        if let Some(idx) = sample_id_idx {
            let fields: Vec<&str> = line.split(',').collect();
            if let Some(id) = fields.get(idx) {
                let trimmed = id.trim();
                if !trimmed.is_empty() {
                    sample_ids.push(trimmed.to_string());
                }
            }
        }
    }
    // Parse nextflow.log if provided - SINGLE PASS OPTIMIZATION
    use regex::Regex;
    use std::collections::HashMap;
    // Pre-allocate capacity for HashMaps to reduce rehashing
    let mut status_map: HashMap<(String, String), String> = HashMap::with_capacity(1000);
    // Track which processes are global (no sample in completion line)
    let mut global_completed: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(20);
    // Track process start times for elapsed time calculation
    let mut process_start_times: HashMap<(String, String), String> = HashMap::with_capacity(1000);
    // Track process end times for runtime calculation
    let mut process_end_times: HashMap<(String, String), String> = HashMap::with_capacity(1000);
    // Track runtime duration in human-readable format
    let mut process_runtimes: HashMap<(String, String), String> = HashMap::with_capacity(1000);
    let mut started_processes: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(50);
    let mut process_order: Vec<String> = Vec::with_capacity(50);
    let mut seen_processes = std::collections::HashSet::with_capacity(50);

    if let Ok(file) = File::open(nextflow_log_path) {
        let reader = BufReader::new(file);
        // Compile all regexes once outside the loop
        let re_submit = Regex::new(r"Submitted process > ([^ ]+) \(([^)]+)\)").unwrap();
        let re_complete = Regex::new(
            r"(\w{3}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}) .*Task completed > TaskHandler\[.*name: ([^ ]+)(?: \(([^)]+)\))?; status: ([A-Z]+);",
        )
        .unwrap();
        let re_start_time = Regex::new(
            r"(\w{3}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}) .*Submitted process > ([^ ]+) \(([^)]+)\)",
        )
        .unwrap();
        let re_starting = Regex::new(r"Starting process > ([^\s]+)").unwrap();

        for line in reader.lines() {
            if let Ok(line) = line {
                // Check for starting process - extract process order
                if let Some(caps) = re_starting.captures(&line) {
                    let proc = caps[1].split(':').last().unwrap_or("");
                    if proc != "PASSFAILED" && seen_processes.insert(proc.to_string()) {
                        process_order.push(proc.to_string());
                        started_processes.insert(proc.to_string());
                    }
                }

                // Check for submitted process
                if let Some(caps) = re_submit.captures(&line) {
                    let process = caps[1].split(':').last().unwrap_or("");
                    let sample = &caps[2];
                    let key = (sample.to_string(), process.to_string());

                    // Try to extract timestamp for this submission
                    if let Some(time_caps) = re_start_time.captures(&line) {
                        let timestamp = time_caps[1].to_string();
                        process_start_times.insert(key.clone(), timestamp);
                    }
                    status_map
                        .entry(key)
                        .or_insert_with(|| "running".to_string());
                }

                // Check for completed process
                if let Some(caps) = re_complete.captures(&line) {
                    let end_time_str = &caps[1];
                    let process = caps[2].split(':').last().unwrap_or("");
                    let sample_opt = caps.get(3).map(|m| m.as_str());
                    let status = match &caps[4] {
                        s if s == "COMPLETED" => "completed",
                        s if s == "FAILED" || s == "ERROR" => "error",
                        _ => "",
                    };

                    if !status.is_empty() {
                        if let Some(sample) = sample_opt {
                            if !sample.is_empty() && sample != "1" {
                                let key = (sample.to_string(), process.to_string());
                                status_map.insert(key.clone(), status.to_string());
                                process_end_times.insert(key.clone(), end_time_str.to_string());

                                // Calculate runtime if we have start time
                                if let Some(start_str) = process_start_times.get(&key) {
                                    if let (Some(start), Some(end)) =
                                        (parse_log_time(start_str), parse_log_time(end_time_str))
                                    {
                                        let duration = end - start;
                                        let hours = duration.num_hours();
                                        let mins = duration.num_minutes() % 60;
                                        let secs = duration.num_seconds() % 60;
                                        process_runtimes.insert(
                                            key,
                                            format!("{:02}:{:02}:{:02}", hours, mins, secs),
                                        );
                                    }
                                }
                            } else {
                                // sample == "1" or empty: global process
                                global_completed.insert(process.to_string());
                            }
                        } else {
                            // No sample: global process
                            global_completed.insert(process.to_string());
                        }
                    }
                }
            }
        }
    }
    // Remove all experiment_type and get_processes_for_experiment fallback logic
    // If no log or no processes found, just leave process_order empty and print only sample_id column
    if process_order.is_empty() {
        // Only print sample_id column
        print!("SAMPLE ID\n");
        for sample in sample_ids {
            println!("{}", sample);
        }
        return Ok(());
    }
    // Print table header with process labels (last part after ':')
    print!("SAMPLE ID");
    for proc in &process_order {
        print!(",{}", proc);
    }
    println!();
    // Print rows for each sample
    // Collect table data for plotly
    // Build table header, move CHECKMIRAVERSION to first column after sample id if present
    let mut table_header = vec!["SAMPLE ID".to_string()];
    if let Some(idx) = process_order.iter().position(|p| p == "CHECKMIRAVERSION") {
        table_header.push(process_order[idx].clone());
        let mut rest: Vec<String> = process_order
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != idx)
            .map(|(_, p)| p.clone())
            .collect();
        table_header.append(&mut rest);
    } else {
        table_header.extend(process_order.iter().cloned());
    }
    // Store cell data with runtime info for hover
    struct CellData {
        display: String,
        hover: Option<String>,
    }

    let mut table_rows: Vec<Vec<CellData>> = Vec::with_capacity(sample_ids.len());
    for sample in &sample_ids {
        let mut row = Vec::with_capacity(table_header.len());
        row.push(CellData {
            display: sample.clone(),
            hover: None,
        });
        // Use reordered process columns for row
        let process_cols = &table_header[1..];
        for proc in process_cols {
            if proc == "PASSFAILED" {
                continue;
            }
            let key = (sample.clone(), proc.clone());
            let runtime_hover = process_runtimes.get(&key).cloned();

            let status = status_map
                .get(&key)
                .map(|s| s.as_str())
                .or_else(|| {
                    if global_completed.contains(proc) {
                        Some("completed")
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    if !started_processes.contains(proc) {
                        Some("staged")
                    } else {
                        None
                    }
                });
            if let Some("running") = status {
                if let Some(start_str) = process_start_times.get(&key) {
                    if let Some(start_time) = parse_log_time(start_str) {
                        let now = chrono::Local::now().naive_local();
                        let duration = now - start_time;
                        let hours = duration.num_hours();
                        let mins = duration.num_minutes() % 60;
                        let secs = duration.num_seconds() % 60;
                        let elapsed = format!("{:02}:{:02}:{:02}", hours, mins, secs);
                        row.push(CellData {
                            display: elapsed.clone(),
                            hover: Some(format!("Running: {}", elapsed)),
                        });
                        continue;
                    }
                }
                row.push(CellData {
                    display: "running".to_string(),
                    hover: None,
                });
            } else if let Some("completed") = status {
                row.push(CellData {
                    display: "✅".to_string(),
                    hover: runtime_hover.map(|r| format!("Runtime: {}", r)),
                });
            } else if let Some("error") = status {
                row.push(CellData {
                    display: "⁉️".to_string(),
                    hover: runtime_hover.map(|r| format!("Failed after: {}", r)),
                });
            } else {
                row.push(CellData {
                    display: "🛄".to_string(),
                    hover: Some("Staged".to_string()),
                });
            }
        }
        table_rows.push(row);
    }

    // Calculate summary row: count completed samples per process
    let total_samples = sample_ids.len();
    let mut summary_row = vec![CellData {
        display: "Summary".to_string(),
        hover: None,
    }];
    for i in 1..table_header.len() {
        let completed_count = table_rows
            .iter()
            .filter(|row| row.get(i).map(|cell| cell.display == "✅").unwrap_or(false))
            .count();
        summary_row.push(CellData {
            display: format!("{}/{}", completed_count, total_samples),
            hover: Some(format!(
                "{} of {} samples completed",
                completed_count, total_samples
            )),
        });
    }

    // Output as raw HTML table
    if generate_table {
        // Pre-allocate capacity for HTML string to reduce allocations
        let estimated_size = 2000 + (table_rows.len() * table_header.len() * 50);
        let mut html = String::with_capacity(estimated_size);
        html.push_str(r#"<!doctype html>
        <html lang="en")
        <head>
        <meta charset="utf-8" />
        <title>nf_status_table</title>
        <style>
            html, body { height: 100%; margin: 0; padding: 0; box-sizing: border-box; }
            body { min-height: 100vh; width: 100vw; overflow-x: auto; overflow-y: auto; }
            body, table, th, td { font-family: 'Segoe UI', 'Arial', 'Liberation Sans', 'DejaVu Sans', 'sans-serif'; }
            table { border-collapse: collapse; width: 90vw; height-max: 80vw; table-layout: auto; }
            th, td { border: 1px solid #B8D4ED; padding: 4px; text-align: center; }
            th {
                writing-mode: vertical-rl;
                white-space: nowrap;
                vertical-align: bottom;
                text-align: left;
                height: 160px;
                font-size: 12px;
                padding: 0 2px;
                background: #ECF5FF;
            }
            tr:nth-child(even) { background: #F4FCFC; }
            tr.summary { background: #DBE8F7; font-weight: bold; }
            tr.summary td:first-child { text-align: left; padding-left: 8px; }
            td[title] { cursor: help; }
        </style>
        </head>
        <body>
        "#);
        html.push_str("<table>\n<thead>\n<tr>\n");
        for col in &table_header {
            html.push_str(&format!("<th>{}</th>", col));
        }
        html.push_str("</tr>\n</thead>\n<tbody>\n");

        // Add summary row first
        html.push_str("<tr class=\"summary\">");
        for cell in &summary_row {
            if let Some(hover) = &cell.hover {
                html.push_str(&format!("<td title=\"{}\">{}</td>", hover, cell.display));
            } else {
                html.push_str(&format!("<td>{}</td>", cell.display));
            }
        }
        html.push_str("</tr>\n");

        // Then add all sample rows
        for row in &table_rows {
            html.push_str("<tr>");
            for cell in row {
                if let Some(hover) = &cell.hover {
                    html.push_str(&format!("<td title=\"{}\">{}</td>", hover, cell.display));
                } else {
                    html.push_str(&format!("<td>{}</td>", cell.display));
                }
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</tbody>\n</table>\n</body>\n</html>\n");

        if inline_table {
            print!("{}", html);
        } else {
            // Ensure output path ends with .html
            let html_output_path = if html_output_path.ends_with(".html") {
                html_output_path.to_string()
            } else {
                format!("{}.html", html_output_path)
            };
            std::fs::write(&html_output_path, html).expect("Failed to write HTML table");
            println!("Raw HTML table written to {}", html_output_path);
        }
    }

    // Generate progress visualization HTML
    if generate_progress {
        let progress_output_path = if html_output_path.ends_with(".html") {
            html_output_path.replace(".html", "_progress.html")
        } else {
            format!("{}_progress.html", html_output_path)
        };

        // Calculate total workflow runtime
        let mut workflow_start: Option<chrono::NaiveDateTime> = None;
        let mut workflow_end: Option<chrono::NaiveDateTime> = None;

        // Find earliest start time
        for time_str in process_start_times.values() {
            if let Some(time) = parse_log_time(time_str) {
                workflow_start = Some(workflow_start.map_or(time, |existing| existing.min(time)));
            }
        }

        // Find latest end time
        for time_str in process_end_times.values() {
            if let Some(time) = parse_log_time(time_str) {
                workflow_end = Some(workflow_end.map_or(time, |existing| existing.max(time)));
            }
        }

        // Calculate total runtime string
        let total_runtime_str = if let (Some(start), Some(end)) = (workflow_start, workflow_end) {
            let duration = end - start;
            let hours = duration.num_hours();
            let mins = duration.num_minutes() % 60;
            let secs = duration.num_seconds() % 60;
            format!("{:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            "N/A".to_string()
        };

        // Calculate statistics for each process with sample lists
        let mut progress_stats: Vec<(
            String,
            usize,
            usize,
            usize,
            usize,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
        )> = Vec::new();
        for i in 1..table_header.len() {
            let process_name = &table_header[i];
            let mut completed = 0;
            let mut running = 0;
            let mut error = 0;
            let mut staged = 0;
            let mut completed_samples = Vec::new();
            let mut running_samples = Vec::new();
            let mut error_samples = Vec::new();
            let mut staged_samples = Vec::new();

            for (idx, row) in table_rows.iter().enumerate() {
                let sample_name = &sample_ids[idx];
                if let Some(cell) = row.get(i) {
                    match cell.display.as_str() {
                        "✅" => {
                            completed += 1;
                            completed_samples.push(sample_name.clone());
                        }
                        "⁉️" => {
                            error += 1;
                            error_samples.push(sample_name.clone());
                        }
                        "🛄" => {
                            staged += 1;
                            staged_samples.push(sample_name.clone());
                        }
                        _ => {
                            running += 1;
                            running_samples.push(sample_name.clone());
                        }
                    }
                }
            }
            progress_stats.push((
                process_name.clone(),
                completed,
                running,
                error,
                staged,
                completed_samples,
                running_samples,
                error_samples,
                staged_samples,
            ));
        }

        // Pre-allocate capacity for progress HTML to reduce allocations
        let estimated_html_size = 10000 + (progress_stats.len() * 2000);
        let mut progress_html = String::with_capacity(estimated_html_size);
        progress_html.push_str(r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<title>MIRA Progress</title>
<style>
    * { box-sizing: border-box; }
    body {
        margin: 0;
        padding: 20px;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', sans-serif;
        background: linear-gradient(135deg, #47264F 0%, #722161 100%);
        min-height: 100vh;
    }
    .container {
        max-width: 1200px;
        margin: 0 auto;
        background: white;
        border-radius: 16px;
        padding: 40px;
        box-shadow: 0 20px 60px rgba(0,0,0,0.3);
    }
    h1 {
        margin: 0 0 10px 0;
        font-size: 32px;
        color: #032659;
        font-weight: 700;
    }
    .subtitle {
        margin: 0 0 40px 0;
        color: #0057B7;
        font-size: 16px;
    }
    .process-card {
        background: #F4FCFC;
        border-radius: 12px;
        padding: 24px;
        margin-bottom: 20px;
        border: 1px solid #D5F7F9;
        transition: transform 0.2s, box-shadow 0.2s;
    }
    .process-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    }
    .process-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 16px;
        cursor: pointer;
        user-select: none;
    }
    .process-header-left {
        display: flex;
        align-items: center;
        gap: 12px;
    }
    .collapse-icon {
        font-size: 20px;
        color: #0057B7;
        transition: transform 0.3s ease;
        display: inline-block;
    }
    .collapse-icon.collapsed {
        transform: rotate(-90deg);
    }
    .process-name {
        font-size: 18px;
        font-weight: 600;
        color: #032659;
    }
    .process-stats {
        font-size: 14px;
        color: #0057B7;
    }
    .process-details {
        transition: max-height 0.3s ease, opacity 0.3s ease;
        overflow: visible;
    }
    .process-details.collapsed {
        max-height: 0 !important;
        opacity: 0;
        overflow: hidden;
    }
    .progress-container {
        width: 100%;
        height: 32px;
        background: #DBE8F7;
        border-radius: 16px;
        overflow: hidden;
        display: flex;
        position: relative;
    }
    .progress-segment {
        height: 100%;
        transition: width 0.5s ease;
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
        font-size: 12px;
        font-weight: 600;
        position: relative;
    }
    .progress-completed {
        background: linear-gradient(90deg, #0081A1, #00B1CE);
    }
    .progress-running {
        background: linear-gradient(90deg, #0057B7, #3382CF);
    }
    .progress-error {
        background: linear-gradient(90deg, #CC1B22, #F0695E);
    }
    .progress-staged {
        background: linear-gradient(90deg, #8F4A8F, #B278B2);
    }
    .legend {
        display: flex;
        gap: 24px;
        margin-top: 30px;
        padding-top: 20px;
        border-top: 2px solid #D5F7F9;
        flex-wrap: wrap;
    }
    .legend-item {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 14px;
    }
    .legend-color {
        width: 20px;
        height: 20px;
        border-radius: 4px;
    }
    .stats-grid {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 12px;
        margin-top: 12px;
        overflow: visible;
    }
    .stat-box {
        padding: 8px 12px;
        border-radius: 8px;
        text-align: center;
        font-size: 13px;
        cursor: help;
        transition: transform 0.1s;
        position: relative;
        overflow: visible;
    }
    .stat-box:hover {
        transform: scale(1.05);
        z-index: 10000;
    }
    .stat-box:hover .tooltip {
        visibility: visible;
        opacity: 1;
    }
    .tooltip {
        visibility: hidden;
        opacity: 0;
        position: absolute;
        z-index: 10001;
        bottom: 125%;
        left: 50%;
        transform: translateX(-50%);
        background: #032659;
        color: white;
        padding: 12px;
        border-radius: 8px;
        font-size: 12px;
        white-space: pre-line;
        text-align: left;
        max-height: 300px;
        overflow-y: auto;
        min-width: 150px;
        max-width: 300px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        transition: opacity 0.2s, visibility 0.2s;
        user-select: text;
        cursor: text;
        pointer-events: auto;
    }
    .tooltip::after {
        content: "";
        position: absolute;
        top: 100%;
        left: 50%;
        margin-left: -8px;
        border-width: 8px;
        border-style: solid;
        border-color: #032659 transparent transparent transparent;
    }
    .tooltip-header {
        font-weight: 600;
        margin-bottom: 8px;
        border-bottom: 1px solid rgba(255,255,255,0.3);
        padding-bottom: 4px;
    }
    .stat-box-completed {
        background: #D5F7F9;
        color: #125261;
    }
    .stat-box-running {
        background: #DBE8F7;
        color: #032659;
    }
    .stat-box-error {
        background: #FCDEDB;
        color: #660F14;
    }
    .stat-box-staged {
        background: #E8D6EB;
        color: #47264F;
    }
    .stat-number {
        font-size: 20px;
        font-weight: 700;
        display: block;
    }
    .overall-stats {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 20px;
        margin-bottom: 40px;
    }
    .overall-card {
        background: linear-gradient(135deg, #47264F 0%, #722161 100%);
        color: white;
        padding: 24px;
        border-radius: 12px;
        text-align: center;
    }
    .overall-number {
        font-size: 36px;
        font-weight: 700;
        margin-bottom: 8px;
    }
    .overall-label {
        font-size: 14px;
        opacity: 0.9;
    }
</style>
<script>
function toggleProcess(element) {
    const card = element.closest('.process-card');
    const details = card.querySelector('.process-details');
    const icon = card.querySelector('.collapse-icon');
    
    details.classList.toggle('collapsed');
    icon.classList.toggle('collapsed');
}

// Initialize cards: collapse if 100% completed, expand otherwise
document.addEventListener('DOMContentLoaded', function() {
    document.querySelectorAll('.process-card').forEach(function(card) {
        const details = card.querySelector('.process-details');
        const icon = card.querySelector('.collapse-icon');
        const statsText = card.querySelector('.process-stats').textContent;
        
        // Set max-height for proper animation
        details.style.maxHeight = details.scrollHeight + 'px';
        
        // Check if process is 100% completed (look for "100.0%" or similar)
        if (statsText.includes('100.0%')) {
            details.classList.add('collapsed');
            icon.classList.add('collapsed');
        }
    });
});
</script>
</head>
<body>
<div class="container">
    <h1>MIRA Progress</h1>
    
    <div class="overall-stats">
        <div class="overall-card">
            <div class="overall-number">"#);

        progress_html.push_str(&format!("{}", total_samples));
        progress_html.push_str(
            r#"</div>
            <div class="overall-label">Total Samples</div>
        </div>
        <div class="overall-card">
            <div class="overall-number">"#,
        );

        progress_html.push_str(&format!("{}", table_header.len() - 1));
        progress_html.push_str(
            r#"</div>
            <div class="overall-label">Pipeline Processes</div>
        </div>
        <div class="overall-card">
            <div class="overall-number">"#,
        );

        progress_html.push_str(&format!("{}", total_runtime_str));
        progress_html.push_str(
            r#"</div>
            <div class="overall-label">Total Runtime</div>
        </div>
    </div>
"#,
        );

        // Add progress bars for each process
        for (
            process_name,
            completed,
            running,
            error,
            staged,
            completed_samples,
            running_samples,
            error_samples,
            staged_samples,
        ) in &progress_stats
        {
            let total = completed + running + error + staged;
            if total == 0 {
                continue;
            }

            let completed_pct = (*completed as f64 / total as f64) * 100.0;
            let running_pct = (*running as f64 / total as f64) * 100.0;
            let error_pct = (*error as f64 / total as f64) * 100.0;
            let staged_pct = (*staged as f64 / total as f64) * 100.0;

            progress_html.push_str(&format!(
                r#"
    <div class="process-card">
        <div class="process-header" onclick="toggleProcess(this)">
            <div class="process-header-left">
                <span class="collapse-icon">▼</span>
                <div class="process-name">{}</div>
            </div>
            <div class="process-stats">{}/{} completed ({:.1}%)</div>
        </div>
        <div class="process-details">
        <div class="progress-container">
"#,
                process_name, completed, total, completed_pct
            ));

            if *completed > 0 {
                progress_html.push_str(&format!(
                    r#"<div class="progress-segment progress-completed" style="width: {:.1}%;" title="Completed: {}"></div>"#,
                    completed_pct, completed
                ));
            }
            if *running > 0 {
                progress_html.push_str(&format!(
                    r#"<div class="progress-segment progress-running" style="width: {:.1}%;" title="Running: {}"></div>"#,
                    running_pct, running
                ));
            }
            if *error > 0 {
                progress_html.push_str(&format!(
                    r#"<div class="progress-segment progress-error" style="width: {:.1}%;" title="Failed: {}"></div>"#,
                    error_pct, error
                ));
            }
            if *staged > 0 {
                progress_html.push_str(&format!(
                    r#"<div class="progress-segment progress-staged" style="width: {:.1}%;" title="Staged: {}"></div>"#,
                    staged_pct, staged
                ));
            }

            // Create sample list tooltips HTML
            let completed_tooltip_html = if completed_samples.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="tooltip"><div class="tooltip-header">Completed ({})</div>{}</div>"#,
                    completed,
                    completed_samples.join("\n")
                )
            };

            let running_tooltip_html = if running_samples.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="tooltip"><div class="tooltip-header">Running ({})</div>{}</div>"#,
                    running,
                    running_samples.join("\n")
                )
            };

            let error_tooltip_html = if error_samples.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="tooltip"><div class="tooltip-header">Failed ({})</div>{}</div>"#,
                    error,
                    error_samples.join("\n")
                )
            };

            let staged_tooltip_html = if staged_samples.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="tooltip"><div class="tooltip-header">Staged ({})</div>{}</div>"#,
                    staged,
                    staged_samples.join("\n")
                )
            };

            progress_html.push_str(
                r#"
        </div>
        <div class="stats-grid">"#,
            );

            // Completed box
            progress_html.push_str(&format!(
                r#"
            <div class="stat-box stat-box-completed">
                <span class="stat-number">{}</span>
                Completed
                {}
            </div>"#,
                completed, completed_tooltip_html
            ));

            // Running box
            progress_html.push_str(&format!(
                r#"
            <div class="stat-box stat-box-running">
                <span class="stat-number">{}</span>
                Running
                {}
            </div>"#,
                running, running_tooltip_html
            ));

            // Error box
            progress_html.push_str(&format!(
                r#"
            <div class="stat-box stat-box-error">
                <span class="stat-number">{}</span>
                Failed
                {}
            </div>"#,
                error, error_tooltip_html
            ));

            // Staged box
            progress_html.push_str(&format!(
                r#"
            <div class="stat-box stat-box-staged">
                <span class="stat-number">{}</span>
                Staged
                {}
            </div>"#,
                staged, staged_tooltip_html
            ));

            progress_html.push_str(
                r#"
        </div>
        </div>
    </div>
"#,
            );
        }

        progress_html.push_str(
            r#"
    <div class="legend">
        <div class="legend-item">
            <div class="legend-color progress-completed"></div>
            <span>Completed</span>
        </div>
        <div class="legend-item">
            <div class="legend-color progress-running"></div>
            <span>Running</span>
        </div>
        <div class="legend-item">
            <div class="legend-color progress-error"></div>
            <span>Failed</span>
        </div>
        <div class="legend-item">
            <div class="legend-color progress-staged"></div>
            <span>Staged</span>
        </div>
    </div>
</div>
</body>
</html>
"#,
        );

        if inline_progress {
            print!("{}", progress_html);
        } else {
            std::fs::write(&progress_output_path, progress_html)
                .expect("Failed to write progress HTML");
            println!("Progress dashboard written to {}", progress_output_path);
        }
    }

    // Print rows for each sample
    fn parse_log_time(s: &str) -> Option<chrono::NaiveDateTime> {
        // Try to parse e.g. "Jun-02 16:39:24.913"
        let fmt = "%b-%d %H:%M:%S%.3f";
        let year = chrono::Local::now().year();
        chrono::NaiveDateTime::parse_from_str(&format!("{} {}", year, s), &format!("%Y {}", fmt))
            .ok()
    }

    Ok(())
}
