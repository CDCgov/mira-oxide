use crate::utils::data_processing::IRMASummary;
use serde_json::json;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;

fn assign_number(reason: &str) -> i32 {
    if reason == "No assembly" {
        4
    } else if reason == "Pass" {
        -4
    } else if reason.split(';').count() > 1 {
        let count = reason.split(';').count() as i32;
        count
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
#[warn(clippy::too_many_lines)]
pub fn create_passfail_heatmap(summaries: &[IRMASummary], virus: &str, output_path: &str) {
    println!("Building pass_fail_heatmap as flat JSON");

    let mut records = Vec::new();

    for summary in summaries {
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
        reason = remove_brace_content(&reason);

        records.push((sample, reference, reason));
    }

    // Remove duplicates
    let mut seen = HashSet::new();
    let mut unique_records = Vec::new();
    for (sample, reference, reason) in records {
        let key = format!("{sample}|{reference}");
        if !seen.contains(&key) {
            seen.insert(key.clone());
            unique_records.push((sample, reference, reason));
        }
    }

    // Flat arrays for x, y, z, customdata
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut z = Vec::new();
    let mut customdata = Vec::new();

    for (sample, reference, reason) in &unique_records {
        x.push(sample.clone());
        y.push(reference.clone());
        z.push(assign_number(reason));
        customdata.push(reason.clone());
    }

    // Custom colorscale
    let colorscale = vec![
        (0.0, "rgb(160,200,255)"),
        (0.25, "rgb(255,255,255)"),
        (0.5, "rgb(230,210,0)"),
        (0.75, "rgb(230,0,0)"),
        (1.0, "rgb(0,0,0)"),
    ];

    // The full Plotly template from your example (copy-paste as is)
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
            "colorway": ["#636efa","#EF553B","#00cc96","#ab63fa","#FFA15A","#19d3f3","#FF6692","#B6E880","#FF97FF","#FECB52"],
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
                "diverging": [
                    [0,"#8e0152"],[0.1,"#c51b7d"],[0.2,"#de77ae"],[0.3,"#f1b6da"],[0.4,"#fde0ef"],[0.5,"#f7f7f7"],[0.6,"#e6f5d0"],[0.7,"#b8e186"],[0.8,"#7fbc41"],[0.9,"#4d9221"],[1,"#276419"]
                ]
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

    // Build the heatmap trace
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

    // Build the layout
    let layout = json!({
        "template": plotly_template,
        "xaxis": {"side": "top"},
        "paper_bgcolor": "white",
        "plot_bgcolor": "white"
    });

    // Combine into final plot JSON
    let plot_json = json!({
        "data": [heatmap],
        "layout": layout
    });

    // Save JSON to file
    let file_path = format!("{output_path}/pass_fail_heatmap.json");
    let mut file = File::create(&file_path).expect("Unable to create file");
    file.write_all(plot_json.to_string().as_bytes())
        .expect("Unable to write data");

    println!("  -> pass_fail heatmap json saved to {file_path}");
}
