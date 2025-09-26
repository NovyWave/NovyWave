use wellen::viewers;

fn main() {
    let path = std::env::args().nth(1).expect("usage: inspect_fst <file>");
    let options = wellen::LoadOptions::default();
    let header = viewers::read_header_from_file(&path, &options).expect("header");
    println!("format: {:?}", header.file_format);
    if let Some(ts) = header.hierarchy.timescale() {
        println!(
            "embedded timescale: factor={} unit={:?}",
            ts.factor, ts.unit
        );
    }
    let body = viewers::read_body(header.body, &header.hierarchy, None).expect("body");
    if let (Some(min), Some(max)) = (body.time_table.first(), body.time_table.last()) {
        println!("raw min {} raw max {} raw range {}", min, max, max - min);
    }
}
