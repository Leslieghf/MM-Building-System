//! Background compilation worker and bounded script-package I/O.
mod discovery;
use super::{
    engine::RhaiEngineFactory,
    package::{DiscoveredScript, MAX_MANIFEST_BYTES, PreparedScript, ScriptManifest},
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    },
    time::Duration,
};

/// Capacity of each bounded loader request and result queue.
pub const LOADER_QUEUE_CAPACITY: usize = 8;
/// Maximum package rows returned by one directory scan.
pub const MAX_DISCOVERED_PACKAGES: usize = 64;
/// Maximum child directories examined during package discovery.
pub const MAX_SCANNED_DIRECTORIES: usize = 512;
/// Maximum UTF-8 bytes retained for one package directory name.
pub const MAX_PACKAGE_NAME_BYTES: usize = 128;
/// Maximum UTF-8 bytes accepted for a package discovery root path.
pub const MAX_DISCOVERY_ROOT_BYTES: usize = 512;
#[derive(Debug, Clone)]
/// Request to compile a package on the worker thread.
pub struct LoadScriptRequest {
    /// Caller-supplied identifier used for status correlation.
    pub request_id: u64,
    /// Package to compile.
    pub id: ScriptId,
    /// Whether this result may replace an active generation.
    pub replace: bool,
    pub(crate) snapshot: Option<super::auto_reload::AutoReloadSnapshot>,
}
#[derive(Debug)]
/// Package compilation failure returned by the worker.
pub struct ScriptLoadFailure {
    /// Identifier copied from the load request.
    pub request_id: u64,
    /// Package that failed compilation or validation.
    pub id: ScriptId,
    /// Human-readable error text.
    pub message: String,
}
/// Compiled package payload or its worker-reported failure.
pub type ScriptLoadResult = Result<PreparedScript, ScriptLoadFailure>;

#[derive(Debug, Clone, Copy)]
/// Request to scan packages without compiling their source.
pub struct DiscoverScriptsRequest {
    /// Optional caller-supplied identifier for result correlation.
    pub request_id: Option<u64>,
}

#[derive(Debug, Clone)]
/// Owned result of one package-directory scan.
pub struct ScriptDiscoveryResult {
    /// Identifier copied from the request.
    pub request_id: Option<u64>,
    /// Display form of the scanned root path.
    pub root: String,
    /// Discovered packages, including invalid rows with their issue text.
    pub packages: Vec<DiscoveredScript>,
    /// Root-level error when the scan could not complete.
    pub error: Option<String>,
}

enum LoaderRequest {
    Load(LoadScriptRequest),
    Discover(DiscoverScriptsRequest),
    SetAutoReload { id: ScriptId, enabled: bool },
}

struct LoaderWorker {
    root: Result<PathBuf, String>,
    canonical_root: Option<PathBuf>,
    factory: RhaiEngineFactory,
    requests: Receiver<LoaderRequest>,
    results: SyncSender<ScriptLoadResult>,
    discoveries: SyncSender<ScriptDiscoveryResult>,
    auto_reload: SyncSender<AutoReloadNotice>,
    overflow: Arc<AtomicUsize>,
    discovery_overflow: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
    watches: WorkerWatches,
}

impl LoaderWorker {
    fn run(mut self) {
        loop {
            self.watches.flush(&self.auto_reload);
            match self.requests.recv_timeout(Duration::from_millis(500)) {
                Ok(request) => {
                    if !self.handle(request) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {
                    if let Ok(root) = &self.root {
                        self.watches.poll(root, &self.auto_reload);
                    }
                }
            }
        }
        self.stopped.store(true, Ordering::Release);
    }

    fn handle(&mut self, request: LoaderRequest) -> bool {
        match request {
            LoaderRequest::SetAutoReload { id, enabled } => {
                if let Ok(root) = &self.root {
                    self.watches.configure(root, id, enabled);
                }
                true
            }
            LoaderRequest::Load(request) => self.handle_load(request),
            LoaderRequest::Discover(request) => self.handle_discovery(request),
        }
    }

    fn handle_load(&mut self, request: LoadScriptRequest) -> bool {
        let prepared = self.prepare_request(&request);
        let result = prepared.map_err(|message| ScriptLoadFailure {
            request_id: request.request_id,
            id: request.id,
            message: bounded_text(&message, MAX_EVENT_BYTES),
        });
        match self.results.try_send(result) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.overflow.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }

    fn prepare_request(&mut self, request: &LoadScriptRequest) -> Result<PreparedScript, String> {
        let configured = self.root.as_ref().map_err(Clone::clone)?;
        if self.canonical_root.is_none() {
            self.canonical_root = Some(
                configured
                    .canonicalize()
                    .map_err(|error| error.to_string())?,
            );
        }
        prepare(
            self.canonical_root.as_ref().expect("root resolved"),
            request,
            self.factory,
        )
    }

    fn handle_discovery(&self, request: DiscoverScriptsRequest) -> bool {
        let configured = self.root.as_ref().ok().map(PathBuf::as_path);
        let result = discovery::discover(configured, &self.root, request.request_id);
        match self.discoveries.try_send(result) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                self.discovery_overflow.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }
}

fn resolve_script_root(root: Option<PathBuf>) -> Result<PathBuf, String> {
    let Some(root) = root else {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        return Ok(executable
            .parent()
            .ok_or("game executable has no parent")?
            .join("MadMaxModding")
            .join("scripts"));
    };
    Ok(root)
}

/// The worker owns all filesystem access and compilation. Drop never joins it.
///
/// Request methods are non-blocking; full queues are reported to the caller.
pub struct ScriptLoader {
    requests: SyncSender<LoaderRequest>,
    results: Receiver<ScriptLoadResult>,
    discovery_results: Receiver<ScriptDiscoveryResult>,
    auto_reload: Receiver<AutoReloadNotice>,
    auto_reload_overflow: Receiver<AutoReloadOverflow>,
    overflow: Arc<AtomicUsize>,
    discovery_overflow: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
}

/// Cloneable UI-facing handle for queuing discovery without submitting a console command.
#[derive(Clone)]
pub struct ScriptDiscoveryRequester {
    requests: SyncSender<LoaderRequest>,
    stopped: Arc<AtomicBool>,
}
impl ScriptDiscoveryRequester {
    /// Submit a package scan request without blocking the calling thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker has stopped or the bounded queue is full.
    pub fn try_discover(&self, request: DiscoverScriptsRequest) -> Result<(), &'static str> {
        if self.stopped.load(Ordering::Acquire) {
            return Err("script loader unavailable");
        }
        self.requests
            .try_send(LoaderRequest::Discover(request))
            .map_err(|error| match error {
                TrySendError::Full(_) => "script loader request queue full",
                TrySendError::Disconnected(_) => "script loader unavailable",
            })
    }
}
impl ScriptLoader {
    /// Start a worker using the supplied package root.
    pub fn start(root: PathBuf, engine_factory: RhaiEngineFactory) -> Self {
        Self::start_worker(Some(root), engine_factory)
    }
    /// Start the worker at `MadMaxModding/scripts` beside the executable.
    ///
    /// The executable path is resolved on the worker thread.
    pub fn start_for_game(engine_factory: RhaiEngineFactory) -> Self {
        Self::start_worker(None, engine_factory)
    }
    fn start_worker(root: Option<PathBuf>, factory: RhaiEngineFactory) -> Self {
        let (requests, request_rx) = sync_channel::<LoaderRequest>(LOADER_QUEUE_CAPACITY);
        let (result_tx, results) = sync_channel(LOADER_QUEUE_CAPACITY);
        let (discovery_result_tx, discovery_results) = sync_channel(LOADER_QUEUE_CAPACITY);
        let (auto_reload_tx, auto_reload) = sync_channel(LOADER_QUEUE_CAPACITY);
        let (auto_reload_overflow_tx, auto_reload_overflow) =
            sync_channel(super::manager::MAX_ACTIVE_SCRIPTS);
        let overflow = Arc::new(AtomicUsize::new(0));
        let worker_overflow = overflow.clone();
        let discovery_overflow = Arc::new(AtomicUsize::new(0));
        let worker_discovery_overflow = discovery_overflow.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = stopped.clone();
        let spawn = std::thread::Builder::new()
            .name("madmax-script-loader".into())
            .spawn(move || {
                LoaderWorker {
                    root: resolve_script_root(root),
                    canonical_root: None,
                    factory,
                    requests: request_rx,
                    results: result_tx,
                    discoveries: discovery_result_tx,
                    auto_reload: auto_reload_tx,
                    overflow: worker_overflow,
                    discovery_overflow: worker_discovery_overflow,
                    stopped: worker_stopped,
                    watches: WorkerWatches::new(auto_reload_overflow_tx),
                }
                .run()
            });
        if spawn.is_err() {
            stopped.store(true, Ordering::Release);
        }
        Self {
            requests,
            results,
            discovery_results,
            auto_reload,
            auto_reload_overflow,
            overflow,
            discovery_overflow,
            stopped,
        }
    }
    /// Submit a bounded package compilation request without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker stopped or the request queue is full.
    pub fn try_request(&self, request: LoadScriptRequest) -> Result<(), &'static str> {
        if self.stopped.load(Ordering::Acquire) {
            return Err("script loader unavailable");
        }
        self.requests
            .try_send(LoaderRequest::Load(request))
            .map_err(|error| match error {
                TrySendError::Full(_) => "script loader request queue full",
                TrySendError::Disconnected(_) => "script loader unavailable",
            })
    }
    /// Take one completed compilation result, if available.
    pub fn try_result(&self) -> Option<ScriptLoadResult> {
        self.results.try_recv().ok()
    }
    /// Enable or disable file-change watching for one package.
    pub fn set_auto_reload(&self, id: ScriptId, enabled: bool) -> Result<(), &'static str> {
        if self.stopped.load(Ordering::Acquire) {
            return Err("script loader unavailable");
        }
        self.requests
            .try_send(LoaderRequest::SetAutoReload { id, enabled })
            .map_err(|error| match error {
                TrySendError::Full(_) => "script loader request queue full",
                TrySendError::Disconnected(_) => "script loader unavailable",
            })
    }
    /// Drain at most one request; cancelled notices are bounded by channel capacity.
    /// Take one queued file-change reload request, if available.
    pub fn try_auto_reload(&self) -> Option<LoadScriptRequest> {
        for _ in 0..LOADER_QUEUE_CAPACITY {
            let notice = self.auto_reload.try_recv().ok()?;
            notice.pending.store(false, Ordering::Release);
            if notice.active.load(Ordering::Acquire) {
                return Some(notice.request);
            }
        }
        None
    }
    /// Coalesced saturation markers, bounded by the maximum loaded script count.
    /// Each pending notice reports at most once, even if retries keep finding Full.
    /// Take package IDs whose file-change notices overflowed the queue.
    pub fn take_auto_reload_overflow(&self) -> Vec<ScriptId> {
        let mut ids = BTreeSet::new();
        for _ in 0..super::manager::MAX_ACTIVE_SCRIPTS {
            let Ok(marker) = self.auto_reload_overflow.try_recv() else {
                break;
            };
            if marker.active.load(Ordering::Acquire) {
                ids.insert(marker.id);
            }
        }
        ids.into_iter().collect()
    }
    /// Submit a bounded package discovery request without blocking.
    pub fn try_discover(&self, request: DiscoverScriptsRequest) -> Result<(), &'static str> {
        self.discovery_requester().try_discover(request)
    }
    /// Create a cloneable discovery-only request handle.
    pub fn discovery_requester(&self) -> ScriptDiscoveryRequester {
        ScriptDiscoveryRequester {
            requests: self.requests.clone(),
            stopped: self.stopped.clone(),
        }
    }
    /// Take one completed directory scan, if available.
    pub fn try_discovery_result(&self) -> Option<ScriptDiscoveryResult> {
        self.discovery_results.try_recv().ok()
    }
    /// Retained overload state survives a full event/result queue for the next drain.
    /// Take and reset the number of compilation results dropped due to overflow.
    pub fn take_overflow(&self) -> usize {
        self.overflow.swap(0, Ordering::Relaxed)
    }
    /// Take and reset the number of discovery results dropped due to overflow.
    pub fn take_discovery_overflow(&self) -> usize {
        self.discovery_overflow.swap(0, Ordering::Relaxed)
    }
}

struct AutoReloadNotice {
    request: LoadScriptRequest,
    active: Arc<AtomicBool>,
    pending: Arc<AtomicBool>,
}

struct AutoReloadOverflow {
    id: ScriptId,
    active: Arc<AtomicBool>,
}

struct WatchDelivery {
    active: Arc<AtomicBool>,
    pending: Arc<AtomicBool>,
    unsent: Option<LoadScriptRequest>,
    overflow_pending: bool,
    overflow_reported: bool,
}

struct WorkerWatches {
    tracker: super::auto_reload::AutoReloadTracker,
    deliveries: BTreeMap<ScriptId, WatchDelivery>,
    overflow: SyncSender<AutoReloadOverflow>,
}

impl WorkerWatches {
    fn new(overflow: SyncSender<AutoReloadOverflow>) -> Self {
        Self {
            tracker: super::auto_reload::AutoReloadTracker::default(),
            deliveries: BTreeMap::new(),
            overflow,
        }
    }
    fn configure(&mut self, root: &Path, id: ScriptId, enabled: bool) {
        if !enabled {
            self.tracker.remove(&id);
            if let Some(delivery) = self.deliveries.remove(&id) {
                delivery.active.store(false, Ordering::Release);
            }
        } else if !self.deliveries.contains_key(&id)
            && self.deliveries.len() < super::manager::MAX_ACTIVE_SCRIPTS
        {
            self.tracker.set_enabled(id.clone(), true);
            // Only readable packages establish a silent baseline. Initial missing
            // or unreadable files must pass through normal debounce and notify.
            let snapshot = package_snapshot(root, &id);
            let readable = snapshot.manifest.is_some() && snapshot.main.is_some();
            if readable {
                self.tracker.observe(&id, snapshot.fingerprint.clone());
                self.tracker.observe(&id, snapshot.fingerprint);
            }
            self.deliveries.insert(
                id,
                WatchDelivery {
                    active: Arc::new(AtomicBool::new(true)),
                    pending: Arc::new(AtomicBool::new(false)),
                    unsent: None,
                    overflow_pending: false,
                    overflow_reported: false,
                },
            );
        }
    }

    fn poll(&mut self, root: &Path, notifications: &SyncSender<AutoReloadNotice>) {
        for (id, delivery) in &mut self.deliveries {
            // Do not acknowledge newer content while a notice is outstanding:
            // it will be observed and debounced after the consumer drains it.
            if delivery.pending.load(Ordering::Acquire) {
                continue;
            }
            let snapshot = package_snapshot(root, id);
            if self.tracker.observe(id, snapshot.fingerprint.clone())
                && let Ok(request_id) = super::console::next_request_id()
            {
                // Allocate now: a later manual submission must have a newer ID,
                // even when this notice waits in a saturated queue.
                delivery.unsent = Some(LoadScriptRequest {
                    request_id,
                    id: id.clone(),
                    replace: true,
                    snapshot: Some(snapshot),
                });
                delivery.pending.store(true, Ordering::Release);
                delivery.overflow_reported = false;
            }
        }
        self.flush(notifications);
    }

    fn flush(&mut self, notifications: &SyncSender<AutoReloadNotice>) {
        for (id, delivery) in &mut self.deliveries {
            if let Some(request) = delivery.unsent.take() {
                let notice = AutoReloadNotice {
                    request,
                    active: delivery.active.clone(),
                    pending: delivery.pending.clone(),
                };
                if let Err(error) = notifications.try_send(notice) {
                    let notice = match error {
                        TrySendError::Full(notice) => {
                            if !delivery.overflow_reported {
                                delivery.overflow_pending = true;
                            }
                            notice
                        }
                        TrySendError::Disconnected(notice) => notice,
                    };
                    delivery.unsent = Some(notice.request);
                }
            }
            if delivery.overflow_pending {
                let marker = AutoReloadOverflow {
                    id: id.clone(),
                    active: delivery.active.clone(),
                };
                if self.overflow.try_send(marker).is_ok() {
                    delivery.overflow_pending = false;
                    delivery.overflow_reported = true;
                }
            }
        }
    }
}

fn package_snapshot(root: &Path, id: &ScriptId) -> super::auto_reload::AutoReloadSnapshot {
    let contents = root.canonicalize().ok().and_then(|root| {
        let package = confined(&root, &root.join(id.as_str())).ok()?;
        Some((
            read_bounded_bytes(&root, &package.join("manifest.toml"), MAX_MANIFEST_BYTES).ok(),
            read_bounded_bytes(
                &root,
                &package.join("main.rhai"),
                super::engine::MAX_SOURCE_BYTES,
            )
            .ok(),
        ))
    });
    let (manifest, main) = contents.unwrap_or_default();
    super::auto_reload::AutoReloadSnapshot::new(manifest, main)
}

fn confined(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root) {
        return Err("script path escapes configured root".into());
    }
    Ok(canonical)
}
fn read_bounded(root: &Path, path: &Path, limit: usize) -> Result<String, String> {
    String::from_utf8(read_bounded_bytes(root, path, limit)?)
        .map_err(|_| "package must contain UTF-8 text".into())
}
fn read_bounded_bytes(root: &Path, path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let path = confined(root, path)?;
    let file = File::open(path).map_err(|e| e.to_string())?;
    // Validate the opened object as well as the path: junction/symlink edits
    // between canonicalization and open cannot redirect reads outside the root.
    if !opened_path(&file)?.starts_with(root) {
        return Err("opened script file escapes configured root".into());
    }
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("package entry is not a regular file".into());
    }
    let mut bytes = Vec::with_capacity(limit.min(4096));
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("package file byte limit exceeded".into());
    }
    Ok(bytes)
}
fn prepare(
    root: &Path,
    request: &LoadScriptRequest,
    factory: RhaiEngineFactory,
) -> Result<PreparedScript, String> {
    if let Some(snapshot) = &request.snapshot {
        return prepare_snapshot(request, snapshot, factory);
    }
    // ScriptId's private constructor prevents arbitrary/absolute path segments.
    let root = root.to_path_buf();
    let package = confined(&root, &root.join(request.id.as_str()))?;
    if !package.is_dir() {
        return Err("script package is not a directory".into());
    }
    let manifest = read_bounded(&root, &package.join("manifest.toml"), MAX_MANIFEST_BYTES)?;
    let manifest: ScriptManifest =
        toml::from_str(&manifest).map_err(|e| bounded_text(&e.to_string(), MAX_EVENT_BYTES))?;
    let source = read_bounded(
        &root,
        &package.join("main.rhai"),
        super::engine::MAX_SOURCE_BYTES,
    )?;
    PreparedScript::compile_package(
        request.request_id,
        request.id.clone(),
        manifest,
        &source,
        request.replace,
        factory,
    )
}

fn prepare_snapshot(
    request: &LoadScriptRequest,
    snapshot: &super::auto_reload::AutoReloadSnapshot,
    factory: RhaiEngineFactory,
) -> Result<PreparedScript, String> {
    if !snapshot
        .fingerprint
        .matches_contents(snapshot.manifest.as_deref(), snapshot.main.as_deref())
    {
        return Err("stable package snapshot fingerprint mismatch".into());
    }
    let manifest = snapshot
        .manifest
        .as_deref()
        .ok_or_else(|| "stable package snapshot has no readable manifest".to_owned())?;
    let manifest =
        std::str::from_utf8(manifest).map_err(|_| "package must contain UTF-8 text".to_owned())?;
    let manifest: ScriptManifest =
        toml::from_str(manifest).map_err(|e| bounded_text(&e.to_string(), MAX_EVENT_BYTES))?;
    let source = snapshot
        .main
        .as_deref()
        .ok_or_else(|| "stable package snapshot has no readable main.rhai".to_owned())?;
    let source =
        std::str::from_utf8(source).map_err(|_| "package must contain UTF-8 text".to_owned())?;
    PreparedScript::compile_package(
        request.request_id,
        request.id.clone(),
        manifest,
        source,
        request.replace,
        factory,
    )
}

#[cfg(windows)]
fn opened_path(file: &File) -> Result<PathBuf, String> {
    use std::os::windows::{ffi::OsStringExt, io::AsRawHandle};
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFinalPathNameByHandleW(
            handle: *mut std::ffi::c_void,
            path: *mut u16,
            length: u32,
            flags: u32,
        ) -> u32;
    }
    let mut path = vec![0u16; 32_768];
    // SAFETY: file owns a live handle and path is a writable UTF-16 buffer.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            path.as_mut_ptr(),
            path.len() as u32,
            0,
        )
    } as usize;
    if length == 0 || length >= path.len() {
        return Err("cannot verify opened script path".into());
    }
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        &path[..length],
    )))
}
#[cfg(not(windows))]
fn opened_path(_file: &File) -> Result<PathBuf, String> {
    Err("root-confined package loading requires Windows handle validation".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, Instant};

    struct PackageRoot(PathBuf);
    impl PackageRoot {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "madmax-watch-{}-{}",
                std::process::id(),
                super::super::console::next_request_id().unwrap()
            ));
            fs::create_dir_all(path.join("example")).unwrap();
            fs::write(path.join("example/manifest.toml"), "manifest").unwrap();
            fs::write(path.join("example/main.rhai"), "original").unwrap();
            Self(path.canonicalize().unwrap())
        }
        fn valid() -> Self {
            let root = Self::new();
            fs::write(root.0.join("example/manifest.toml"), "id = \"example\"\napi_version = 1\ncapabilities = []\n[callbacks]\n\"game.post_sim\" = \"on_post_sim\"\n").unwrap();
            root.edit("fn on_post_sim(ctx) {}\n");
            root
        }
        fn edit(&self, source: &str) {
            fs::write(self.0.join("example/main.rhai"), source).unwrap();
        }
    }
    impl Drop for PackageRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn watches() -> WorkerWatches {
        let (tx, _rx) = sync_channel(super::super::manager::MAX_ACTIVE_SCRIPTS);
        WorkerWatches::new(tx)
    }
    fn id() -> ScriptId {
        ScriptId::parse("example").unwrap()
    }

    #[test]
    fn enabled_watch_requires_two_changed_polls_and_disabled_notice_is_cancelled() {
        let root = PackageRoot::new();
        let (tx, rx) = sync_channel(1);
        let mut watches = watches();
        watches.configure(&root.0, id(), true);
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        assert!(
            rx.try_recv().is_err(),
            "unchanged initial package must not reload"
        );
        root.edit("changed");
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
        watches.poll(&root.0, &tx);
        let notice = rx.try_recv().unwrap();
        watches.configure(&root.0, id(), false);
        assert!(!notice.active.load(Ordering::Acquire));
        root.edit("disabled");
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn edit_while_notice_pending_coalesces_until_consumed_then_debounces() {
        let root = PackageRoot::new();
        let (tx, rx) = sync_channel(2);
        let mut watches = watches();
        watches.configure(&root.0, id(), true);
        root.edit("first");
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        let notice = rx.try_recv().unwrap();
        root.edit("second");
        watches.poll(&root.0, &tx);
        root.edit("third");
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err(), "only one pending notice per script");
        notice.pending.store(false, Ordering::Release);
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
        watches.poll(&root.0, &tx);
        let next = rx.try_recv().unwrap();
        next.pending.store(false, Ordering::Release);
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        assert!(
            rx.try_recv().is_err(),
            "stable fingerprint only reports once"
        );
    }

    #[test]
    fn full_notice_queue_retries_without_losing_the_changed_fingerprint() {
        let root = PackageRoot::new();
        let other = ScriptId::parse("other").unwrap();
        fs::create_dir(root.0.join("other")).unwrap();
        fs::write(root.0.join("other/manifest.toml"), "manifest").unwrap();
        fs::write(root.0.join("other/main.rhai"), "original").unwrap();
        let (tx, rx) = sync_channel(1);
        let mut watches = watches();
        watches.configure(&root.0, id(), true);
        watches.configure(&root.0, other.clone(), true);
        root.edit("changed");
        fs::write(root.0.join("other/main.rhai"), "changed").unwrap();
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        let manual_id = super::super::console::next_request_id().unwrap();
        let first = rx.try_recv().unwrap();
        assert_eq!(first.request.id, id());
        assert!(first.request.request_id < manual_id);
        first.pending.store(false, Ordering::Release);
        watches.flush(&tx);
        let notice = rx.try_recv().unwrap();
        assert_eq!(notice.request.id, other);
        assert!(
            notice.request.request_id < manual_id,
            "retry retains ordering before later manual requests"
        );
        notice.pending.store(false, Ordering::Release);
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn enabling_after_file_deletion_reports_stable_missing_state_once() {
        for missing in ["manifest.toml", "main.rhai"] {
            let root = PackageRoot::new();
            fs::remove_file(root.0.join("example").join(missing)).unwrap();
            let (tx, rx) = sync_channel(1);
            let mut watches = watches();
            watches.configure(&root.0, id(), true);
            watches.poll(&root.0, &tx);
            assert!(rx.try_recv().is_err(), "{missing} must debounce first");
            watches.poll(&root.0, &tx);
            let notice = rx.try_recv().expect("initial missing file must notify");
            assert_eq!(notice.request.id, id());
            notice.pending.store(false, Ordering::Release);
            watches.poll(&root.0, &tx);
            watches.poll(&root.0, &tx);
            assert!(rx.try_recv().is_err(), "unchanged {missing} reports once");
        }
    }

    #[test]
    fn missing_file_reports_once_and_restoration_is_watched() {
        let root = PackageRoot::new();
        let (tx, rx) = sync_channel(1);
        let mut watches = watches();
        watches.configure(&root.0, id(), true);
        fs::remove_file(root.0.join("example/main.rhai")).unwrap();
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
        watches.poll(&root.0, &tx);
        rx.try_recv()
            .unwrap()
            .pending
            .store(false, Ordering::Release);
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
        root.edit("restored");
        watches.poll(&root.0, &tx);
        assert!(rx.try_recv().is_err());
        watches.poll(&root.0, &tx);
        assert_eq!(rx.try_recv().unwrap().request.id, id());
    }

    #[test]
    fn malformed_utf8_edits_have_distinct_bounded_content_fingerprints() {
        let root = PackageRoot::new();
        fs::write(root.0.join("example/main.rhai"), [0xff, 0x61]).unwrap();
        let first = package_snapshot(&root.0, &id()).fingerprint;
        fs::write(root.0.join("example/main.rhai"), [0xff, 0x62]).unwrap();
        assert_ne!(first, package_snapshot(&root.0, &id()).fingerprint);
    }

    #[test]
    fn full_notice_queue_reports_one_overflow_marker_per_pending_notice() {
        let root = PackageRoot::new();
        let (status_tx, status_rx) = sync_channel(1);
        let mut watches = WorkerWatches::new(status_tx);
        let (tx, _rx) = sync_channel(0);
        watches.configure(&root.0, id(), true);
        root.edit("changed");
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        let marker = status_rx.try_recv().unwrap();
        assert_eq!(marker.id, id());
        watches.flush(&tx);
        watches.poll(&root.0, &tx);
        assert!(
            status_rx.try_recv().is_err(),
            "same blocked notice reports once"
        );
        watches.configure(&root.0, id(), false);
        assert!(!marker.active.load(Ordering::Acquire));
    }

    #[test]
    fn worker_watch_api_keeps_discovery_prompt_and_uses_shared_request_ids() {
        let root = PackageRoot::new();
        let loader = ScriptLoader::start(root.0.clone(), RhaiEngineFactory);
        assert!(loader.take_auto_reload_overflow().is_empty());
        loader.set_auto_reload(id(), true).unwrap();
        loader
            .try_discover(DiscoverScriptsRequest {
                request_id: Some(41),
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while loader.try_discovery_result().is_none() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        root.edit("changed");
        let before = super::super::console::next_request_id().unwrap();
        let request = loop {
            if let Some(request) = loader.try_auto_reload() {
                break request;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        };
        let after = super::super::console::next_request_id().unwrap();
        assert!(before < request.request_id && request.request_id < after);
        assert_eq!(request.id, id());
        assert!(request.replace);
        loader.set_auto_reload(id(), false).unwrap();
    }

    #[test]
    fn automatic_notice_prepares_only_the_debounced_snapshot() {
        for later_source in [
            "fn on_post_sim(ctx) { let later = true; }",
            "this is invalid rhai !!!",
        ] {
            let root = PackageRoot::valid();
            let (tx, rx) = sync_channel(1);
            let mut watches = watches();
            watches.configure(&root.0, id(), true);
            root.edit("fn on_post_sim(ctx) { let accepted = true; }");
            watches.poll(&root.0, &tx);
            watches.poll(&root.0, &tx);
            let accepted = rx.try_recv().unwrap();
            assert_eq!(
                accepted.request.snapshot.as_ref().unwrap().main.as_deref(),
                Some(b"fn on_post_sim(ctx) { let accepted = true; }".as_slice())
            );

            // The editor changes the live package after the stable notice, before
            // the loader consumes it. Preparation must still compile the accepted bytes.
            root.edit(later_source);
            assert!(prepare(&root.0, &accepted.request, RhaiEngineFactory).is_ok());
            accepted.pending.store(false, Ordering::Release);
            watches.poll(&root.0, &tx);
            watches.poll(&root.0, &tx);
            let later = rx.try_recv().expect("later complete content must debounce");
            assert_eq!(
                later.request.snapshot.as_ref().unwrap().main.as_deref(),
                Some(later_source.as_bytes())
            );
            assert_eq!(
                prepare(&root.0, &later.request, RhaiEngineFactory).is_ok(),
                later_source.starts_with("fn "),
                "later content is only prepared after its own stable notice"
            );
        }
    }

    #[test]
    fn partial_save_is_not_compiled_before_valid_content_debounces() {
        let root = PackageRoot::valid();
        let (tx, rx) = sync_channel(1);
        let mut watches = watches();
        watches.configure(&root.0, id(), true);
        root.edit("fn on_post_sim(ctx) { let accepted = true; }");
        watches.poll(&root.0, &tx);
        watches.poll(&root.0, &tx);
        let accepted = rx.try_recv().unwrap();

        root.edit("fn on_post_sim(");
        watches.poll(&root.0, &tx);
        accepted.pending.store(false, Ordering::Release);
        watches.poll(&root.0, &tx);
        assert!(
            rx.try_recv().is_err(),
            "partial save must not emit a notice"
        );
        root.edit("fn on_post_sim(ctx) { let restored = true; }");
        watches.poll(&root.0, &tx);
        assert!(
            rx.try_recv().is_err(),
            "first valid observation must debounce"
        );
        watches.poll(&root.0, &tx);
        let restored = rx
            .try_recv()
            .expect("valid content should notify after debounce");
        assert!(prepare(&root.0, &restored.request, RhaiEngineFactory).is_ok());
    }
}
