use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{SecondsFormat, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const EXPECTED_PLATFORMS: &[&str] = &[
    "darwin-aarch64",
    "darwin-x86_64",
    "linux-x86_64",
    "windows-x86_64",
];

#[derive(Parser)]
#[command(about = "NovyWave release artifact tooling")]
struct Args {
    #[command(subcommand)]
    command: ReleaseCommand,
}

#[derive(Subcommand)]
enum ReleaseCommand {
    GenerateLatest {
        #[arg(long)]
        artifacts: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        release_url_base: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value = "src-tauri/tauri.conf.json")]
        config: PathBuf,
    },
    CollectFiles {
        #[arg(long)]
        artifacts: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        output: PathBuf,
    },
    GenerateManifest {
        #[arg(long)]
        release_files: PathBuf,
        #[arg(long)]
        proofs_root: PathBuf,
        #[arg(long)]
        version: String,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        notes_output: PathBuf,
    },
}

#[derive(Clone, Debug)]
struct UpdaterSpec {
    platform: &'static str,
    signature_dir: &'static str,
    bundle_dir: &'static str,
    bundle_suffix: &'static str,
    signature_suffix: &'static str,
}

impl UpdaterSpec {
    fn all() -> Vec<Self> {
        vec![
            Self {
                platform: "linux-x86_64",
                signature_dir: "linux-updater",
                bundle_dir: "linux-updater",
                bundle_suffix: ".AppImage",
                signature_suffix: ".AppImage.sig",
            },
            Self {
                platform: "darwin-aarch64",
                signature_dir: "macos-updater-arm",
                bundle_dir: "macos-updater-arm",
                bundle_suffix: ".tar.gz",
                signature_suffix: ".tar.gz.sig",
            },
            Self {
                platform: "darwin-x86_64",
                signature_dir: "macos-updater-x64",
                bundle_dir: "macos-updater-x64",
                bundle_suffix: ".tar.gz",
                signature_suffix: ".tar.gz.sig",
            },
            Self {
                platform: "windows-x86_64",
                signature_dir: "windows-updater",
                bundle_dir: "windows-nsis",
                bundle_suffix: ".exe",
                signature_suffix: ".sig",
            },
        ]
    }
}

#[derive(Clone, Debug)]
struct ReleaseArtifact {
    filename: String,
    platform: &'static str,
    architecture: &'static str,
    bundle_type: &'static str,
    smoke: &'static str,
    signing: &'static str,
    updater: bool,
    release_note_label: Option<&'static str>,
}

#[derive(Deserialize)]
struct SmokeProof {
    job: String,
    platform: String,
    architecture: String,
    source_commit: String,
    version: String,
    proved_artifacts: Vec<String>,
    proof: Vec<String>,
}

#[derive(Serialize)]
struct ManifestEntry {
    filename: String,
    size_bytes: u64,
    sha256: String,
    source_commit: String,
    platform: String,
    architecture: String,
    bundle_type: String,
    updater_artifact: bool,
    signing_status: String,
    smoke_result: String,
    release_note_label: Option<String>,
}

#[derive(Serialize)]
struct ReleaseManifest {
    version: String,
    source_commit: String,
    artifact_count: usize,
    artifacts: Vec<ManifestEntry>,
}

fn main() -> Result<()> {
    match Args::parse().command {
        ReleaseCommand::GenerateLatest {
            artifacts,
            version,
            release_url_base,
            output,
            config,
        } => generate_latest(&artifacts, &version, &release_url_base, &output, &config),
        ReleaseCommand::CollectFiles {
            artifacts,
            version,
            output,
        } => collect_files(&artifacts, &version, &output),
        ReleaseCommand::GenerateManifest {
            release_files,
            proofs_root,
            version,
            source_commit,
            output,
            notes_output,
        } => generate_manifest(
            &release_files,
            &proofs_root,
            &version,
            &source_commit,
            &output,
            &notes_output,
        ),
    }
}

fn generate_latest(
    artifacts: &Path,
    version: &str,
    release_url_base: &str,
    output: &Path,
    config: &Path,
) -> Result<()> {
    let pubkey_text = public_key_text(config)?;
    let temp_dir = unique_temp_dir("novywave-updater")?;
    let pubkey_file = temp_dir.join("minisign.pub");
    fs::write(&pubkey_file, pubkey_text)?;

    let mut platforms = serde_json::Map::new();
    for spec in UpdaterSpec::all() {
        let signature = exactly_one_with_suffix(
            &artifacts.join(spec.signature_dir),
            spec.signature_suffix,
            &format!("{} signature", spec.platform),
        )?;
        let bundle = exactly_one_with_suffix(
            &artifacts.join(spec.bundle_dir),
            spec.bundle_suffix,
            &format!("{} bundle", spec.platform),
        )?;
        verify_signature(&bundle, &signature, &pubkey_file, &temp_dir)?;
        platforms.insert(
            spec.platform.to_string(),
            json!({
                "signature": read_trimmed(&signature)?,
                "url": format!("{}/{}", release_url_base.trim_end_matches('/'), file_name(&bundle)?),
            }),
        );
    }

    let latest = json!({
        "notes": "See release notes on GitHub",
        "platforms": platforms,
        "pub_date": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "version": version,
    });
    write_json(output, &latest)?;
    println!("Generated verified updater metadata: {}", output.display());
    println!("{}", serde_json::to_string_pretty(&latest)?);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn collect_files(artifacts: &Path, version: &str, output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    let expected = curated_release_files(version);
    let all_files = files_by_basename(artifacts)?;
    fail_on_bad_release_inputs(&all_files)?;

    for filename in &expected {
        let candidates = all_files
            .get(filename)
            .with_context(|| format!("missing release file source for {filename}"))?;
        let source = choose_duplicate_source(filename, candidates)?;
        fs::copy(source, output.join(filename))
            .with_context(|| format!("copy {} to release files", source.display()))?;
    }

    println!("Collected release files in {}", output.display());
    for file in list_files(output)? {
        println!("  {}", file_name(&file)?);
    }
    Ok(())
}

fn generate_manifest(
    release_files: &Path,
    proofs_root: &Path,
    version: &str,
    source_commit: &str,
    output: &Path,
    notes_output: &Path,
) -> Result<()> {
    let release_paths = files_by_basename_shallow(release_files)?;
    reject_published_noise(&release_paths)?;

    let latest_path = release_paths
        .get("latest.json")
        .context("missing latest.json in release files")?;
    let latest_platforms = validate_latest_json(latest_path)?;

    validate_smoke_proofs(proofs_root, version, source_commit)?;

    let mut specs = expected_release_artifacts(version);
    for (filename, platform) in latest_platforms {
        specs
            .entry(filename.clone())
            .or_insert_with(|| updater_bundle_spec(filename, platform));
    }

    let missing: Vec<_> = specs
        .keys()
        .filter(|filename| !release_paths.contains_key(filename.as_str()))
        .cloned()
        .collect();
    if !missing.is_empty() {
        bail!("missing expected release files: {}", missing.join(", "));
    }

    let allowed: BTreeSet<_> = specs.keys().cloned().collect();
    let extras: Vec<_> = release_paths
        .keys()
        .filter(|filename| {
            !allowed.contains(*filename) && !filename.starts_with("release-manifest-")
        })
        .cloned()
        .collect();
    if !extras.is_empty() {
        bail!("unexpected release files: {}", extras.join(", "));
    }

    let mut artifacts = Vec::new();
    for (filename, path) in &release_paths {
        if filename.starts_with("release-manifest-") {
            continue;
        }
        let spec = specs
            .get(filename)
            .with_context(|| format!("no manifest metadata rule for {filename}"))?;
        artifacts.push(artifact_entry(spec, path, source_commit)?);
    }

    let manifest = ReleaseManifest {
        version: version.to_string(),
        source_commit: source_commit.to_string(),
        artifact_count: artifacts.len(),
        artifacts,
    };
    write_json(output, &manifest)?;
    write_release_notes(version, &manifest.artifacts, notes_output)?;
    println!("Generated release manifest: {}", output.display());
    println!("Generated release notes: {}", notes_output.display());
    Ok(())
}

fn expected_release_artifacts(version: &str) -> BTreeMap<String, ReleaseArtifact> {
    let mut artifacts = BTreeMap::new();
    for artifact in [
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_amd64.AppImage"),
            platform: "linux",
            architecture: "x86_64",
            bundle_type: "tauri-appimage",
            smoke: "linux AppImage desktop bridge smoke",
            signing: "updater signature verified",
            updater: true,
            release_note_label: Some("Linux AppImage"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_amd64.deb"),
            platform: "linux",
            architecture: "x86_64",
            bundle_type: "tauri-deb",
            smoke: "linux deb extracted-package smoke",
            signing: "not updater-signed",
            updater: false,
            release_note_label: Some("Linux Debian/Ubuntu"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave-{version}-1.x86_64.rpm"),
            platform: "linux",
            architecture: "x86_64",
            bundle_type: "tauri-rpm",
            smoke: "linux rpm extracted-package smoke",
            signing: "not updater-signed",
            updater: false,
            release_note_label: Some("Linux Fedora/RHEL"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_chrome-linux-x64.AppImage"),
            platform: "linux",
            architecture: "x86_64",
            bundle_type: "chrome-appimage",
            smoke: "fake-browser route smoke and real Chromium smoke",
            signing: "not updater-signed",
            updater: false,
            release_note_label: Some("Linux Chrome launcher AppImage"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_aarch64.dmg"),
            platform: "macos",
            architecture: "aarch64",
            bundle_type: "tauri-dmg",
            smoke: "macOS DMG desktop bridge smoke",
            signing: "Apple notarization policy not enforced by this workflow",
            updater: false,
            release_note_label: Some("macOS Apple Silicon DMG"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_x64.dmg"),
            platform: "macos",
            architecture: "x86_64",
            bundle_type: "tauri-dmg",
            smoke: "macOS DMG desktop bridge smoke",
            signing: "Apple notarization policy not enforced by this workflow",
            updater: false,
            release_note_label: Some("macOS Intel DMG"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_chrome-macos-aarch64.tar.gz"),
            platform: "macos",
            architecture: "aarch64",
            bundle_type: "chrome-tarball",
            smoke: "packaged from validated WEB frontend artifacts",
            signing: "not updater-signed",
            updater: false,
            release_note_label: Some("macOS Chrome launcher ARM"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_chrome-macos-x64.tar.gz"),
            platform: "macos",
            architecture: "x86_64",
            bundle_type: "chrome-tarball",
            smoke: "packaged from validated WEB frontend artifacts",
            signing: "not updater-signed",
            updater: false,
            release_note_label: Some("macOS Chrome launcher Intel"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_x64-setup.exe"),
            platform: "windows",
            architecture: "x86_64",
            bundle_type: "tauri-nsis",
            smoke: "NSIS installed-app desktop bridge smoke",
            signing: "updater signature verified; Authenticode policy not enforced by this workflow",
            updater: true,
            release_note_label: Some("Windows NSIS installer"),
        },
        ReleaseArtifact {
            filename: format!("NovyWave_{version}_x64_en-US.msi"),
            platform: "windows",
            architecture: "x86_64",
            bundle_type: "tauri-msi",
            smoke: "MSI administrative-install desktop bridge smoke",
            signing: "not updater-signed; Authenticode policy not enforced by this workflow",
            updater: false,
            release_note_label: Some("Windows MSI"),
        },
        ReleaseArtifact {
            filename: "latest.json".to_string(),
            platform: "updater",
            architecture: "multi",
            bundle_type: "tauri-latest-json",
            smoke: "generated only after signature verification",
            signing: "contains verified updater signatures",
            updater: false,
            release_note_label: Some("Updater metadata"),
        },
    ] {
        artifacts.insert(artifact.filename.clone(), artifact);
    }
    artifacts
}

fn curated_release_files(version: &str) -> BTreeSet<String> {
    let mut files: BTreeSet<_> = expected_release_artifacts(version).into_keys().collect();
    files.insert("NovyWave.app_aarch64.tar.gz".to_string());
    files.insert("NovyWave.app_x64.tar.gz".to_string());
    files
}

fn updater_bundle_spec(filename: String, platform: String) -> ReleaseArtifact {
    let architecture = match platform.as_str() {
        "darwin-aarch64" => "aarch64",
        "darwin-x86_64" => "x86_64",
        "linux-x86_64" | "windows-x86_64" => "x86_64",
        _ => "multi",
    };
    ReleaseArtifact {
        filename,
        platform: "updater",
        architecture,
        bundle_type: "tauri-updater-bundle",
        smoke: "referenced by verified latest.json",
        signing: "signature verified before latest.json generation",
        updater: true,
        release_note_label: None,
    }
}

fn artifact_entry(
    spec: &ReleaseArtifact,
    path: &Path,
    source_commit: &str,
) -> Result<ManifestEntry> {
    Ok(ManifestEntry {
        filename: spec.filename.clone(),
        size_bytes: path.metadata()?.len(),
        sha256: sha256(path)?,
        source_commit: source_commit.to_string(),
        platform: spec.platform.to_string(),
        architecture: spec.architecture.to_string(),
        bundle_type: spec.bundle_type.to_string(),
        updater_artifact: spec.updater,
        signing_status: spec.signing.to_string(),
        smoke_result: spec.smoke.to_string(),
        release_note_label: spec.release_note_label.map(str::to_string),
    })
}

fn validate_latest_json(path: &Path) -> Result<BTreeMap<String, String>> {
    let data: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let platforms = data
        .get("platforms")
        .and_then(Value::as_object)
        .context("latest.json missing platforms object")?;
    let actual: BTreeSet<_> = platforms.keys().map(String::as_str).collect();
    let expected: BTreeSet<_> = EXPECTED_PLATFORMS.iter().copied().collect();
    if actual != expected {
        bail!(
            "latest.json updater platforms mismatch. expected {:?}, got {:?}",
            expected,
            actual
        );
    }

    let mut urls = BTreeMap::new();
    for (platform, metadata) in platforms {
        let signature = metadata
            .get("signature")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .with_context(|| format!("latest.json platform {platform} is missing signature"))?;
        let url = metadata
            .get("url")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .with_context(|| format!("latest.json platform {platform} is missing url"))?;
        if signature.starts_with("http://") || signature.starts_with("https://") {
            bail!("latest.json platform {platform} signature must be inline content, not a URL");
        }
        let filename = filename_from_url_path(url)
            .with_context(|| format!("latest.json platform {platform} has invalid url: {url}"))?;
        urls.insert(filename, platform.clone());
    }
    Ok(urls)
}

fn validate_smoke_proofs(proofs_root: &Path, version: &str, source_commit: &str) -> Result<()> {
    for (dir, file, job, platform, architecture, proved) in [
        (
            "linux-smoke-proof",
            "smoke-proof-linux-x86_64.json",
            "build-linux",
            "linux",
            "x86_64",
            vec![
                format!("NovyWave_{version}_amd64.AppImage"),
                format!("NovyWave_{version}_amd64.deb"),
                format!("NovyWave-{version}-1.x86_64.rpm"),
                format!("NovyWave_{version}_chrome-linux-x64.AppImage"),
            ],
        ),
        (
            "macos-smoke-proof-arm",
            "smoke-proof-macos-aarch64.json",
            "build-macos-arm",
            "macos",
            "aarch64",
            vec![
                format!("NovyWave_{version}_aarch64.dmg"),
                format!("NovyWave_{version}_chrome-macos-aarch64.tar.gz"),
            ],
        ),
        (
            "macos-smoke-proof-x64",
            "smoke-proof-macos-x86_64.json",
            "build-macos-x64",
            "macos",
            "x86_64",
            vec![
                format!("NovyWave_{version}_x64.dmg"),
                format!("NovyWave_{version}_chrome-macos-x64.tar.gz"),
            ],
        ),
        (
            "windows-smoke-proof",
            "smoke-proof-windows-x86_64.json",
            "build-windows",
            "windows",
            "x86_64",
            vec![
                format!("NovyWave_{version}_x64-setup.exe"),
                format!("NovyWave_{version}_x64_en-US.msi"),
                format!("NovyWave_{version}_chrome-windows-x64.zip"),
            ],
        ),
    ] {
        let path = proofs_root.join(dir).join(file);
        let proof: SmokeProof = serde_json::from_str(
            &fs::read_to_string(&path)
                .with_context(|| format!("read smoke proof {}", path.display()))?,
        )?;
        if proof.job != job {
            bail!("{file} job mismatch: expected {job}, got {}", proof.job);
        }
        if proof.platform != platform {
            bail!(
                "{file} platform mismatch: expected {platform}, got {}",
                proof.platform
            );
        }
        if proof.architecture != architecture {
            bail!(
                "{file} architecture mismatch: expected {architecture}, got {}",
                proof.architecture
            );
        }
        if proof.version != version {
            bail!(
                "{file} version mismatch: expected {version}, got {}",
                proof.version
            );
        }
        if proof.source_commit != source_commit {
            bail!(
                "{file} source_commit mismatch: expected {source_commit}, got {}",
                proof.source_commit
            );
        }
        if proof.proved_artifacts != proved {
            bail!("{file} proved_artifacts mismatch");
        }
        if proof.proof.is_empty() {
            bail!("{file} proof list is empty");
        }
    }
    Ok(())
}

fn write_release_notes(version: &str, artifacts: &[ManifestEntry], output: &Path) -> Result<()> {
    let mut rows = Vec::new();
    for artifact in artifacts {
        let Some(label) = artifact.release_note_label.as_deref() else {
            continue;
        };
        rows.push(format!(
            "| {} | `{}` | `{}` |",
            label,
            artifact.filename,
            &artifact.sha256[..12]
        ));
    }

    let content = [
        "## Downloads".to_string(),
        "".to_string(),
        "| Artifact | File | SHA-256 |".to_string(),
        "|----------|------|---------|".to_string(),
        rows.join("\n"),
        "".to_string(),
        "## Chrome Launcher".to_string(),
        "".to_string(),
        "`novywave-chrome` opens NovyWave in Chrome/Chromium app mode.".to_string(),
        "Use it if the standard desktop app has performance issues, especially on Linux."
            .to_string(),
        "It requires Chrome, Chromium, Edge, Brave, or another Chromium-based browser installed."
            .to_string(),
        "".to_string(),
        "## Auto-Update".to_string(),
        "".to_string(),
        "Auto-update is available for Linux AppImage, macOS app, and Windows NSIS installs."
            .to_string(),
        "Debian, RPM, and MSI installs update manually by downloading the new package from this release."
            .to_string(),
        "macOS `NovyWave.app_*.tar.gz` files are updater payloads; use the `.dmg` files for manual macOS installs."
            .to_string(),
        "".to_string(),
        "## Release Proof".to_string(),
        "".to_string(),
        format!(
            "`release-manifest-{version}.json` contains every uploaded filename, size, SHA-256, source commit, bundle type, signing status, and smoke result."
        ),
        "".to_string(),
    ]
    .join("\n");
    fs::write(output, content)?;
    Ok(())
}

fn public_key_text(config_path: &Path) -> Result<Vec<u8>> {
    let config: Value = serde_json::from_str(&fs::read_to_string(config_path)?)?;
    let pubkey = config
        .get("plugins")
        .and_then(|plugins| plugins.get("updater"))
        .and_then(|updater| updater.get("pubkey"))
        .and_then(Value::as_str)
        .context("src-tauri updater pubkey is missing")?;
    Ok(STANDARD.decode(pubkey)?)
}

fn verify_signature(
    bundle: &Path,
    signature: &Path,
    pubkey_file: &Path,
    temp_dir: &Path,
) -> Result<()> {
    let decoded_signature = temp_dir.join(format!("{}.decoded", file_name(signature)?));
    fs::write(
        &decoded_signature,
        STANDARD.decode(read_trimmed(signature)?)?,
    )?;
    let status = Command::new("minisign")
        .arg("-V")
        .arg("-m")
        .arg(bundle)
        .arg("-x")
        .arg(&decoded_signature)
        .arg("-p")
        .arg(pubkey_file)
        .status()
        .context("failed to run minisign")?;
    let _ = fs::remove_file(&decoded_signature);
    if !status.success() {
        bail!(
            "minisign verification failed for {} with signature {}",
            bundle.display(),
            signature.display()
        );
    }
    Ok(())
}

fn exactly_one_with_suffix(root: &Path, suffix: &str, label: &str) -> Result<PathBuf> {
    let matches: Vec<_> = list_files(root)?
        .into_iter()
        .filter(|path| {
            file_name(path)
                .map(|name| name.ends_with(suffix))
                .unwrap_or(false)
        })
        .collect();
    if matches.len() != 1 {
        bail!(
            "expected exactly one {} matching {}/*{}, found {}",
            label,
            root.display(),
            suffix,
            matches.len()
        );
    }
    Ok(matches[0].clone())
}

fn choose_duplicate_source<'a>(filename: &str, candidates: &'a [PathBuf]) -> Result<&'a Path> {
    let first = candidates
        .first()
        .with_context(|| format!("no candidates for {filename}"))?;
    let first_hash = sha256(first)?;
    for candidate in candidates.iter().skip(1) {
        let candidate_hash = sha256(candidate)?;
        if candidate_hash != first_hash {
            bail!("duplicate release basename {filename} has different content");
        }
    }
    Ok(first)
}

fn fail_on_bad_release_inputs(all_files: &BTreeMap<String, Vec<PathBuf>>) -> Result<()> {
    let disallowed_release_inputs: Vec<_> = all_files
        .keys()
        .filter(|name| name.starts_with("release-manifest-"))
        .cloned()
        .collect();
    if !disallowed_release_inputs.is_empty() {
        bail!(
            "release manifest files must not be pre-existing inputs: {}",
            disallowed_release_inputs.join(", ")
        );
    }
    Ok(())
}

fn reject_published_noise(paths: &BTreeMap<String, PathBuf>) -> Result<()> {
    let noise: Vec<_> = paths
        .keys()
        .filter(|name| name.ends_with(".sig") || name.starts_with("smoke-proof-"))
        .cloned()
        .collect();
    if !noise.is_empty() {
        bail!(
            "release files contain CI-only or signature noise: {}",
            noise.join(", ")
        );
    }
    Ok(())
}

fn files_by_basename(root: &Path) -> Result<BTreeMap<String, Vec<PathBuf>>> {
    let mut files = BTreeMap::<String, Vec<PathBuf>>::new();
    for path in list_files_recursive(root)? {
        files.entry(file_name(&path)?).or_default().push(path);
    }
    Ok(files)
}

fn files_by_basename_shallow(root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut files = BTreeMap::new();
    for path in list_files(root)? {
        let name = file_name(&path)?;
        if files.insert(name.clone(), path.clone()).is_some() {
            bail!("duplicate file in release directory: {name}");
        }
    }
    Ok(files)
}

fn list_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        bail!("directory does not exist: {}", root.display());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn list_files_recursive(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    visit_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn visit_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        bail!("directory does not exist: {}", root.display());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            visit_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn read_trimmed(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn file_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .with_context(|| format!("path has no UTF-8 file name: {}", path.display()))
}

fn filename_from_url_path(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn unique_temp_dir(prefix: &str) -> Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_json_rejects_signature_urls() {
        let dir = temp_dir("latest-json");
        fs::create_dir_all(&dir).unwrap();
        let latest = dir.join("latest.json");
        fs::write(
            &latest,
            r#"{
              "version": "0.2.1",
              "platforms": {
                "linux-x86_64": {"url": "https://example.test/app.AppImage", "signature": "https://example.test/app.sig"},
                "darwin-aarch64": {"url": "https://example.test/app-a.tar.gz", "signature": "sig"},
                "darwin-x86_64": {"url": "https://example.test/app-x.tar.gz", "signature": "sig"},
                "windows-x86_64": {"url": "https://example.test/app.exe", "signature": "sig"}
              }
            }"#,
        )
        .unwrap();
        assert!(validate_latest_json(&latest).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn curated_files_include_updater_payloads_but_not_signatures() {
        let files = curated_release_files("0.2.1");
        assert!(files.contains("NovyWave.app_aarch64.tar.gz"));
        assert!(files.contains("NovyWave.app_x64.tar.gz"));
        assert!(files.contains("latest.json"));
        assert!(!files.contains("NovyWave.app_x64.tar.gz.sig"));
        assert!(!files.contains("NovyWave_0.2.1_chrome-windows-x64.zip"));
        assert!(!files.contains("smoke-proof-linux-x86_64.json"));
    }

    #[test]
    fn duplicate_basename_with_different_content_is_rejected() {
        let dir = temp_dir("duplicates");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let one = a.join("same.txt");
        let two = b.join("same.txt");
        fs::write(&one, "one").unwrap();
        fs::write(&two, "two").unwrap();
        let candidates = vec![one, two];
        assert!(choose_duplicate_source("same.txt", &candidates).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "novywave-release-tools-test-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
