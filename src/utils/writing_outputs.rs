use serde::Serialize;
use serde_json::json;
use std::error::Error;

/// Function to serialize a vector of structs into split-oriented JSON with precision and indexing
pub fn write_structs_to_split_json_file<T: Serialize>(
    file_path: &str,
    data: &Vec<T>,
    columns: Vec<&str>,
    headers: Vec<&str>,
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
            //TODO: fix header vs column nme situation
            headers.iter().map(|&headers| {
                if let Some(value) = object.get(headers) {
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
