use crate::utils::data_ingest::ReadsData;
use arrow::array::{ArrayRef, Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::FileWriter;
use arrow::record_batch::RecordBatch;
use csv::Writer;
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::{error::Error, fs::File};

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

/// Write parquet
///
pub fn extract_field<T, U, F>(data: Vec<T>, extractor: F) -> Vec<U>
where
    F: Fn(&T) -> U,
{
    data.iter().map(extractor).collect()
}

pub fn write_reads_to_parquet(
    reads_data: &Vec<ReadsData>,
    output_file: &str,
) -> Result<(), Box<dyn Error>> {
    //Convert values in struct to vector of values
    let sample_ids_vec: Vec<Option<String>> =
        extract_field(reads_data.clone(), |item| item.sample_id.clone());
    println!("data {sample_ids_vec:?}");
    let record_vec: Vec<String> = extract_field(reads_data.clone(), |item| item.record.clone());
    let reads_vec: Vec<i32> = extract_field(reads_data.clone(), |item| item.reads);
    let patterns_vec: Vec<String> = extract_field(reads_data.clone(), |item| item.patterns.clone());
    let pairs_and_windows_vec: Vec<String> =
        extract_field(reads_data.clone(), |item| item.pairs_and_windows.clone());
    let stages_vec: Vec<Option<String>> =
        extract_field(reads_data.clone(), |item| item.stage.clone());

    // Print the extracted fields
    //println!("Sample IDs: {:?}", sample_ids_vec);
    //println!("Records: {:?}", record_vec);
    //println!("Reads: {:?}", reads_vec);
    //println!("Patterns: {:?}", patterns_vec);
    //println!("Pairs and Windows: {:?}", pairs_and_windows_vec);
    //println!("Stages: {:?}", stages_vec);

    // Convert the vectors into Arrow columns
    let sample_array: ArrayRef = Arc::new(StringArray::from(sample_ids_vec));
    let record_array: ArrayRef = Arc::new(StringArray::from(record_vec));
    let reads_array: ArrayRef = Arc::new(Int32Array::from(reads_vec));
    let patterns_array: ArrayRef = Arc::new(StringArray::from(patterns_vec));
    let pairs_and_windows_array: ArrayRef = Arc::new(StringArray::from(pairs_and_windows_vec));
    let stage_array: ArrayRef = Arc::new(StringArray::from(stages_vec));
    println!("Sample IDs: {:?}", sample_array);
    println!("Records: {:?}", record_array);
    println!("Reads: {:?}", reads_array);
    //println!("Patterns: {:?}", patterns_vec);
    //println!("Pairs and Windows: {:?}", pairs_and_windows_vec);
    //println!("Stages: {:?}", stages_vec);

    // Define the schema for the Arrow IPC file
    let fields = vec![
        Field::new("sample_id", DataType::Utf8, true),
        Field::new("record", DataType::Utf8, true),
        Field::new("reads", DataType::Int32, true), // Fixed: Use Int32 for reads
        Field::new("patterns", DataType::Utf8, true),
        Field::new("pairs_and_windows", DataType::Utf8, true),
        Field::new("stage", DataType::Utf8, true),
    ];
    let schema = Arc::new(Schema::new(fields));
    println!("SCHEMA {schema:?}");

    // Create a RecordBatch
    let record_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            sample_array,
            record_array,
            reads_array,
            patterns_array,
            pairs_and_windows_array,
            stage_array,
        ],
    )?;

    //Write the RecordBatch to an Arrow IPC file
    let file = File::create(output_file)?;
    let mut writer = FileWriter::try_new(file, &schema)?;
    writer.write(&record_batch)?;
    writer.finish()?;

    println!("RECORD BATCH {record_batch:?}");

    Ok(())
}
