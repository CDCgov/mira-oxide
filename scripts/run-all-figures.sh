#!/usr/bin/env bash
#
# run-all-figures.sh - run every figure-producing mira-oxide subcommand across
#                      all MIRA run directories and organize the outputs.
#
# For each run found under $MIRA_ROOT (a directory containing outputs/samples)
# it produces, under $OUTROOT/<Virus>_<Platform>_<run>/:
#
#   plotter/    per-sample coverage, segmented-coverage and read-flow figures
#               (<sample>.html, <sample>_seg.html, <sample>_read_assignment.html)
#   reports/    prepare-mira-reports output (coverage figure JSONs, dashboards)
#   di-stats/   di_stats.txt
#   logs/       stderr for every command
#
# Env overrides:
#   BIN        mira-oxide binary        (default: repo target/release/mira-oxide)
#   MIRA_ROOT  MIRA data root           (default: ~/mira-local-data/MIRA)
#   MNF        MIRA-NF repo (qc yaml)    (default: ~/repos/Mira-nf)
#   OUTROOT    output root              (default: ~/mira-oxide-testing)
#
set -uo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-$REPO_DIR/target/release/mira-oxide}"
MIRA_ROOT="${MIRA_ROOT:-$HOME/mira-local-data/MIRA}"
MNF="${MNF:-$HOME/repos/Mira-nf}"
OUTROOT="${OUTROOT:-$HOME/mira-oxide-testing}"
QC="$MNF/bin/irma_config/qc_pass_fail_settings.yaml"

[[ -x "$BIN" ]] || { echo "!! binary not found: $BIN (run: cargo build --release)" >&2; exit 2; }
[[ -d "$MIRA_ROOT" ]] || { echo "!! MIRA_ROOT not found: $MIRA_ROOT" >&2; exit 2; }

echo ">> binary   : $BIN"
echo ">> mira root: $MIRA_ROOT"
echo ">> mira-nf  : $MNF"
echo ">> output   : $OUTROOT"
echo

# infer virus / platform from the run's relative path
infer_virus() {
  case "$1" in
    RSV/*|*/RSV/*|rsv*) echo rsv ;;
    SC2*|*/SC2*|sc2*)   echo sc2-wgs ;;
    *)                  echo flu ;;
  esac
}
infer_platform() {
  case "$1" in *ONT*|*ont*) echo ont ;; *) echo illumina ;; esac
}

runs=$(cd "$MIRA_ROOT" && find . -type d -name samples -path '*/outputs/*' 2>/dev/null \
        | sed 's#/outputs/samples##; s#^\./##' | sort)

[[ -n "$runs" ]] || { echo "no runs found under $MIRA_ROOT" >&2; exit 1; }

total_runs=0; total_figs=0
while IFS= read -r rel; do
  [[ -n "$rel" ]] || continue
  run="$MIRA_ROOT/$rel"
  runid="$(basename "$rel")"
  virus="$(infer_virus "$rel")"
  platform="$(infer_platform "$rel")"
  label="${rel//\//_}"
  out="$OUTROOT/$label"
  mkdir -p "$out/plotter" "$out/reports" "$out/di-stats" "$out/logs"
  total_runs=$((total_runs + 1))

  echo "=== $rel  (virus=$virus platform=$platform, id=$runid) ==="

  # 1) per-sample figures via plotter. Each figure mode is run independently so
  #    an empty/low-abundance sample that breaks one mode still yields the rest.
  n_fig=0; n_empty=0
  for irma in "$run"/outputs/samples/*/IRMA; do
    [[ -d "$irma" ]] || continue
    sample="$(basename "$(dirname "$irma")")"
    if ! ls "$irma"/tables/*coverage.txt >/dev/null 2>&1; then
      n_empty=$((n_empty + 1)); continue          # no assembly -> nothing to plot
    fi
    log="$out/logs/plotter_$sample.log"; : >"$log"
    for mode in "-c coverage" "-s coverage" "-r coverage"; do
      set -- $mode
      "$BIN" plotter -i "$irma" "$1" -o "$out/plotter/$sample.html" >>"$log" 2>&1 \
        || echo "   !! plotter $1 failed for $sample (see $(basename "$log"))"
    done
    c=$(find "$out/plotter" -maxdepth 1 -name "$sample*.html" -size +0c | wc -l | tr -d ' ')
    n_fig=$((n_fig + c))
  done
  echo "   plotter : $n_fig figure(s) from $(( $(ls -d "$run"/outputs/samples/*/IRMA 2>/dev/null | wc -l | tr -d ' ') - n_empty )) sample(s) ($n_empty empty) -> $out/plotter/"
  total_figs=$((total_figs + n_fig))

  # 2) full report figures via prepare-mira-reports
  ss="$run/samplesheet.csv"
  if [[ -f "$ss" && -f "$QC" ]]; then
    if ( cd "$out/reports" && "$BIN" prepare-mira-reports \
          -i "$run/outputs/samples" -o ./ -s "$ss" -q "$QC" \
          -p "$platform" -v "$virus" -r "$runid" -w "$MNF" ) \
          >"$out/logs/prepare-mira-reports.log" 2>&1; then
      j=$(find "$out/reports" -maxdepth 1 -name '*.json' | wc -l | tr -d ' ')
      echo "   reports : $j json figure/report file(s) -> $out/reports/"
    else
      echo "   !! prepare-mira-reports failed (see logs/prepare-mira-reports.log)"
    fi
  else
    echo "   reports : skipped (missing samplesheet or qc yaml)"
  fi

  # 3) DI stats
  if ( cd "$out/di-stats" && "$BIN" di-stats -a "$run/outputs/samples" -r "$runid" ) \
        >"$out/logs/di-stats.log" 2>&1; then
    echo "   di-stats: $(( $(wc -l <"$out/di-stats/di_stats.txt") - 1 )) row(s) -> $out/di-stats/di_stats.txt"
  else
    echo "   !! di-stats failed (see logs/di-stats.log)"
  fi
  echo
done <<<"$runs"

echo ">> done: $total_runs run(s), $total_figs plotter figure(s), output under $OUTROOT"
