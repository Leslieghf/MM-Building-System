//! Bounded directory scanning and structural manifest validation.
//!
//! Discovery accepts package directories beneath the configured root and
//! validates their manifests without compiling their scripts. Directory,
//! manifest-size, and package-count limits prevent an unbounded scan; source
//! compilation belongs to the loader worker after a package is selected.
use super::*;
use std::{collections::BTreeMap, fs, path::Path};

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
    annotate_dependency_status(root, &mut packages);
    result(
        request_id,
        display_root,
        packages,
        truncated.then(|| "scan limit reached; results are truncated".into()),
    )
}

fn annotate_dependency_status(root: &Path, packages: &mut [DiscoveredScript]) {
    let manifests = packages
        .iter()
        .filter_map(|package| {
            if package.issue.is_some() {
                return None;
            }
            let id = package.id.as_ref()?;
            let path = root.join(package.package_name.as_str());
            let path = confined(root, &path).ok()?;
            let text = read_bounded(root, &path.join("manifest.toml"), MAX_MANIFEST_BYTES).ok()?;
            let manifest = toml::from_str::<ScriptManifest>(&text).ok()?;
            manifest.validate(id).ok()?;
            Some((id.clone(), manifest))
        })
        .collect::<BTreeMap<_, _>>();

    for package in packages {
        let (Some(id), Some(summary)) = (&package.id, &mut package.manifest) else {
            continue;
        };
        let Some(manifest) = manifests.get(id) else {
            continue;
        };
        let state = match super::super::package_graph::resolve_order(id, &manifests) {
            Err(reason) => {
                super::super::package::DependencyState::Blocked(bounded_text(&reason, 160))
            }
            Ok(_) => {
                let unavailable = manifest
                    .requires
                    .iter()
                    .filter(|requirement| requirement.optional)
                    .filter(|requirement| match requirement.kind {
                        crate::api::scripts::package::PackageRequirementKind::HostApi => {
                            !super::super::package_graph::host_api_version(&requirement.id)
                                .is_some_and(|version| requirement.version.matches(&version))
                        }
                        crate::api::scripts::package::PackageRequirementKind::Package => {
                            ScriptId::parse(requirement.id.clone())
                                .ok()
                                .and_then(|dependency| manifests.get(&dependency))
                                .is_none_or(|dependency| {
                                    !requirement.version.matches(&dependency.version.0)
                                })
                        }
                    })
                    .map(|requirement| requirement.id.as_str())
                    .collect::<Vec<_>>();
                if unavailable.is_empty() {
                    super::super::package::DependencyState::Ready
                } else {
                    super::super::package::DependencyState::OptionalUnavailable(bounded_text(
                        &format!("optional unavailable: {}", unavailable.join(", ")),
                        160,
                    ))
                }
            }
        };
        summary.dependency_state = state;
    }
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
        .or_else(|| validate_entries(root, &package, &manifest).err());
    DiscoveredScript {
        package_name: name,
        id: Some(id),
        manifest: Some(summary),
        issue: issue.map(|issue| bounded_text(&issue, MAX_EVENT_BYTES)),
    }
}

fn validate_entries(root: &Path, package: &Path, manifest: &ScriptManifest) -> Result<(), String> {
    for relative in &manifest.sources {
        let source = confined(root, &package.join(relative))
            .map_err(|error| format!("{relative}: {error}"))?;
        if !source.is_file() {
            return Err(format!("{relative} is not a regular file"));
        }
    }
    Ok(())
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
