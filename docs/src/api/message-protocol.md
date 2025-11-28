# Message Protocol

NovyWave uses a message-based protocol for frontend-backend communication.

## Overview

```
Frontend (WASM) ←→ Backend
      │                │
    UpMsg            DownMsg
```

- **UpMsg** - Messages from frontend to backend
- **DownMsg** - Messages from backend to frontend

## UpMsg (Frontend → Backend)

### File Operations

```rust
// Load configuration from disk
UpMsg::LoadConfig

// Save configuration to disk
UpMsg::SaveConfig {
    config: AppConfig
}

// Load a waveform file
UpMsg::LoadWaveformFile {
    path: String
}

// Browse a directory
UpMsg::BrowseDirectory {
    path: String
}

// Browse multiple directories at once
UpMsg::BrowseDirectories {
    paths: Vec<String>
}
```

### Signal Data

```rust
// Query signal values at a specific time
UpMsg::QuerySignalValues {
    file_path: String,
    signal_ids: Vec<String>,
    time_ns: u64
}

// Query signal transitions for timeline
UpMsg::QuerySignalTransitions {
    requests: Vec<SignalTransitionRequest>
}

// Get current parsing progress
UpMsg::GetParsingProgress {
    file_path: String
}
```

## DownMsg (Backend → Frontend)

### Configuration

```rust
// Configuration loaded from disk
DownMsg::ConfigLoaded(AppConfig)

// Configuration saved successfully
DownMsg::ConfigSaved
```

### File Loading

```rust
// File loaded successfully
DownMsg::FileLoaded {
    path: String,
    file: WaveformFile
}

// File loading failed
DownMsg::FileError {
    path: String,
    error: String
}

// Parsing progress update
DownMsg::ParsingProgress {
    path: String,
    percentage: u8,
    message: String
}
```

### Directory Browsing

```rust
// Directory contents
DownMsg::DirectoryContents {
    path: String,
    items: Vec<FileSystemItem>
}

// Directory error
DownMsg::DirectoryError {
    path: String,
    error: String
}
```

### Signal Data

```rust
// Signal values at cursor position
DownMsg::SignalValues {
    values: HashMap<String, String>
}

// Signal transitions for timeline
DownMsg::SignalTransitions {
    signal_id: String,
    transitions: Vec<SignalTransition>
}
```

## Data Structures

### SignalTransitionRequest

Request for signal data:

```rust
pub struct SignalTransitionRequest {
    pub file_path: String,
    pub scope_path: String,
    pub variable_name: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub canvas_width_px: u32,
}
```

### FileSystemItem

Directory listing item:

```rust
pub struct FileSystemItem {
    pub name: String,
    pub path: String,
    pub item_type: FileSystemItemType,
}

pub enum FileSystemItemType {
    Directory,
    File,
}
```

## Message Flow Examples

### Loading a File

```
1. User clicks "Load Files"
2. Frontend: UpMsg::BrowseDirectory { path: "/" }
3. Backend: DownMsg::DirectoryContents { ... }
4. User selects file
5. Frontend: UpMsg::LoadWaveformFile { path: "..." }
6. Backend: DownMsg::ParsingProgress { ... }  (multiple)
7. Backend: DownMsg::FileLoaded { ... }
```

### Timeline Navigation

```
1. User moves cursor
2. Frontend: UpMsg::QuerySignalTransitions { ... }
3. Backend: DownMsg::SignalTransitions { ... }
4. Frontend updates waveform display
```

### Configuration Save

```
1. User changes setting
2. Frontend: UpMsg::SaveConfig { config: ... }
3. Backend: Writes .novywave file
4. Backend: DownMsg::ConfigSaved
```

## Error Handling

Backend errors are communicated through error messages:

```rust
DownMsg::FileError {
    path: String,
    error: String,  // Human-readable error message
}

DownMsg::DirectoryError {
    path: String,
    error: String,
}
```

Frontend displays these errors via toast notifications or contextual UI.
