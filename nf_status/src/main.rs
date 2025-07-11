use chrono::{Datelike, Local};
use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(version, about = "Create a process status table for MIRA-NF runs.")]
struct Args {
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
}

fn main() {
    let args = Args::parse();
    let samplesheet_path = &args.samplesheet;
    let nextflow_log_path = &args.nextflow_log;
    let html_output_path = args.output.as_deref().unwrap_or("nf_status_table.html");

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
    let mut started_processes: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Ok(file) = File::open(nextflow_log_path) {
        let reader = BufReader::new(file);
        let re_submit = Regex::new(r"Submitted process > ([^ ]+) \(([^)]+)\)").unwrap();
        let re_complete = Regex::new(
            r"Task completed > TaskHandler\[.*name: ([^ ]+)(?: \(([^)]+)\))?; status: ([A-Z]+);",
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
                    let process = caps[1].split(':').last().unwrap_or("").to_string();
                    let sample_opt = caps.get(2).map(|m| m.as_str());
                    let status = match &caps[3] {
                        s if s == "COMPLETED" => "completed",
                        s if s == "FAILED" || s == "ERROR" => "error",
                        _ => "",
                    };
                    if !status.is_empty() {
                        if let Some(sample) = sample_opt {
                            if !sample.is_empty() && sample != "1" {
                                let key = (sample.to_string(), process.clone());
                                status_map.insert(key, status.to_string());
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
        return;
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
            .filter(|(i, p)| *i != idx)
            .map(|(_, p)| p.clone())
            .collect();
        table_header.append(&mut rest);
    } else {
        table_header.extend(process_order.iter().cloned());
    }
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    for sample in &sample_ids {
        let mut row = vec![sample.clone()];
        // Use reordered process columns for row
        let process_cols = &table_header[1..];
        for proc in process_cols {
            if proc == "PASSFAILED" {
                continue;
            }
            let key = (sample.clone(), proc.clone());
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
                        row.push(format!("{:02}:{:02}:{:02}", hours, mins, secs));
                        continue;
                    }
                }
                row.push("running".to_string());
            } else if let Some("completed") = status {
                row.push("✅".to_string());
            } else if let Some("error") = status {
                row.push("⁉️".to_string());
            } else {
                row.push("🛄".to_string());
            }
        }
        table_rows.push(row);
    }
    // Output as raw HTML table
    {
        let mut html = String::new();
        html.push_str(r#"<!doctype html>
        <html lang=\"en\">
        <head>
        <meta charset=\"utf-8\" />
        <title>nf_status_table</title>
        <style>
            html, body { height: 100%; margin: 0; padding: 0; box-sizing: border-box; }
            body { min-height: 100vh; width: 100vw; overflow-x: auto; overflow-y: auto; }
            body, table, th, td { font-family: 'Segoe UI', 'Arial', 'Liberation Sans', 'DejaVu Sans', 'sans-serif'; }
            table { border-collapse: collapse; width: 90vw; height-max: 80vw; table-layout: auto; }
            th, td { border: 1px solid #ccc; padding: 4px; text-align: center; }
            th {
                writing-mode: vertical-rl;
                transform: rotate(180deg);
                white-space: nowrap;
                vertical-align: bottom;
                text-align: left;
                height: 160px;
                font-size: 12px;
                padding: 0 2px;
                background: #f8f8f8;
            }
            tr:nth-child(even) { background: #f4f4f4; }
        </style>
        </head>
        <body>
        "#);
        html.push_str("<table>\n<thead>\n<tr>\n");
        for col in &table_header {
            html.push_str(&format!("<th>{}</th>", col));
        }
        html.push_str("</tr>\n</thead>\n<tbody>\n");
        for row in &table_rows {
            html.push_str("<tr>");
            for cell in row {
                html.push_str(&format!("<td>{}</td>", cell));
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</tbody>\n</table>\n</body>\n</html>\n");
        // Ensure output path ends with .html
        let html_output_path = if html_output_path.ends_with(".html") {
            html_output_path.to_string()
        } else {
            format!("{}.html", html_output_path)
        };
        std::fs::write(&html_output_path, html).expect("Failed to write HTML table");
        println!("Raw HTML table written to {}", html_output_path);
    }
    // Print rows for each sample
    fn parse_log_time(s: &str) -> Option<chrono::NaiveDateTime> {
        // Try to parse e.g. "Jun-02 16:39:24.913"
        let fmt = "%b-%d %H:%M:%S%.3f";
        let year = chrono::Local::now().year();
        chrono::NaiveDateTime::parse_from_str(&format!("{} {}", year, s), &format!("%Y {}", fmt))
            .ok()
    }
}
