//! Keep the geocoding index in sync with its source (the BAN export).
//!
//! The source's `Last-Modified` / `ETag` are kept in a sidecar file next to
//! the index (`<index>.source.json`): when they did not change, nothing is
//! done unless forced. Otherwise the export is downloaded next to the index,
//! the index is rebuilt (written atomically) and the download removed.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use custom_error::custom_error;
use futures::StreamExt;
use geocoder_core::readers::Source;
use reqwest::header::{CONTENT_LENGTH, ETAG, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tracing::info;

custom_error! { pub Error
    UnknownProfile { name: String } = "Unknown geocoding profile {name}",
    UnknownSource { name: String } = "Unknown geocoding source format {name}",
    Request { source: reqwest::Error } = "Unable to download the geocoding source: {source}",
    Status { status: u16, url: String } = "Geocoding source {url} answered HTTP {status}",
    Truncated { expected: u64, got: u64 } = "Geocoding source download truncated ({got} of {expected} bytes)",
    Io { source: std::io::Error } = "Geocoding I/O error: {source}",
    Build { source: geocoder_core::GeocoderError } = "Unable to build the geocoding index: {source}",
    Join { source: tokio::task::JoinError } = "Geocoding index build interrupted: {source}",
    Json { source: serde_json::Error } = "Unable to write the geocoding sidecar: {source}",
}

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub index_path: PathBuf,
    /// URL (http/https) or local path of the source, gzipped when it ends
    /// with `.gz`.
    pub source_url: String,
    /// `addok` (BAN exports) or `bano`.
    pub source_format: String,
    /// geocoder-core profile, `fr` for French addresses.
    pub profile: String,
    /// Rebuild even if the source did not change.
    pub force: bool,
}

/// What `<index>.source.json` records about the build.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SourceInfo {
    pub url: String,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub profile: String,
    pub profile_version: u32,
    #[serde(default)]
    pub documents: usize,
    #[serde(default)]
    pub built_at: Option<DateTime<Utc>>,
}

impl SourceInfo {
    /// Same source, same version, same profile.
    fn same_as(&self, other: &SourceInfo) -> bool {
        self.url == other.url
            && self.profile == other.profile
            && self.profile_version == other.profile_version
            && (self.last_modified.is_some() || self.etag.is_some())
            && self.last_modified == other.last_modified
            && self.etag == other.etag
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncOutcome {
    /// The index already matches the source.
    UpToDate { source: SourceInfo },
    /// The index was (re)built.
    Built {
        source: SourceInfo,
        documents: usize,
        skipped: usize,
        invalid: usize,
        #[serde(with = "seconds")]
        download: Duration,
        #[serde(with = "seconds")]
        build: Duration,
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

fn is_remote(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub async fn sync(config: &SyncConfig) -> Result<SyncOutcome, Error> {
    let profile =
        geocoder_core::profile::by_name(&config.profile).ok_or_else(|| Error::UnknownProfile {
            name: config.profile.clone(),
        })?;
    let source = Source::from_name(&config.source_format).ok_or_else(|| Error::UnknownSource {
        name: config.source_format.clone(),
    })?;

    if let Some(parent) = config.index_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(3 * 3600))
        .build()?;

    // What the source is now.
    let mut current = SourceInfo {
        url: config.source_url.clone(),
        profile: profile.name().to_string(),
        profile_version: profile.version(),
        ..Default::default()
    };
    let mut expected_length = None;
    if is_remote(&config.source_url) {
        let head = client.head(&config.source_url).send().await?;
        if !head.status().is_success() {
            return Err(Error::Status {
                status: head.status().as_u16(),
                url: config.source_url.clone(),
            });
        }
        let header = |name| {
            head.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        current.last_modified = header(LAST_MODIFIED);
        current.etag = header(ETAG);
        expected_length = header(CONTENT_LENGTH).and_then(|l| l.parse::<u64>().ok());
    } else {
        let modified = tokio::fs::metadata(&config.source_url).await?.modified()?;
        current.last_modified = Some(DateTime::<Utc>::from(modified).to_rfc2822());
    }

    // What the index was built from.
    let sidecar = sidecar_path(&config.index_path);
    let previous: Option<SourceInfo> = tokio::fs::read(&sidecar)
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let index_exists = tokio::fs::try_exists(&config.index_path)
        .await
        .unwrap_or(false);
    if !config.force
        && index_exists
        && let Some(previous) = previous
        && previous.same_as(&current)
    {
        info!("Geocoding index up to date ({:?})", previous.last_modified);
        return Ok(SyncOutcome::UpToDate { source: previous });
    }

    // Download next to the index (same volume, no /tmp size surprise).
    let started = Instant::now();
    let (input, downloaded) = if is_remote(&config.source_url) {
        let target = with_suffix(&config.index_path, ".download");
        info!("Downloading geocoding source {}", config.source_url);
        download(&client, &config.source_url, &target, expected_length).await?;
        (target, true)
    } else {
        (PathBuf::from(&config.source_url), false)
    };
    let download_time = started.elapsed();

    // Build off the async runtime: CPU bound for minutes.
    info!("Building geocoding index {}", config.index_path.display());
    let index_path = config.index_path.clone();
    let gzipped = config.source_url.ends_with(".gz");
    let input_for_build = input.clone();
    let built = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        let file = std::fs::File::open(&input_for_build)?;
        let reader: Box<dyn std::io::Read> = if gzipped {
            Box::new(flate2::read::MultiGzDecoder::new(file))
        } else {
            Box::new(file)
        };
        let reader = BufReader::with_capacity(1 << 20, reader);
        Ok(geocoder_core::index::import(
            reader,
            source,
            profile,
            &index_path,
            |p| {
                info!(
                    "Geocoding index: {} lines, {} documents, {:?}",
                    p.lines, p.documents, p.elapsed
                );
            },
        )?)
    })
    .await;
    if downloaded {
        let _ = tokio::fs::remove_file(&input).await;
    }
    let stats = built??;

    current.documents = stats.documents;
    current.built_at = Some(Utc::now());
    tokio::fs::write(&sidecar, serde_json::to_vec_pretty(&current)?).await?;
    info!(
        "Geocoding index built: {} documents ({} skipped, {} invalid lines) in {:?}",
        stats.documents, stats.skipped, stats.invalid, stats.elapsed
    );

    Ok(SyncOutcome::Built {
        source: current,
        documents: stats.documents,
        skipped: stats.skipped,
        invalid: stats.invalid,
        download: download_time,
        build: stats.elapsed,
    })
}

async fn download(
    client: &reqwest::Client,
    url: &str,
    target: &Path,
    expected: Option<u64>,
) -> Result<(), Error> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(Error::Status {
            status: response.status().as_u16(),
            url: url.to_string(),
        });
    }
    let mut file = tokio::fs::File::create(target).await?;
    let mut stream = response.bytes_stream();
    let mut written = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        written += chunk.len() as u64;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    if let Some(expected) = expected
        && written != expected
    {
        let _ = tokio::fs::remove_file(target).await;
        return Err(Error::Truncated {
            expected,
            got: written,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(last_modified: Option<&str>, etag: Option<&str>) -> SourceInfo {
        SourceInfo {
            url: "u".into(),
            last_modified: last_modified.map(str::to_string),
            etag: etag.map(str::to_string),
            profile: "fr".into(),
            profile_version: 3,
            ..Default::default()
        }
    }

    #[test]
    fn freshness() {
        assert!(info(Some("a"), None).same_as(&info(Some("a"), None)));
        assert!(!info(Some("a"), None).same_as(&info(Some("b"), None)));
        // Without any validator, always rebuild.
        assert!(!info(None, None).same_as(&info(None, None)));
        // A new profile version needs a rebuild.
        let mut other = info(Some("a"), None);
        other.profile_version = 4;
        assert!(!info(Some("a"), None).same_as(&other));
    }

    #[test]
    fn sidecar_next_to_the_index() {
        assert_eq!(
            sidecar_path(Path::new("/data/index.bin")),
            PathBuf::from("/data/index.bin.source.json")
        );
    }
}
