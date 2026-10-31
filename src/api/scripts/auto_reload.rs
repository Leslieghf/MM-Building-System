//! Debounced content observation for opt-in script file watching.
//!
//! The loader watches only a package's manifest and main source file. A change
//! is reported after the same content fingerprint appears in consecutive
//! polls; the accepted bytes travel with the notice so a later save cannot
//! silently change which version is compiled. This is change detection, not a
//! security or authenticity mechanism.
use super::ScriptId;
use std::collections::BTreeMap;

/// Content fingerprint for a package's manifest and main source.
///
/// `None` distinguishes a missing or unreadable file from an empty file;
/// `Some` represents the complete file contents. This non-cryptographic value
/// is used only to detect repeated observations, never to validate trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFingerprint {
    manifest: Option<u64>,
    main: Option<u64>,
}

impl PackageFingerprint {
    /// Fingerprint the optional complete contents of both package files.
    ///
    /// Each input should be bounded by the package loader's size limits before
    /// hashing. The result is stable for identical byte sequences in this build.
    pub fn from_contents(manifest: Option<&[u8]>, main: Option<&[u8]>) -> Self {
        Self {
            manifest: manifest.map(content_hash),
            main: main.map(content_hash),
        }
    }

    pub(crate) fn matches_contents(&self, manifest: Option<&[u8]>, main: Option<&[u8]>) -> bool {
        self == &Self::from_contents(manifest, main)
    }
}

/// Snapshot of bounded file contents accepted by the debounce policy.
///
/// The fields are private because snapshots are created by the loader. The
/// contents stay paired with their fingerprint so subsequent edits cannot
/// change the source associated with an already accepted notice.
#[derive(Debug, Clone)]
pub struct AutoReloadSnapshot {
    pub(crate) fingerprint: PackageFingerprint,
    pub(crate) manifest: Option<Vec<u8>>,
    pub(crate) main: Option<Vec<u8>>,
}

impl AutoReloadSnapshot {
    pub(crate) fn new(manifest: Option<Vec<u8>>, main: Option<Vec<u8>>) -> Self {
        let fingerprint = PackageFingerprint::from_contents(manifest.as_deref(), main.as_deref());
        Self {
            fingerprint,
            manifest,
            main,
        }
    }
}

fn content_hash(contents: &[u8]) -> u64 {
    // FNV-1a over bytes (including length implicitly through the full byte stream).
    let mut hash = 0xcbf29ce484222325u64;
    for byte in contents {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Per-package debounce state for tests and loader-side change tracking.
///
/// Tracking starts from a clean baseline each time it is enabled. Disabled IDs
/// are ignored until enabled again.
#[derive(Default)]
pub struct AutoReloadTracker {
    watches: BTreeMap<ScriptId, WatchState>,
}

#[derive(Default)]
struct WatchState {
    previous: Option<PackageFingerprint>,
    issued: Option<PackageFingerprint>,
}

impl AutoReloadTracker {
    /// Enable or disable tracking for one package.
    ///
    /// Enabling resets the baseline; disabling removes all remembered
    /// fingerprints for the ID.
    pub fn set_enabled(&mut self, id: ScriptId, enabled: bool) {
        if enabled {
            self.watches.insert(id, WatchState::default());
        } else {
            self.remove(&id);
        }
    }

    /// Observe one poll and report a stable, previously unseen change once.
    ///
    /// The first observation establishes a baseline. A later identical
    /// observation emits `true`; subsequent identical polls return `false`
    /// until the fingerprint changes.
    pub fn observe(&mut self, id: &ScriptId, fingerprint: PackageFingerprint) -> bool {
        let Some(state) = self.watches.get_mut(id) else {
            return false;
        };
        let stable = state.previous.as_ref() == Some(&fingerprint);
        state.previous = Some(fingerprint.clone());
        if stable && state.issued.as_ref() != Some(&fingerprint) {
            state.issued = Some(fingerprint);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: &ScriptId) {
        self.watches.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::{AutoReloadTracker, PackageFingerprint};
    use crate::api::scripts::ScriptId;

    fn id() -> ScriptId {
        ScriptId::parse("example").unwrap()
    }
    fn fp(manifest: &[u8], main: &[u8]) -> PackageFingerprint {
        PackageFingerprint::from_contents(Some(manifest), Some(main))
    }

    #[test]
    fn stable_fingerprint_notifies_after_two_consecutive_polls_once() {
        let mut tracker = AutoReloadTracker::default();
        let id = id();
        tracker.set_enabled(id.clone(), true);
        let fingerprint = fp(b"manifest", b"source");
        assert!(!tracker.observe(&id, fingerprint.clone()));
        assert!(tracker.observe(&id, fingerprint.clone()));
        assert!(!tracker.observe(&id, fingerprint));
    }

    #[test]
    fn new_fingerprint_arms_a_new_notification_after_debounce() {
        let mut tracker = AutoReloadTracker::default();
        let id = id();
        tracker.set_enabled(id.clone(), true);
        let first = fp(b"manifest", b"source");
        let second = fp(b"manifest", b"changed");
        assert!(!tracker.observe(&id, first.clone()));
        assert!(tracker.observe(&id, first));
        assert!(!tracker.observe(&id, second.clone()));
        assert!(tracker.observe(&id, second));
    }

    #[test]
    fn fingerprints_cover_same_length_and_manifest_only_edits_and_missing_files() {
        assert_ne!(fp(b"manifest", b"aaaa"), fp(b"manifest", b"bbbb"));
        assert_ne!(fp(b"manifest", b"source"), fp(b"changed", b"source"));
        assert_ne!(
            PackageFingerprint::from_contents(Some(b"m"), Some(b"s")),
            PackageFingerprint::from_contents(None, Some(b"s"))
        );
        assert_ne!(
            PackageFingerprint::from_contents(Some(b"m"), Some(b"s")),
            PackageFingerprint::from_contents(Some(b"m"), None)
        );
    }

    #[test]
    fn disabling_removes_watch_and_reenable_starts_a_fresh_baseline() {
        let mut tracker = AutoReloadTracker::default();
        let id = id();
        tracker.set_enabled(id.clone(), true);
        let fingerprint = fp(b"manifest", b"source");
        assert!(!tracker.observe(&id, fingerprint.clone()));
        tracker.set_enabled(id.clone(), false);
        assert!(!tracker.observe(&id, fingerprint.clone()));
        tracker.set_enabled(id.clone(), true);
        assert!(!tracker.observe(&id, fingerprint.clone()));
        assert!(tracker.observe(&id, fingerprint));
    }
}
