//! Mock update server for testing NovyWave's Tauri updater.
//! Serves update metadata and bundles with throttled download speed.

use clap::Parser;
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::PathBuf,
    thread,
    time::Duration,
};
use tiny_http::{Header, Response, Server};

#[derive(Parser)]
#[command(name = "mock-update-server")]
#[command(about = "Mock server for testing Tauri updater")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8888")]
    port: u16,

    /// Download throttle in KB/s (0 = unlimited)
    #[arg(short, long, default_value = "100")]
    throttle_kbps: u32,

    /// Path to the update bundle (.tar.gz or .AppImage)
    #[arg(short, long)]
    bundle: PathBuf,

    /// Path to the signature file (.sig)
    #[arg(short, long)]
    signature: PathBuf,

    /// Version to advertise
    #[arg(short = 'V', long, default_value = "99.0.0")]
    version: String,
}

#[derive(Serialize)]
struct UpdateResponse {
    version: String,
    notes: String,
    pub_date: String,
    platforms: HashMap<String, PlatformInfo>,
}

#[derive(Serialize)]
struct PlatformInfo {
    signature: String,
    url: String,
}

fn main() {
    let args = Args::parse();

    // Read signature file
    let signature = fs::read_to_string(&args.signature)
        .expect("Failed to read signature file");

    // Verify bundle exists
    if !args.bundle.exists() {
        eprintln!("Bundle file not found: {:?}", args.bundle);
        std::process::exit(1);
    }

    let bundle_size = fs::metadata(&args.bundle).unwrap().len();

    let addr = format!("127.0.0.1:{}", args.port);
    let server = Server::http(&addr).expect("Failed to start server");

    println!("======================================================================");
    println!("           NovyWave Mock Update Server (Rust)                         ");
    println!("======================================================================");
    println!("  URL: http://{}", addr);
    println!("  Version: {}", args.version);
    println!("  Bundle: {:?} ({} bytes)", args.bundle.file_name().unwrap(), bundle_size);
    println!("  Throttle: {} KB/s", args.throttle_kbps);
    println!("----------------------------------------------------------------------");
    println!("  Endpoints:");
    println!("    GET /latest.json     - Update metadata");
    println!("    GET /bundle          - Update bundle (throttled)");
    println!("----------------------------------------------------------------------");
    println!("  Press Ctrl+C to stop");
    println!("======================================================================");
    println!();

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        println!("[{}] {} {}", timestamp(), request.method(), url);

        match url.as_str() {
            "/latest.json" => {
                serve_latest_json(request, &args, &signature);
            }
            "/bundle" => {
                serve_bundle(request, &args);
            }
            _ => {
                let _ = request.respond(Response::empty(404));
                println!("  -> 404 Not Found");
            }
        }
    }
}

fn serve_latest_json(
    request: tiny_http::Request,
    args: &Args,
    signature: &str,
) {
    let base_url = format!("http://127.0.0.1:{}/bundle", args.port);

    let mut platforms = HashMap::new();
    for platform in ["linux-x86_64", "darwin-x86_64", "darwin-aarch64", "windows-x86_64"] {
        platforms.insert(
            platform.to_string(),
            PlatformInfo {
                signature: signature.trim().to_string(),
                url: base_url.clone(),
            },
        );
    }

    let response = UpdateResponse {
        version: args.version.clone(),
        notes: "Test update from mock server".to_string(),
        pub_date: "2025-12-23T00:00:00Z".to_string(),
        platforms,
    };

    let body = serde_json::to_string_pretty(&response).unwrap();
    let content_type = Header::from_bytes("Content-Type", "application/json").unwrap();

    let _ = request.respond(Response::from_string(body).with_header(content_type));
    println!("  -> Served latest.json (v{})", args.version);
}

fn serve_bundle(request: tiny_http::Request, args: &Args) {
    let mut file = match fs::File::open(&args.bundle) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  -> Error opening bundle: {}", e);
            let _ = request.respond(Response::empty(500));
            return;
        }
    };

    let file_size = file.metadata().unwrap().len();

    // Calculate chunk size and delay for throttling
    let chunk_size: usize = 8192; // 8KB chunks
    let delay_ms = if args.throttle_kbps > 0 {
        (chunk_size as u64 * 1000) / (args.throttle_kbps as u64 * 1024)
    } else {
        0
    };

    let estimated_seconds = if args.throttle_kbps > 0 {
        file_size / (args.throttle_kbps as u64 * 1024)
    } else {
        0
    };

    println!(
        "  -> Serving bundle ({} bytes, ~{}s at {} KB/s)",
        file_size, estimated_seconds, args.throttle_kbps
    );

    // Read entire file into memory for streaming
    let mut file_contents = Vec::new();
    if let Err(e) = file.read_to_end(&mut file_contents) {
        eprintln!("  -> Error reading bundle: {}", e);
        let _ = request.respond(Response::empty(500));
        return;
    }

    // Create response with proper headers
    let content_type = Header::from_bytes("Content-Type", "application/octet-stream").unwrap();
    let content_length = Header::from_bytes("Content-Length", file_size.to_string()).unwrap();

    // For throttled streaming, we need to use a custom approach
    // tiny_http doesn't support chunked streaming easily, so we'll use a slower approach
    if args.throttle_kbps > 0 && delay_ms > 0 {
        // Stream with throttling using raw writer
        let mut writer = request.into_writer();

        // Write HTTP response headers
        let _ = write!(writer, "HTTP/1.1 200 OK\r\n");
        let _ = write!(writer, "Content-Type: application/octet-stream\r\n");
        let _ = write!(writer, "Content-Length: {}\r\n", file_size);
        let _ = write!(writer, "\r\n");

        // Stream file with throttling
        let mut total_sent = 0u64;
        let mut last_percent = 0;

        for chunk in file_contents.chunks(chunk_size) {
            if let Err(e) = writer.write_all(chunk) {
                eprintln!("\n  -> Error writing chunk: {}", e);
                return;
            }
            let _ = writer.flush();

            total_sent += chunk.len() as u64;
            let percent = (total_sent * 100) / file_size;

            if percent != last_percent {
                print!("\r  -> Progress: {}% ({}/{} bytes)", percent, total_sent, file_size);
                let _ = std::io::stdout().flush();
                last_percent = percent;
            }

            thread::sleep(Duration::from_millis(delay_ms));
        }

        println!("\n  -> Bundle transfer complete");
    } else {
        // Fast path: no throttling, serve directly
        let response = Response::from_data(file_contents)
            .with_header(content_type)
            .with_header(content_length);
        let _ = request.respond(response);
        println!("  -> Bundle served (no throttling)");
    }
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple timestamp - last 5 digits for readability
    format!("{:05}", secs % 100000)
}
