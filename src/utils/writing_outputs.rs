use serde::Serialize;
use serde_json::json;
use std::error::Error;

// Function to serialize a vector of structs into split-oriented JSON with precision and index
pub fn write_structs_to_split_json_file<T: Serialize>(
    file_path: &str,
    data: &Vec<T>,
    columns: Vec<&str>,
) -> Result<(), Box<dyn Error>> {
    // Create the "split-oriented" JSON structure
    let split_json = json!({
        "columns": columns,
        "index": (0..data.len()).collect::<Vec<_>>(),
        "data": data.iter().map(|item| {
            // Serialize each struct into a JSON value
            let serialized = serde_json::to_value(item).unwrap();
            let object = serialized.as_object().unwrap();
            println!("{object:?}");

            // Extract fields in the order specified by `columns`
            columns.iter().map(|&column| {
                if let Some(value) = object.get(column) {
                    if value.is_number() {
                        // Format floating-point numbers to 3 decimal places
                        if let Some(f) = value.as_f64() {
                            json!(format!("{:.3}", f))
                        } else {
                            value.clone()
                        }
                    } else {
                        value.clone() // Use the value directly for strings and other types
                    }
                } else {
                    // Handle missing fields gracefully
                    json!(null)
                }
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    });

    // Write the JSON to a file
    std::fs::write(file_path, serde_json::to_string_pretty(&split_json)?)?;

    println!("Split-oriented JSON written to {file_path}");

    Ok(())
}
