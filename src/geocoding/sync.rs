//! Keep the geocoding index up to date by downloading the prebuilt one.
//!
//! The French index is built (from the BAN export, ~5 GB of memory) and
//! published daily by the geocoder-core CI: `<base>/v<format>/fr.json`, a
//! manifest signed with Ed25519, and the index it names. Nothing is built
//! here, and nothing from the storage is trusted before its signature:
//!
//! 1. the manifest must be signed by a trusted key, describe a French index
//!    this build of geocoder-core reads (format and profile versions), and
//!    not be older than the local index (no rollback);
//! 2. the index must have exactly the announced size and SHA-256, and open;
//! 3. only then it replaces the current one (atomic rename); `serve` picks
//!    it up without a restart.
//!
//! `<index>.source.json` records what the local index is; when its hash
//! matches the published one, nothing is downloaded.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use custom_error::custom_error;
use futures::StreamExt;
use geocoder_core::signed::{self, Hasher, Manifest, PublicKey, SourceVersion};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tracing::info;

/// Where the geocoder-core CI publishes its indexes.
pub const DEFAULT_BASE_URL: &str = "https://creatiwity-geocoder-index.s3.gra.io.cloud.ovh.net";

/// Keys of the geocoder-core CI, hex encoded (`INDEX_SIGNING_KEY` of its
/// `index` environment). `GEOCODING_INDEX_PUBLIC_KEYS` replaces them.
pub const TRUSTED_KEYS: &[&str] =
    &["8b1390df97da26d2c93f1d0b8a8b030950a5cdac6130fd8400f11d522634ec25"];

const COUNTRY: &str = "fr";
const PROFILE: &str = "fr";

/// Largest signed manifest accepted (they are about 1 KB).
const MAX_MANIFEST_BYTES: usize = 64 * 1024;

custom_error! { pub Error
    Config { message: String } = "Invalid geocoding configuration: {message}",
    Request { source: reqwest::Error } = "Unable to download the geocoding index: {source}",
    Status { status: u16, url: String } = "{url} answered HTTP {status}",
    Size { url: String, expected: u64, got: u64 } = "Download of {url} truncated or too long ({got} of {expected} bytes)",
    Io { source: std::io::Error } = "Geocoding I/O error: {source}",
    Open { source: geocoder_core::GeocoderError } = "Downloaded geocoding index unreadable: {source}",
    Join { source: tokio::task::JoinError } = "Geocoding index check interrupted: {source}",
    Json { source: serde_json::Error } = "Unable to write the geocoding sidecar: {source}",
    Signature { source: signed::SignatureError } = "Geocoding manifest rejected: {source}",
    Manifest { message: String } = "Geocoding manifest rejected: {message}",
    Integrity { message: String } = "Downloaded geocoding index rejected: {message}",
}

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub index_path: PathBuf,
    /// `<base>/v<index format version>`, without a trailing slash.
    pub base_url: String,
    pub keys: Vec<PublicKey>,
    /// Download even when the local index is the published one.
    pub force: bool,
}

impl SyncConfig {
    /// `keys`: comma separated, hex; `None` for [`TRUSTED_KEYS`].
    pub fn new(
        index_path: PathBuf,
        base_url: &str,
        keys: Option<&str>,
        force: bool,
    ) -> Result<Self, Error> {
        let base_url = base_url.trim().trim_end_matches('/');
        if base_url.is_empty() {
            return Err(Error::Config {
                message: "empty index base URL".into(),
            });
        }
        let keys: Vec<&str> = match keys {
            Some(keys) => keys
                .split(',')
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .collect(),
            None => TRUSTED_KEYS.to_vec(),
        };
        let keys = keys
            .into_iter()
            .map(|k| {
                k.parse().map_err(|_| Error::Config {
                    message: format!("invalid public key {k}"),
                })
            })
            .collect::<Result<Vec<PublicKey>, _>>()?;
        if keys.is_empty() {
            return Err(Error::Config {
                message: "no trusted public key".into(),
            });
        }
        Ok(Self {
            index_path,
            base_url: format!("{base_url}/v{}", geocoder_core::index::format::VERSION),
            keys,
            force,
        })
    }
}

/// What `<index>.source.json` records about the local index.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceInfo {
    /// What the index was built from (the BAN export, as last seen).
    pub sources: Vec<SourceVersion>,
    pub profile: String,
    pub profile_version: u32,
    #[serde(default)]
    pub documents: usize,
    #[serde(default)]
    pub built_at: Option<DateTime<Utc>>,
    /// Of the index file.
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncOutcome {
    /// The local index is the published one.
    UpToDate { source: SourceInfo },
    /// The published index was downloaded.
    Downloaded {
        source: SourceInfo,
        #[serde(with = "seconds")]
        download: Duration,
    },
}

mod seconds {
    pub fn serialize<S: serde::Serializer>(
        d: &std::time::Duration,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.serialize_f64((d.as_secs_f64() * 10.0).round() / 10.0)
    }
}

pub fn sidecar_path(index: &Path) -> PathBuf {
    with_suffix(index, ".source.json")
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

fn parse_date(date: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

pub async fn sync(config: &SyncConfig) -> Result<SyncOutcome, Error> {
    if let Some(parent) = config.index_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(3 * 3600))
        .build()?;

    let url = format!("{}/{COUNTRY}.json", config.base_url);
    let envelope = get_small(&client, &url).await?;
    let manifest = signed::verify(&envelope, &config.keys)?;
    check_manifest(&manifest)?;

    let sidecar = sidecar_path(&config.index_path);
    let local: Option<SourceInfo> = tokio::fs::read(&sidecar)
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let index_exists = tokio::fs::try_exists(&config.index_path)
        .await
        .unwrap_or(false);
    if !config.force
        && index_exists
        && let Some(local) = local
    {
        if local.sha256.as_deref() == Some(manifest.sha256.as_str()) {
            info!("Geocoding index up to date (built {})", manifest.built_at);
            return Ok(SyncOutcome::UpToDate { source: local });
        }
        // An older (genuinely signed) manifest replayed: keep ours.
        if let (Some(ours), Some(theirs)) = (local.built_at, parse_date(&manifest.built_at))
            && theirs < ours
        {
            return Err(Error::Manifest {
                message: format!(
                    "published index built {theirs}, older than the local one ({ours}); --force to take it"
                ),
            });
        }
    }

    let index_url = format!("{}/{}", config.base_url, manifest.file);
    let target = with_suffix(&config.index_path, ".download");
    info!(
        "Downloading geocoding index {index_url} ({} bytes)",
        manifest.size
    );
    let started = Instant::now();
    if let Err(e) = download_checked(&client, &index_url, &target, &manifest).await {
        let _ = tokio::fs::remove_file(&target).await;
        return Err(e);
    }
    let download = started.elapsed();

    // Index first: a crash in between leaves the new index with the old
    // sidecar, whose hash no longer matches: downloaded again next time.
    tokio::fs::rename(&target, &config.index_path).await?;
    let source = SourceInfo {
        sources: manifest.sources.clone(),
        profile: manifest.profile.clone(),
        profile_version: manifest.profile_version,
        documents: manifest.documents,
        built_at: parse_date(&manifest.built_at),
        sha256: Some(manifest.sha256.clone()),
    };
    tokio::fs::write(&sidecar, serde_json::to_vec_pretty(&source)?).await?;
    info!(
        "Geocoding index downloaded: {} documents, built {}",
        manifest.documents, manifest.built_at
    );
    Ok(SyncOutcome::Downloaded { source, download })
}

/// A manifest signed by a trusted key still has to describe a French index
/// this build can read.
fn check_manifest(manifest: &Manifest) -> Result<(), Error> {
    let reject = |message: String| Err(Error::Manifest { message });
    let profile = geocoder_core::profile::by_name(PROFILE).expect("the fr profile exists");
    if manifest.country != COUNTRY {
        return reject(format!("for country {}", manifest.country));
    }
    if manifest.format_version != geocoder_core::index::format::VERSION {
        return reject(format!(
            "index format v{}, this build reads v{}",
            manifest.format_version,
            geocoder_core::index::format::VERSION
        ));
    }
    if manifest.profile != profile.name() || manifest.profile_version != profile.version() {
        return reject(format!(
            "profile {} v{}, this build has {} v{}",
            manifest.profile,
            manifest.profile_version,
            profile.name(),
            profile.version()
        ));
    }
    // Only ever appended to the base URL, but no path tricks either way.
    let name_ok = manifest.file.strip_suffix(".bin").is_some_and(|stem| {
        !stem.is_empty()
            && stem
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    if !name_ok {
        return reject(format!("file name {:?}", manifest.file));
    }
    if parse_date(&manifest.built_at).is_none() {
        return reject(format!("build date {:?}", manifest.built_at));
    }
    Ok(())
}

/// The signed manifest, bounded in size.
async fn get_small(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, Error> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        // S3 answers 403 for a missing key without the ListBucket right.
        let status = response.status();
        if matches!(status, StatusCode::NOT_FOUND | StatusCode::FORBIDDEN) {
            return Err(Error::Manifest {
                message: format!("nothing published at {url} (HTTP {status})"),
            });
        }
        return Err(Error::Status {
            status: status.as_u16(),
            url: url.to_string(),
        });
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        body.extend_from_slice(&chunk?);
        if body.len() > MAX_MANIFEST_BYTES {
            return Err(Error::Size {
                url: url.to_string(),
                expected: MAX_MANIFEST_BYTES as u64,
                got: body.len() as u64,
            });
        }
    }
    Ok(body)
}

/// Downloads exactly the announced size (more is an error, stopped right
/// away), checks the SHA-256, then opens the index.
async fn download_checked(
    client: &reqwest::Client,
    url: &str,
    target: &Path,
    manifest: &Manifest,
) -> Result<(), Error> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(Error::Status {
            status: response.status().as_u16(),
            url: url.to_string(),
        });
    }
    let size_error = |got| Error::Size {
        url: url.to_string(),
        expected: manifest.size,
        got,
    };
    let mut file = tokio::fs::File::create(target).await?;
    let mut hasher = Hasher::default();
    let mut stream = response.bytes_stream();
    let mut written = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        written += chunk.len() as u64;
        if written > manifest.size {
            return Err(size_error(written));
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    file.sync_all().await?;
    if written != manifest.size {
        return Err(size_error(written));
    }
    let sha256 = hasher.finish();
    if sha256 != manifest.sha256 {
        return Err(Error::Integrity {
            message: format!("SHA-256 {sha256}, the manifest says {}", manifest.sha256),
        });
    }

    // Format, profile and FST checksums, before it replaces anything.
    let path = target.to_path_buf();
    let documents = tokio::task::spawn_blocking(move || {
        geocoder_core::Geocoder::open(&path).map(|g| g.meta().documents as usize)
    })
    .await??;
    if documents != manifest.documents {
        return Err(Error::Integrity {
            message: format!(
                "{documents} documents, the manifest says {}",
                manifest.documents
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        let profile = geocoder_core::profile::by_name("fr").unwrap();
        Manifest {
            country: "fr".into(),
            format_version: geocoder_core::index::format::VERSION,
            profile: "fr".into(),
            profile_version: profile.version(),
            file: "fr-0123456789abcdef.bin".into(),
            size: 1,
            sha256: "00".repeat(32),
            documents: 1,
            built_at: "2026-11-16T04:32:38Z".into(),
            sources: Vec::new(),
        }
    }

    #[test]
    fn manifests_must_fit_this_build() {
        assert!(check_manifest(&manifest()).is_ok());
        let bad = [
            Manifest {
                country: "be".into(),
                ..manifest()
            },
            Manifest {
                format_version: 1,
                ..manifest()
            },
            Manifest {
                profile_version: 999,
                ..manifest()
            },
            Manifest {
                file: "../fr.bin".into(),
                ..manifest()
            },
            Manifest {
                file: ".bin".into(),
                ..manifest()
            },
            Manifest {
                built_at: "yesterday".into(),
                ..manifest()
            },
        ];
        for m in bad {
            assert!(check_manifest(&m).is_err(), "{m:?}");
        }
    }

    #[test]
    fn config() {
        let c = SyncConfig::new("/d/index.bin".into(), "https://x/", None, false).unwrap();
        let format = geocoder_core::index::format::VERSION;
        assert_eq!(c.base_url, format!("https://x/v{format}"));
        assert_eq!(c.keys.len(), TRUSTED_KEYS.len());
        assert!(SyncConfig::new("/d".into(), "https://x", Some("nope"), false).is_err());
        assert!(SyncConfig::new("/d".into(), "https://x", Some(""), false).is_err());
        assert!(SyncConfig::new("/d".into(), " ", None, false).is_err());
    }

    #[test]
    fn sidecar_next_to_the_index() {
        assert_eq!(
            sidecar_path(Path::new("/data/index.bin")),
            PathBuf::from("/data/index.bin.source.json")
        );
    }
}
