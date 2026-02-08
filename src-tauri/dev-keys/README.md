# Development Signing Keys

These keys are **for local testing only** - NOT for production releases.

## Purpose

Enable testing the Tauri auto-updater with valid signatures without needing real releases.

## Files

- `novywave-dev.key` - Private key (passwordless) for signing test bundles
- `novywave-dev.key.pub` - Public key (already embedded in `tauri.dev.updater.conf.json`)

## Usage

### 1. Build a signed test bundle

```bash
# The key is in base64-wrapped minisign format, decode it first
cat src-tauri/dev-keys/novywave-dev.key | base64 -d > /tmp/dev.key

# Sign the AppImage with rsign2
rsign sign -W -s /tmp/dev.key target/release/bundle/appimage/NovyWave_*.AppImage

# Rename signature
mv target/release/bundle/appimage/*.minisig target/release/bundle/appimage/NovyWave_*.AppImage.sig
```

### 2. Start the mock update server

```bash
cd test_files/mock-update-server
cargo run --release -- \
  --bundle ../update-test-artifacts/test-bundle.AppImage \
  --signature ../update-test-artifacts/test-bundle.AppImage.sig \
  --throttle-kbps 50 \
  --version 99.0.0
```

### 3. Run app with dev config

```bash
cargo tauri dev --config src-tauri/tauri.dev.updater.conf.json
```

## Security Note

These keys are intentionally committed to the repository because:
1. They're separate from production keys
2. They have no security value (only for testing)
3. Makes it easy for any developer to test the update flow

**Never use these keys for production releases.**
