//! NovyWave Example Waveform Validation Tool
//!
//! Validates VCD, FST, and GHW waveform files using the same wellen library
//! that NovyWave uses for parsing.
//!
//! Usage: cargo run --release -- [--verbose]

use colored::Colorize;
use std::path::Path;
use wellen::{viewers, FileFormat, LoadOptions, ScopeOrVar};

struct ExampleFile {
    path: &'static str,
    expected_signals: &'static [&'static str],
    description: &'static str,
}

const EXAMPLES: &[ExampleFile] = &[
    ExampleFile {
        path: "vhdl/counter/counter.ghw",
        expected_signals: &["clk", "reset", "enable", "count", "overflow"],
        description: "VHDL 8-bit counter (GHDL)",
    },
    ExampleFile {
        path: "verilog/counter/counter.vcd",
        expected_signals: &["clk", "reset", "enable", "count", "overflow"],
        description: "Verilog 8-bit counter (Icarus)",
    },
    ExampleFile {
        path: "spinalhdl/counter/counter.vcd",
        expected_signals: &["clk", "reset", "io_enable", "io_count", "io_overflow"],
        description: "SpinalHDL 8-bit counter (Verilator)",
    },
    ExampleFile {
        path: "amaranth/counter/counter.vcd",
        expected_signals: &["clk", "rst", "enable", "count", "overflow"],
        description: "Amaranth 8-bit counter (Python)",
    },
    ExampleFile {
        path: "spade/counter/counter.vcd",
        expected_signals: &["clk", "rst", "enable", "count", "overflow"],
        description: "Spade 8-bit counter (Icarus)",
    },
];

#[derive(Default)]
struct ValidationResult {
    passed: bool,
    format: Option<FileFormat>,
    timescale: Option<String>,
    signal_count: usize,
    scope_count: usize,
    time_min: Option<u64>,
    time_max: Option<u64>,
    found_signals: Vec<String>,
    missing_signals: Vec<String>,
    error: Option<String>,
}

fn validate_file(base_path: &Path, example: &ExampleFile, verbose: bool) -> ValidationResult {
    let mut result = ValidationResult::default();
    let file_path = base_path.join(example.path);

    if !file_path.exists() {
        result.error = Some(format!("File not found: {}", file_path.display()));
        return result;
    }

    let options = LoadOptions::default();
    let header = match viewers::read_header_from_file(&file_path, &options) {
        Ok(h) => h,
        Err(e) => {
            result.error = Some(format!("Failed to parse header: {}", e));
            return result;
        }
    };

    result.format = Some(header.file_format);

    if let Some(ts) = header.hierarchy.timescale() {
        result.timescale = Some(format!("{}*{:?}", ts.factor, ts.unit));
    }

    let mut signal_names = Vec::new();
    collect_signals(&header.hierarchy, "", &mut signal_names);
    result.signal_count = signal_names.len();

    result.scope_count = count_scopes(&header.hierarchy);

    for expected in example.expected_signals {
        let found = signal_names
            .iter()
            .any(|name| name.to_lowercase().contains(&expected.to_lowercase()));
        if found {
            result.found_signals.push(expected.to_string());
        } else {
            result.missing_signals.push(expected.to_string());
        }
    }

    match viewers::read_body(header.body, &header.hierarchy, None) {
        Ok(body) => {
            if let Some(&min) = body.time_table.first() {
                result.time_min = Some(min);
            }
            if let Some(&max) = body.time_table.last() {
                result.time_max = Some(max);
            }
        }
        Err(e) => {
            if verbose {
                eprintln!("  {} Could not read body: {}", "!".yellow(), e);
            }
        }
    }

    result.passed = result.error.is_none()
        && result.signal_count > 0
        && result.missing_signals.is_empty();

    result
}

fn collect_signals(hierarchy: &wellen::Hierarchy, prefix: &str, signals: &mut Vec<String>) {
    for item_ref in hierarchy.items() {
        match item_ref.deref(hierarchy) {
            ScopeOrVar::Scope(scope) => {
                let new_prefix = if prefix.is_empty() {
                    scope.name(hierarchy).to_string()
                } else {
                    format!("{}.{}", prefix, scope.name(hierarchy))
                };
                collect_signals_from_scope(hierarchy, scope, &new_prefix, signals);
            }
            ScopeOrVar::Var(var) => {
                let name = if prefix.is_empty() {
                    var.name(hierarchy).to_string()
                } else {
                    format!("{}.{}", prefix, var.name(hierarchy))
                };
                signals.push(name);
            }
        }
    }
}

fn collect_signals_from_scope(
    hierarchy: &wellen::Hierarchy,
    scope: &wellen::Scope,
    prefix: &str,
    signals: &mut Vec<String>,
) {
    for item_ref in scope.items(hierarchy) {
        match item_ref.deref(hierarchy) {
            ScopeOrVar::Scope(child_scope) => {
                let new_prefix = format!("{}.{}", prefix, child_scope.name(hierarchy));
                collect_signals_from_scope(hierarchy, child_scope, &new_prefix, signals);
            }
            ScopeOrVar::Var(var) => {
                let name = format!("{}.{}", prefix, var.name(hierarchy));
                signals.push(name);
            }
        }
    }
}

fn count_scopes(hierarchy: &wellen::Hierarchy) -> usize {
    let mut count = 0;
    for item_ref in hierarchy.items() {
        if let ScopeOrVar::Scope(scope) = item_ref.deref(hierarchy) {
            count += 1;
            count += count_scopes_recursive(hierarchy, scope);
        }
    }
    count
}

fn count_scopes_recursive(hierarchy: &wellen::Hierarchy, scope: &wellen::Scope) -> usize {
    let mut count = 0;
    for item_ref in scope.items(hierarchy) {
        if let ScopeOrVar::Scope(child_scope) = item_ref.deref(hierarchy) {
            count += 1;
            count += count_scopes_recursive(hierarchy, child_scope);
        }
    }
    count
}

fn format_time(time: u64) -> String {
    if time >= 1_000_000_000_000 {
        format!("{:.2}s", time as f64 / 1_000_000_000_000.0)
    } else if time >= 1_000_000_000 {
        format!("{:.2}ms", time as f64 / 1_000_000_000.0)
    } else if time >= 1_000_000 {
        format!("{:.2}us", time as f64 / 1_000_000.0)
    } else if time >= 1_000 {
        format!("{:.2}ns", time as f64 / 1_000.0)
    } else {
        format!("{}ps", time)
    }
}

fn main() {
    let verbose = std::env::args().any(|arg| arg == "--verbose" || arg == "-v");

    println!();
    println!("{}", "=".repeat(60).bold());
    println!("{}", "  NovyWave Example Waveform Validation (Rust/wellen)".bold());
    println!("{}", "=".repeat(60).bold());
    println!();

    let exe_path = std::env::current_exe().ok();
    let base_path = exe_path
        .as_ref()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let examples_path = if base_path.join("vhdl").exists() {
        base_path.clone()
    } else if base_path.join("examples").join("vhdl").exists() {
        base_path.join("examples")
    } else {
        std::env::current_dir().unwrap()
    };

    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;

    for example in EXAMPLES {
        total += 1;
        let format_str = example
            .path
            .rsplit('.')
            .next()
            .unwrap_or("???")
            .to_uppercase();

        println!(
            "{}",
            format!("[{}] {}", format_str, example.path).bold()
        );
        println!("{}", "-".repeat(50));
        println!("  {} {}", "Description:".blue(), example.description);

        let result = validate_file(&examples_path, example, verbose);

        if let Some(ref err) = result.error {
            println!("  {} {}", "X".red(), err);
            failed += 1;
            println!();
            println!("  {}", "FAILED".red().bold());
            println!();
            continue;
        }

        let file_path = examples_path.join(example.path);
        let file_size = std::fs::metadata(&file_path)
            .map(|m| m.len())
            .unwrap_or(0);
        println!(
            "  {} File exists ({} bytes)",
            "v".green(),
            file_size.to_string().cyan()
        );

        if let Some(format) = &result.format {
            println!("  {} Format: {:?}", "v".green(), format);
        }

        if let Some(ts) = &result.timescale {
            println!("  {} Timescale: {}", "i".blue(), ts);
        }

        println!(
            "  {} Scopes: {}, Signals: {}",
            "i".blue(),
            result.scope_count.to_string().cyan(),
            result.signal_count.to_string().cyan()
        );

        if let (Some(min), Some(max)) = (result.time_min, result.time_max) {
            println!(
                "  {} Time range: {} - {}",
                "i".blue(),
                format_time(min).cyan(),
                format_time(max).cyan()
            );
        }

        if result.missing_signals.is_empty() {
            println!(
                "  {} All expected signals found ({}/{})",
                "v".green(),
                result.found_signals.len(),
                example.expected_signals.len()
            );
        } else {
            println!(
                "  {} Missing signals: {:?}",
                "!".yellow(),
                result.missing_signals
            );
        }

        if verbose {
            println!("  {} Found signals:", "i".blue());
            for sig in &result.found_signals {
                println!("    - {}", sig.green());
            }
        }

        println!();
        if result.passed {
            passed += 1;
            println!("  {}", "PASSED".green().bold());
        } else {
            failed += 1;
            println!("  {}", "FAILED".red().bold());
        }
        println!();
    }

    println!("{}", "=".repeat(60).bold());
    println!("{}", "  Summary".bold());
    println!("{}", "=".repeat(60).bold());
    println!();
    println!("  Total:  {}", total);
    println!("  {}: {}", "Passed".green(), passed);
    if failed > 0 {
        println!("  {}: {}", "Failed".red(), failed);
    }
    println!();

    std::process::exit(if failed > 0 { 1 } else { 0 });
}
