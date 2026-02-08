//! Mock update server for testing NovyWave's Tauri updater.
//! Serves update metadata and bundles with throttled download speed.

use base64::{engine::general_purpose::STANDARD, Engine};
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

    let encoded_signature = STANDARD.encode(signature.trim());

    let mut platforms = HashMap::new();
    for platform in ["linux-x86_64", "darwin-x86_64", "darwin-aarch64", "windows-x86_64"] {
        platforms.insert(
            platform.to_string(),
            PlatformInfo {
                signature: encoded_signature.clone(),
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

struct ThrottledReader {
    data: std::io::Cursor<Vec<u8>>,
    target_bytes_per_sec: u64,
    total_read: u64,
    file_size: u64,
    last_percent: u64,
}

impl ThrottledReader {
    fn new(data: Vec<u8>, target_bytes_per_sec: u64) -> Self {
        let file_size = data.len() as u64;
        Self {
            data: std::io::Cursor::new(data),
            target_bytes_per_sec,
            total_read: 0,
            file_size,
            last_percent: 0,
        }
    }
}

impl Read for ThrottledReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.data.read(buf)?;
        if n > 0 {
            self.total_read += n as u64;
            let percent = (self.total_read * 100) / self.file_size;
            if percent != self.last_percent {
                print!(
                    "\r  -> Progress: {}% ({}/{} bytes)",
                    percent, self.total_read, self.file_size
                );
                let _ = std::io::stdout().flush();
                self.last_percent = percent;
            }
            if self.target_bytes_per_sec > 0 {
                let delay_us = (n as u64 * 1_000_000) / self.target_bytes_per_sec;
                if delay_us > 0 {
                    thread::sleep(Duration::from_micros(delay_us));
                }
            }
        }
        Ok(n)
    }
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

    let target_bytes_per_sec = args.throttle_kbps as u64 * 1024;

    let estimated_seconds = if target_bytes_per_sec > 0 {
        file_size / target_bytes_per_sec
    } else {
        0
    };

    println!(
        "  -> Serving bundle ({} bytes, ~{}s at {} KB/s)",
        file_size, estimated_seconds, args.throttle_kbps
    );

    let mut file_contents = Vec::new();
    if let Err(e) = file.read_to_end(&mut file_contents) {
        eprintln!("  -> Error reading bundle: {}", e);
        let _ = request.respond(Response::empty(500));
        return;
    }

    let content_type = Header::from_bytes("Content-Type", "application/octet-stream").unwrap();

    let reader = ThrottledReader::new(file_contents, target_bytes_per_sec);
    let response = Response::new(
        tiny_http::StatusCode(200),
        vec![content_type],
        reader,
        Some(file_size as usize),
        None,
    )
    .with_chunked_threshold(file_size as usize + 1);
    let _ = request.respond(response);
    println!("\n  -> Bundle transfer complete");
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
