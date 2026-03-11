# Testing

This guide covers testing strategies for NovyWave development.

## Test Types

### Unit Tests

Standard Rust unit tests for pure functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(format_time_ns(1000), "1μs");
        assert_eq!(format_time_ns(1000000000), "1s");
    }
}
```

Run with:
```bash
cargo test --workspace
```

### Integration Tests

Test full workflows in the `tests/` directory:

```rust
#[test]
fn test_vcd_loading() {
    let waveform = load_waveform("test_files/simple.vcd");
    assert!(waveform.is_ok());
}
```

### Manual Testing

For UI and interaction testing:

1. Start development server: `makers start`
2. Load test files from `test_files/`
3. Test specific features

When the dev server is started by someone else, inspect `dev_server.log` instead of trying to attach to their terminal output. The same rule applies to `dev_plugins.log` and `dev_tauri.log`.

### Desktop Test Bridge

When the Tauri desktop app is running, it also exposes a localhost-only desktop test bridge on `http://127.0.0.1:9226` by default. Override the port with `NOVYWAVE_DESKTOP_TEST_PORT`.

Useful endpoints:

```bash
curl http://127.0.0.1:9226/health
curl -X POST http://127.0.0.1:9226/eval -H 'Content-Type: application/json' \
  -d '{"expression":"Object.keys(window.__novywave_test_api || {}).sort()"}'
curl http://127.0.0.1:9226/state/selected-variables
curl http://127.0.0.1:9226/state/visible-rows
curl http://127.0.0.1:9226/state/markers
curl http://127.0.0.1:9226/state/file-picker-roots
curl -X POST --data-binary '/tmp/novywave_ai_workspace' http://127.0.0.1:9226/workspace/select
curl -X POST http://127.0.0.1:9226/action/set-cursor-ps -H 'Content-Type: application/json' \
  -d '{"timePs":21000}'
curl -X POST http://127.0.0.1:9226/action/add-marker -H 'Content-Type: application/json' \
  -d '{"name":"Bridge Marker"}'
curl -X POST http://127.0.0.1:9226/action/set-row-height -H 'Content-Type: application/json' \
  -d '{"uniqueId":"...","rowHeight":140}'
curl -X POST http://127.0.0.1:9226/action/set-analog-limits -H 'Content-Type: application/json' \
  -d '{"uniqueId":"...","auto":false,"min":-1,"max":4}'
curl -X POST http://127.0.0.1:9226/action/create-group -H 'Content-Type: application/json' \
  -d '{"name":"Bus Group","memberIds":["...","..."]}'
```

The bridge queries and drives the live desktop webview through `window.__novywave_test_api`, so it can verify desktop behavior without browser-only tooling and without stealing focus from the active desktop window. `POST /window/focus` still exists for debugging, but normal automation should prefer the `/action/*` endpoints.

## Test Files

The `test_files/` directory contains waveform files for testing:

| File | Format | Description |
|------|--------|-------------|
| `simple.vcd` | VCD | Basic test signals |
| `wave_27.fst` | FST | Larger test file |
| `simple_test.ghw` | GHW | GHDL-generated file |

### Creating Test Files

**VCD (Verilog):**
```verilog
initial begin
    $dumpfile("test.vcd");
    $dumpvars(0, testbench);
end
```

**FST (Verilator):**
```cpp
VerilatedFstC* tfp = new VerilatedFstC;
top->trace(tfp, 99);
tfp->open("test.fst");
```

**GHW (GHDL):**
```bash
ghdl -r testbench --wave=test.ghw
```

## Testing Checklist

### File Loading

- [ ] VCD files load correctly
- [ ] FST files load correctly
- [ ] GHW files load correctly
- [ ] Invalid files show error messages
- [ ] Large files don't freeze UI

### Navigation

- [ ] Zoom in/out works (W/S keys)
- [ ] Pan works (A/D keys)
- [ ] Cursor movement works (Q/E keys)
- [ ] Shift modifiers accelerate movement
- [ ] Reset (R) shows full timeline
- [ ] Zoom center (Z) resets to 0

### UI

- [ ] Theme toggle works (Ctrl+T)
- [ ] Dock mode toggle works (Ctrl+D)
- [ ] Panel resizing works
- [ ] Scrollbars appear when needed
- [ ] Keyboard shortcuts work when not in inputs

### State Persistence

- [ ] Selected files persist
- [ ] Selected variables persist
- [ ] Panel dimensions persist
- [ ] Theme preference persists
- [ ] Dock mode persists

### Platform Testing

- [ ] Works in Firefox
- [ ] Works in Chrome
- [ ] Works in Safari
- [ ] Desktop app launches
- [ ] Desktop app updates UI correctly

## Debugging

### Console Logging

```rust
// Use zoon::println! for WASM
zoon::println!("Debug: {}", value);
```

### Browser DevTools

1. Open browser developer tools (F12)
2. Check Console for JavaScript errors
3. Check Network for failed requests
4. Check Application > Local Storage for saved state

### Compilation Errors

Watch the newest development-server log chunk:
```bash
tail -n 120 dev_server.log
```

## Performance Testing

### Large Files

Test with large waveform files to verify:
- Reasonable loading time
- No UI freezing during load
- Smooth navigation after load
- Memory usage stays bounded

### Stress Testing

1. Load multiple large files
2. Select many variables
3. Perform rapid zoom/pan operations
4. Verify UI remains responsive

## CI/CD

GitHub Actions runs on pull requests:
- Compilation check
- Unit tests
- Build verification

See `.github/workflows/` for configuration.
