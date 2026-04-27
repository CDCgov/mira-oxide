use crate::processes::nf_status::{
    RunRecord, RunStatus, StatusReport, build_status_report, default_pipeline_dir,
    default_state_dir, ensure_pipeline_exists, now_utc, persist_run_record, render_command_preview,
    update_run_after_exit,
};
use anyhow::{Context, Result, anyhow, bail};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, Path as RoutePath, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use clap::Parser;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::process::Command;
use uuid::Uuid;

const APP_CSS: &str = r#"
:root {
  --bg: #f3efe5;
  --bg-deep: #e7dece;
  --panel: rgba(255, 252, 247, 0.88);
  --panel-strong: rgba(255, 249, 240, 0.96);
  --ink: #1f2824;
  --muted: #59665e;
  --border: rgba(71, 60, 42, 0.14);
  --accent: #b45b2f;
  --accent-strong: #8d3b16;
  --accent-soft: #f0c5a8;
  --success: #24563d;
  --danger: #8d2c24;
  --shadow: 0 22px 60px rgba(55, 40, 18, 0.12);
  --radius: 20px;
}

* { box-sizing: border-box; }

html {
  min-height: 100%;
  background:
    radial-gradient(circle at top left, rgba(247, 199, 159, 0.6), transparent 28%),
    radial-gradient(circle at top right, rgba(195, 213, 199, 0.75), transparent 24%),
    linear-gradient(180deg, var(--bg) 0%, #efe7da 48%, #ece2d1 100%);
}

body {
  margin: 0;
  color: var(--ink);
  font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Georgia, serif;
  line-height: 1.5;
}

a {
  color: var(--accent-strong);
  text-decoration-thickness: 1px;
  text-underline-offset: 0.16em;
}

main {
  width: min(1160px, calc(100vw - 32px));
  margin: 0 auto;
  padding: 24px 0 56px;
}

.nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 28px;
  padding: 16px 18px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: rgba(255, 250, 242, 0.74);
  backdrop-filter: blur(12px);
  box-shadow: 0 12px 35px rgba(40, 27, 10, 0.08);
}

.brand {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.brand strong {
  font-size: 1rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.brand span,
.eyebrow,
.meta,
.helper,
.empty {
  color: var(--muted);
}

.nav-links {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.nav-links a,
.button,
button,
input[type="submit"] {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 42px;
  padding: 0 16px;
  border-radius: 999px;
  border: 1px solid rgba(132, 73, 36, 0.18);
  background: linear-gradient(180deg, #fff6ed 0%, #fae6d3 100%);
  color: #4b2614;
  font-family: "Avenir Next", "Trebuchet MS", "Gill Sans", sans-serif;
  font-size: 0.95rem;
  font-weight: 600;
  text-decoration: none;
  cursor: pointer;
  transition: transform 140ms ease, box-shadow 140ms ease, border-color 140ms ease;
  box-shadow: 0 10px 24px rgba(107, 58, 26, 0.08);
}

.nav-links a:hover,
.button:hover,
button:hover,
input[type="submit"]:hover {
  transform: translateY(-1px);
  border-color: rgba(141, 59, 22, 0.32);
  box-shadow: 0 14px 28px rgba(107, 58, 26, 0.12);
}

.button.secondary,
button.secondary {
  background: rgba(255, 251, 246, 0.9);
  box-shadow: none;
}

.hero,
.card,
.shell {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--panel);
  backdrop-filter: blur(10px);
  box-shadow: var(--shadow);
}

.hero {
  padding: 28px;
  margin-bottom: 24px;
}

.hero h1,
.card h2,
.card h3 {
  margin: 0 0 12px;
  line-height: 1.1;
}

.hero h1 {
  font-size: clamp(2.3rem, 5vw, 4.2rem);
  max-width: 12ch;
}

.hero p {
  max-width: 65ch;
  margin: 0 0 16px;
  color: var(--muted);
}

.hero-grid,
.grid {
  display: grid;
  gap: 18px;
}

.hero-grid {
  grid-template-columns: 1.3fr 0.7fr;
  align-items: start;
}

.grid.two {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.grid.three {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.card {
  padding: 22px;
}

.stat {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 18px;
  border-radius: 18px;
  border: 1px solid rgba(71, 60, 42, 0.1);
  background: rgba(255, 255, 255, 0.52);
}

.stat strong {
  font-size: 1.6rem;
  line-height: 1;
}

.run-list {
  display: grid;
  gap: 14px;
}

.run-item {
  display: grid;
  gap: 10px;
  padding: 18px;
  border-radius: 18px;
  border: 1px solid rgba(71, 60, 42, 0.1);
  background: rgba(255, 255, 255, 0.55);
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 34px;
  padding: 0 12px;
  border-radius: 999px;
  font-family: "Avenir Next", "Trebuchet MS", "Gill Sans", sans-serif;
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.03em;
}

.status-pending {
  background: rgba(89, 102, 94, 0.12);
  color: #445149;
}

.status-running {
  background: rgba(180, 91, 47, 0.14);
  color: var(--accent-strong);
}

.status-succeeded {
  background: rgba(36, 86, 61, 0.14);
  color: var(--success);
}

.status-failed {
  background: rgba(141, 44, 36, 0.14);
  color: var(--danger);
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.field-grid .field.full {
  grid-column: 1 / -1;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field > label,
.legend {
  font-family: "Avenir Next", "Trebuchet MS", "Gill Sans", sans-serif;
  font-size: 0.92rem;
  font-weight: 700;
  letter-spacing: 0.03em;
}

.field input[type="text"],
.field input[type="email"],
.field input[type="number"],
.field input[type="file"],
.field select,
.field textarea,
.path-box,
.browser-panel,
pre,
code {
  font-family: "Berkeley Mono", "SFMono-Regular", "Cascadia Code", "Menlo", monospace;
}

.field input[type="text"],
.field input[type="email"],
.field input[type="number"],
.field input[type="file"],
.field select,
.field textarea {
  width: 100%;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid rgba(71, 60, 42, 0.14);
  background: rgba(255, 255, 255, 0.78);
  color: var(--ink);
  font-size: 0.96rem;
}

.field textarea {
  min-height: 120px;
  resize: vertical;
}

.field input[readonly] {
  background: rgba(239, 235, 226, 0.9);
}

.checkbox {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 50px;
  padding: 12px 14px;
  border-radius: 16px;
  border: 1px solid rgba(71, 60, 42, 0.14);
  background: rgba(255, 255, 255, 0.78);
}

.checkbox input {
  width: 18px;
  height: 18px;
}

.helper {
  font-size: 0.88rem;
  margin: 0;
}

.form-actions,
.inline-actions,
.path-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
}

.browser-shell {
  display: grid;
  gap: 14px;
}

.path-box,
.browser-panel,
.shell {
  padding: 16px;
  border-radius: 18px;
  border: 1px solid rgba(71, 60, 42, 0.1);
  background: rgba(255, 255, 255, 0.56);
}

.browser-list {
  display: grid;
  gap: 10px;
}

.browser-entry {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 14px;
  border: 1px solid rgba(71, 60, 42, 0.08);
  background: rgba(255, 253, 250, 0.92);
}

.browser-entry small {
  color: var(--muted);
}

.details {
  margin-top: 18px;
}

details > summary {
  cursor: pointer;
  font-family: "Avenir Next", "Trebuchet MS", "Gill Sans", sans-serif;
  font-weight: 700;
}

.command-preview,
.iframe-shell {
  overflow: hidden;
}

pre {
  margin: 0;
  padding: 18px;
  border-radius: 18px;
  background: #221b16;
  color: #f9eadb;
  overflow-x: auto;
  font-size: 0.9rem;
  line-height: 1.5;
}

iframe {
  width: 100%;
  min-height: 880px;
  border: 0;
  background: #fff;
}

.history-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95rem;
}

.history-table th,
.history-table td {
  padding: 10px 12px;
  border-bottom: 1px solid rgba(71, 60, 42, 0.1);
  text-align: left;
  vertical-align: top;
}

.error {
  padding: 16px 18px;
  border-radius: 18px;
  border: 1px solid rgba(141, 44, 36, 0.2);
  background: rgba(252, 235, 231, 0.85);
  color: var(--danger);
}

@media (max-width: 960px) {
  .hero-grid,
  .grid.two,
  .grid.three,
  .field-grid {
    grid-template-columns: 1fr;
  }

  .nav {
    border-radius: 24px;
  }
}
"#;

const NEW_RUN_JS: &str = r#"
const form = document.getElementById('run-form');
const runpathInput = document.querySelector('[name="runpath"]');
const inputPathInput = document.querySelector('[name="input"]');
const outdirInput = document.querySelector('[name="outdir"]');
const browserCurrent = document.getElementById('browser-current');
const browserEntries = document.getElementById('browser-entries');
const browserMeta = document.getElementById('browser-meta');
const parentButton = document.getElementById('browser-parent');
const chooseButton = document.getElementById('browser-choose');
const preview = document.getElementById('command-preview');

function shellQuote(value) {
  if (!value) return \"''\";
  if (/^[A-Za-z0-9_./,:=-]+$/.test(value)) return value;
  return `'${String(value).replace(/'/g, `'\"'\"'`)}'`;
}

function currentFormValues() {
  const data = new FormData(form);
  const values = new Map();
  for (const [key, value] of data.entries()) {
    if (value instanceof File) continue;
    values.set(key, String(value));
  }
  return values;
}

function normalizeDir(path) {
  return String(path || '').replace(/\/+$/, '');
}

function applyRunDirectory(path) {
  const clean = normalizeDir(path);
  runpathInput.value = clean;
  inputPathInput.value = clean ? `${clean}/samplesheet.csv` : '';
  if (!outdirInput.dataset.userEdited || !outdirInput.value) {
    outdirInput.value = clean ? `${clean}/results` : '';
  }
  updatePreview();
}

function applySamplesheetPath(path) {
  const clean = String(path || '');
  const runDir = clean.replace(/\/samplesheet\.csv$/i, '');
  inputPathInput.value = clean;
  runpathInput.value = runDir;
  if (!outdirInput.dataset.userEdited || !outdirInput.value) {
    outdirInput.value = runDir ? `${runDir}/results` : '';
  }
  updatePreview();
}

function updatePreview() {
  const values = currentFormValues();
  const command = ['nextflow', 'run', values.get('pipeline_hint') || '/opt/MIRA-NF/main.nf'];
  const profile = values.get('profile');
  if (profile) {
    command.push('-profile', profile);
  }
  const extra = values.get('extra_nextflow_args');
  if (extra) {
    command.push(extra);
  }

  const ordered = Array.from(document.querySelectorAll('[data-param-name]'))
    .map((element) => element.getAttribute('data-param-name'))
    .filter(Boolean);

  for (const name of ordered) {
    const value = values.get(name);
    if (!value) continue;
    command.push(`--${name}`, value);
  }

  preview.textContent = command.map(shellQuote).join(' ');
}

async function loadDirectory(path = '') {
  const response = await fetch(`/api/directories?path=${encodeURIComponent(path)}`);
  const payload = await response.json();

  browserCurrent.textContent = payload.current;
  browserMeta.textContent = payload.has_fastq_layout
    ? (payload.samplesheet_exists
      ? 'This directory contains fastqs or fastq_pass and already has samplesheet.csv.'
      : 'This directory contains fastqs or fastq_pass. You can use it as the run directory or select/create samplesheet.csv here.')
    : 'Browse downward until you reach a directory that contains fastqs or fastq_pass.';

  parentButton.disabled = !payload.parent;
  parentButton.dataset.path = payload.parent || '';
  chooseButton.disabled = !payload.has_fastq_layout;
  chooseButton.dataset.path = payload.current;

  browserEntries.innerHTML = '';
  if (!payload.entries.length) {
    const empty = document.createElement('div');
    empty.className = 'empty';
    empty.textContent = 'No child directories here.';
    browserEntries.appendChild(empty);
    return;
  }

  for (const entry of payload.entries) {
    const row = document.createElement('div');
    row.className = 'browser-entry';

    const info = document.createElement('div');
    info.innerHTML = `<strong>${entry.name}</strong><br><small>${entry.has_fastq_layout ? 'Contains fastqs/fastq_pass' : 'Browse into directory'}${entry.samplesheet_exists ? ' · has samplesheet.csv' : ''}</small>`;

    const actions = document.createElement('div');
    actions.className = 'inline-actions';

    const open = document.createElement('button');
    open.type = 'button';
    open.className = 'secondary';
    open.textContent = 'Open';
    open.addEventListener('click', () => loadDirectory(entry.path));
    actions.appendChild(open);

    if (entry.samplesheet_exists) {
      const selectSamplesheet = document.createElement('button');
      selectSamplesheet.type = 'button';
      selectSamplesheet.textContent = 'Select samplesheet';
      selectSamplesheet.addEventListener('click', () => applySamplesheetPath(entry.samplesheet_path));
      actions.appendChild(selectSamplesheet);
    }

    row.appendChild(info);
    row.appendChild(actions);
    browserEntries.appendChild(row);
  }
}

parentButton?.addEventListener('click', () => loadDirectory(parentButton.dataset.path || ''));
chooseButton?.addEventListener('click', () => {
  applyRunDirectory(chooseButton.dataset.path || '');
});
inputPathInput?.addEventListener('input', () => {
  const value = inputPathInput.value.trim();
  if (value.endsWith('/samplesheet.csv')) {
    applySamplesheetPath(value);
  } else {
    updatePreview();
  }
});
outdirInput?.addEventListener('input', () => {
  outdirInput.dataset.userEdited = 'true';
  updatePreview();
});

form?.addEventListener('input', updatePreview);
document.addEventListener('DOMContentLoaded', () => {
  updatePreview();
  loadDirectory(runpathInput.value || '');
});
"#;

#[derive(Debug, Parser, Clone)]
#[command(about = "Serve a local browser UI for launching and monitoring MIRA-NF runs")]
pub struct ServeArgs {
    /// Address to bind the local web server to
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub listen: String,

    /// Root directory exposed in the in-app directory browser
    #[arg(long, default_value_os_t = default_data_root())]
    pub data_root: PathBuf,

    /// Directory containing the MIRA-NF pipeline
    #[arg(long, default_value_os_t = default_pipeline_dir())]
    pub pipeline_dir: PathBuf,

    /// Directory where run records and logs should be written
    #[arg(long, default_value_os_t = default_state_dir())]
    pub state_dir: PathBuf,

    /// Nextflow executable to invoke
    #[arg(long, default_value = "nextflow")]
    pub nextflow_bin: String,

    /// Default Nextflow profile string shown in the UI
    #[arg(long)]
    pub default_profile: Option<String>,
}

#[derive(Debug, Clone)]
struct AppContext {
    data_root: PathBuf,
    pipeline_dir: PathBuf,
    state_dir: PathBuf,
    nextflow_bin: String,
    default_profile: String,
    schema: PipelineSchema,
}

#[derive(Debug, Clone)]
struct PipelineSchema {
    title: String,
    description: String,
    sections: Vec<SchemaSection>,
    ordered_fields: Vec<SchemaField>,
}

#[derive(Debug, Clone)]
struct SchemaSection {
    title: String,
    description: Option<String>,
    fields: Vec<SchemaField>,
}

#[derive(Debug, Clone)]
struct SchemaField {
    name: String,
    kind: String,
    description: Option<String>,
    help_text: Option<String>,
    enum_values: Vec<String>,
    format: Option<String>,
    hidden: bool,
    default_value: Option<String>,
    required: bool,
}

#[derive(Debug, Deserialize)]
struct RawSchema {
    title: String,
    description: String,
    #[serde(rename = "$defs")]
    defs: BTreeMap<String, RawSchemaSection>,
    #[serde(default, rename = "allOf")]
    all_of: Vec<RawSchemaRef>,
}

#[derive(Debug, Deserialize)]
struct RawSchemaRef {
    #[serde(rename = "$ref")]
    reference: String,
}

#[derive(Debug, Deserialize)]
struct RawSchemaSection {
    title: String,
    description: Option<String>,
    #[serde(default)]
    required: Vec<String>,
    properties: BTreeMap<String, RawSchemaProperty>,
}

#[derive(Debug, Deserialize)]
struct RawSchemaProperty {
    #[serde(rename = "type")]
    kind: String,
    description: Option<String>,
    #[serde(rename = "enum")]
    enum_values: Option<Vec<String>>,
    format: Option<String>,
    hidden: Option<bool>,
    default: Option<Value>,
    help_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DirectoryQuery {
    path: Option<String>,
}

#[derive(Debug, Serialize)]
struct DirectoryListing {
    current: String,
    parent: Option<String>,
    has_fastq_layout: bool,
    samplesheet_exists: bool,
    entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Serialize)]
struct DirectoryEntry {
    name: String,
    path: String,
    has_fastq_layout: bool,
    samplesheet_exists: bool,
    samplesheet_path: String,
}

pub async fn serve(args: ServeArgs) -> Result<()> {
    let data_root = canonicalized_existing_dir(&args.data_root, true)?;
    let pipeline_dir = canonicalized_existing_dir(&args.pipeline_dir, false)?;
    ensure_pipeline_exists(&pipeline_dir)?;
    fs::create_dir_all(&args.state_dir)
        .with_context(|| format!("failed to create {}", args.state_dir.display()))?;
    let state_dir = args
        .state_dir
        .canonicalize()
        .with_context(|| format!("failed to open {}", args.state_dir.display()))?;
    let schema = load_pipeline_schema(&pipeline_dir.join("nextflow_schema.json"))?;

    let context = Arc::new(AppContext {
        data_root,
        pipeline_dir,
        state_dir,
        nextflow_bin: args.nextflow_bin,
        default_profile: args
            .default_profile
            .unwrap_or_else(default_profile_for_current_arch),
        schema,
    });

    let router = Router::new()
        .route("/", get(home_page))
        .route("/runs/new", get(new_run_page))
        .route("/runs", post(create_run))
        .route("/runs/{id}", get(run_page))
        .route("/runs/{id}/nf-status.html", get(nf_status_document))
        .route("/runs/{id}/artifacts/{kind}", get(run_artifact))
        .route("/api/directories", get(directory_listing))
        .route("/healthz", get(|| async { Html("ok") }))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(context);

    let listener = tokio::net::TcpListener::bind(&args.listen)
        .await
        .with_context(|| format!("failed to bind {}", args.listen))?;
    axum::serve(listener, router)
        .await
        .context("web server exited unexpectedly")
}

pub fn render_nf_status_document(
    report: &StatusReport,
    auto_refresh_seconds: Option<u64>,
) -> String {
    let title = format!("Nextflow status · {}", report.record.id);
    let head = html! {
        style { (APP_CSS) }
        @if let Some(refresh) = auto_refresh_seconds {
            meta http-equiv="refresh" content=(refresh);
        }
    };
    let body = html! {
        main {
            div class="card" {
                div class="inline-actions" style="justify-content: space-between;" {
                    div {
                        p class="eyebrow" { "mira-oxide nf-status" }
                        h1 style="margin: 0;" { "Run " (report.record.id) }
                    }
                    (status_badge(&report.effective_status))
                }

                p class="helper" style="margin-top: 12px;" {
                    "This HTML is produced from the same status renderer used by the browser UI and the "
                    code { "mira-oxide nf-status" }
                    " command."
                }

                div class="grid two" style="margin-top: 18px;" {
                    div class="stat" {
                        span class="meta" { "Run name" }
                        strong { (report.run_name.as_deref().unwrap_or("pending")) }
                    }
                    div class="stat" {
                        span class="meta" { "Session UUID" }
                        strong style="font-size: 1rem;" {
                            (report.session_uuid.as_deref().unwrap_or("not discovered yet"))
                        }
                    }
                    div class="stat" {
                        span class="meta" { "Profile" }
                        strong style="font-size: 1rem;" { (&report.record.profile) }
                    }
                    div class="stat" {
                        span class="meta" { "Nextflow" }
                        strong style="font-size: 1rem;" {
                            (report.nextflow_version.as_deref().unwrap_or("not detected yet"))
                        }
                    }
                }
            }

            @if let Some(message) = &report.latest_message {
                div class="card" style="margin-top: 20px;" {
                    h2 { "Latest signal" }
                    p style="margin: 0; font-family: 'Berkeley Mono', 'SFMono-Regular', monospace;" {
                        (message)
                    }
                }
            }

            @if !report.errors.is_empty() {
                div class="card" style="margin-top: 20px;" {
                    h2 { "Recent errors" }
                    div class="error" {
                        @for line in &report.errors {
                            div { (line) }
                        }
                    }
                }
            }

            div class="grid two" style="margin-top: 20px;" {
                div class="card" {
                    h2 { "Interesting log lines" }
                    @if report.interesting_lines.is_empty() {
                        p class="empty" { "No status lines yet." }
                    } @else {
                        pre { @for line in &report.interesting_lines { (line) "\n" } }
                    }
                }
                div class="card" {
                    h2 { "Artifacts" }
                    div class="run-list" {
                        div class="run-item" {
                            strong { "Output directory" }
                            code { (report.record.outdir.display()) }
                        }
                        div class="run-item" {
                            strong { "Run directory" }
                            code { (report.record.run_dir.display()) }
                        }
                        div class="run-item" {
                            strong { "Report files" }
                            p class="helper" style="margin: 0;" {
                                "execution report: " (exists_label(report.report_exists))
                                " · timeline: " (exists_label(report.timeline_exists))
                                " · trace: " (exists_label(report.trace_exists))
                            }
                        }
                    }
                }
            }

            div class="card" style="margin-top: 20px;" {
                h2 { "Launch command" }
                pre { (render_command_preview(&report.record.launch_command)) }
            }

            div class="grid two" style="margin-top: 20px;" {
                div class="card" {
                    h2 { "Nextflow log tail" }
                    @if report.nextflow_log_tail.is_empty() {
                        p class="empty" { "No Nextflow log content yet." }
                    } @else {
                        pre { (&report.nextflow_log_tail) }
                    }
                }
                div class="card" {
                    h2 { "Command stdout/stderr tail" }
                    @if report.command_log_tail.is_empty() {
                        p class="empty" { "The command log is still empty." }
                    } @else {
                        pre { (&report.command_log_tail) }
                    }
                }
            }

            @if !report.recent_history.is_empty() {
                div class="card" style="margin-top: 20px;" {
                    h2 { "Recent Nextflow history" }
                    table class="history-table" {
                        thead {
                            tr {
                                th { "Started" }
                                th { "Run" }
                                th { "Status" }
                                th { "Duration" }
                            }
                        }
                        tbody {
                            @for entry in &report.recent_history {
                                tr {
                                    td { (&entry.started_at) }
                                    td { (&entry.run_name) }
                                    td { (&entry.status) }
                                    td { (&entry.duration) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    layout_document(&title, head, body)
}

async fn home_page(State(context): State<Arc<AppContext>>) -> Response {
    respond(render_home_page(&context))
}

async fn new_run_page(State(context): State<Arc<AppContext>>) -> Response {
    respond(render_new_run(&context))
}

async fn run_page(
    State(context): State<Arc<AppContext>>,
    RoutePath(id): RoutePath<String>,
) -> Response {
    respond(render_run_page(&context, &id))
}

async fn nf_status_document(
    State(context): State<Arc<AppContext>>,
    RoutePath(id): RoutePath<String>,
) -> Response {
    let response = || -> Result<String> {
        let report = build_status_report(&context.state_dir, &context.pipeline_dir, Some(&id))?;
        let refresh = if report.effective_status.is_finished() {
            None
        } else {
            Some(5)
        };
        Ok(render_nf_status_document(&report, refresh))
    };

    match response() {
        Ok(html) => Html(html).into_response(),
        Err(error) => error_response(&error),
    }
}

async fn run_artifact(
    State(context): State<Arc<AppContext>>,
    RoutePath((id, kind)): RoutePath<(String, String)>,
) -> Response {
    let response = || -> Result<Response> {
        let record = crate::processes::nf_status::load_run_record(&context.state_dir, &id)?;
        let (path, content_type) = match kind.as_str() {
            "report" => (record.report_path, "text/html; charset=utf-8"),
            "timeline" => (record.timeline_path, "text/html; charset=utf-8"),
            "trace" => (record.trace_path, "text/plain; charset=utf-8"),
            "nextflow-log" => (record.nextflow_log, "text/plain; charset=utf-8"),
            "command-log" => (record.command_log, "text/plain; charset=utf-8"),
            "samplesheet" => (record.samplesheet_path, "text/plain; charset=utf-8"),
            _ => bail!("unknown artifact `{kind}`"),
        };

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(content_type).context("invalid content type")?,
        );
        Ok((headers, contents).into_response())
    };

    match response() {
        Ok(response) => response,
        Err(error) => error_response(&error),
    }
}

async fn directory_listing(
    State(context): State<Arc<AppContext>>,
    Query(query): Query<DirectoryQuery>,
) -> Response {
    let response = || -> Result<axum::Json<DirectoryListing>> {
        let listing = build_directory_listing(&context, query.path.as_deref())?;
        Ok(axum::Json(listing))
    };

    match response() {
        Ok(json) => json.into_response(),
        Err(error) => error_response(&error),
    }
}

async fn create_run(State(context): State<Arc<AppContext>>, mut multipart: Multipart) -> Response {
    let response = async move {
        let mut values = BTreeMap::<String, String>::new();
        let mut samplesheet_bytes = None;

        while let Some(field) = multipart
            .next_field()
            .await
            .context("failed to read form field")?
        {
            let name = field.name().unwrap_or_default().to_string();
            if name == "samplesheet_upload" {
                let data = field
                    .bytes()
                    .await
                    .context("failed to read uploaded samplesheet")?;
                if !data.is_empty() {
                    samplesheet_bytes = Some(data.to_vec());
                }
                continue;
            }

            values.insert(
                name,
                field.text().await.context("failed to decode text field")?,
            );
        }

        let input_value = values
            .get("input")
            .map(String::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let run_dir_value = values
            .get("runpath")
            .map(String::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();

        let (run_dir, samplesheet_path) = if let Some(bytes) = samplesheet_bytes {
            if run_dir_value.is_empty() {
                bail!("choose a destination run directory for the uploaded samplesheet.csv");
            }
            let run_dir = resolve_within_root(&context.data_root, &run_dir_value)?;
            if !has_fastq_layout(&run_dir) {
                bail!(
                    "{} does not contain fastqs or fastq_pass",
                    run_dir.display()
                );
            }
            let samplesheet_path = run_dir.join("samplesheet.csv");
            tokio::fs::write(&samplesheet_path, bytes)
                .await
                .with_context(|| format!("failed to write {}", samplesheet_path.display()))?;
            (run_dir, samplesheet_path)
        } else if !input_value.is_empty() {
            let samplesheet_path = resolve_file_within_root(&context.data_root, &input_value)?;
            if samplesheet_path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                != Some("samplesheet.csv")
            {
                bail!("select a file named samplesheet.csv");
            }
            let run_dir = samplesheet_path
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| {
                    anyhow!("could not determine run directory from samplesheet path")
                })?;
            if !has_fastq_layout(&run_dir) {
                bail!(
                    "{} does not contain fastqs or fastq_pass",
                    run_dir.display()
                );
            }
            (run_dir, samplesheet_path)
        } else {
            bail!("select an existing samplesheet.csv or upload one");
        };

        let outdir = if let Some(value) = values
            .get("outdir")
            .filter(|value| !value.trim().is_empty())
        {
            PathBuf::from(value)
        } else {
            run_dir.join("results")
        };

        tokio::fs::create_dir_all(outdir.join("pipeline_info"))
            .await
            .with_context(|| format!("failed to create {}", outdir.display()))?;
        tokio::fs::create_dir_all(context.state_dir.join("logs"))
            .await
            .with_context(|| format!("failed to create {}", context.state_dir.display()))?;

        values.insert(
            "runpath".to_string(),
            run_dir.to_string_lossy().into_owned(),
        );
        values.insert(
            "input".to_string(),
            samplesheet_path.to_string_lossy().into_owned(),
        );
        values.insert("outdir".to_string(), outdir.to_string_lossy().into_owned());

        let profile = values
            .remove("profile")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| context.default_profile.clone());
        let extra_nextflow_args = parse_extra_args(values.remove("extra_nextflow_args"))?;
        let run_id = Uuid::new_v4().to_string();
        let nextflow_log = context
            .state_dir
            .join("logs")
            .join(format!("{run_id}.nextflow.log"));
        let command_log = context
            .state_dir
            .join("logs")
            .join(format!("{run_id}.command.log"));
        let report_path = outdir
            .join("pipeline_info")
            .join(format!("execution_report_{run_id}.html"));
        let timeline_path = outdir
            .join("pipeline_info")
            .join(format!("execution_timeline_{run_id}.html"));
        let trace_path = outdir
            .join("pipeline_info")
            .join(format!("execution_trace_{run_id}.txt"));

        let mut parameters = BTreeMap::new();
        for field in &context.schema.ordered_fields {
            let value = values
                .get(&field.name)
                .cloned()
                .or_else(|| field.default_value.clone())
                .unwrap_or_default();
            if value.trim().is_empty() {
                continue;
            }
            parameters.insert(field.name.clone(), value);
        }

        for required in ["input", "outdir", "runpath", "e"] {
            if !parameters.contains_key(required) {
                bail!("missing required parameter `{required}`");
            }
        }

        let mut record = RunRecord {
            id: run_id.clone(),
            created_at: now_utc(),
            updated_at: now_utc(),
            status: RunStatus::Pending,
            pipeline_dir: context.pipeline_dir.clone(),
            run_dir,
            outdir,
            profile,
            samplesheet_path,
            nextflow_log,
            command_log,
            report_path,
            timeline_path,
            trace_path,
            parameters,
            extra_nextflow_args,
            launch_command: Vec::new(),
            pid: None,
            exit_code: None,
            run_name: None,
            session_uuid: None,
            last_error: None,
        };

        record.launch_command = build_nextflow_command(&context, &record);
        persist_run_record(&context.state_dir, &record)?;

        let log_file = fs::File::create(&record.command_log)
            .with_context(|| format!("failed to create {}", record.command_log.display()))?;
        let log_file_clone = log_file
            .try_clone()
            .context("failed to duplicate command log handle")?;

        let mut command = Command::new(&context.nextflow_bin);
        for arg in record.launch_command.iter().skip(1) {
            command.arg(arg);
        }
        command
            .current_dir(&context.pipeline_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_file_clone));

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                record.status = RunStatus::Failed;
                record.updated_at = now_utc();
                record.last_error = Some(error.to_string());
                persist_run_record(&context.state_dir, &record)?;
                return Err(anyhow!(error).context("failed to spawn nextflow"));
            }
        };

        record.status = RunStatus::Running;
        record.updated_at = now_utc();
        record.pid = child.id();
        persist_run_record(&context.state_dir, &record)?;

        let state_dir = context.state_dir.clone();
        let run_id_for_task = run_id.clone();
        tokio::spawn(async move {
            let exit_code = match child.wait().await {
                Ok(status) => status.code(),
                Err(_) => None,
            };
            let _ = update_run_after_exit(&state_dir, &run_id_for_task, exit_code);
        });

        Ok::<_, anyhow::Error>(Redirect::to(&format!("/runs/{run_id}")))
    }
    .await;

    match response {
        Ok(redirect) => redirect.into_response(),
        Err(error) => error_response(&error),
    }
}

fn render_home_page(context: &AppContext) -> Result<String> {
    let runs = crate::processes::nf_status::load_all_runs(&context.state_dir)?;
    let running = runs
        .iter()
        .filter(|run| run.status == RunStatus::Running)
        .count();

    let body = html! {
        main {
            (nav())
            div class="hero" {
                div class="hero-grid" {
                    div {
                        p class="eyebrow" { "Rust server · browser UI · Nextflow launcher" }
                        h1 { "A local cockpit for MIRA-NF runs." }
                        p {
                            "Launch "
                            code { "main.nf" }
                            " from a single form, deposit the uploaded "
                            code { "samplesheet.csv" }
                            " directly into the selected run folder, and monitor status from an HTML page rendered by "
                            code { "mira-oxide nf-status" }
                            "."
                        }
                        p class="helper" { (&context.schema.description) }
                        div class="form-actions" {
                            a href="/runs/new" { "New Run" }
                        }
                    }
                    div class="grid" {
                        div class="stat" {
                            span class="meta" { "Data root" }
                            strong style="font-size: 1rem;" { (context.data_root.display()) }
                        }
                        div class="stat" {
                            span class="meta" { "Pipeline" }
                            strong style="font-size: 1rem;" { (context.pipeline_dir.display()) }
                        }
                        div class="stat" {
                            span class="meta" { "Recent runs" }
                            strong { (runs.len()) }
                        }
                        div class="stat" {
                            span class="meta" { "Currently running" }
                            strong { (running) }
                        }
                    }
                }
            }

            div class="grid two" {
                div class="card" {
                    h2 { "What the app handles" }
                    div class="run-list" {
                        div class="run-item" {
                            strong { "Schema-driven inputs" }
                            p class="helper" style="margin: 0;" {
                                "The form is generated from MIRA-NF's "
                                code { "nextflow_schema.json" }
                                ", including advanced parameters and hidden defaults."
                            }
                        }
                        div class="run-item" {
                            strong { "Samplesheet placement" }
                            p class="helper" style="margin: 0;" {
                                "The uploaded samplesheet is written into the chosen run folder so it sits beside "
                                code { "fastqs" }
                                " or "
                                code { "fastq_pass" }
                                "."
                            }
                        }
                        div class="run-item" {
                            strong { "Status rendering" }
                            p class="helper" style="margin: 0;" {
                                "Each run gets a dedicated Nextflow log and report paths. The monitoring page uses the same HTML renderer exposed by "
                                code { "mira-oxide nf-status" }
                                "."
                            }
                        }
                    }
                }

                div class="card" {
                    h2 { "Recent runs" }
                    @if runs.is_empty() {
                        p class="empty" { "No runs have been launched from the UI yet." }
                    } @else {
                        div class="run-list" {
                            @for run in runs.iter().take(8) {
                                div class="run-item" {
                                    div class="inline-actions" style="justify-content: space-between;" {
                                        strong { (&run.id) }
                                        (status_badge(&run.status))
                                    }
                                    p class="helper" style="margin: 0;" {
                                        "Created " (display_timestamp(run.created_at))
                                    }
                                    p class="helper" style="margin: 0;" {
                                        code { (run.outdir.display()) }
                                    }
                                    div class="inline-actions" {
                                        a href={ "/runs/" (run.id) } { "Open Status" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(layout_document(
        "MIRA-Oxide UI",
        html! { style { (APP_CSS) } },
        body,
    ))
}

fn render_new_run(context: &AppContext) -> Result<String> {
    let profile_options = [
        "podman,local",
        "podman_arm64,local",
        "docker,local",
        "docker_arm64,local",
        "singularity,local",
        "singularity_arm64,local",
        "local",
        "sge",
        "slurm",
    ];

    let mut visible_sections = Vec::new();
    let mut hidden_sections = Vec::new();
    for section in &context.schema.sections {
        let visible = section
            .fields
            .iter()
            .filter(|field| {
                !field.hidden && !matches!(field.name.as_str(), "input" | "runpath" | "outdir")
            })
            .cloned()
            .collect::<Vec<_>>();
        let hidden = section
            .fields
            .iter()
            .filter(|field| field.hidden)
            .cloned()
            .collect::<Vec<_>>();
        visible_sections.push((section.clone(), visible));
        if !hidden.is_empty() {
            hidden_sections.push((section.clone(), hidden));
        }
    }

    let body = html! {
        main {
            (nav())
            div class="hero" {
                p class="eyebrow" { "Launch a run" }
                h1 { "Compose the Nextflow command without leaving the browser." }
                p {
                    "Choose a mounted run directory, upload "
                    code { "samplesheet.csv" }
                    ", review the exact command preview, and submit. The server writes the samplesheet into the chosen folder and launches "
                    code { "main.nf" }
                    " from "
                    code { (context.pipeline_dir.display()) }
                    "."
                }
                p class="helper" { (&context.schema.title) }
            }

            form id="run-form" class="grid" action="/runs" method="post" enctype="multipart/form-data" {
                input type="hidden" name="pipeline_hint" value=(context.pipeline_dir.join("main.nf").display());

                div class="card" {
                    h2 { "Run environment" }
                    div class="field-grid" {
                        div class="field" {
                            label for="profile" { "Profile" }
                            input id="profile" type="text" name="profile" list="profile-options" value=(context.default_profile);
                            datalist id="profile-options" {
                                @for option in profile_options {
                                    option value=(option) {}
                                }
                            }
                            p class="helper" {
                                "Comma-separated Nextflow profile string. The default is chosen from the current CPU architecture."
                            }
                        }
                        div class="field" {
                            label for="extra_nextflow_args" { "Extra Nextflow CLI args" }
                            input id="extra_nextflow_args" type="text" name="extra_nextflow_args" placeholder="-resume";
                            p class="helper" { "Optional raw flags appended after the built-in report and timeline arguments." }
                        }
                    }
                }

                div class="card" {
                    h2 { "Run directory and samplesheet" }
                    div class="grid two" {
                        div class="browser-shell" {
                            div class="path-box" {
                                div class="legend" { "Data root" }
                                p class="helper" style="margin-top: 8px;" { (context.data_root.display()) }
                            }
                            div class="browser-panel" {
                                div class="legend" { "Directory browser" }
                                p id="browser-current" class="helper" style="margin-top: 8px;" {
                                    (context.data_root.display())
                                }
                                p id="browser-meta" class="helper" style="margin-top: 8px;" {
                                    "Browse into a folder that contains fastqs or fastq_pass."
                                }
                                div class="path-actions" style="margin-top: 14px;" {
                                    button id="browser-parent" type="button" class="secondary" { "Up" }
                                    button id="browser-choose" type="button" { "Use This Directory" }
                                }
                                div id="browser-entries" class="browser-list" style="margin-top: 16px;" {}
                            }
                        }

                        div class="field-grid" {
                            div class="field full" {
                                label for="samplesheet_upload" { "Upload samplesheet.csv" }
                                input id="samplesheet_upload" type="file" name="samplesheet_upload" accept=".csv,text/csv";
                                p class="helper" {
                                    "If you upload a file, the server saves it as "
                                    code { "samplesheet.csv" }
                                    " in the selected run directory."
                                }
                            }
                            div class="field full" {
                                label for="runpath" { "Run directory" }
                                input id="runpath" type="text" name="runpath" readonly value="";
                                p class="helper" { "Chosen with the browser above. This must resolve under the mounted data root." }
                            }
                            div class="field full" {
                                label for="input" { "Input samplesheet path" }
                                input id="input" type="text" name="input" readonly value="";
                            }
                            div class="field full" {
                                label for="outdir" { "Output directory" }
                                input id="outdir" type="text" name="outdir" value="";
                                p class="helper" { "Defaults to runpath/results. You can override it if needed." }
                            }
                        }
                    }
                }

                @for (section, fields) in &visible_sections {
                    @if !fields.is_empty() {
                        div class="card" {
                            h2 { (&section.title) }
                            @if let Some(description) = &section.description {
                                p class="helper" { (description) }
                            }
                            div class="field-grid" {
                                @for field in fields {
                                    (render_schema_field(field))
                                }
                            }
                        }
                    }
                }

                @if !hidden_sections.is_empty() {
                    div class="card details" {
                        details {
                            summary { "Advanced and hidden parameters" }
                            p class="helper" style="margin-top: 12px;" {
                                "These fields are present so the form can cover the full schema and defaults used for "
                                code { "main.nf" }
                                "."
                            }
                            @for (section, fields) in &hidden_sections {
                                div class="card" style="margin-top: 16px; background: rgba(255,255,255,0.42);" {
                                    h3 { (&section.title) }
                                    div class="field-grid" {
                                        @for field in fields {
                                            (render_schema_field(field))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div class="card command-preview" {
                    h2 { "Command preview" }
                    p class="helper" {
                        "The actual launch command adds a per-run "
                        code { "-log" }
                        ", "
                        code { "-with-report" }
                        ", "
                        code { "-with-timeline" }
                        ", and "
                        code { "-with-trace" }
                        " path automatically."
                    }
                    pre id="command-preview" {}
                }

                div class="form-actions" {
                    input type="submit" value="Launch MIRA-NF";
                    a class="button secondary" href="/" { "Back Home" }
                }
            }
        }
    };

    Ok(layout_document(
        "New Run · MIRA-Oxide UI",
        html! {
            style { (APP_CSS) }
            script { (PreEscaped(NEW_RUN_JS)) }
        },
        body,
    ))
}

fn render_run_page(context: &AppContext, id: &str) -> Result<String> {
    let report = build_status_report(&context.state_dir, &context.pipeline_dir, Some(id))?;
    let body = html! {
        main {
            (nav())
            div class="hero" {
                div class="inline-actions" style="justify-content: space-between;" {
                    div {
                        p class="eyebrow" { "Run status" }
                        h1 { "Nextflow run " (id) }
                        p {
                            "This page embeds the HTML produced by "
                            code { "mira-oxide nf-status" }
                            " and links the dedicated log files and generated reports for this run."
                        }
                    }
                    (status_badge(&report.effective_status))
                }
            }

            div class="grid two" {
                div class="card" {
                    h2 { "Run summary" }
                    div class="run-list" {
                        (summary_item("Run directory", &report.record.run_dir.display().to_string()))
                        (summary_item("Output directory", &report.record.outdir.display().to_string()))
                        (summary_item("Profile", &report.record.profile))
                        (summary_item("Created", &display_timestamp(report.record.created_at)))
                        (summary_item("Updated", &display_timestamp(report.record.updated_at)))
                        @if let Some(run_name) = &report.run_name {
                            (summary_item("Nextflow run name", run_name))
                        }
                        @if let Some(session_uuid) = &report.session_uuid {
                            (summary_item("Session UUID", session_uuid))
                        }
                    }
                }

                div class="card" {
                    h2 { "Files" }
                    div class="run-list" {
                        div class="run-item" {
                            strong { "Logs" }
                            div class="inline-actions" {
                                a href={ "/runs/" (id) "/artifacts/nextflow-log" } target="_blank" { "Nextflow log" }
                                a href={ "/runs/" (id) "/artifacts/command-log" } target="_blank" { "Command log" }
                                a href={ "/runs/" (id) "/artifacts/samplesheet" } target="_blank" { "Samplesheet" }
                            }
                        }
                        div class="run-item" {
                            strong { "Reports" }
                            div class="inline-actions" {
                                @if report.report_exists {
                                    a href={ "/runs/" (id) "/artifacts/report" } target="_blank" { "Execution report" }
                                }
                                @if report.timeline_exists {
                                    a href={ "/runs/" (id) "/artifacts/timeline" } target="_blank" { "Timeline" }
                                }
                                @if report.trace_exists {
                                    a href={ "/runs/" (id) "/artifacts/trace" } target="_blank" { "Trace" }
                                }
                            }
                        }
                        div class="run-item" {
                            strong { "Command" }
                            pre { (render_command_preview(&report.record.launch_command)) }
                        }
                    }
                }
            }

            div class="card iframe-shell" style="margin-top: 20px;" {
                h2 { "Embedded nf-status HTML" }
                iframe src={ "/runs/" (id) "/nf-status.html" } title="nf-status output" {}
            }
        }
    };

    Ok(layout_document(
        &format!("Run {id} · MIRA-Oxide UI"),
        html! { style { (APP_CSS) } },
        body,
    ))
}

fn render_schema_field(field: &SchemaField) -> Markup {
    let help = field.help_text.as_ref().or(field.description.as_ref());
    let field_id = format!("field-{}", field.name);
    let label = if field.required {
        format!("{} *", field.name)
    } else {
        field.name.clone()
    };

    let field_markup = match field.kind.as_str() {
        "boolean" => {
            let checked = field.default_value.as_deref() == Some("true");
            html! {
                div class="checkbox" {
                    input type="hidden" name=(field.name) value="false";
                    input
                        id=(field_id)
                        type="checkbox"
                        name=(field.name)
                        value="true"
                        checked[checked]
                        data-param-name=(field.name);
                    label for=(field_id) style="margin: 0;" { (label) }
                }
            }
        }
        _ if !field.enum_values.is_empty() => {
            html! {
                label for=(field_id) { (label) }
                select
                    id=(field_id)
                    name=(field.name)
                    data-param-name=(field.name) {
                    option value="" selected[field.default_value.is_none()] { "Choose…" }
                    @for value in &field.enum_values {
                        option value=(value) selected[field.default_value.as_ref() == Some(value)] {
                            (value)
                        }
                    }
                }
            }
        }
        "integer" => {
            html! {
                label for=(field_id) { (label) }
                input
                    id=(field_id)
                    type="number"
                    name=(field.name)
                    value=(field.default_value.as_deref().unwrap_or_default())
                    placeholder=(field.format.as_deref().unwrap_or("integer"))
                    data-param-name=(field.name);
            }
        }
        _ => {
            let readonly = field.name == "input" || field.name == "runpath";
            html! {
                label for=(field_id) { (label) }
                input
                    id=(field_id)
                    type=(input_type_for_field(field))
                    name=(field.name)
                    value=(field.default_value.as_deref().unwrap_or_default())
                    placeholder=(field_placeholder(field))
                    readonly[readonly]
                    data-param-name=(field.name);
            }
        }
    };

    html! {
        div class={ "field " (field_span(field)) } {
            (field_markup)
            @if let Some(help) = help {
                p class="helper" { (help) }
            }
        }
    }
}

fn build_directory_listing(
    context: &AppContext,
    requested: Option<&str>,
) -> Result<DirectoryListing> {
    let current = resolve_within_root(&context.data_root, requested.unwrap_or(""))?;
    let current_samplesheet = current.join("samplesheet.csv");
    let parent = current
        .parent()
        .filter(|path| path.starts_with(&context.data_root))
        .map(|path| path.to_string_lossy().into_owned());

    let mut entries = fs::read_dir(&current)
        .with_context(|| format!("failed to read {}", current.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            Some(DirectoryEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: path.to_string_lossy().into_owned(),
                has_fastq_layout: has_fastq_layout(&path),
                samplesheet_exists: path.join("samplesheet.csv").is_file(),
                samplesheet_path: path.join("samplesheet.csv").to_string_lossy().into_owned(),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    Ok(DirectoryListing {
        current: current.to_string_lossy().into_owned(),
        parent,
        has_fastq_layout: has_fastq_layout(&current),
        samplesheet_exists: current_samplesheet.is_file(),
        entries,
    })
}

fn build_nextflow_command(context: &AppContext, record: &RunRecord) -> Vec<String> {
    let mut command = vec![
        context.nextflow_bin.clone(),
        "-log".to_string(),
        record.nextflow_log.to_string_lossy().into_owned(),
        "run".to_string(),
        record
            .pipeline_dir
            .join("main.nf")
            .to_string_lossy()
            .into_owned(),
        "-ansi-log".to_string(),
        "false".to_string(),
    ];

    if !record.profile.trim().is_empty() {
        command.push("-profile".to_string());
        command.push(record.profile.clone());
    }

    command.extend([
        "-with-report".to_string(),
        record.report_path.to_string_lossy().into_owned(),
        "-with-timeline".to_string(),
        record.timeline_path.to_string_lossy().into_owned(),
        "-with-trace".to_string(),
        record.trace_path.to_string_lossy().into_owned(),
    ]);

    command.extend(record.extra_nextflow_args.iter().cloned());

    for field in &context.schema.ordered_fields {
        if let Some(value) = record.parameters.get(&field.name) {
            command.push(format!("--{}", field.name));
            command.push(value.clone());
        }
    }

    command
}

fn load_pipeline_schema(path: &Path) -> Result<PipelineSchema> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let raw: RawSchema = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let mut sections = Vec::new();
    let mut ordered_fields = Vec::new();
    for reference in &raw.all_of {
        let key = reference
            .reference
            .trim_start_matches("#/$defs/")
            .to_string();
        let section = raw
            .defs
            .get(&key)
            .with_context(|| format!("schema section `{key}` not found"))?;
        let required = section.required.iter().cloned().collect::<Vec<_>>();

        let mut fields = section
            .properties
            .iter()
            .map(|(name, property)| SchemaField {
                name: name.clone(),
                kind: property.kind.clone(),
                description: property.description.clone(),
                help_text: property.help_text.clone(),
                enum_values: property.enum_values.clone().unwrap_or_default(),
                format: property.format.clone(),
                hidden: property.hidden.unwrap_or(false),
                default_value: property.default.as_ref().and_then(schema_value_to_string),
                required: required.iter().any(|required_name| required_name == name),
            })
            .collect::<Vec<_>>();

        fields.sort_by_key(|field| {
            (
                field_sort_order(&key, &field.name),
                field.name.to_ascii_lowercase(),
            )
        });

        ordered_fields.extend(fields.iter().cloned());
        sections.push(SchemaSection {
            title: section.title.clone(),
            description: section.description.clone(),
            fields,
        });
    }

    Ok(PipelineSchema {
        title: raw.title,
        description: raw.description,
        sections,
        ordered_fields,
    })
}

fn field_sort_order(section_key: &str, name: &str) -> usize {
    let known = match section_key {
        "input_output_options" => vec!["input", "outdir", "runpath", "e"],
        "additional_options" => vec![
            "p",
            "custom_primers",
            "primer_kmer_len",
            "primer_restrict_window",
            "email",
            "process_q",
            "parquet_files",
            "amd_platform",
            "sourcepath",
            "read_qc",
            "subsample_reads",
            "irma_module",
            "custom_irma_config",
            "custom_qc_settings",
            "ecr_registry",
            "restage",
            "variants_of_interest",
            "positions_of_interest",
            "reference_seq_table",
            "dais_module",
            "check_version",
            "custom_runid",
            "nextclade",
        ],
        _ => Vec::new(),
    };

    known
        .iter()
        .position(|candidate| candidate == &name)
        .unwrap_or(usize::MAX / 2)
}

fn schema_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}

fn parse_extra_args(value: Option<String>) -> Result<Vec<String>> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(Vec::new());
    };

    shlex::split(&value).ok_or_else(|| anyhow!("failed to parse extra Nextflow args: {value}"))
}

fn canonicalized_existing_dir(path: &Path, create_if_missing: bool) -> Result<PathBuf> {
    if create_if_missing {
        fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    }

    path.canonicalize()
        .with_context(|| format!("failed to open {}", path.display()))
}

fn resolve_within_root(root: &Path, requested: &str) -> Result<PathBuf> {
    let candidate = if requested.trim().is_empty() {
        root.to_path_buf()
    } else {
        PathBuf::from(requested)
    };
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to open {}", candidate.display()))?;
    if !canonical.starts_with(root) {
        bail!(
            "{} is outside the allowed data root {}",
            canonical.display(),
            root.display()
        );
    }
    if !canonical.is_dir() {
        bail!("{} is not a directory", canonical.display());
    }
    Ok(canonical)
}

fn resolve_file_within_root(root: &Path, requested: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(requested);
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to open {}", candidate.display()))?;
    if !canonical.starts_with(root) {
        bail!(
            "{} is outside the allowed data root {}",
            canonical.display(),
            root.display()
        );
    }
    if !canonical.is_file() {
        bail!("{} is not a file", canonical.display());
    }
    Ok(canonical)
}

fn has_fastq_layout(path: &Path) -> bool {
    path.join("fastqs").is_dir() || path.join("fastq_pass").is_dir()
}

fn input_type_for_field(field: &SchemaField) -> &'static str {
    if field.name == "email" {
        "email"
    } else {
        "text"
    }
}

fn field_placeholder(field: &SchemaField) -> &str {
    match field.format.as_deref() {
        Some("directory-path") => "/path/to/directory",
        Some("file-path") => "/path/to/file",
        _ => "value",
    }
}

fn field_span(field: &SchemaField) -> &'static str {
    match field.kind.as_str() {
        "boolean" => "",
        _ if matches!(
            field.format.as_deref(),
            Some("file-path" | "directory-path")
        ) =>
        {
            "full"
        }
        _ if field.name == "multiqc_methods_description" => "full",
        _ => "",
    }
}

fn nav() -> Markup {
    html! {
        nav class="nav" {
            div class="brand" {
                strong { "MIRA-Oxide UI" }
                span { "Rust server for launching MIRA-NF locally" }
            }
            div class="nav-links" {
                a href="/" { "Home" }
                a href="/runs/new" { "New Run" }
            }
        }
    }
}

fn summary_item(label: &str, value: &str) -> Markup {
    html! {
        div class="run-item" {
            strong { (label) }
            code { (value) }
        }
    }
}

fn status_badge(status: &RunStatus) -> Markup {
    let class = match status {
        RunStatus::Pending => "status-badge status-pending",
        RunStatus::Running => "status-badge status-running",
        RunStatus::Succeeded => "status-badge status-succeeded",
        RunStatus::Failed => "status-badge status-failed",
    };

    html! {
        span class=(class) { (status.label()) }
    }
}

fn exists_label(value: bool) -> &'static str {
    if value { "ready" } else { "waiting" }
}

fn layout_document(title: &str, head: Markup, body: Markup) -> String {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                (head)
            }
            body { (body) }
        }
    }
    .into_string()
}

fn display_timestamp(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .unwrap_or_else(|_| "invalid timestamp".to_string())
}

fn respond(result: Result<String>) -> Response {
    match result {
        Ok(html) => Html(html).into_response(),
        Err(error) => error_response(&error),
    }
}

fn error_response(error: &anyhow::Error) -> Response {
    let body = html! {
        main {
            (nav())
            div class="card" {
                h1 { "Request failed" }
                div class="error" { (error.to_string()) }
                div class="form-actions" style="margin-top: 18px;" {
                    a href="/runs/new" { "Back to run form" }
                    a class="button secondary" href="/" { "Home" }
                }
            }
        }
    };

    (
        StatusCode::BAD_REQUEST,
        Html(layout_document(
            "Request failed",
            html! { style { (APP_CSS) } },
            body,
        )),
    )
        .into_response()
}

fn default_profile_for_current_arch() -> String {
    match std::env::consts::ARCH {
        "aarch64" => "podman_arm64,local".to_string(),
        _ => "podman,local".to_string(),
    }
}

fn default_data_root() -> PathBuf {
    std::env::var_os("MIRA_UI_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/workspace"))
}
