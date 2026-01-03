use wellen::viewers::read_header_from_file;
use wellen::LoadOptions;

fn main() {
    let path = "examples/spade/counter/counter.vcd";
    println!("Testing wellen parsing of: {}", path);

    let options = LoadOptions::default();

    println!("Calling read_header_from_file...");
    match read_header_from_file(path, &options) {
        Ok(header) => {
            println!("SUCCESS! File format: {:?}", header.file_format);
            println!("Timescale: {:?}", header.hierarchy.timescale());
        }
        Err(e) => {
            println!("ERROR: {}", e);
        }
    }
    println!("Done.");
}
