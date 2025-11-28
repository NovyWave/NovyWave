# Auto-Updater Setup Guide

NovyWave uses Tauri's built-in updater plugin to provide automatic updates.

## Generating Signing Keys

Before your first release, you need to generate a signing keypair:

```bash
# Install tauri-cli if not already installed
cargo install tauri-cli

# Generate signing keys
cargo tauri signer generate -w ~/.tauri/novywave.key
```

This will output:
- Private key saved to `~/.tauri/novywave.key`
- Public key displayed in console (also saved with `.pub` extension)

## Configuration

### 1. Update tauri.conf.json

Replace `REPLACE_WITH_PUBLIC_KEY` with your actual public key:

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/NovyWave/NovyWave/releases/latest/download/latest.json"
      ],
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6..."
    }
  }
}
```

### 2. Add Secrets to GitHub

Add these secrets to your GitHub repository (Settings → Secrets → Actions):

- `TAURI_SIGNING_PRIVATE_KEY`: Contents of `~/.tauri/novywave.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: Password used when generating the key (if any)

### 3. Update GitHub Actions Workflow

Uncomment the signing environment variables in `.github/workflows/release.yml`:

```yaml
env:
  TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
```

## Update Manifest

The CI pipeline automatically generates `latest.json` with each release:

```json
{
  "version": "0.2.0",
  "notes": "Bug fixes and performance improvements",
  "pub_date": "2025-01-15T00:00:00Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "...",
      "url": "https://github.com/NovyWave/NovyWave/releases/download/v0.2.0/..."
    }
  }
}
```

## How Updates Work

1. App starts and waits 5 seconds (to not slow startup)
2. Checks `endpoints` URL for `latest.json`
3. Compares current version with `version` in manifest
4. If update available, can download and install

## Testing Updates

1. Build and install current version
2. Bump version in `tauri.conf.json`
3. Build new version and upload to releases
4. Old version should detect and offer update

## Security

- Private key must be kept secret
- Public key can be shared (embedded in app)
- Updates are signature-verified before installation
- Never commit private keys to repository
