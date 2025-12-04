#![allow(clippy::format_push_string, clippy::too_many_lines)]
use super::coverage_json_per_sample::SampleCoverageJson;
use super::data_ingest::{AllelesData, IndelsData};
use super::reads_to_sankey_json::SampleSankeyJson;
use crate::utils::data_processing::{DaisVarsData, IRMASummary};
use glob::glob;
use serde_json::json;
use std::fs::{read, write};
use std::path::{Path, PathBuf};

// Helper functions to base64 encode an image file
fn base64_encode(input: &[u8]) -> String {
    const BASE64_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);

    let chunks = input.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);

        output.push(BASE64_TABLE[(b0 >> 2) as usize] as char);
        output.push(BASE64_TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            output.push(BASE64_TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }

        if chunk.len() > 2 {
            output.push(BASE64_TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn base64_img(path: &Path) -> String {
    read(path)
        .map(|bytes| base64_encode(&bytes))
        .unwrap_or_default()
}

// Helper to read plotly JSON value
fn plotly_json_script(div_id: &str, plotly_json: &str) -> String {
    format!(
        r#"
<div id="{div_id}" style="width:95vw; margin:auto;"></div>
<script type="text/javascript">
(function() {{
    var fig = {plotly_json};
    Plotly.newPlot('{div_id}', fig.data, fig.layout, {{displayModeBar: false}});
}})();
</script>
"#
    )
}

fn write_sample_plot_html(
    output_path: &Path,
    sample: &str,
    coverage_json: &serde_json::Value,
    sankey_json: &serde_json::Value,
) -> std::io::Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{sample} Coverage & Sankey</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
</head>
<body>
    <h2>{sample} Coverage Plot</h2>
    <div id="coverage_plot" style="width:90vw; height:40vh;"></div>
    <h2>{sample} Sankey Plot</h2>
    <div id="sankey_plot" style="width:90vw; height:40vh;"></div>
    <script>
        var coverage_fig = {coverage_json};
        Plotly.newPlot('coverage_plot', coverage_fig.data, coverage_fig.layout);

        var sankey_fig = {sankey_json};
        Plotly.newPlot('sankey_plot', sankey_fig.data, sankey_fig.layout);
    </script>
</body>
</html>
"#
    );
    let out_path = output_path.join(format!("MIRA_{sample}_coverage.html"));
    write(out_path, html)
}

//Format plotly table
fn plotly_table_script(div_id: &str, table_json: &str, table_title: &str) -> String {
    format!(
        r#"
<div id="{div_id}" style="width:95vw; margin:auto;"></div>
<script type="text/javascript">
(function() {{
    var data = {table_json};
    var trace = {{
        type: 'table',
        header: {{
            values: data.header,
            align: "center",
            line: {{width: 1, color: 'black'}},
            fill: {{color: "lightgrey"}},
            font: {{family: "Helvetica", size: 14, color: "black"}}
        }},
        cells: {{
            values: data.columns,
            align: "center",
            line: {{color: "black", width: 1}},
            font: {{family: "Helvetica", size: 12, color: ["black"]}}
        }}
    }};
    Plotly.newPlot('{div_id}', [trace], {{
        title: '{table_title}',
        margin: {{t: 40, l: 10, r: 10, b: 10}},
        autosize: true
    }}, {{displayModeBar: false}});
}})();
</script>
"#
    )
}

// functions to read in table data and render as HTML table
fn irma_summary_to_plotly_json(summary: &[IRMASummary]) -> String {
    let headers = [
        "Sample",
        "Total Reads",
        "Pass QC",
        "Reads Mapped",
        "Reference",
        "% Reference Covered",
        "Median Coverage",
        "Count of Minor SNVs >= 0.05",
        "Count of Minor Indels >= 0.05",
        "Pass/Fail Reason",
        "Subtype",
        "MIRA module",
        "Run ID",
        "Instrument",
    ];
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); headers.len()];

    for row in summary {
        columns[0].push(row.sample_id.as_deref().unwrap_or("").to_string());
        columns[1].push(row.total_reads.map_or(String::new(), |v| v.to_string()));
        columns[2].push(row.pass_qc.map_or(String::new(), |v| v.to_string()));
        columns[3].push(row.reads_mapped.map_or(String::new(), |v| v.to_string()));
        columns[4].push(row.reference.as_deref().unwrap_or("").to_string());
        columns[5].push(
            row.percent_reference_coverage
                .map_or(String::new(), |v| format!("{v:.2}")),
        );
        columns[6].push(row.median_coverage.map_or(String::new(), |v| v.to_string()));
        columns[7].push(row.count_minor_snv.map_or(String::new(), |v| v.to_string()));
        columns[8].push(
            row.count_minor_indel
                .map_or(String::new(), |v| v.to_string()),
        );
        columns[9].push(row.pass_fail_reason.as_deref().unwrap_or("").to_string());
        columns[10].push(row.subtype.as_deref().unwrap_or("").to_string());
        columns[11].push(row.mira_module.as_deref().unwrap_or("").to_string());
        columns[12].push(row.runid.as_deref().unwrap_or("").to_string());
        columns[13].push(row.instrument.as_deref().unwrap_or("").to_string());
    }

    json!({
        "header": headers,
        "columns": columns
    })
    .to_string()
}

fn dais_vars_to_plotly_json(vars: &[DaisVarsData]) -> String {
    let headers = [
        "Sample",
        "Reference",
        "Protein",
        "AA Variant Count",
        "AA Variants",
    ];
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); headers.len()];

    for row in vars {
        columns[0].push(row.sample_id.as_deref().unwrap_or("").to_string());
        columns[1].push(row.reference_id.to_string());
        columns[2].push(row.protein.to_string());
        columns[3].push(row.aa_variant_count.to_string());
        columns[4].push(row.aa_variants.to_string());
    }

    serde_json::json!({
        "header": headers,
        "columns": columns
    })
    .to_string()
}

fn alleles_to_plotly_json(data: &[AllelesData]) -> String {
    let headers = [
        "Sample",
        "Reference",
        "HMM Position",
        "Sample Position",
        "Coverage",
        "Consensus Allele",
        "Minority Allele",
        "Consensus Count",
        "Minority Count",
        "Minority Frequency",
        "Run ID",
        "Instrument",
    ];
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); headers.len()];

    for row in data {
        columns[0].push(row.sample_id.as_deref().unwrap_or("").to_string());
        columns[1].push(row.reference.to_string());
        columns[2].push(
            row.reference_position
                .map_or(String::new(), |v| v.to_string()),
        );
        columns[3].push(row.sample_position.to_string());
        columns[4].push(row.coverage.to_string());
        columns[5].push(row.consensus_allele.to_string());
        columns[6].push(row.minority_allele.to_string());
        columns[7].push(row.consensus_count.to_string());
        columns[8].push(row.minority_count.to_string());
        columns[9].push(format!("{:.4}", row.minority_frequency));
        columns[10].push(row.run_id.as_deref().unwrap_or("").to_string());
        columns[11].push(row.instrument.as_deref().unwrap_or("").to_string());
    }

    serde_json::json!({
        "header": headers,
        "columns": columns
    })
    .to_string()
}

fn indels_to_plotly_json(data: &[IndelsData]) -> String {
    let headers = [
        "Sample",
        "Reference",
        "HMM Upstream Position",
        "Sample Upstream Position",
        "Insert",
        "Length",
        "Context",
        "Called",
        "Count",
        "Total",
        "Frequency",
        "Average Quality",
        "ConfidenceNotMacErr",
        "PairedUB",
        "QualityUB",
        "Run ID",
        "Instrument",
    ];
    let mut columns: Vec<Vec<String>> = vec![Vec::new(); headers.len()];

    for row in data {
        columns[0].push(row.sample_id.as_deref().unwrap_or("").to_string());
        columns[1].push(row.reference_name.to_string());
        columns[2].push(
            row.reference_upstream_position
                .as_deref()
                .unwrap_or("")
                .to_string(),
        );
        columns[3].push(
            row.sample_upstream_position
                .as_deref()
                .unwrap_or("")
                .to_string(),
        );
        columns[4].push(row.insert.as_deref().unwrap_or("").to_string());
        columns[5].push(row.length.map_or(String::new(), |v| v.to_string()));
        columns[6].push(row.context.to_string());
        columns[7].push(row.called.to_string());
        columns[8].push(row.count.to_string());
        columns[9].push(row.total.to_string());
        columns[10].push(format!("{:.4}", row.frequency));
        columns[11].push(row.average_quality.as_deref().unwrap_or("").to_string());
        columns[12].push(
            row.confidence_not_mac_err
                .as_deref()
                .unwrap_or("")
                .to_string(),
        );
        columns[13].push(row.paired_ub.to_string());
        columns[14].push(row.quality_ub.as_deref().unwrap_or("").to_string());
        columns[15].push(row.run_id.as_deref().unwrap_or("").to_string());
        columns[16].push(row.instrument.as_deref().unwrap_or("").to_string());
    }

    serde_json::json!({
        "header": headers,
        "columns": columns
    })
    .to_string()
}

#[allow(clippy::too_many_arguments)]
pub fn generate_html_report(
    output_path: &Path,
    irma_summary: &[IRMASummary],
    dais_vars_data: &[DaisVarsData],
    minor_variants: &[AllelesData],
    indels: &[IndelsData],
    barcode_distribution_json: &serde_json::Value,
    pass_fail_heatmap_json: &serde_json::Value,
    cov_heatmap_json: &serde_json::Value,
    coverage_json_per_sample: &[SampleCoverageJson],
    sankey_json_per_sample: &[SampleSankeyJson],
    runid: &str,
    logo_path: Option<&Path>,
) -> std::io::Result<()> {
    // Set up asset paths
    let (mira_logo, favicon, excel_logo) = if let Some(logo_path) = logo_path {
        (
            logo_path.join("assets/mira-logo-midjourney_20230526_rmbkgnd.png"),
            logo_path.join("assets/favicon.ico"),
            logo_path.join("assets/Microsoft_Excel-Logo.png"),
        )
    } else {
        (PathBuf::new(), PathBuf::new(), PathBuf::new())
    };

    let base64_logo = base64_img(&mira_logo);
    let base64_favicon = base64_img(&favicon);
    let base64_excellogo = base64_img(&excel_logo);

    // Read all the required files
    let barcode_distribution_json_str = barcode_distribution_json.to_string();
    let bdp_html = plotly_json_script("barcode_distribution_plot", &barcode_distribution_json_str);

    let pass_fail_heatmap_json_str = pass_fail_heatmap_json.to_string();
    let pfhm_html = plotly_json_script("pass_fail_heatmap_plot", &pass_fail_heatmap_json_str);

    let cov_heatmap_json_str = cov_heatmap_json.to_string();
    let chm_html = plotly_json_script("cov_heatmap_plot", &cov_heatmap_json_str);

    // Pull in data tables for htmls
    let irma_summary_json = irma_summary_to_plotly_json(irma_summary);
    let irma_sum_html = plotly_table_script(
        "irma_summary_table",
        &irma_summary_json,
        "MIRA Summary Table",
    );
    let dais_vars_json = dais_vars_to_plotly_json(dais_vars_data);
    let dais_var_html =
        plotly_table_script("dais_vars_table", &dais_vars_json, "AA Variants Table");

    let minorvars_json = alleles_to_plotly_json(minor_variants);
    let minorvars_table_html =
        plotly_table_script("minor_vars_table", &minorvars_json, "Minor Variants Table");

    let indels_json = indels_to_plotly_json(indels);
    let indels_table_html = plotly_table_script("indels_table", &indels_json, "Minor Indels Table");

    // Coverage links

    let mut coverage_links_html =
        String::from("<h3>Individual Sample Coverage & Sankey Figures</h3><p2>");
    for coverage_json in coverage_json_per_sample {
        let sample = &coverage_json.sample_id;

        // Find the matching sankey_json by sample_id
        if let Some(sankey_json) = sankey_json_per_sample
            .iter()
            .find(|s| s.sample_id == *sample)
        {
            // Write the per-sample HTML file
            write_sample_plot_html(output_path, sample, &coverage_json.json, &sankey_json.json)?;

            // Add the link to the main HTML (relative path)
            let link = format!(
                r#"<a href="MIRA_{sample}_coverage.html" target="_blank">{sample}</a><br>"#
            );
            coverage_links_html.push_str(&link);
        }
    }
    coverage_links_html.push_str("</p2>");

    // Fasta links
    let mut fasta_links_html = String::from(
        r#"<h2>Fasta downloads</h2>
        <p>(Right-click → "Save link as...")</p>
        <div>
    "#,
    );

    for entry in glob(&format!("{}/{}*.fasta", output_path.display(), runid))
        .unwrap()
        .flatten()
    {
        if let Some(kind) = entry.file_name().and_then(|n| n.to_str()) {
            let link = format!(r#"<a href="./{kind}" download>{kind}</a><br>"#);
            fasta_links_html.push_str(&link);
        }
    }

    fasta_links_html.push_str("</div>");

    // Compose HTML
    let html_string = format!(
        r#"
<html>
    <head>
        <style>
        h1 {{text-align: center; font-family: Helvetica;}}
        h2 {{text-align: center; font-family: Helvetica; margin-bottom: 2px;}}
        head {{text-align: center; font-family: Helvetica; margin-top: 20px; margin-left: 100px; margin-right: 100px;}}
        body {{text-align: center; font-family: Helvetica; margin-bottom: 20px; margin-left: 100px; margin-right: 100px;}}
        p1 {{text-align: left; font-family: Helvetica; margin-top: 20px; margin-bottom: 20px; margin-left: 300px; margin-right: 300px;}}
        p2 {{text-align: center; font-size: 25px; font-family: Helvetica; margin-bottom: 20px;}}
        </style>
        <title>MIRA Summary</title>
        <link rel="icon" type="image/x-icon" href="data:image/png;base64,{base64_favicon}">
        <img src="data:image/png;base64,{base64_logo}">
        <h1>MIRA Summary Report</h1>
        <h2>{runid}</h2>
        <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    </head>
    <hr>
    <hr>
    <body>
        <h2>Barcode Assignment</h2>
        {bdp_html}
        <hr>
        <h2>Automatic Quality Control Decisions</h2>
        {pfhm_html}
        <hr>
        <h2>Median Coverage</h2>
        {chm_html}
        <hr>
        {irma_sum_html}
        <a href="./{runid}_summary.csv" download style="display: inline-block; text-align: center;">MIRA Summary Download<br>
        <img src="data:image/png;base64,{base64_excellogo}" alt="Download excel" width="60" height="40">
        </a>
        <hr>
        {coverage_links_html}
        <hr>
        {dais_var_html}
        <a href="./{runid}_aavars.csv" download style="display: inline-block; text-align: center;">AA Variants Table Download<br>
        <img src="data:image/png;base64,{base64_excellogo}" alt="Download excel" width="60" height="40">
        </a>
        <hr>
        {minorvars_table_html}
        <a href="./{runid}_all_alleles.csv" download style="display: inline-block; text-align: center;">Minor Variant Table Download<br>
        <img src="data:image/png;base64,{base64_excellogo}" alt="Download excel" width="60" height="40">
        </a> 
        <hr>
        {indels_table_html}
        <a href="./{runid}_indels.csv" download style="display: inline-block; text-align: center;">Minor Indels Table Download<br>
        <img src="data:image/png;base64,{base64_excellogo}" alt="Download excel" width="60" height="40">
        </a> 
        <hr>
        {fasta_links_html}
    </body>
</html>
"#
    );

    // Write to file
    let out_path = output_path.join(format!("MIRA-summary-{runid}.html"));
    write(out_path, html_string)?;

    Ok(())
}
