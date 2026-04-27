use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;

#[derive(Debug, Parser)]
#[command(about = "Render HTML-ready status data for a launched MIRA-NF run")]
pub struct NfStatusArgs {
    /// Run identifier written by `mira-oxide serve`
    #[arg(long)]
    pub run_id: Option<String>,

    /// Directory where the web app stores per-run JSON and logs
    #[arg(long, default_value_os_t = default_state_dir())]
    pub state_dir: PathBuf,

    /// Directory containing the MIRA-NF workflow and `.nextflow/history`
    #[arg(long, default_value_os_t = default_pipeline_dir())]
    pub pipeline_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl RunStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub status: RunStatus,
    pub pipeline_dir: PathBuf,
    pub run_dir: PathBuf,
    pub outdir: PathBuf,
    pub profile: String,
    pub samplesheet_path: PathBuf,
    pub nextflow_log: PathBuf,
    pub command_log: PathBuf,
    pub report_path: PathBuf,
    pub timeline_path: PathBuf,
    pub trace_path: PathBuf,
    pub parameters: BTreeMap<String, String>,
    pub extra_nextflow_args: Vec<String>,
    pub launch_command: Vec<String>,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub run_name: Option<String>,
    pub session_uuid: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub started_at: String,
    pub duration: String,
    pub run_name: String,
    pub status: String,
    pub revision: String,
    pub session_uuid: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub record: RunRecord,
    pub effective_status: RunStatus,
    pub run_name: Option<String>,
    pub session_uuid: Option<String>,
    pub nextflow_version: Option<String>,
    pub latest_message: Option<String>,
    pub interesting_lines: Vec<String>,
    pub errors: Vec<String>,
    pub nextflow_log_tail: String,
    pub command_log_tail: String,
    pub history_entry: Option<HistoryEntry>,
    pub recent_history: Vec<HistoryEntry>,
    pub report_exists: bool,
    pub timeline_exists: bool,
    pub trace_exists: bool,
}

#[derive(Debug, Default)]
struct LogSummary {
    run_name: Option<String>,
    session_uuid: Option<String>,
    nextflow_version: Option<String>,
    interesting_lines: Vec<String>,
    errors: Vec<String>,
}

pub fn default_state_dir() -> PathBuf {
    std::env::var_os("MIRA_UI_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".mira-oxide-ui"))
}

pub fn default_pipeline_dir() -> PathBuf {
    std::env::var_os("MIRA_NF_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../MIRA-NF"))
}

pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn run_record_path(state_dir: &Path, run_id: &str) -> PathBuf {
    state_dir.join("runs").join(format!("{run_id}.json"))
}

pub fn persist_run_record(state_dir: &Path, record: &RunRecord) -> Result<()> {
    let runs_dir = state_dir.join("runs");
    fs::create_dir_all(&runs_dir)
        .with_context(|| format!("failed to create {}", runs_dir.display()))?;
    let path = run_record_path(state_dir, &record.id);
    let json = serde_json::to_vec_pretty(record).context("failed to serialize run record")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn load_run_record(state_dir: &Path, run_id: &str) -> Result<RunRecord> {
    let path = run_record_path(state_dir, run_id);
    let bytes = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn load_all_runs(state_dir: &Path) -> Result<Vec<RunRecord>> {
    let runs_dir = state_dir.join("runs");
    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    for entry in
        fs::read_dir(&runs_dir).with_context(|| format!("failed to read {}", runs_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
            continue;
        }
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let record: RunRecord = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        runs.push(record);
    }

    runs.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(runs)
}

pub fn load_latest_run(state_dir: &Path) -> Result<RunRecord> {
    let runs = load_all_runs(state_dir)?;
    runs.into_iter()
        .next()
        .ok_or_else(|| anyhow!("no recorded runs found in {}", state_dir.display()))
}

pub fn build_status_report(
    state_dir: &Path,
    pipeline_dir: &Path,
    run_id: Option<&str>,
) -> Result<StatusReport> {
    let record = if let Some(run_id) = run_id {
        load_run_record(state_dir, run_id)?
    } else {
        load_latest_run(state_dir)?
    };

    let log_summary = parse_nextflow_log(&record.nextflow_log)?;
    let history_entries = parse_history_entries(&pipeline_dir.join(".nextflow").join("history"))?;
    let history_entry = match_history_entry(&record, &history_entries);

    let mut effective_status = record.status.clone();
    if effective_status == RunStatus::Running {
        if let Some(history_entry) = &history_entry {
            effective_status = history_status(history_entry);
        } else if let Some(exit_code) = record.exit_code {
            effective_status = if exit_code == 0 {
                RunStatus::Succeeded
            } else {
                RunStatus::Failed
            };
        }
    }

    let nextflow_log_tail = read_tail(&record.nextflow_log, 160)?;
    let command_log_tail = read_tail(&record.command_log, 120)?;
    let latest_message = log_summary
        .interesting_lines
        .last()
        .cloned()
        .or_else(|| record.last_error.clone());

    Ok(StatusReport {
        effective_status,
        run_name: record
            .run_name
            .clone()
            .or_else(|| log_summary.run_name.clone()),
        session_uuid: record
            .session_uuid
            .clone()
            .or_else(|| log_summary.session_uuid.clone()),
        nextflow_version: log_summary.nextflow_version,
        latest_message,
        interesting_lines: last_n(&log_summary.interesting_lines, 8),
        errors: last_n(&log_summary.errors, 8),
        nextflow_log_tail,
        command_log_tail,
        history_entry,
        recent_history: history_entries.into_iter().rev().take(6).collect(),
        report_exists: record.report_path.exists(),
        timeline_exists: record.timeline_path.exists(),
        trace_exists: record.trace_path.exists(),
        record,
    })
}

pub fn update_run_after_exit(
    state_dir: &Path,
    run_id: &str,
    exit_code: Option<i32>,
) -> Result<RunRecord> {
    let mut record = load_run_record(state_dir, run_id)?;
    record.updated_at = now_utc();
    record.exit_code = exit_code;
    record.status = if exit_code == Some(0) {
        RunStatus::Succeeded
    } else {
        RunStatus::Failed
    };

    let log_summary = parse_nextflow_log(&record.nextflow_log)?;
    if record.run_name.is_none() {
        record.run_name = log_summary.run_name;
    }
    if record.session_uuid.is_none() {
        record.session_uuid = log_summary.session_uuid;
    }
    if let Some(last_error) = log_summary.errors.last() {
        record.last_error = Some(last_error.clone());
    }

    persist_run_record(state_dir, &record)?;
    Ok(record)
}

pub fn render_command_preview(command: &[String]) -> String {
    command
        .iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn history_status(entry: &HistoryEntry) -> RunStatus {
    match entry.status.as_str() {
        "OK" => RunStatus::Succeeded,
        "ERR" => RunStatus::Failed,
        _ => RunStatus::Running,
    }
}

fn parse_history_entries(path: &Path) -> Result<Vec<HistoryEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read Nextflow history {}", path.display()))?;

    let mut entries = Vec::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 7 {
            continue;
        }

        entries.push(HistoryEntry {
            started_at: fields[0].to_string(),
            duration: fields[1].to_string(),
            run_name: fields[2].to_string(),
            status: fields[3].to_string(),
            revision: fields[4].to_string(),
            session_uuid: fields[5].to_string(),
            command: fields[6].to_string(),
        });
    }

    Ok(entries)
}

fn match_history_entry(record: &RunRecord, entries: &[HistoryEntry]) -> Option<HistoryEntry> {
    entries
        .iter()
        .rev()
        .find(|entry| {
            record
                .session_uuid
                .as_ref()
                .is_some_and(|uuid| uuid == &entry.session_uuid)
                || record
                    .run_name
                    .as_ref()
                    .is_some_and(|run_name| run_name == &entry.run_name)
                || entry.command.contains(&record.id)
                || entry
                    .command
                    .contains(record.outdir.to_string_lossy().as_ref())
        })
        .cloned()
}

fn parse_nextflow_log(path: &Path) -> Result<LogSummary> {
    if !path.exists() {
        return Ok(LogSummary::default());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read Nextflow log {}", path.display()))?;

    let mut summary = LogSummary::default();
    for line in contents.lines() {
        if let Some(value) = strip_after(line, "Run name:") {
            summary.run_name = Some(value.to_string());
        }
        if let Some(value) = strip_after(line, "Session UUID:") {
            summary.session_uuid = Some(value.to_string());
        }
        if let Some(value) = line.split("version ").nth(1)
            && line.contains("N E X T F L O W")
        {
            summary.nextflow_version = Some(value.trim().to_string());
        }

        if is_interesting_line(line) {
            summary.interesting_lines.push(line.to_string());
        }
        if is_error_line(line) {
            summary.errors.push(line.to_string());
        }
    }

    Ok(summary)
}

fn is_interesting_line(line: &str) -> bool {
    [
        "Launching `",
        "Session start",
        "Starting process >",
        "Submitted process >",
        "Cached process >",
        "executor >",
        "ERROR",
        "WARN",
        "Execution complete",
        "Session aborted",
        "Workflow started",
        "Task completed >",
        "Task failed >",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn is_error_line(line: &str) -> bool {
    [
        " ERROR ",
        "Session aborted",
        "Caused by:",
        "Cannot invoke",
        "No such file",
        "failed",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn strip_after<'a>(line: &'a str, needle: &str) -> Option<&'a str> {
    line.split_once(needle).map(|(_, value)| value.trim())
}

fn read_tail(path: &Path, lines: usize) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }

    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let collected = contents.lines().rev().take(lines).collect::<Vec<_>>();
    Ok(collected.into_iter().rev().collect::<Vec<_>>().join("\n"))
}

fn last_n(values: &[String], count: usize) -> Vec<String> {
    let start = values.len().saturating_sub(count);
    values[start..].to_vec()
}

fn shell_quote(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }

    if arg.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':' | ',' | '=')
    }) {
        return arg.to_string();
    }

    format!("'{}'", arg.replace('\'', "'\"'\"'"))
}

pub fn ensure_pipeline_exists(pipeline_dir: &Path) -> Result<()> {
    if !pipeline_dir.join("main.nf").exists() {
        bail!("{} does not contain main.nf", pipeline_dir.display());
    }
    if !pipeline_dir.join("nextflow_schema.json").exists() {
        bail!(
            "{} does not contain nextflow_schema.json",
            pipeline_dir.display()
        );
    }
    Ok(())
}
