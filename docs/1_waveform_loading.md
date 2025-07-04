# Waveform Loading Implementation Plan

## Overview

This document outlines the design and implementation plan for the Files & Scopes functionality in NovyWave, specifically focusing on how users will load and parse waveform files (VCD/FST format).

## Initial Requirements

1. **File Selection**: Let users pick files from native browser dialog, but get only file paths and pass them to backend
2. **Single Selection**: Only one item in TreeView can be selected at once for Variables panel loading
3. **Backend Processing**: Backend handles all file loading and parsing
4. **Large File Support**: Expect very large files (multi-GB), cannot transfer entire content to backend
5. **Multiple Files**: Support loading multiple waveforms simultaneously (improvement over GTKWave)
6. **Platform Strategy**: Browser mode for development, future Tauri desktop integration

## Research Findings

### Browser Security Limitations

**Critical Discovery**: Modern browsers intentionally block access to file paths for security reasons.

- **File System Access API**: Even with the newest browser APIs, you cannot pass file paths to backend
- **Security Model**: Browsers only provide filename, size, MIME type - never full paths
- **No Workarounds**: This is fundamental security design, not a technical limitation

**Traditional Solutions Investigated**:
- Chunked file uploads (10MB chunks via File API)
- File System Access API streaming
- Origin Private File System (OPFS)

**Conclusion**: All browser-based solutions require transferring file content to backend, defeating the purpose for GB-scale files.

### Waveform Parsing Libraries

**Wellen Crate Investigation**:
- **Original Repository**: https://github.com/ekiwi/wellen (actively maintained, v0.16.1 June 2025)
- **Martin's Fork**: https://github.com/MartinKavik/wellen (`new_pub_types` branch)
- **Fork Purpose**: Adds public APIs (`SignalType`, `Waveform::new()`) needed for external usage
- **Status**: Fork is minimal, safe, and still relevant
- **WASM Compatibility**: Potential issues with Rayon dependency, but proven to work in production apps

**Alternative Libraries**:
- `vcd` crate: Simpler, single-threaded, likely WASM-compatible
- `fstapi`: FST-specific Rust wrapper

**Recommendation**: Use Martin's wellen fork for the needed public APIs.

### Custom File Browser Solutions

**Research Results**: Extensive ecosystem of web file manager libraries exists:

**Frontend Libraries**:
- Vuefinder (Vue): Versatile file manager with backend API support
- @cubone/react-file-manager: React component with full backend integration
- SVAR File Manager: Cross-framework solution
- Features: Drag-drop, multi-select, tree view, search, REST API communication

**Backend Solutions**:
- simple-file-server: Minimalist Actix-based file server with REST API
- Spacedrive: Full file explorer with Rust core
- Custom Actix endpoints: Build file system API

## Final Architecture Decision

After extensive research, we decided on a **two-phase approach**:

### Phase 1: Simplified Text Input (Immediate Implementation)

**File Selection Method**:
- Native browser `window.prompt()` dialog via `web_sys`
- User enters absolute file paths, comma-delimited
- Simple but functional for development and testing

**Technical Implementation**:
```rust
// Frontend: web_sys::window().prompt_with_message()
// Backend: Direct file system access via absolute paths
// Communication: MoonZoon UpMsg with file paths only
```

**Benefits**:
- Zero file size limits (backend reads from disk)
- No file transfer needed
- Simple to implement and debug
- Works immediately for development

### Phase 2: Advanced File Browser (Future Enhancement)

**Options for Later**:
- Custom Zoon file browser with server communication
- Integration inspired by Vuefinder/SVAR file manager
- LazyLoaded NovyUI TreeView with Up/DownMsg backend

## Technical Specifications

### Message Protocol

```rust
#[derive(Serialize, Deserialize)]
pub enum UpMsg {
    LoadWaveformFile(String),  // Absolute file path
    GetParsingProgress(String),
}

#[derive(Serialize, Deserialize)]
pub enum DownMsg {
    ParsingStarted { file_id: String, filename: String },
    ParsingProgress { file_id: String, progress: f32 },
    FileLoaded { file_id: String, hierarchy: FileHierarchy },
    ParsingError { file_id: String, error: String },
}
```

### Backend Dependencies

```toml
[dependencies]
wellen = { git = "https://github.com/MartinKavik/wellen", branch = "new_pub_types" }
```

### Frontend Integration

- Modify existing "Load Files" button to show prompt dialog
- Update button label with loading progress during parsing
- Populate TreeView with parsed file hierarchy
- Connect TreeView selection to Variables panel

## Implementation Considerations

### Parallel Loading

**Potential Issues**:
- Memory pressure from multiple GB files
- CPU bottleneck during simultaneous parsing
- I/O contention on traditional HDDs

**Strategy**:
- Start with parallel loading
- Monitor performance
- Add queuing/concurrency limits if needed

### Error Handling

- Simple text popup with error messages
- Technical details logged to browser console
- Assumption: Users are technical and can interpret errors

### Security and Permissions

- Local server only (development/desktop use)
- No file system restrictions beyond user permissions
- No web security concerns (not public deployment)

### Progress Tracking

- Update "Load Files" button label with percentage
- Real-time progress during waveform parsing
- Simple visual feedback for long operations

## Development and Testing Strategy

### Testing Tools

- Browser MCP for debugging UI interactions
- Test files: `/test_files/simple.vcd`, `/test_files/wave_27.fst`
- Manual testing with various file sizes and formats

### Debug Workflow

1. Navigate to `http://localhost:8080` via browser MCP
2. Click "Load Files" button
3. Enter test file paths in prompt dialog
4. Monitor loading progress and error handling
5. Verify TreeView population and selection behavior

## Future Enhancements

### Tauri Desktop Integration

When transitioning to desktop mode:
- Replace web prompt with Tauri native file dialogs
- Same backend file processing logic
- Direct file path access without browser limitations

### Advanced File Browser

Potential upgrades:
- Visual file system browser
- Drag-and-drop support
- Recent files list
- Bookmarked directories
- File format validation and preview

## Limitations and Trade-offs

### Current Limitations

1. **Browser Mode Only**: Phase 1 solution works only when backend runs locally
2. **Manual Path Entry**: No visual file browsing in Phase 1
3. **No Path Validation**: User must enter correct absolute paths
4. **Local Server Requirement**: Cannot work with remote servers in current design

### Accepted Trade-offs

1. **Simplicity over Features**: Chose simple text input over complex file browser
2. **Development Focus**: Optimized for browser development workflow
3. **Future Flexibility**: Architecture allows easy upgrade to advanced file browser

## Success Criteria

### Phase 1 Completion

- [x] User can enter file paths via browser prompt
- [x] Backend loads and parses waveform files directly from disk
- [x] Progress tracking during file loading
- [x] TreeView populated with file hierarchy
- [x] Error handling for invalid files or paths
- [x] Multiple file loading support

### Phase 2 Goals

- [ ] Visual file browser interface
- [ ] Server-side directory browsing API
- [ ] Enhanced user experience with drag-drop
- [ ] File format validation and preview

## Conclusion

This approach provides an elegant solution to the large file loading problem by leveraging direct file system access on the backend while maintaining a simple, functional user interface. The two-phase strategy allows immediate development progress while preserving options for future enhancements.

The decision to use text input for Phase 1, while seemingly primitive, actually provides the most direct path to a working solution without the complexity of building a full file browser system. This aligns well with the development-focused use case and technical user base.

---

*Document created: January 2025*  
*Implementation status: Planning complete, ready for development*