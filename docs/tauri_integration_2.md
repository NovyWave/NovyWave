# Tauri Integration Plan for NovyWave (Using tauri-wasm)

## Executive Summary

Integration strategy using **tauri-wasm** for pure Rust communication between NovyWave's WASM frontend and Tauri backend, avoiding TypeScript/JavaScript glue code complexity while maintaining web compatibility.

## Known Limitations & Issues

### Tauri Fundamental Limitations

1. **SSE/EventSource Protocol Mismatch**: WebView origin (`tauri://localhost`) incompatible with backend (`http://localhost:8080`)
2. **CORS Cannot Be Disabled**: Tauri doesn't use Chromium directly, protocol restrictions are fundamental
3. **JSON Serialization Required**: Binary protocol planned but not available (as of 2024)
4. **WASM Cannot Direct Call Native**: JavaScript bridge required for system access (WebAssembly sandbox)

### Library-Specific Issues

**tauri-wasm:**
- Minimal documentation (only basic examples)
- Not published on crates.io (git dependency required)
- Limited API surface (basic invoke/emit)
- May need forking for advanced features

**tauri-sys (avoided due to):**
- Requires global esbuild installation
- More complex setup
- Overkill for NovyWave's simple needs

**tauri-bindgen:**
- Uses .wit files (additional complexity)
- More suited for complex multi-language projects

## Architecture Design

### Two Compilation Modes
```
Web Mode (NOVYWAVE_PLATFORM="WEB"):
  Frontend (WASM) <--SSE/WebSocket--> MoonZoon Backend

Desktop Mode (NOVYWAVE_PLATFORM="TAURI"):  
  Frontend (WASM) <--tauri-wasm--> Tauri Commands <--Direct--> File System
```

## Implementation Plan

### Phase 1: Setup Infrastructure

**1.1 Add tauri-wasm dependency**
```toml
# frontend/Cargo.toml
[dependencies]
tauri-wasm = { git = "https://github.com/nanoqsh/tauri-wasm", optional = true }

[features]
tauri = ["tauri-wasm"]
```

**1.2 Build configuration**
```rust
// frontend/build.rs
fn main() {
    let platform = std::env::var("NOVYWAVE_PLATFORM")
        .unwrap_or_else(|_| "WEB".to_string());
    
    println!("cargo:rustc-cfg=NOVYWAVE_PLATFORM=\"{}\"", platform);
    
    if platform == "TAURI" {
        println!("cargo:rustc-cfg=feature=\"tauri\"");
    }
}
```

**1.3 Makefile tasks**
```toml
# Makefile.toml
[tasks.start-web]
env = { NOVYWAVE_PLATFORM = "WEB" }
script = "mzoon start"

[tasks.start-tauri]
env = { NOVYWAVE_PLATFORM = "TAURI" }
script = "cargo tauri dev"
```

### Phase 2: Connection Abstraction

**2.1 Create platform abstraction**
```rust
// frontend/src/platform/mod.rs
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
pub use web::WebPlatform as CurrentPlatform;

#[cfg(NOVYWAVE_PLATFORM = "TAURI")]
pub use tauri::TauriPlatform as CurrentPlatform;

pub trait Platform {
    fn is_available() -> bool;
    async fn send_message(msg: UpMsg) -> Result<(), String>;
    fn init_message_handler(handler: Box<dyn Fn(DownMsg)>);
}
```

**2.2 Tauri implementation**
```rust
// frontend/src/platform/tauri.rs
use tauri_wasm;
use shared::{UpMsg, DownMsg};

pub struct TauriPlatform;

impl Platform for TauriPlatform {
    fn is_available() -> bool {
        tauri_wasm::is_tauri()
    }
    
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        match msg {
            UpMsg::LoadWaveformFile(path) => {
                tauri_wasm::invoke("load_waveform_file", &serde_json::json!({
                    "path": path
                })).await.map_err(|e| format!("{:?}", e))
            }
            UpMsg::LoadConfig => {
                let config = tauri_wasm::invoke("load_config", &()).await
                    .map_err(|e| format!("{:?}", e))?;
                // Process config directly
                Ok(())
            }
            // ... other messages
        }
    }
    
    fn init_message_handler(handler: Box<dyn Fn(DownMsg)>) {
        // Listen for Tauri events and convert to DownMsg
        wasm_bindgen_futures::spawn_local(async move {
            // Event listening would go here if tauri-wasm supported it
            // May need to extend tauri-wasm or use polling
        });
    }
}
```

### Phase 3: Backend Commands

**3.1 Tauri command handlers**
```rust
// src-tauri/src/commands.rs
use shared::{AppConfig, WaveformFile};

#[tauri::command]
async fn load_config() -> Result<AppConfig, String> {
    // Direct file system access
    let config_path = dirs::config_dir()
        .ok_or("No config dir")?
        .join("novywave/config.toml");
    
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| e.to_string())?;
    
    toml::from_str(&content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(config: AppConfig) -> Result<(), String> {
    // Direct save to file system
}

#[tauri::command]
async fn load_waveform_file(
    path: String,
    window: tauri::Window
) -> Result<(), String> {
    // Parse file with progress events
    for progress in 0..100 {
        window.emit("parsing_progress", progress)?;
        // Actual parsing logic
    }
    
    window.emit("file_loaded", waveform_data)?;
    Ok(())
}
```

### Phase 4: Migrate Connection Points

**4.1 Replace send_up_msg calls**
```rust
// Before (frontend/src/main.rs:90)
send_up_msg(UpMsg::LoadConfig);

// After  
use crate::platform::CurrentPlatform;
CurrentPlatform::send_message(UpMsg::LoadConfig).await;
```

**4.2 Update all usage sites:**
- frontend/src/main.rs (1 location)
- frontend/src/config.rs (2 locations)  
- frontend/src/views.rs (4 locations)
- frontend/src/file_utils.rs (2 locations)
- frontend/src/waveform_canvas.rs (1 location)

## Critical Considerations

### What Works
- Simple command invocation (load_config, save_config)
- File system operations (faster than HTTP)
- Progress updates via events
- Clipboard access (WebView supports it)

### What Needs Workarounds
- **No SSE replacement**: Must poll or use Tauri events
- **No streaming**: Convert to event-based updates
- **Limited tauri-wasm API**: May need to fork/extend

### Potential Fork Requirements
If tauri-wasm lacks needed features:
1. Event listening support
2. Better error types
3. Streaming response support

## Development Workflow

```bash
# Web development (existing SSE/MoonZoon)
NOVYWAVE_PLATFORM=WEB makers start

# Desktop development (Tauri IPC)
NOVYWAVE_PLATFORM=TAURI makers tauri

# Production builds
NOVYWAVE_PLATFORM=WEB makers build         # Deploy to web
NOVYWAVE_PLATFORM=TAURI makers tauri-build # Desktop installer
```

## Migration Order (Risk Minimization)

1. **Config operations** (lowest risk, simple request/response)
2. **Directory browsing** (medium risk, needs batching strategy)
3. **File loading** (high risk, needs progress event system)
4. **Signal queries** (highest risk, performance critical)

## Success Metrics

- Zero TypeScript/JavaScript glue code
- Compilation time platform selection
- Type safety maintained
- Performance improvement in desktop mode
- Web mode remains unchanged

## Fallback Plan

If tauri-wasm proves insufficient:
1. Fork and extend tauri-wasm
2. Use minimal JavaScript bridge (5-10 lines max)
3. Consider tauri-sys despite esbuild requirement

This approach minimizes complexity while providing native desktop performance.