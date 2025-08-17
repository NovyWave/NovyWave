# Tauri Integration for NovyWave

## Overview

This document outlines the architecture and implementation for integrating Tauri desktop support into NovyWave while maintaining full compatibility with the existing MoonZoon web application.

## Architecture Decisions

### Dual-Mode Support Strategy

**Web Mode (Existing):**
- Uses MoonZoon framework with integrated frontend/backend
- Frontend: Rust/WASM via Zoon framework
- Backend: Moon framework on Actix Web
- Development: `makers start` (port 8080)
- Deployment: Standard web application

**Desktop Mode (New):**
- Tauri wrapper around existing frontend
- Backend: MoonZoon subprocess with dynamic port
- Frontend: Same Rust/WASM code, different connection logic
- Development: `makers tauri` 
- Deployment: Native desktop application

### Subprocess vs Embedded Server

**Decision: Subprocess Approach**

**Rationale:**
1. **Proven Pattern**: MoonZoon Tauri examples use this successfully
2. **Separation of Concerns**: Backend and desktop wrapper remain independent
3. **Debugging**: Each process can be developed/tested separately
4. **Risk Mitigation**: No complex threading or runtime integration issues
5. **Maintenance**: Easier to update MoonZoon or Tauri independently

**Considered Alternatives:**
- **Embedded Server**: More complex, requires deep integration with Moon framework
- **Tauri Localhost Plugin**: Only serves static files, doesn't run backend logic
- **Separate Codebases**: Code duplication, maintenance overhead

### Dynamic Port Allocation

**Requirements:**
- No hardcoded ports in production
- Automatic conflict resolution
- Development/production flexibility

**Implementation:**
- Use `portpicker` crate for automatic port selection
- Tauri spawns backend on available port
- Frontend gets backend URL via Tauri IPC
- Fallback mechanisms for port conflicts

## Technical Implementation

### 1. Backend Lifecycle Management

**src-tauri/src/lib.rs:**
```rust
use portpicker::pick_unused_port;
use std::process::{Child, Command};
use std::sync::Mutex;

static BACKEND_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static BACKEND_PORT: Mutex<Option<u16>> = Mutex::new(None);

#[tauri::command]
async fn start_backend() -> Result<u16, String> {
    let port = pick_unused_port()
        .ok_or("No available ports")?;
    
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "backend"])
        .env("PORT", port.to_string())
        .spawn()
        .map_err(|e| e.to_string())?;
    
    *BACKEND_PROCESS.lock().unwrap() = Some(child);
    *BACKEND_PORT.lock().unwrap() = Some(port);
    
    Ok(port)
}

#[tauri::command]
fn get_backend_port() -> Option<u16> {
    *BACKEND_PORT.lock().unwrap()
}
```

### 2. Frontend Connection Adaptation

**frontend/src/connection.rs:**
```rust
fn get_backend_url() -> String {
    if is_tauri_environment() {
        // Get dynamic port from Tauri
        let port = invoke_tauri_command("get_backend_port").unwrap_or(8080);
        format!("http://localhost:{}", port)
    } else {
        // Standard web mode
        "http://localhost:8080".to_string()
    }
}

fn is_tauri_environment() -> bool {
    // Detect if running in Tauri context
    js_sys::global().has_type_of("object") && 
    js_sys::Reflect::has(&js_sys::global(), &"__TAURI__".into()).unwrap_or(false)
}
```

### 3. Build System Integration

**Makefile.toml additions:**
```toml
[tasks.tauri-dev]
description = "Start Tauri desktop development mode"
dependencies = ["install", "install_tauri_cli"]
script = '''
echo "Starting Tauri desktop mode..."
cd src-tauri && cargo tauri dev
'''

[tasks.tauri-build]
description = "Build Tauri desktop application"
dependencies = ["install", "install_tauri_cli", "build"]
script = '''
echo "Building Tauri desktop application..."
cd src-tauri && cargo tauri build
'''
```

## Development Workflow

### Web Development (Unchanged)
```bash
makers start      # Start MoonZoon dev server
makers build      # Build for web deployment
```

### Desktop Development
```bash
makers tauri      # Start Tauri development mode
makers tauri-build # Build desktop application
```

### Dual Development
```bash
# Terminal 1: Web mode
makers start

# Terminal 2: Desktop mode (different port)
makers tauri
```

## Configuration Management

### Environment Detection
The frontend detects its environment and adapts accordingly:
- **Browser**: Standard MoonZoon connection to localhost:8080
- **Tauri**: Dynamic connection via IPC to subprocess backend

### Port Configuration
- **Development**: Dynamic allocation prevents conflicts
- **Production**: Same dynamic strategy for deployment
- **Override**: Environment variables can specify ports if needed

## Security Considerations

### Subprocess Approach Benefits
- **Process Isolation**: Backend runs as separate process
- **No Network Exposure**: Only localhost communication
- **Standard Permissions**: Uses OS process security model

### Compared to Alternatives
- **Tauri Localhost Plugin**: Official warnings about security risks
- **Embedded Server**: Shared memory space, more attack surface
- **External Server**: Network exposure, authentication complexity

## Future Enhancements

### Potential Improvements
1. **Backend Status Monitoring**: Health checks and restart logic
2. **Error Recovery**: Automatic backend restart on crashes
3. **Resource Management**: CPU/memory monitoring
4. **Advanced IPC**: More sophisticated Tauri ↔ Backend communication

### Migration Path
- **Phase 1**: Basic subprocess integration (this implementation)
- **Phase 2**: Enhanced monitoring and error handling
- **Phase 3**: Optional embedded mode for specific use cases

## Testing Strategy

### Verification Points
1. **Web Mode Compatibility**: Ensure `makers start` unchanged
2. **Desktop Mode Functionality**: All features work in Tauri
3. **Dual Mode**: Both can run simultaneously
4. **Port Allocation**: Dynamic ports work reliably
5. **Build Process**: Both deployment modes build successfully

### Test Scenarios
- Development workflow switching between modes
- Production builds for both web and desktop
- Backend crash recovery
- Port conflict resolution
- Feature parity between modes

## Conclusion

This subprocess-based approach provides a robust foundation for Tauri integration while maintaining the existing MoonZoon development experience. The dual-mode architecture ensures NovyWave can be deployed as both a web application and native desktop application from a single codebase, with minimal complexity and maximum reliability.