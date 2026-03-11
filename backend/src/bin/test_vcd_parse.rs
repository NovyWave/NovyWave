use std::collections::HashMap;

use wellen::LoadOptions;
use wellen::viewers::{read_body, read_header_from_file};

fn build_signals_for_scope_recursive(
    hierarchy: &wellen::Hierarchy,
    scope_ref: wellen::ScopeRef,
    signals: &mut HashMap<String, wellen::SignalRef>,
) {
    let scope = &hierarchy[scope_ref];
    let scope_path = scope.full_name(hierarchy);

    for var_ref in scope.vars(hierarchy) {
        let var = &hierarchy[var_ref];
        let variable_name = var.name(hierarchy);
        signals.insert(
            format!("{}|{}", scope_path, variable_name),
            var.signal_ref(),
        );
    }

    for child_scope_ref in scope.scopes(hierarchy) {
        build_signals_for_scope_recursive(hierarchy, child_scope_ref, signals);
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/novywave_ai_workspace/analog.vcd".to_string());
    let requested_signal = std::env::args().nth(2);
    println!("Testing wellen parsing of: {}", path);

    let options = LoadOptions::default();

    println!("Calling read_header_from_file...");
    let header = match read_header_from_file(&path, &options) {
        Ok(header) => header,
        Err(e) => {
            println!("ERROR: {}", e);
            return;
        }
    };

    println!("SUCCESS! File format: {:?}", header.file_format);
    println!("Timescale: {:?}", header.hierarchy.timescale());

    let hierarchy = header.hierarchy;
    let body = header.body;
    let mut body_result = match read_body(body, &hierarchy, None) {
        Ok(body) => body,
        Err(e) => {
            println!("BODY ERROR: {}", e);
            return;
        }
    };

    println!("time table len: {}", body_result.time_table.len());
    let mut signals = HashMap::new();
    for scope_ref in hierarchy.scopes() {
        build_signals_for_scope_recursive(&hierarchy, scope_ref, &mut signals);
    }
    println!("signal count: {}", signals.len());
    println!(
        "available signals: {:?}",
        signals.keys().take(10).collect::<Vec<_>>()
    );

    let signal_key = requested_signal.unwrap_or_else(|| "top|analog".to_string());
    let Some(signal_ref) = signals.get(&signal_key).copied() else {
        println!("signal not found: {}", signal_key);
        return;
    };

    let loaded = body_result
        .source
        .load_signals(&[signal_ref], &hierarchy, true)
        .into_iter()
        .next();
    let Some((_, signal)) = loaded else {
        println!("signal load returned nothing");
        return;
    };

    for (index, time) in body_result.time_table.iter().enumerate().take(16) {
        println!("index={index} time={time}");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            signal
                .get_offset(index as u32)
                .map(|offset| signal.get_value_at(&offset, 0))
        }));
        match result {
            Ok(Some(value)) => {
                let bits = match &value {
                    wellen::SignalValue::Real(_) => None,
                    _ => value.to_bit_string(),
                };
                println!("  value={value:?} bits={bits:?}");
            }
            Ok(None) => println!("  no offset"),
            Err(_) => {
                println!("  PANIC while reading value at index {index}");
                return;
            }
        }
    }

    println!("Done.");
}
