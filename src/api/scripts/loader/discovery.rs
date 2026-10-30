//! Bounded script-package discovery and manifest validation.
use super::*;
use std::{fs, path::Path};

pub(super) fn discover(
    configured_root: Option<&Path>,
    root_result: &Result<PathBuf, String>,
    request_id: Option<u64>,
) -> ScriptDiscoveryResult {
    let Some(configured) = configured_root
        .map(Path::to_path_buf)
        .or_else(|| root_result.as_ref().ok().cloned())
    else {
        return unresolved_root(request_id);
    };
    let display_root = bounded_text(&configured.display().to_string(), MAX_DISCOVERY_ROOT_BYTES);
    let root = match configured.canonicalize() {
        Ok(root) if root.is_dir() => root,
        Ok(_) => {
            return result(
                request_id,
                display_root,
                Vec::new(),
                Some("scripts path is not a directory".into()),
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return result(request_id, display_root, Vec::new(), None);
        }
        Err(error) => {
            return result(
                request_id,
                display_root,
                Vec::new(),
                Some(bounded_text(&error.to_string(), MAX_EVENT_BYTES)),
            );
        }
    };
    scan_root(&root, display_root, request_id)
}

fn scan_root(root: &Path, display_root: String, request_id: Option<u64>) -> ScriptDiscoveryResult {
    let mut entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return result(
                request_id,
                display_root,
                Vec::new(),
                Some(bounded_text(&error.to_string(), MAX_EVENT_BYTES)),
            );
        }
    };
    let mut packages = Vec::new();
    let mut scanned = 0;
    let mut truncated = false;
    while let Some(entry) = entries.next() {
        scanned += 1;
        if scanned > MAX_SCANNED_DIRECTORIES {
            truncated = true;
            break;
        }
        match entry {
            Ok(entry) => {
                match discover_entry(root, entry, packages.len() >= MAX_DISCOVERED_PACKAGES) {
                    Ok(Some(package)) => packages.push(package),
                    Ok(None) => {}
                    Err(()) => {
                        truncated = true;
                        break;
                    }
                }
            }
            Err(error) => packages.push(invalid_row(
                "<directory entry>".into(),
                None,
                format!("cannot enumerate entry: {error}"),
            )),
        }
    }
    packages.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    result(
        request_id,
        display_root,
        packages,
        truncated.then(|| "scan limit reached; results are truncated".into()),
    )
}

fn discover_entry(
    root: &Path,
    entry: fs::DirEntry,
    package_limit_reached: bool,
) -> Result<Option<DiscoveredScript>, ()> {
    let name = bounded_text(&entry.file_name().to_string_lossy(), MAX_PACKAGE_NAME_BYTES);
    if name.is_empty() || name.starts_with('.') {
        return Ok(None);
    }
    let path = entry.path();
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Ok(Some(invalid_row(
                name,
                None,
                format!("cannot inspect package: {error}"),
            )));
        }
    };
    if !metadata.is_dir() {
        return Ok(None);
    }
    if package_limit_reached {
        return Err(());
    }
    let id = ScriptId::parse(name.clone()).ok();
    let package = match confined(root, &path) {
        Ok(package) => package,
        Err(error) => return Ok(Some(invalid_row(name, id, error))),
    };
    let Some(id) = id else {
        return Ok(Some(invalid_row(
            name,
            None,
            "folder name is not a valid script ID".into(),
        )));
    };
    Ok(Some(discover_package(root, package, name, id)))
}

fn discover_package(root: &Path, package: PathBuf, name: String, id: ScriptId) -> DiscoveredScript {
    let manifest_text = match read_bounded(root, &package.join("manifest.toml"), MAX_MANIFEST_BYTES)
    {
        Ok(text) => text,
        Err(error) => return invalid_row(name, Some(id), format!("manifest: {error}")),
    };
    let manifest: ScriptManifest = match toml::from_str(&manifest_text) {
        Ok(manifest) => manifest,
        Err(error) => {
            return invalid_row(
                name,
                Some(id),
                format!(
                    "invalid manifest: {}",
                    bounded_text(&error.to_string(), MAX_EVENT_BYTES)
                ),
            );
        }
    };
    let summary = manifest.summary();
    let issue = manifest
        .validate(&id)
        .err()
        .or_else(|| {
            ScriptId::parse(manifest.id.clone())
                .err()
                .map(|_| "manifest ID is invalid".into())
        })
        .or_else(|| validate_entry(root, &package).err());
    DiscoveredScript {
        package_name: name,
        id: Some(id),
        manifest: Some(summary),
        issue: issue.map(|issue| bounded_text(&issue, MAX_EVENT_BYTES)),
    }
}

fn validate_entry(root: &Path, package: &Path) -> Result<(), String> {
    let main = confined(root, &package.join("main.rhai"))?;
    if main.is_file() {
        Ok(())
    } else {
        Err("main.rhai is not a regular file".into())
    }
}

fn unresolved_root(request_id: Option<u64>) -> ScriptDiscoveryResult {
    result(
        request_id,
        "MadMaxModding/scripts".into(),
        Vec::new(),
        Some("cannot resolve game scripts directory".into()),
    )
}

fn result(
    request_id: Option<u64>,
    root: String,
    packages: Vec<DiscoveredScript>,
    error: Option<String>,
) -> ScriptDiscoveryResult {
    ScriptDiscoveryResult {
        request_id,
        root,
        packages,
        error,
    }
}

fn invalid_row(package_name: String, id: Option<ScriptId>, issue: String) -> DiscoveredScript {
    DiscoveredScript {
        package_name,
        id,
        manifest: None,
        issue: Some(bounded_text(&issue, MAX_EVENT_BYTES)),
    }
}
