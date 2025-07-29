use csv::Writer;
use serde::Serialize;
use serde_json::{Value, json};
use std::error::Error;

/// Function to serialize a vector of structs into split-oriented JSON with precision and indexing
pub fn write_structs_to_split_json_file<T: Serialize>(
    file_path: &str,
    data: &Vec<T>,
    columns: &Vec<&str>,
    struct_values: &Vec<&str>,
) -> Result<(), Box<dyn Error>> {
    // Create the "split-oriented" JSON structure
    let split_json = json!({
        "columns": columns,
        "index": (0..data.len()).collect::<Vec<_>>(),
        "data": data.iter().map(|item| {
            // Serialize each struct into a JSON value
            let serialized = serde_json::to_value(item).unwrap();
            let object = serialized.as_object().unwrap();

            // Extract fields in the order specified by `columns`
            //TODO add in float handling
            struct_values.iter().map(|&struct_values| {
                if let Some(value) = object.get(struct_values) {
                    if value.is_f64() {
                        // float precision set to 3 decimal places
                        if let Some(f) = value.as_f64() {
                            json!(format!("{:.3}", f))
                        } else {
                            value.clone()
                        }
                    } else {
                        value.clone()
                    }
                } else {
                    json!(null)
                }
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    });

    std::fs::write(file_path, serde_json::to_string_pretty(&split_json)?)?;

    println!("Split-oriented JSON written to {file_path}");

    Ok(())
}

/// Write to CSV
pub fn write_structs_to_csv_file<T: Serialize>(
    file_path: &str,
    data: &Vec<T>,
    columns: &Vec<&str>,
    struct_values: &Vec<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut csv_writer = Writer::from_path(file_path)?;

    // Write custom headers to the CSV file
    csv_writer.write_record(columns)?;

    for line in data {
        // Serialize the struct into a JSON object
        // This was the most effectient way to select columns for csv file
        let json_value: Value = serde_json::to_value(line)?;

        // Extract the specified fields from the JSON object
        let row: Vec<String> = struct_values
            .iter()
            .map(|field| {
                json_value
                    .get(*field)
                    .map_or(String::new(), |v| v.to_string().replace('"', ""))
            })
            .collect();

        csv_writer.write_record(row)?;
    }

    csv_writer.flush()?;
    println!("CSV written to {file_path}");

    Ok(())
}
