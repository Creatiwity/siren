use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use geocoder_core::Geocoder;
use tracing::{error, info};

/// The geocoding index served by the API.
///
/// `update geocoding` replaces the file atomically (rename); the handle
/// notices the new modification time and swaps the index in. Requests keep
/// the index they started with, the previous one is unmapped when the last
/// of them ends.
pub struct GeocoderHandle {
    path: Option<PathBuf>,
    state: RwLock<State>,
}

#[derive(Default)]
struct State {
    geocoder: Option<Arc<Geocoder>>,
    modified: Option<SystemTime>,
}

impl GeocoderHandle {
    /// Open the index at `path`, if any. A missing file is not an error: it
    /// is picked up once built.
    pub fn open(path: Option<PathBuf>) -> Arc<Self> {
        let handle = Arc::new(Self {
            path,
            state: RwLock::default(),
        });
        handle.reload_if_changed();
        handle
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The current index, `None` until one is loaded.
    pub fn get(&self) -> Option<Arc<Geocoder>> {
        self.state
            .read()
            .expect("geocoder lock poisoned")
            .geocoder
            .clone()
    }

    /// Load the index file if it changed since the last load. Returns
    /// whether a new index is now served.
    pub fn reload_if_changed(&self) -> bool {
        let Some(path) = self.path.as_deref() else {
            return false;
        };
        let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) else {
            return false;
        };
        if self.state.read().expect("geocoder lock poisoned").modified == Some(modified) {
            return false;
        }
        match Geocoder::open(path) {
            Ok(geocoder) => {
                let meta = geocoder.meta();
                info!(
                    "Geocoding index loaded from {} ({} documents, profile {} v{})",
                    path.display(),
                    meta.documents,
                    meta.profile,
                    meta.profile_version
                );
                let mut state = self.state.write().expect("geocoder lock poisoned");
                state.geocoder = Some(Arc::new(geocoder));
                state.modified = Some(modified);
                true
            }
            Err(e) => {
                // Keep serving the previous index; retried on next change.
                error!("Unable to load geocoding index {}: {e}", path.display());
                self.state.write().expect("geocoder lock poisoned").modified = Some(modified);
                false
            }
        }
    }

    /// Check the index file every `every`, in the background.
    pub fn watch(self: &Arc<Self>, every: Duration) {
        if self.path.is_none() {
            return;
        }
        let handle = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(every);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                let h = Arc::clone(&handle);
                // Opening maps the file and copies its FSTs: off the runtime.
                let _ = tokio::task::spawn_blocking(move || h.reload_if_changed()).await;
            }
        });
    }
}
