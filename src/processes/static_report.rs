use base64::{Engine as _, engine::general_purpose};
use clap::Parser;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Generate MIRA HTML summary report from JSON data files"
)]
pub struct StaticReportArgs {
    /// Path to the data directory containing JSON files
    #[arg(short = 'd', long = "data-path")]
    data_path: Option<String>,

    /// Path to the run directory (defaults to data_path)
    #[arg(short = 'r', long = "run-path")]
    run_path: Option<String>,

    /// Path to the assets folder containing logos
    #[arg(short = 'l', long = "logo-path")]
    logo_path: Option<String>,

    /// Output HTML file path (optional, defaults to MIRA-summary-{run}.html)
    #[arg(short = 'o', long = "output")]
    output: Option<String>,
}

pub fn static_report_process(args: StaticReportArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Determine paths
    let data_root = args.data_path.unwrap_or_else(|| ".".to_string());
    let run_root = args.run_path.as_ref().unwrap_or(&data_root).clone();
    let logo_path = args.logo_path.unwrap_or_default();

    let run_name = Path::new(&run_root)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Image locations
    let mira_logo = format!(
        "{}/assets/mira-logo-midjourney_20230526_rmbkgnd.png",
        logo_path
    );
    let favicon = format!("{}/assets/favicon.ico", logo_path);
    let excel_logo = format!("{}/assets/Microsoft_Excel-Logo.png", logo_path);

    // Base64 encode images
    let base64_logo = base64_encode_file(&mira_logo)?;
    let base64_favicon = base64_encode_file(&favicon)?;
    let base64_excellogo = base64_encode_file(&excel_logo)?;

    // Read and process barcode distribution
    let bdp_html = read_json_to_html(
        &format!("{}/barcode_distribution.json", data_root),
        "MIRA_barcode_distribution_pie.svg",
    )
    .unwrap_or_else(|_| "<p>No barcode results</p>".to_string());

    // Read and process pass/fail heatmap
    let pfhm_html = read_json_to_html(
        &format!("{}/pass_fail_heatmap.json", data_root),
        "MIRA_passfail_heatmap.svg",
    )
    .unwrap_or_else(|_| "<p>No automatic qc results</p>".to_string());

    // Read and process coverage heatmap
    let chm_html = read_json_to_html(
        &format!("{}/heatmap.json", data_root),
        "MIRA_coverage_summary_heatmap.svg",
    )
    .unwrap_or_else(|_| "<p>No coverage results</p>".to_string());

    // Process IRMA summary table
    let irma_sum_html = process_irma_summary(&data_root, &run_name)
        .unwrap_or_else(|_| "<p>No MIRA summary results</p>".to_string());

    // Generate coverage links
    let coverage_links_html = generate_coverage_links(&data_root).unwrap_or_else(|_| {
        "<h2>Individual Sample Coverage Figures</h2><p2><p>No coverage results</p></p2>".to_string()
    });

    // Process DAIS variants table
    let dais_var_html = process_dais_vars(&data_root, &run_name)
        .unwrap_or_else(|_| "<p>No MIRA amino acid variant results</p>".to_string());

    // Generate minor variants and indels links
    let minorvars_links_html = process_minor_variants(&data_root, &run_name)
        .unwrap_or_else(|_| "<p2>No minor variants table</p2>".to_string());

    let indels_links_html = process_indels(&data_root, &run_name)
        .unwrap_or_else(|_| "<p2>No indels table</p2>".to_string());

    // Generate fasta links
    let fasta_links_html = generate_fasta_links(&data_root, &run_name)
        .unwrap_or_else(|_| "<h2>Fasta downloads</h2><p3>(Right-click->\"Save link as...\")</p3><br><p>No fasta files</p2>".to_string());

    // Build HTML report
    let html_string = format!(
        r#"
<html>
    <head>
        <style>
        h1 {{text-align: center;
            font-family: Helvetica;}}
        h2 {{text-align: center;
            font-family: Helvetica;
            margin-bottom: 2px;}}
        head {{text-align: center; 
            font-family: Helvetica;
            margin-top: 20px; 
            margin-left: 100px;
            margin-right: 100px;}}
        body {{text-align: center; 
            font-family: Helvetica;
            margin-bottom: 20px;
            margin-left: 100px;
            margin-right: 100px;}}
        p1 {{text-align: left; 
            font-family: Helvetica;
            margin-top: 20px; 
            margin-bottom: 20px;
            margin-left: 300px;
            margin-right: 300px;}}
        p2 {{text-align: center;
            font-size: 25px;
            font-family: Helvetica;
            margin-bottom: 20px;}}
        p2 {{text-align: center;
            font-family: Helvetica;
            margin-bottom: 20px;}}
        </style>
        <title>MIRA Summary</title>
        <link rel="icon" type="image/x-icon" href="data:image/png;base64,{base64_favicon}">
        <img src="data:image/png;base64,{base64_logo}">
        <h1>MIRA Summary Report</h1>
        <h2>{run_name}</h2>
    </head>
    <hr>
    <hr>
    <body>
        <h2>Barcode Assignment</h2>
        {bdp_html}
            <p1>The ideal result would be a similar number of reads assigned to each test and positive 
            control. However, it is ok to not have similar read numbers per sample. Samples with a low 
            proportion of reads may indicate higher Ct of starting material or less performant PCR 
            during library preparation. What is most important for sequencing assembly is raw count of 
            reads and their quality.</p1>            
        <hr>
        <h2>Automatic Quality Control Decisions</h2>
        {pfhm_html}
            <p1>MIRA requires a minimum median coverage of 50x, a minimum coverage of the reference 
            length of 90%, and less than 10 minor variants >=5%. These are marked in yellow to orange 
            according to the number of these failure types. Samples that failed to generate any assembly 
            are marked in red. In addition, premature stop codons are flagged in yellow. CDC does not 
            submit sequences with premature stop codons, particularly in HA, NA or SARS-CoV-2 Spike. 
            Outside of those genes, premature stop codons near the end of the gene may be ok for 
            submission. Hover your mouse over the figure to see individual results.</p1>
        <hr>
        <h2>Median Coverage</h2>
        {chm_html}
            <p1>The heatmap summarizes the mean coverage per sample per reference.</p1>
        <hr>
        <h2>MIRA Summary Table</h2>
        <a href="./MIRA_{run_name}_summary.xlsx" download>
            <img src="data:image/png;base64,{base64_excellogo}" alt="Download excel" width="60" height="40">
        </a>
        {irma_sum_html}
        <hr>
        {coverage_links_html}
        <hr>
        <h2>AA Variants Table</h2>
        <a href="./MIRA_{run_name}_aavars.xlsx" download>
            <img src="data:image/png;base64,{base64_excellogo}" alt="Download excel" width="60" height="40">
        </a>
        {dais_var_html}
        <hr>
        <h2>Minor Table Download</h2>
        {minorvars_links_html} {indels_links_html}
        <hr>
        {fasta_links_html}
    </body>
</html>"#
    );

    // Write output
    let output_path = args
        .output
        .unwrap_or_else(|| format!("MIRA-summary-{}.html", run_name));

    fs::write(&output_path, html_string)?;
    println!("MIRA summary report written to {}", output_path);

    Ok(())
}

fn base64_encode_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}

fn read_json_to_html(
    json_path: &str,
    download_filename: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let json_content = fs::read_to_string(json_path)?;
    // For now, we'll return a placeholder since we don't have plotly in Rust
    // In a real implementation, you'd need to either:
    // 1. Use a JavaScript library to render plotly on the client side
    // 2. Convert the JSON to an image server-side
    // 3. Embed the plotly JSON and let the browser render it

    Ok(format!(
        r#"<div id="plotly-{}">
            <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
            <script>
                var plotlyData = {};
                Plotly.newPlot('plotly-{}', plotlyData.data, plotlyData.layout, {{
                    toImageButtonOptions: {{
                        format: 'svg',
                        filename: '{}'
                    }}
                }});
            </script>
        </div>"#,
        download_filename.replace(".svg", ""),
        json_content,
        download_filename.replace(".svg", ""),
        download_filename
    ))
}

fn process_irma_summary(
    data_root: &str,
    run_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let json_path = format!("{}/irma_summary.json", data_root);
    let json_content = fs::read_to_string(&json_path)?;

    // Parse JSON and convert to Excel (would need rust_xlsxwriter or similar crate)
    // For now, create a simple HTML table
    let excel_path = format!("MIRA_{}_summary.xlsx", run_name);

    // Note: In production, you'd use serde_json to parse and rust_xlsxwriter to create Excel
    // This is a simplified placeholder
    Ok(format!(
        r#"<div>
            <p>Summary data from {}</p>
            <p><i>Note: Excel export would be generated here with proper implementation</i></p>
        </div>"#,
        json_path
    ))
}

fn generate_coverage_links(data_root: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut html = String::from("<h2>Individual Sample Coverage Figures</h2><p2>");

    let pattern = format!("{}/coveragefig*linear.json", data_root);
    let paths = glob::glob(&pattern)?;

    let mut found_any = false;
    for path in paths.flatten() {
        found_any = true;
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let sample = filename
            .replace("coveragefig_", "")
            .replace("_linear.json", "");

        // Generate individual coverage HTML files
        let coverage_json = fs::read_to_string(&path)?;
        let sankey_path = format!("{}/readsfig_{}.json", data_root, sample);

        if let Ok(sankey_json) = fs::read_to_string(&sankey_path) {
            let output_path = format!("MIRA_{}_coverage.html", sample);
            let coverage_html = format!(
                r#"<html><body>
                <div id="sankey"></div>
                <div id="coverage"></div>
                <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
                <script>
                    var sankeyData = {};
                    Plotly.newPlot('sankey', sankeyData.data, sankeyData.layout, {{
                        toImageButtonOptions: {{ format: 'svg', filename: 'MIRA_{}_sankey.svg' }}
                    }});
                    var coverageData = {};
                    Plotly.newPlot('coverage', coverageData.data, coverageData.layout, {{
                        toImageButtonOptions: {{ format: 'svg', filename: 'MIRA_{}_coverage.svg' }}
                    }});
                </script>
                </body></html>"#,
                sankey_json, sample, coverage_json, sample
            );
            fs::write(&output_path, coverage_html)?;
        }

        html.push_str(&format!(
            r#"<a href="./MIRA_{}_coverage.html" target="_blank">{}</a><br>"#,
            sample, sample
        ));
    }

    html.push_str("</p2>");

    if !found_any {
        return Err("No coverage files found".into());
    }

    Ok(html)
}

fn process_dais_vars(
    data_root: &str,
    run_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let json_path = format!("{}/dais_vars.json", data_root);
    let _json_content = fs::read_to_string(&json_path)?;

    // Would create Excel file here
    let _excel_path = format!("MIRA_{}_aavars.xlsx", run_name);

    // Placeholder for actual table generation
    Ok("<p>AA Variants table would be generated here</p>".to_string())
}

fn process_minor_variants(
    data_root: &str,
    run_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let json_path = format!("{}/alleles.json", data_root);
    let _json_content = fs::read_to_string(&json_path)?;

    // Would create Excel file here
    let excel_path = format!("MIRA_{}_minorvariants.xlsx", run_name);

    Ok(format!(
        r#"<p2><a href="./{}" download>Download minor variants table</a></p2><br>"#,
        excel_path
    ))
}

fn process_indels(data_root: &str, run_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let json_path = format!("{}/indels.json", data_root);
    let _json_content = fs::read_to_string(&json_path)?;

    // Would create Excel file here
    let excel_path = format!("MIRA_{}_minorindels.xlsx", run_name);

    Ok(format!(
        r#"<p2><a href="./{}" download>Download minor indels table</a></p2><br>"#,
        excel_path
    ))
}

fn generate_fasta_links(
    data_root: &str,
    run_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut html =
        String::from(r#"<h2>Fasta downloads</h2><p3>(Right-click->"Save link as...")</p3><br><p>"#);

    let pattern = format!("{}/*fasta", data_root);
    let paths = glob::glob(&pattern)?;

    let mut found_any = false;
    for path in paths.flatten() {
        found_any = true;
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let content = fs::read_to_string(&path)?;
        let output_path = format!("MIRA_{}_{}", run_name, filename);
        fs::write(&output_path, content)?;

        html.push_str(&format!(
            r#"<a href="./{}" download>{}</a><br><br>"#,
            output_path, filename
        ));
    }

    html.push_str("</p2>");

    if !found_any {
        return Err("No fasta files found".into());
    }

    Ok(html)
}
