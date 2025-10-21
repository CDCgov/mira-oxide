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
    let mut sample_ids = Vec::new();
    let mut header_found = false;
    let mut sample_id_idx = None;
    for line in reader.lines() {
        let line = line.expect("Error reading line");
        let fields: Vec<&str> = line.split(',').collect();
        if !header_found {
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
            if let Some(id) = fields.get(idx) {
                sample_ids.push(id.trim().to_string());
            }
        }
    }
    // Parse nextflow.log if provided
    use regex::Regex;
    use std::collections::HashMap;
    let mut status_map: HashMap<(String, String), String> = HashMap::new();
    // Track which processes are global (no sample in completion line)
    let mut global_completed: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track process start times for elapsed time calculation
    let mut process_start_times: HashMap<(String, String), String> = HashMap::new();
    // Track process end times for runtime calculation
    let mut process_end_times: HashMap<(String, String), String> = HashMap::new();
    // Track runtime duration in human-readable format
    let mut process_runtimes: HashMap<(String, String), String> = HashMap::new();
    let mut started_processes: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Ok(file) = File::open(nextflow_log_path) {
        let reader = BufReader::new(file);
        let re_submit = Regex::new(r"Submitted process > ([^ ]+) \(([^)]+)\)").unwrap();
        let re_complete = Regex::new(
            r"(\w{3}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}) .*Task completed > TaskHandler\[.*name: ([^ ]+)(?: \(([^)]+)\))?; status: ([A-Z]+);",
        )
        .unwrap();
        let re_start_time = Regex::new(
            r"(\w{3}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}) .*Submitted process > ([^ ]+) \(([^)]+)\)",
        )
        .unwrap();
        let mut log_lines: Vec<String> = Vec::new();
        for line in reader.lines() {
            if let Ok(line) = line {
                log_lines.push(line.clone());
                if let Some(caps) = re_submit.captures(&line) {
                    let process = caps[1].split(':').last().unwrap_or("").to_string();
                    let sample = caps[2].to_string();
                    let key = (sample.clone(), process.clone());
                    // Try to extract timestamp for this submission
                    if let Some(time_caps) = re_start_time.captures(&line) {
                        let timestamp = time_caps[1].to_string();
                        process_start_times.insert(key.clone(), timestamp);
                    }
                    status_map.entry(key).or_insert("running".to_string());
                }
                if let Some(caps) = re_complete.captures(&line) {
                    let end_time_str = caps[1].to_string();
                    let process = caps[2].split(':').last().unwrap_or("").to_string();
                    let sample_opt = caps.get(3).map(|m| m.as_str());
                    let status = match &caps[4] {
                        s if s == "COMPLETED" => "completed",
                        s if s == "FAILED" || s == "ERROR" => "error",
                        _ => "",
                    };
                    if !status.is_empty() {
                        if let Some(sample) = sample_opt {
                            if !sample.is_empty() && sample != "1" {
                                let key = (sample.to_string(), process.clone());
                                status_map.insert(key.clone(), status.to_string());
                                process_end_times.insert(key.clone(), end_time_str.clone());

                                // Calculate runtime if we have start time
                                if let Some(start_str) = process_start_times.get(&key) {
                                    if let (Some(start), Some(end)) =
                                        (parse_log_time(start_str), parse_log_time(&end_time_str))
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
                                global_completed.insert(process.clone());
                            }
                        } else {
                            // No sample: global process
                            global_completed.insert(process.clone());
                        }
                    }
                }
                // Track started processes for staged logic
                if let Some(caps) = Regex::new(r"Starting process > ([^\s]+)")
                    .unwrap()
                    .captures(&line)
                {
                    let proc = caps[1].split(':').last().unwrap_or("").to_string();
                    started_processes.insert(proc);
                }
            }
        }
    }
    // If log is provided, extract process order from log ("Starting process > ...")
    let mut process_order: Vec<String> = Vec::new();
    if let Ok(file) = File::open(nextflow_log_path) {
        let reader = BufReader::new(file);
        let re_start = regex::Regex::new(r"Starting process > ([^\s]+)").unwrap();
        let mut seen = std::collections::HashSet::new();
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Some(caps) = re_start.captures(&line) {
                    let proc = caps[1].split(':').last().unwrap_or("").to_string();
                    if proc == "PASSFAILED" {
                        continue;
                    }
                    if seen.insert(proc.clone()) {
                        process_order.push(proc);
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

    let mut table_rows: Vec<Vec<CellData>> = Vec::new();
    for sample in &sample_ids {
        let mut row = vec![CellData {
            display: sample.clone(),
            hover: None,
        }];
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
        let mut html = String::new();
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

        // Calculate statistics for each process
        let mut progress_stats: Vec<(String, usize, usize, usize, usize)> = Vec::new();
        for i in 1..table_header.len() {
            let process_name = &table_header[i];
            let mut completed = 0;
            let mut running = 0;
            let mut error = 0;
            let mut staged = 0;

            for row in &table_rows {
                if let Some(cell) = row.get(i) {
                    match cell.display.as_str() {
                        "✅" => completed += 1,
                        "⁉️" => error += 1,
                        "🛄" => staged += 1,
                        _ => running += 1, // Anything else is running (HH:MM:SS format)
                    }
                }
            }
            progress_stats.push((process_name.clone(), completed, running, error, staged));
        }

        let mut progress_html = String::new();
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
    }
    .stat-box {
        padding: 8px 12px;
        border-radius: 8px;
        text-align: center;
        font-size: 13px;
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
        for (process_name, completed, running, error, staged) in &progress_stats {
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
        <div class="process-header">
            <div class="process-name">{}</div>
            <div class="process-stats">{}/{} completed ({:.1}%)</div>
        </div>
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

            progress_html.push_str(
                r#"
        </div>
        <div class="stats-grid">
            <div class="stat-box stat-box-completed">
                <span class="stat-number">"#,
            );
            progress_html.push_str(&format!("{}", completed));
            progress_html.push_str(
                r#"</span>
                Completed
            </div>
            <div class="stat-box stat-box-running">
                <span class="stat-number">"#,
            );
            progress_html.push_str(&format!("{}", running));
            progress_html.push_str(
                r#"</span>
                Running
            </div>
            <div class="stat-box stat-box-error">
                <span class="stat-number">"#,
            );
            progress_html.push_str(&format!("{}", error));
            progress_html.push_str(
                r#"</span>
                Failed
            </div>
            <div class="stat-box stat-box-staged">
                <span class="stat-number">"#,
            );
            progress_html.push_str(&format!("{}", staged));
            progress_html.push_str(
                r#"</span>
                Staged
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
            <span>Completed ✅</span>
        </div>
        <div class="legend-item">
            <div class="legend-color progress-running"></div>
            <span>Running ⏱️</span>
        </div>
        <div class="legend-item">
            <div class="legend-color progress-error"></div>
            <span>Failed ⁉️</span>
        </div>
        <div class="legend-item">
            <div class="legend-color progress-staged"></div>
            <span>Staged 🛄</span>
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
