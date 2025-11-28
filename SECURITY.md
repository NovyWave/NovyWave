# Security Policy

## Supported Versions

NovyWave is currently in active development. Security updates are provided for the latest release only.

| Version | Supported          |
| ------- | ------------------ |
| Latest  | :white_check_mark: |
| < Latest| :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please send an email to: **martin@kavik.cz**

Include the following information:
- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact assessment
- Any suggested fixes (if available)

### What to Expect

- **Initial Response**: Within 72 hours
- **Status Update**: Within 7 days with assessment
- **Resolution Timeline**: Depends on severity and complexity

### Disclosure Policy

- We will work with you to understand and resolve the issue
- We will keep you informed of progress
- We will credit reporters in security advisories (unless you prefer to remain anonymous)
- We ask that you give us reasonable time to address the issue before public disclosure

## Security Considerations

### File Handling

NovyWave processes waveform files (VCD, FST, GHW) from user-provided sources. The application:

- Uses the Wellen library for parsing, which is designed for safety
- Does not execute arbitrary code from waveform files
- Processes files locally (no data sent to external servers in standard operation)

### Desktop Application (Tauri)

The Tauri-based desktop application:

- Uses secure defaults for IPC communication
- Runs frontend code in a sandboxed WebView
- Requires explicit capability grants for system access
- Signs releases with verified signatures (when auto-updater is enabled)

### Browser Mode

The browser-based version:

- Runs entirely in WebAssembly (sandboxed)
- Uses HTTPS for all external communications
- Does not store sensitive data in browser storage

### WebAssembly Plugins (Future)

When WASM plugin support is fully implemented:

- Plugins run in isolated WASM sandboxes
- Plugins cannot access the filesystem directly
- Plugins require explicit capability grants

## Security Best Practices for Users

1. **Download from official sources**: Only download NovyWave from the official GitHub releases or trusted package managers
2. **Verify signatures**: When available, verify release signatures before installation
3. **Keep updated**: Use the latest version to benefit from security fixes
4. **Report suspicious files**: If a waveform file causes unexpected behavior, report it

## Acknowledgments

We appreciate the security research community's efforts to responsibly disclose vulnerabilities.
