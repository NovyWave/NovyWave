#!/usr/bin/env python3
import argparse
import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse


@dataclass(frozen=True)
class ExpectedArtifact:
    filename: str
    platform: str
    architecture: str
    bundle_type: str
    smoke: str
    signing: str
    updater: bool = False
    release_note_label: str | None = None


def expected_artifacts(version: str) -> list[ExpectedArtifact]:
    return [
        ExpectedArtifact(
            f"NovyWave_{version}_amd64.AppImage",
            "linux",
            "x86_64",
            "tauri-appimage",
            "linux AppImage desktop bridge smoke",
            "updater signature verified",
            updater=True,
            release_note_label="Linux AppImage",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_amd64.deb",
            "linux",
            "x86_64",
            "tauri-deb",
            "linux deb extracted-package smoke",
            "not updater-signed",
            release_note_label="Linux Debian/Ubuntu",
        ),
        ExpectedArtifact(
            f"NovyWave-{version}-1.x86_64.rpm",
            "linux",
            "x86_64",
            "tauri-rpm",
            "linux rpm extracted-package smoke",
            "not updater-signed",
            release_note_label="Linux Fedora/RHEL",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_chrome-linux-x64.AppImage",
            "linux",
            "x86_64",
            "chrome-appimage",
            "fake-browser route smoke and real Chromium smoke",
            "not updater-signed",
            release_note_label="Linux Chrome launcher AppImage",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_aarch64.dmg",
            "macos",
            "aarch64",
            "tauri-dmg",
            "macOS DMG desktop bridge smoke",
            "Apple notarization policy not enforced by this workflow",
            release_note_label="macOS Apple Silicon DMG",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_x64.dmg",
            "macos",
            "x86_64",
            "tauri-dmg",
            "macOS DMG desktop bridge smoke",
            "Apple notarization policy not enforced by this workflow",
            release_note_label="macOS Intel DMG",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_chrome-macos-aarch64.tar.gz",
            "macos",
            "aarch64",
            "chrome-tarball",
            "packaged from validated WEB frontend artifacts",
            "not updater-signed",
            release_note_label="macOS Chrome launcher ARM",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_chrome-macos-x64.tar.gz",
            "macos",
            "x86_64",
            "chrome-tarball",
            "packaged from validated WEB frontend artifacts",
            "not updater-signed",
            release_note_label="macOS Chrome launcher Intel",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_x64-setup.exe",
            "windows",
            "x86_64",
            "tauri-nsis",
            "NSIS installed-app desktop bridge smoke",
            "updater signature verified; Authenticode policy not enforced by this workflow",
            updater=True,
            release_note_label="Windows NSIS installer",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_x64_en-US.msi",
            "windows",
            "x86_64",
            "tauri-msi",
            "MSI administrative-install desktop bridge smoke",
            "not updater-signed; Authenticode policy not enforced by this workflow",
            release_note_label="Windows MSI",
        ),
        ExpectedArtifact(
            f"NovyWave_{version}_chrome-windows-x64.zip",
            "windows",
            "x86_64",
            "chrome-zip",
            "packaged from validated WEB frontend artifacts",
            "not updater-signed",
            release_note_label="Windows Chrome launcher",
        ),
        ExpectedArtifact(
            "latest.json",
            "updater",
            "multi",
            "tauri-latest-json",
            "generated only after signature verification",
            "contains verified updater signatures",
            release_note_label="Updater metadata",
        ),
        ExpectedArtifact(
            "smoke-proof-linux-x86_64.json",
            "ci-proof",
            "linux-x86_64",
            "smoke-proof",
            "uploaded only after Linux package smoke steps passed",
            "not applicable",
        ),
        ExpectedArtifact(
            "smoke-proof-macos-aarch64.json",
            "ci-proof",
            "macos-aarch64",
            "smoke-proof",
            "uploaded only after macOS ARM package smoke steps passed",
            "not applicable",
        ),
        ExpectedArtifact(
            "smoke-proof-macos-x86_64.json",
            "ci-proof",
            "macos-x86_64",
            "smoke-proof",
            "uploaded only after macOS Intel package smoke steps passed",
            "not applicable",
        ),
        ExpectedArtifact(
            "smoke-proof-windows-x86_64.json",
            "ci-proof",
            "windows-x86_64",
            "smoke-proof",
            "uploaded only after Windows package smoke steps passed",
            "not applicable",
        ),
    ]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact_entry(spec: ExpectedArtifact, path: Path, source_commit: str) -> dict[str, object]:
    return {
        "filename": spec.filename,
        "size_bytes": path.stat().st_size,
        "sha256": sha256(path),
        "source_commit": source_commit,
        "platform": spec.platform,
        "architecture": spec.architecture,
        "bundle_type": spec.bundle_type,
        "updater_artifact": spec.updater,
        "signing_status": spec.signing,
        "smoke_result": spec.smoke,
        "release_note_label": spec.release_note_label,
    }


def validate_latest_json(path: Path) -> dict[str, str]:
    expected_platforms = {
        "linux-x86_64",
        "darwin-aarch64",
        "darwin-x86_64",
        "windows-x86_64",
    }
    data = json.loads(path.read_text(encoding="utf-8"))
    actual = set(data.get("platforms", {}))
    if actual != expected_platforms:
        raise SystemExit(
            "ERROR: latest.json updater platforms mismatch. "
            f"expected {sorted(expected_platforms)}, got {sorted(actual)}"
        )
    urls = {}
    for platform, metadata in data["platforms"].items():
        url = metadata.get("url")
        if not isinstance(url, str) or not url:
            raise SystemExit(f"ERROR: latest.json platform {platform} is missing url")
        filename = Path(urlparse(url).path).name
        if not filename:
            raise SystemExit(f"ERROR: latest.json platform {platform} has invalid url: {url}")
        urls[filename] = platform
    return urls


def dynamic_spec_for_file(
    filename: str, latest_bundle_platforms: dict[str, str]
) -> ExpectedArtifact | None:
    if filename in latest_bundle_platforms:
        platform_key = latest_bundle_platforms[filename]
        return ExpectedArtifact(
            filename,
            "updater",
            platform_key,
            "tauri-updater-bundle",
            "referenced by verified latest.json",
            "signature verified before latest.json generation",
            updater=True,
        )

    if filename.endswith(".sig"):
        return ExpectedArtifact(
            filename,
            "updater",
            "multi",
            "tauri-updater-signature",
            "signature verification gate",
            "verified before latest.json generation",
            updater=True,
        )

    return None


def write_release_notes(version: str, artifacts: list[dict[str, object]], output: Path) -> None:
    rows = []
    for artifact in artifacts:
        if not artifact.get("release_note_label"):
            continue
        rows.append(
            "| {label} | `{filename}` | `{sha256}` | {smoke} |".format(
                label=artifact["release_note_label"],
                filename=artifact["filename"],
                sha256=str(artifact["sha256"])[:12],
                smoke=artifact["smoke_result"],
            )
        )

    content = "\n".join(
        [
            "## Downloads",
            "",
            "| Artifact | File | SHA-256 | Proof |",
            "|----------|------|---------|-------|",
            *rows,
            "",
            "## Chrome Launcher",
            "",
            "`novywave-chrome` opens NovyWave in Chrome/Chromium app mode.",
            "Use it if the standard desktop app has performance issues, especially on Linux.",
            "It requires Chrome, Chromium, Edge, Brave, or another Chromium-based browser installed.",
            "",
            "## Auto-Update",
            "",
            "Auto-update is available for Linux AppImage, macOS app, and Windows NSIS installs.",
            "Debian, RPM, and MSI installs update manually by downloading the new package from this release.",
            "",
            "## Release Proof",
            "",
            f"`release-manifest-{version}.json` contains every uploaded filename, size, SHA-256, source commit, bundle type, signing status, and smoke result.",
            "",
        ]
    )
    output.write_text(content, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate and validate NovyWave release artifact manifest"
    )
    parser.add_argument("--release-files", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--notes-output", required=True, type=Path)
    args = parser.parse_args()

    expected = expected_artifacts(args.version)
    expected_by_name = {artifact.filename: artifact for artifact in expected}
    actual_paths = {
        path.name: path for path in args.release_files.iterdir() if path.is_file()
    }

    missing = sorted(set(expected_by_name) - set(actual_paths))
    if missing:
        raise SystemExit(f"ERROR: missing expected release files: {', '.join(missing)}")

    latest_bundle_platforms = validate_latest_json(actual_paths["latest.json"])
    missing_latest_bundles = sorted(set(latest_bundle_platforms) - set(actual_paths))
    if missing_latest_bundles:
        raise SystemExit(
            "ERROR: latest.json references missing release files: "
            + ", ".join(missing_latest_bundles)
        )

    allowed_names = set(expected_by_name) | set(latest_bundle_platforms)
    allowed_names.update(name for name in actual_paths if name.endswith(".sig"))
    allowed_names.update(name for name in actual_paths if name.startswith("release-manifest-"))

    extras = sorted(name for name in actual_paths if name not in allowed_names)
    if extras:
        raise SystemExit(f"ERROR: unexpected release files: {', '.join(extras)}")

    artifacts = []
    for filename in sorted(actual_paths):
        if filename.startswith("release-manifest-"):
            continue
        spec = expected_by_name.get(filename) or dynamic_spec_for_file(
            filename, latest_bundle_platforms
        )
        if spec is None:
            raise SystemExit(f"ERROR: no manifest metadata rule for {filename}")
        artifacts.append(artifact_entry(spec, actual_paths[filename], args.source_commit))
    manifest = {
        "version": args.version,
        "source_commit": args.source_commit,
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
    }
    args.output.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    write_release_notes(args.version, artifacts, args.notes_output)
    print(f"Generated release manifest: {args.output}")
    print(f"Generated release notes: {args.notes_output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
