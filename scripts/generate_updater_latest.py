#!/usr/bin/env python3
import argparse
import base64
import json
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path


EXPECTED = {
    "linux-x86_64": {
        "artifact_dir": "linux-updater",
        "bundle_dir": "linux-updater",
        "bundle_glob": "*.AppImage",
        "sig_glob": "*.AppImage.sig",
    },
    "darwin-aarch64": {
        "artifact_dir": "macos-updater-arm",
        "bundle_dir": "macos-updater-arm",
        "bundle_glob": "*.tar.gz",
        "sig_glob": "*.tar.gz.sig",
    },
    "darwin-x86_64": {
        "artifact_dir": "macos-updater-x64",
        "bundle_dir": "macos-updater-x64",
        "bundle_glob": "*.tar.gz",
        "sig_glob": "*.tar.gz.sig",
    },
    "windows-x86_64": {
        "artifact_dir": "windows-updater",
        "bundle_dir": "windows-nsis",
        "bundle_glob": "*.exe",
        "sig_glob": "*.sig",
    },
}


def exactly_one(root: Path, pattern: str, label: str) -> Path:
    matches = sorted(path for path in root.glob(pattern) if path.is_file())
    if len(matches) != 1:
        raise SystemExit(
            f"ERROR: expected exactly one {label} matching {root / pattern}, found {len(matches)}"
        )
    return matches[0]


def public_key_text(config_path: Path) -> bytes:
    config = json.loads(config_path.read_text(encoding="utf-8"))
    pubkey = config["plugins"]["updater"]["pubkey"]
    return base64.b64decode(pubkey)


def decode_signature(signature_path: Path, output_path: Path) -> None:
    output_path.write_bytes(base64.b64decode(signature_path.read_text(encoding="utf-8").strip()))


def verify_signature(bundle: Path, signature: Path, pubkey_file: Path) -> None:
    decoded_signature = signature.with_suffix(signature.suffix + ".decoded")
    decode_signature(signature, decoded_signature)
    try:
        subprocess.run(
            [
                "minisign",
                "-V",
                "-m",
                str(bundle),
                "-x",
                str(decoded_signature),
                "-p",
                str(pubkey_file),
            ],
            check=True,
        )
    finally:
        decoded_signature.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify updater artifacts and generate Tauri latest.json"
    )
    parser.add_argument("--artifacts", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--config", default=Path("src-tauri/tauri.conf.json"), type=Path)
    parser.add_argument(
        "--release-url-base",
        required=True,
        help="Base URL ending in the release tag, without trailing slash",
    )
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="novywave-updater-") as temp_dir:
        pubkey_file = Path(temp_dir) / "minisign.pub"
        pubkey_file.write_bytes(public_key_text(args.config))

        platforms = {}
        for platform_key, spec in EXPECTED.items():
            sig_root = args.artifacts / spec["artifact_dir"]
            bundle_root = args.artifacts / spec["bundle_dir"]
            signature = exactly_one(sig_root, spec["sig_glob"], f"{platform_key} signature")
            bundle = exactly_one(bundle_root, spec["bundle_glob"], f"{platform_key} bundle")
            verify_signature(bundle, signature, pubkey_file)
            platforms[platform_key] = {
                "signature": signature.read_text(encoding="utf-8").strip(),
                "url": f"{args.release_url_base}/{bundle.name}",
            }

    if set(platforms) != set(EXPECTED):
        raise SystemExit(
            f"ERROR: updater platforms mismatch. expected {sorted(EXPECTED)}, got {sorted(platforms)}"
        )

    latest = {
        "version": args.version,
        "notes": "See release notes on GitHub",
        "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(latest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Generated verified updater metadata: {args.output}")
    print(json.dumps(latest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
