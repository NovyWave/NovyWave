// Investigation script to understand Wellen's timescale handling
use wellen::*;

fn main() {
    println!("Investigating Wellen timescale handling...");
    
    let file_path = "test_files/simple.vcd";
    let options = LoadOptions::default();
    
    match viewers::read_header_from_file(file_path, &options) {
        Ok(header_result) => {
            println!("File format: {:?}", header_result.file_format);
            
            // Try to find timescale information
            // These are potential ways to access timescale - we'll see which ones exist
            // println!("Timescale: {:?}", header_result.timescale); // Might exist
            // println!("Time unit: {:?}", header_result.time_unit); // Might exist
            // println!("Header: {:?}", header_result); // See all available fields
            
            match viewers::read_body(header_result.body, &header_result.hierarchy, None) {
                Ok(body_result) => {
                    println!("Time table length: {}", body_result.time_table.len());
                    println!("First few time entries:");
                    for (i, &time) in body_result.time_table.iter().take(5).enumerate() {
                        println!("  [{}] = {}", i, time);
                    }
                    
                    // Check if there's timescale info in the body
                    // println!("Body timescale: {:?}", body_result.timescale); // Might exist
                }
                Err(e) => println!("Error reading body: {}", e),
            }
        }
        Err(e) => println!("Error reading header: {}", e),
    }
}