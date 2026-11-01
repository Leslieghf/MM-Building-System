//! Deterministic validation and ordering for local script-package dependencies.
use super::ScriptId;
use super::package::{PackageRequirementKind, ScriptManifest};
use semver::Version;
use std::collections::{BTreeMap, BTreeSet};

/// Host extension APIs currently implemented by the Rust workbench shell.
pub(crate) fn host_api_version(id: &str) -> Option<Version> {
    match id {
        "workbench.tabs" => Some(Version::new(1, 0, 0)),
        _ => None,
    }
}

/// Resolve hard requirements and return package IDs in dependency-first order.
///
/// Optional package requirements never cause a package to be staged here;
/// callers may use the returned set to display their unavailable state.
pub(crate) fn resolve_order(
    requested: &ScriptId,
    manifests: &BTreeMap<ScriptId, ScriptManifest>,
) -> Result<Vec<ScriptId>, String> {
    let mut order = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    visit(
        requested,
        manifests,
        &mut visiting,
        &mut visited,
        &mut order,
    )?;

    for id in &order {
        let manifest = manifests.get(id).expect("visited package has a manifest");
        let version = &manifest.version.0;
        for conflict in &manifest.conflicts {
            if let Some((other_id, other)) = order.iter().find_map(|other_id| {
                (other_id.as_str() == conflict.id)
                    .then(|| manifests.get(other_id).map(|other| (other_id, other)))
                    .flatten()
            }) && conflict.version.matches(&other.version.0)
            {
                return Err(format!(
                    "package {id} {version} conflicts with {other_id} {}",
                    other.version.0
                ));
            }
        }
    }
    Ok(order)
}

fn visit(
    id: &ScriptId,
    manifests: &BTreeMap<ScriptId, ScriptManifest>,
    visiting: &mut BTreeSet<ScriptId>,
    visited: &mut BTreeSet<ScriptId>,
    order: &mut Vec<ScriptId>,
) -> Result<(), String> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id.clone()) {
        return Err(format!("package dependency cycle includes {id}"));
    }
    let manifest = manifests
        .get(id)
        .ok_or_else(|| format!("required package {id} is not installed"))?;
    let mut requirements = manifest.requires.iter().collect::<Vec<_>>();
    requirements.sort_by(|left, right| left.id.cmp(&right.id));
    for requirement in requirements {
        match requirement.kind {
            PackageRequirementKind::HostApi => {
                let compatible = host_api_version(&requirement.id)
                    .is_some_and(|version| requirement.version.matches(&version));
                if !compatible && !requirement.optional {
                    return Err(format!(
                        "package {id} requires unavailable host API {} {}",
                        requirement.id, requirement.version
                    ));
                }
            }
            PackageRequirementKind::Package if requirement.optional => {}
            PackageRequirementKind::Package => {
                let dependency = ScriptId::parse(requirement.id.clone())
                    .map_err(|_| format!("package {id} has invalid dependency ID"))?;
                let dependency_manifest = manifests.get(&dependency).ok_or_else(|| {
                    format!("package {id} requires missing package {}", requirement.id)
                })?;
                if !requirement.version.matches(&dependency_manifest.version.0) {
                    return Err(format!(
                        "package {id} requires {} {}, found {}",
                        requirement.id, requirement.version, dependency_manifest.version.0
                    ));
                }
                visit(&dependency, manifests, visiting, visited, order)?;
            }
        }
    }
    visiting.remove(id);
    visited.insert(id.clone());
    order.push(id.clone());
    Ok(())
}
