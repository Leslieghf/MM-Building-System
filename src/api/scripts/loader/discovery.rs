//! Bounded script-package discovery and manifest validation.
use super::*;
use std::{fs, path::Path};

pub(super) fn discover(
    configured_root: Option<&Path>,
    root_result: &Result<PathBuf, String>,
    request_id: Option<u64>,
) -> ScriptDiscoveryResult {
    let configured = configured_root
        .map(Path::to_path_buf)
        .or_else(|| root_result.as_ref().ok().cloned());
    let Some(configured) = configured else {
        return ScriptDiscoveryResult {
            request_id,
            root: "MadMaxModding/scripts".into(),
            packages: Vec::new(),
            error: Some("cannot resolve game scripts directory".into()),
        };
    };
    let display_root = bounded_text(&configured.display().to_string(), MAX_DISCOVERY_ROOT_BYTES);
    let root = match configured.canonicalize() {
        Ok(root) if root.is_dir() => root,
        Ok(_) => {
            return ScriptDiscoveryResult {
                request_id,
                root: display_root,
                packages: Vec::new(),
                error: Some("scripts path is not a directory".into()),
            };
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ScriptDiscoveryResult {
                request_id,
                root: display_root,
                packages: Vec::new(),
                error: None,
            };
        }
        Err(error) => {
            return ScriptDiscoveryResult {
                request_id,
                root: display_root,
                packages: Vec::new(),
                error: Some(bounded_text(&error.to_string(), MAX_EVENT_BYTES)),
            };
        }
    };
    let mut entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) => {
            return ScriptDiscoveryResult {
                request_id,
                root: display_root,
                packages: Vec::new(),
                error: Some(bounded_text(&error.to_string(), MAX_EVENT_BYTES)),
            };
        }
    };
    let mut packages = Vec::new();
    let mut scanned = 0usize;
    let mut truncated = false;
    loop {
        match entries.next() {
            Some(Ok(entry)) => {
                scanned += 1;
                if scanned > MAX_SCANNED_DIRECTORIES {
                    truncated = true;
                    break;
                }
                let name =
                    bounded_text(&entry.file_name().to_string_lossy(), MAX_PACKAGE_NAME_BYTES);
                if name.is_empty() || name.starts_with('.') {
                    continue;
                }
                let path = entry.path();
                let metadata = match fs::metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        packages.push(invalid_row(
                            name,
                            None,
                            format!("cannot inspect package: {error}"),
                        ));
                        continue;
                    }
                };
                if !metadata.is_dir() {
                    continue;
                }
                if packages.len() >= MAX_DISCOVERED_PACKAGES {
                    truncated = true;
                    break;
                }
                let id = ScriptId::parse(name.clone()).ok();
                let package = match confined(&root, &path) {
                    Ok(package) => package,
                    Err(error) => {
                        packages.push(invalid_row(name, id, error));
                        continue;
                    }
                };
                let Some(id) = id else {
                    packages.push(invalid_row(
                        name,
                        None,
                        "folder name is not a valid script ID".into(),
                    ));
                    continue;
                };
                let manifest_text =
                    match read_bounded(&root, &package.join("manifest.toml"), MAX_MANIFEST_BYTES) {
                        Ok(text) => text,
                        Err(error) => {
                            packages.push(invalid_row(
                                name,
                                Some(id),
                                format!("manifest: {error}"),
                            ));
                            continue;
                        }
                    };
                let manifest: ScriptManifest = match toml::from_str(&manifest_text) {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        packages.push(invalid_row(
                            name,
                            Some(id),
                            format!(
                                "invalid manifest: {}",
                                bounded_text(&error.to_string(), MAX_EVENT_BYTES)
                            ),
                        ));
                        continue;
                    }
                };
                let summary: ScriptManifestSummary = manifest.summary();
                let validation = manifest.validate(&id);
                let manifest_id = ScriptId::parse(manifest.id.clone());
                let entry_issue = confined(&root, &package.join("main.rhai")).and_then(|main| {
                    if main.is_file() {
                        Ok(())
                    } else {
                        Err("main.rhai is not a regular file".into())
                    }
                });
                let issue = validation
                    .err()
                    .or_else(|| manifest_id.err().map(|_| "manifest ID is invalid".into()))
                    .or_else(|| entry_issue.err());
                packages.push(DiscoveredScript {
                    package_name: name,
                    id: Some(id),
                    manifest: Some(summary),
                    issue: issue.map(|issue| bounded_text(&issue, MAX_EVENT_BYTES)),
                });
            }
            Some(Err(error)) => {
                packages.push(invalid_row(
                    "<directory entry>".into(),
                    None,
                    format!("cannot enumerate entry: {error}"),
                ));
            }
            None => break,
        }
    }
    packages.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    ScriptDiscoveryResult {
        request_id,
        root: display_root,
        packages,
        error: truncated.then(|| "scan limit reached; results are truncated".into()),
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
