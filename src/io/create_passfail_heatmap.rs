use crate::constants::heatmap_ref::get_references_for_virus;
use crate::utils::data_processing::IRMASummary;
use serde_json::json;
use std::fs::File;
use std::io::Write;

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn assign_number(reason: &str) -> i32 {
    if reason == "No assembly" || reason == "Fail" {
        4
    } else if reason == "Pass" {
        -4
    } else if reason.split(';').count() > 1 {
        reason.split(';').count() as i32
    } else {
        -1
    }
}

fn normalize_reference(reference: &str, virus: &str) -> String {
    match virus {
        "flu" => {
            let parts: Vec<&str> = reference.split('_').collect();
            if parts.len() >= 2 {
                parts[1].to_string()
            } else {
                reference.to_string()
            }
        }
        "rsv" => reference
            .replace("_AD", "")
            .replace("_BD", "")
            .replace("_A", "")
            .replace("_B", ""),
        _ => reference.to_string(),
    }
}

fn remove_brace_content(s: &str) -> String {
    let mut result = String::new();
    let mut in_brace = false;
    for c in s.chars() {
        if c == '{' {
            in_brace = true;
            continue;
        }
        if c == '}' && in_brace {
            in_brace = false;
            continue;
        }
        if !in_brace {
            result.push(c);
        }
    }
    result.trim().to_string()
}

// Function for grabbing sample_id and pass/fail reason from summaries
// Also checking that there is a record for each sample/reference combination
fn build_records(
    summaries: &[IRMASummary],
    heatmap_refs: &[String],
    sample_list: &[String],
    virus: &str,
) -> Vec<(String, String, String)> {
    let mut records = Vec::new();

    for sample in sample_list {
        for reference in heatmap_refs {
            // Try to find a summary for this sample/reference
            if let Some(summary) = summaries.iter().find(|s| {
                s.sample_id.as_ref() == Some(sample)
                    && normalize_reference(
                        &s.reference.clone().unwrap_or_else(|| "Unknown".to_string()),
                        virus,
                    ) == *reference
            }) {
                let sample = summary
                    .sample_id
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                let reference = normalize_reference(
                    &summary
                        .reference
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string()),
                    virus,
                );
                let mut reason = summary
                    .pass_fail_reason
                    .clone()
                    .unwrap_or_else(|| "No assembly".to_string());
                if reason.is_empty() {
                    reason = "No assembly".to_string();
                }
                let reason = remove_brace_content(&reason);
                records.push((sample, reference, reason));
            } else {
                // Not found: add default record
                records.push((
                    sample.clone(),
                    reference.to_string(),
                    "No assembly".to_string(),
                ));
            }
        }
    }
    records
}

// Build the heatmap given the data
fn build_heatmap_arrays(
    unique_records: &[(String, String, String)],
) -> (Vec<String>, Vec<String>, Vec<i32>, Vec<String>) {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    let mut customdata = Vec::new();

    for (sample, reference, reason) in unique_records {
        x.push(sample.clone());
        y.push(reference.clone());
        z.push(assign_number(reason));
        customdata.push(reason.clone());
    }
    (x, y, z, customdata)
}

// The function to reate the plotly json
// Full plotly template
#[allow(clippy::too_many_lines)]
fn plotly_template(colorscale: &Vec<(f64, &str)>) -> serde_json::Value {
    let plotly_template = json!({
        "data": {
            "histogram2dcontour": [{
                "type": "histogram2dcontour",
                "colorbar": {"outlinewidth": 0, "ticks": ""},
                "colorscale": colorscale
            }],
            "choropleth": [{"type": "choropleth", "colorbar": {"outlinewidth": 0, "ticks": ""}}],
            "histogram2d": [{
                "type": "histogram2d",
                "colorbar": {"outlinewidth": 0, "ticks": ""},
                "colorscale": colorscale
            }],
            "heatmap": [{
                "type": "heatmap",
                "colorbar": {"outlinewidth": 0, "ticks": ""},
                "colorscale": colorscale
            }],
            "heatmapgl": [{
                "type": "heatmapgl",
                "colorbar": {"outlinewidth": 0, "ticks": ""},
                "colorscale": colorscale
            }],
            "contourcarpet": [{"type": "contourcarpet", "colorbar": {"outlinewidth": 0, "ticks": ""}}],
            "contour": [{
                "type": "contour",
                "colorbar": {"outlinewidth": 0, "ticks": ""},
                "colorscale": colorscale
            }],
            "surface": [{
                "type": "surface",
                "colorbar": {"outlinewidth": 0, "ticks": ""},
                "colorscale": colorscale
            }],
            "mesh3d": [{"type": "mesh3d", "colorbar": {"outlinewidth": 0, "ticks": ""}}],
            "scatter": [{
                "fillpattern": {"fillmode": "overlay", "size": 10, "solidity": 0.2},
                "type": "scatter"
            }],
            "parcoords": [{
                "type": "parcoords",
                "line": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "scatterpolargl": [{
                "type": "scatterpolargl",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "bar": [{
                "error_x": {"color": "#2a3f5f"},
                "error_y": {"color": "#2a3f5f"},
                "marker": {"line": {"color": "#E5ECF6", "width": 0.5}, "pattern": {"fillmode": "overlay", "size": 10, "solidity": 0.2}},
                "type": "bar"
            }],
            "scattergeo": [{
                "type": "scattergeo",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "scatterpolar": [{
                "type": "scatterpolar",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "histogram": [{
                "marker": {"pattern": {"fillmode": "overlay", "size": 10, "solidity": 0.2}},
                "type": "histogram"
            }],
            "scattergl": [{
                "type": "scattergl",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "scatter3d": [{
                "type": "scatter3d",
                "line": {"colorbar": {"outlinewidth": 0, "ticks": ""}},
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "scattermapbox": [{
                "type": "scattermapbox",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "scatterternary": [{
                "type": "scatterternary",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "scattercarpet": [{
                "type": "scattercarpet",
                "marker": {"colorbar": {"outlinewidth": 0, "ticks": ""}}
            }],
            "carpet": [{
                "aaxis": {"endlinecolor": "#2a3f5f", "gridcolor": "white", "linecolor": "white", "minorgridcolor": "white", "startlinecolor": "#2a3f5f"},
                "baxis": {"endlinecolor": "#2a3f5f", "gridcolor": "white", "linecolor": "white", "minorgridcolor": "white", "startlinecolor": "#2a3f5f"},
                "type": "carpet"
            }],
            "table": [{
                "cells": {"fill": {"color": "#EBF0F8"}, "line": {"color": "white"}},
                "header": {"fill": {"color": "#C8D4E3"}, "line": {"color": "white"}},
                "type": "table"
            }],
            "barpolar": [{
                "marker": {"line": {"color": "#E5ECF6", "width": 0.5}, "pattern": {"fillmode": "overlay", "size": 10, "solidity": 0.2}},
                "type": "barpolar"
            }],
            "pie": [{
                "automargin": true,
                "type": "pie"
            }]
        },
        "layout": {
            "autotypenumbers": "strict",
            "font": {"color": "#2a3f5f"},
            "hovermode": "closest",
            "hoverlabel": {"align": "left"},
            "paper_bgcolor": "white",
            "plot_bgcolor": "#E5ECF6",
            "polar": {
                "bgcolor": "#E5ECF6",
                "angularaxis": {"gridcolor": "white", "linecolor": "white", "ticks": ""},
                "radialaxis": {"gridcolor": "white", "linecolor": "white", "ticks": ""}
            },
            "ternary": {
                "bgcolor": "#E5ECF6",
                "aaxis": {"gridcolor": "white", "linecolor": "white", "ticks": ""},
                "baxis": {"gridcolor": "white", "linecolor": "white", "ticks": ""},
                "caxis": {"gridcolor": "white", "linecolor": "white", "ticks": ""}
            },
            "coloraxis": {"colorbar": {"outlinewidth": 0, "ticks": ""}},
            "colorscale": {
                "sequential": colorscale,
                "sequentialminus": colorscale,
            },
            "xaxis": {
                "gridcolor": "white",
                "linecolor": "white",
                "ticks": "",
                "title": {"standoff": 15},
                "zerolinecolor": "white",
                "automargin": true,
                "zerolinewidth": 2
            },
            "yaxis": {
                "gridcolor": "white",
                "linecolor": "white",
                "ticks": "",
                "title": {"standoff": 15},
                "zerolinecolor": "white",
                "automargin": true,
                "zerolinewidth": 2
            },
            "scene": {
                "xaxis": {"backgroundcolor": "#E5ECF6", "gridcolor": "white", "linecolor": "white", "showbackground": true, "ticks": "", "zerolinecolor": "white", "gridwidth": 2},
                "yaxis": {"backgroundcolor": "#E5ECF6", "gridcolor": "white", "linecolor": "white", "showbackground": true, "ticks": "", "zerolinecolor": "white", "gridwidth": 2},
                "zaxis": {"backgroundcolor": "#E5ECF6", "gridcolor": "white", "linecolor": "white", "showbackground": true, "ticks": "", "zerolinecolor": "white", "gridwidth": 2}
            },
            "shapedefaults": {"line": {"color": "#2a3f5f"}},
            "annotationdefaults": {"arrowcolor": "#2a3f5f", "arrowhead": 0, "arrowwidth": 1},
            "geo": {"bgcolor": "white", "landcolor": "#E5ECF6", "subunitcolor": "white", "showland": true, "showlakes": true, "lakecolor": "white"},
            "title": {"x": 0.05},
            "mapbox": {"style": "light"}
        }
    });

    plotly_template
}

/// Creates a `pass_fail_heatmap` figure - writes it to a file and returns the JSON object.
#[must_use]
pub fn create_passfail_heatmap(
    summaries: &[IRMASummary],
    sample_list: &[String],
    virus: &str,
    output_path: &str,
) -> serde_json::Value {
    println!("Building pass_fail_heatmap as flat JSON");

    let colorscale = vec![
        (0.0, "rgb(184, 212, 237)"),
        (0.25, "rgb(252, 235, 201)"),
        (0.5, "rgb(251, 126, 56)"),
        (0.75, "rgb(204, 27, 34)"),
        (1.0, "rgb(0,0,0)"),
    ];

    let references = get_references_for_virus(virus);
    let records = build_records(summaries, &references, sample_list, virus);
    let (x, y, z, customdata) = build_heatmap_arrays(&records);

    let heatmap = json!({
        "type": "heatmap",
        "x": x,
        "y": y,
        "z": z,
        "customdata": customdata,
        "colorscale": colorscale,
        "hovertemplate": "%{x} %{customdata} <extra>%{y}</extra>",
        "zmin": -4,
        "zmid": 1,
        "zmax": 6,
        "showscale": false
    });

    let layout = json!({
        "template": plotly_template(&colorscale),
        "xaxis": {"side": "top"},
        "paper_bgcolor": "white",
        "plot_bgcolor": "white"
    });

    let plot_json = json!({
        "data": [heatmap],
        "layout": layout
    });

    let file_path = format!("{output_path}/pass_fail_heatmap.json");
    let mut file = File::create(&file_path).expect("Unable to create file");
    file.write_all(plot_json.to_string().as_bytes())
        .expect("Unable to write data");

    println!("  -> pass_fail heatmap json saved to {file_path}");

    // Return the JSON object
    plot_json
}
