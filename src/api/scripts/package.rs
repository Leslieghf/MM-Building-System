//! Script package identity, manifest validation, and compiled payload.
//!
//! # Package contract
//!
//! A package directory contains a manifest and its declared Rhai source files.
//! Validation checks the identifier, API version, capability names, callback
//! phase map, and bounded source sizes before compilation. [`PreparedScript`]
//! is an owned transfer value; it contains no live filesystem handles or game
//! pointers.
use super::{
    engine::{MAX_SOURCE_BYTES, RhaiEngineFactory},
    *,
};
use crate::api::rhai::ScriptApiRegistry;
use rhai::AST;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Maximum number of declared source files in a package.
pub const MAX_SOURCES_PER_SCRIPT: usize = 32;
/// Maximum package requirements or conflicts declared by one package.
pub const MAX_RELATIONS_PER_SCRIPT: usize = 32;

/// Maximum UTF-8 bytes accepted for `manifest.toml`.
pub const MAX_MANIFEST_BYTES: usize = 8 * 1024;
/// Maximum combined manifest and Rhai source size accepted for a package.
pub const MAX_PACKAGE_BYTES: usize = MAX_MANIFEST_BYTES + MAX_SOURCE_BYTES;
/// Maximum callback entries allowed in one manifest.
pub const MAX_CALLBACKS_PER_SCRIPT: usize = 2;
/// Maximum requested capabilities allowed in one manifest.
pub const MAX_CAPABILITIES_PER_SCRIPT: usize = 16;

#[derive(Debug, Clone)]
/// Display-safe summary of a parsed package manifest.
pub struct ScriptManifestSummary {
    /// Declared package ID.
    pub id: String,
    /// Required scripting API version.
    pub api_version: u32,
    /// Callback phase and function pairs.
    pub callbacks: Vec<String>,
    /// Capability IDs requested by the package.
    pub capabilities: Vec<String>,
    /// Semantic version provided by this package.
    pub version: String,
    /// Declared package and host API requirements.
    pub requires: Vec<String>,
    /// Declared package conflicts.
    pub conflicts: Vec<String>,
    /// Optional workbench tab metadata.
    pub workbench_tab: Option<WorkbenchTabMetadata>,
    /// Local dependency readiness discovered from installed manifests.
    pub dependency_state: DependencyState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Bounded package dependency status for discovery UI.
pub enum DependencyState {
    /// All required local packages and host APIs are available.
    Ready,
    /// Required dependencies are available, but optional requirements are not.
    OptionalUnavailable(String),
    /// A required package/API is missing, incompatible, or cyclic.
    Blocked(String),
}

#[derive(Debug, Clone)]
/// One directory returned by package discovery, valid or invalid.
pub struct DiscoveredScript {
    /// Directory name as displayed by the host.
    pub package_name: String,
    /// Validated package ID, absent when validation failed.
    pub id: Option<ScriptId>,
    /// Parsed manifest summary, absent when parsing failed.
    pub manifest: Option<ScriptManifestSummary>,
    /// Validation or discovery issue, if any.
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
/// Package dependency kind. Dependencies do not grant permissions.
pub enum PackageRequirementKind {
    /// Another locally installed script package.
    #[serde(rename = "package")]
    Package,
    /// A versioned extension API provided by the Rust host.
    #[serde(rename = "host_api")]
    HostApi,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
/// A package's required or optional versioned dependency.
pub struct PackageRequirement {
    /// Dependency namespace.
    pub kind: PackageRequirementKind,
    /// Package ID or host API ID, depending on `kind`.
    pub id: String,
    /// Accepted semantic versions.
    pub version: semver::VersionReq,
    /// Missing or incompatible optional requirements do not block activation.
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
/// A package version that cannot be active alongside this package.
pub struct PackageConflict {
    /// Conflicting local package ID.
    pub id: String,
    /// Versions that conflict with this package.
    pub version: semver::VersionReq,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
/// Static tab metadata owned by a workbench package.
pub struct WorkbenchTabMetadata {
    /// Stable package-local tab ID.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Stable host ordering hint; ties sort by package ID and tab ID.
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
/// Semantic version provided by a script package.
pub struct PackageVersion(pub semver::Version);

impl Default for PackageVersion {
    fn default() -> Self {
        Self(semver::Version::new(0, 0, 0))
    }
}

impl PackageVersion {
    fn as_str(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parsed `manifest.toml` contract for a Rhai package.
///
/// # Example
///
/// ```toml
/// id = "vehicle_watch"
/// api_version = 1
/// version = "1.2.0"
/// sources = ["main.rhai", "ui/events.rhai"]
/// capabilities = ["game.read", "log"]
///
/// [callbacks]
/// "game.post_sim" = "on_post_sim"
/// on_run = "on_run"
/// ```
pub struct ScriptManifest {
    /// Package identifier; it must match the package directory and request ID.
    pub id: String,
    /// Version of the script-visible API required by this package.
    pub api_version: u32,
    /// Supported phase IDs mapped to one-argument function names.
    pub callbacks: BTreeMap<String, String>,
    /// Capability identifiers granted to this package.
    pub capabilities: Vec<String>,
    /// Semantic package version. Older packages default to `0.0.0`.
    #[serde(default)]
    pub version: PackageVersion,
    /// Ordered, package-confined Rhai sources. Older packages default to `main.rhai`.
    #[serde(default = "default_package_sources")]
    pub sources: Vec<String>,
    /// Required and optional package/host API versions.
    #[serde(default)]
    pub requires: Vec<PackageRequirement>,
    /// Package versions that conflict with activation.
    #[serde(default)]
    pub conflicts: Vec<PackageConflict>,
    /// Optional script-owned native workbench tab.
    #[serde(default)]
    pub workbench_tab: Option<WorkbenchTabMetadata>,
}

fn default_package_sources() -> Vec<String> {
    vec!["main.rhai".to_owned()]
}

impl ScriptManifest {
    pub(crate) fn validate(&self, id: &ScriptId) -> Result<(), String> {
        if ScriptId::parse(self.id.clone()).as_ref() != Ok(id) {
            return Err("manifest ID does not match requested package".into());
        }
        if self.api_version != API_VERSION {
            return Err("unsupported script API version".into());
        }
        if self.sources.is_empty()
            || self.sources.len() > MAX_SOURCES_PER_SCRIPT
            || !self.sources.iter().any(|source| source == "main.rhai")
        {
            return Err("declare main.rhai and no more than 32 source files".into());
        }
        for (index, source) in self.sources.iter().enumerate() {
            if !valid_source_path(source) || self.sources[..index].contains(source) {
                return Err("source paths must be unique confined relative paths".into());
            }
        }
        if self.requires.len() > MAX_RELATIONS_PER_SCRIPT
            || self.conflicts.len() > MAX_RELATIONS_PER_SCRIPT
        {
            return Err("too many package requirements or conflicts".into());
        }
        let mut relation_keys = Vec::new();
        for requirement in &self.requires {
            let valid_id = match requirement.kind {
                PackageRequirementKind::Package => ScriptId::parse(requirement.id.clone()).is_ok(),
                PackageRequirementKind::HostApi => valid_host_api_id(&requirement.id),
            };
            let key = (
                match requirement.kind {
                    PackageRequirementKind::Package => "package",
                    PackageRequirementKind::HostApi => "host_api",
                },
                requirement.id.as_str(),
            );
            if !valid_id
                || requirement.id.len() > super::MAX_SCRIPT_ID_BYTES
                || relation_keys.contains(&key)
                || (requirement.kind == PackageRequirementKind::Package
                    && requirement.id == self.id)
            {
                return Err("invalid or duplicate package requirement".into());
            }
            relation_keys.push(key);
        }
        for (index, conflict) in self.conflicts.iter().enumerate() {
            if ScriptId::parse(conflict.id.clone()).is_err()
                || conflict.id.len() > super::MAX_SCRIPT_ID_BYTES
                || conflict.id == self.id
                || self.conflicts[..index]
                    .iter()
                    .any(|prior| prior.id == conflict.id)
            {
                return Err("invalid or duplicate package conflict".into());
            }
        }
        if let Some(tab) = &self.workbench_tab
            && (!valid_manifest_id(&tab.id)
                || tab.title.trim().is_empty()
                || tab.title.len() > 96
                || tab.title.chars().any(char::is_control))
        {
            return Err("invalid workbench tab metadata".into());
        }
        if self.callbacks.is_empty() || self.callbacks.len() > MAX_CALLBACKS_PER_SCRIPT {
            return Err("declare one or two supported callbacks".into());
        }
        let registry = ScriptApiRegistry::v1();
        for (callback, function) in &self.callbacks {
            let descriptor = registry
                .descriptors
                .iter()
                .find(|d| {
                    (callback == d.phase.as_str() && d.callback != "on_run")
                        || (callback == "on_run" && d.callback == "on_run")
                })
                .ok_or("unsupported callback ID")?;
            if function != descriptor.callback {
                return Err("unsupported callback function name".into());
            }
        }
        if self.capabilities.len() > registry.capabilities.len()
            || self.capabilities.len() > MAX_CAPABILITIES_PER_SCRIPT
        {
            return Err("too many capabilities".into());
        }
        for (index, capability) in self.capabilities.iter().enumerate() {
            if !registry.capabilities.contains(&capability.as_str())
                || self.capabilities[..index].contains(capability)
            {
                return Err("unsupported or duplicate capability ID".into());
            }
        }
        if self.workbench_tab.is_some() {
            let declares_host_api = self.requires.iter().any(|requirement| {
                requirement.kind == PackageRequirementKind::HostApi
                    && requirement.id == "workbench.tabs"
                    && requirement.version.matches(&semver::Version::new(1, 0, 0))
                    && !requirement.optional
            });
            if !declares_host_api {
                return Err("workbench tabs require host API workbench.tabs ^1.0".into());
            }
            if !self
                .capabilities
                .iter()
                .any(|capability| capability == "ui.workbench")
                || !self
                    .capabilities
                    .iter()
                    .any(|capability| capability == "ui.panels")
            {
                return Err(
                    "workbench tabs require ui.workbench and ui.panels capabilities".into(),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn summary(&self) -> ScriptManifestSummary {
        ScriptManifestSummary {
            id: super::bounded_text(&self.id, super::MAX_SCRIPT_ID_BYTES),
            api_version: self.api_version,
            callbacks: self
                .callbacks
                .iter()
                .take(MAX_CALLBACKS_PER_SCRIPT + 1)
                .map(|(key, value)| {
                    format!(
                        "{}={}",
                        bounded_package_text(key),
                        bounded_package_text(value)
                    )
                })
                .collect(),
            capabilities: self
                .capabilities
                .iter()
                .take(MAX_CAPABILITIES_PER_SCRIPT + 1)
                .map(|value| bounded_package_text(value))
                .collect(),
            version: self.version.as_str(),
            requires: self
                .requires
                .iter()
                .take(MAX_RELATIONS_PER_SCRIPT + 1)
                .map(|item| {
                    format!(
                        "{} {} {}{}",
                        match item.kind {
                            PackageRequirementKind::Package => "package",
                            PackageRequirementKind::HostApi => "host_api",
                        },
                        bounded_package_text(&item.id),
                        item.version,
                        if item.optional { " (optional)" } else { "" }
                    )
                })
                .collect(),
            conflicts: self
                .conflicts
                .iter()
                .take(MAX_RELATIONS_PER_SCRIPT + 1)
                .map(|item| format!("{} {}", bounded_package_text(&item.id), item.version))
                .collect(),
            workbench_tab: self.workbench_tab.clone(),
            dependency_state: DependencyState::Ready,
        }
    }
}

fn valid_source_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && value.ends_with(".rhai")
}

fn valid_manifest_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

fn valid_host_api_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.split('.').all(valid_manifest_id)
}

fn bounded_package_text(value: &str) -> String {
    super::bounded_text(value, super::MAX_SCRIPT_ID_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_gizmo_example_manifest_and_rhai_compile() {
        let manifest: ScriptManifest = toml::from_str(include_str!(
            "../../../examples/scripts/debug_gizmo/manifest.toml"
        ))
        .expect("debug_gizmo manifest should parse");
        let id = ScriptId::parse("debug_gizmo").expect("example ID should be valid");
        let prepared = PreparedScript::compile_package(
            0,
            id,
            manifest,
            include_str!("../../../examples/scripts/debug_gizmo/main.rhai"),
            true,
            RhaiEngineFactory,
        )
        .expect("debug_gizmo should compile against the current script API");
        assert_eq!(prepared.manifest().id, "debug_gizmo");
    }

    #[test]
    fn debug_orientation_example_manifest_and_rhai_compile() {
        let manifest: ScriptManifest = toml::from_str(include_str!(
            "../../../examples/scripts/debug_orientation/manifest.toml"
        ))
        .expect("debug_orientation manifest should parse");
        let id = ScriptId::parse("debug_orientation").expect("example ID should be valid");
        let prepared = PreparedScript::compile_package(
            0,
            id,
            manifest,
            include_str!("../../../examples/scripts/debug_orientation/main.rhai"),
            true,
            RhaiEngineFactory,
        )
        .expect("debug_orientation should compile against the current script API");
        assert_eq!(prepared.manifest().id, "debug_orientation");
    }

    #[test]
    fn reticle_ray_example_manifest_and_rhai_compile() {
        let manifest: ScriptManifest = toml::from_str(include_str!(
            "../../../examples/scripts/reticle_ray/manifest.toml"
        ))
        .expect("reticle_ray manifest should parse");
        let id = ScriptId::parse("reticle_ray").expect("example ID should be valid");
        let prepared = PreparedScript::compile_package(
            0,
            id,
            manifest,
            include_str!("../../../examples/scripts/reticle_ray/main.rhai"),
            true,
            RhaiEngineFactory,
        )
        .expect("reticle_ray should compile against the current script API");
        assert_eq!(prepared.manifest().id, "reticle_ray");
    }

    #[test]
    fn debug_spawn_menu_example_manifest_and_rhai_compile() {
        let manifest: ScriptManifest = toml::from_str(include_str!(
            "../../../examples/scripts/debug_spawn_menu/manifest.toml"
        ))
        .expect("debug_spawn_menu manifest should parse");
        let id = ScriptId::parse("debug_spawn_menu").expect("example ID should be valid");
        let prepared = PreparedScript::compile_package(
            0,
            id,
            manifest,
            include_str!("../../../examples/scripts/debug_spawn_menu/main.rhai"),
            true,
            RhaiEngineFactory,
        )
        .expect("debug_spawn_menu should compile against the current script API");
        assert_eq!(prepared.manifest().id, "debug_spawn_menu");
    }

    #[test]
    fn spawn_placement_lab_example_manifest_and_rhai_compile() {
        let manifest: ScriptManifest = toml::from_str(include_str!(
            "../../../examples/scripts/spawn_placement_lab/manifest.toml"
        ))
        .expect("spawn_placement_lab manifest should parse");
        let id = ScriptId::parse("spawn_placement_lab").expect("example ID should be valid");
        let prepared = PreparedScript::compile_package(
            0,
            id,
            manifest,
            include_str!("../../../examples/scripts/spawn_placement_lab/main.rhai"),
            true,
            RhaiEngineFactory,
        )
        .expect("spawn_placement_lab should compile against the current script API");
        assert_eq!(prepared.manifest().id, "spawn_placement_lab");
    }
}

/// Compiled package candidate ready for callback-boundary activation.
///
/// Only the constructors can create a candidate with a validated manifest and
/// bounded source. Package top-level statements are compiled but never run;
/// callable functions must be explicitly registered by the manifest.
///
/// See [`ScriptManifest`] for the package contract and [`super::manager::ScriptManager`]
/// for activation and generation replacement behavior.
pub struct PreparedScript {
    request_id: u64,
    manifest: ScriptManifest,
    ast: AST,
    dependencies: Vec<PreparedDependency>,
    replace: bool,
    id: ScriptId,
}

pub(crate) struct PreparedDependency {
    id: ScriptId,
    manifest: ScriptManifest,
    ast: AST,
}

impl PreparedDependency {
    pub(crate) fn into_parts(
        self,
    ) -> (
        ScriptId,
        AST,
        BTreeMap<ScriptPhaseId, String>,
        Option<String>,
        Vec<String>,
        PackageVersion,
        Vec<PackageRequirement>,
        Vec<PackageConflict>,
        Option<WorkbenchTabMetadata>,
    ) {
        let (callbacks, on_run) = callback_parts(&self.manifest);
        (
            self.id,
            self.ast,
            callbacks,
            on_run,
            self.manifest.capabilities,
            self.manifest.version,
            self.manifest.requires,
            self.manifest.conflicts,
            self.manifest.workbench_tab,
        )
    }
}

impl PreparedScript {
    /// Compile a source string against API v1 and construct an in-memory package.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid callback declarations, unsupported or
    /// duplicate capabilities, oversized source, or Rhai compilation failure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::collections::BTreeMap;
    /// use madmax_bootstrap::api::scripts::{PreparedScript, ScriptId, ScriptPhaseId};
    ///
    /// let mut callbacks = BTreeMap::new();
    /// callbacks.insert(ScriptPhaseId::GamePostSim, "on_post_sim".to_owned());
    /// let id = ScriptId::parse("example").expect("valid example ID");
    /// let package = PreparedScript::compile(
    ///     id,
    ///     "fn on_post_sim(ctx) {}",
    ///     callbacks,
    ///     None,
    ///     vec!["game.read".to_owned()],
    /// ).expect("valid example package");
    /// let _ = package;
    /// ```
    pub fn compile(
        id: ScriptId,
        source: &str,
        callbacks: BTreeMap<ScriptPhaseId, String>,
        on_run: Option<String>,
        capabilities: Vec<String>,
    ) -> Result<Self, String> {
        let mut declared: BTreeMap<String, String> = callbacks
            .into_iter()
            .map(|(phase, function)| (phase.as_str().to_owned(), function))
            .collect();
        if let Some(function) = on_run {
            declared.insert("on_run".into(), function);
        }
        let manifest = ScriptManifest {
            id: id.as_str().to_owned(),
            api_version: API_VERSION,
            callbacks: declared,
            capabilities,
            version: PackageVersion::default(),
            sources: default_package_sources(),
            requires: Vec::new(),
            conflicts: Vec::new(),
            workbench_tab: None,
        };
        Self::compile_package(0, id, manifest, source, true, RhaiEngineFactory)
    }
    pub(crate) fn compile_package(
        request_id: u64,
        id: ScriptId,
        manifest: ScriptManifest,
        source: &str,
        replace: bool,
        factory: RhaiEngineFactory,
    ) -> Result<Self, String> {
        let source = PackageSource {
            path: "main.rhai".to_owned(),
            contents: source.to_owned(),
        };
        Self::compile_package_sources(request_id, id, manifest, &[source], replace, factory)
    }

    pub(crate) fn compile_package_sources(
        request_id: u64,
        id: ScriptId,
        manifest: ScriptManifest,
        sources: &[PackageSource],
        replace: bool,
        factory: RhaiEngineFactory,
    ) -> Result<Self, String> {
        manifest.validate(&id)?;
        if sources.len() != manifest.sources.len()
            || sources
                .iter()
                .map(|source| source.path.as_str())
                .ne(manifest.sources.iter().map(String::as_str))
        {
            return Err("prepared source list does not match manifest".into());
        }
        let total_bytes = sources
            .iter()
            .try_fold(0usize, |total, source| {
                (source.contents.len() <= MAX_SOURCE_BYTES)
                    .then(|| total.checked_add(source.contents.len()))
                    .flatten()
            })
            .ok_or("package source byte limit exceeded")?;
        if total_bytes > MAX_SOURCE_BYTES {
            return Err("package source byte limit exceeded".into());
        }
        let (combined, line_map) = combine_sources(sources);
        let ast = factory.create().compile(&combined).map_err(|error| {
            let line = error.position().line().unwrap_or(0);
            let location = line_map
                .iter()
                .rev()
                .find(|(start, end, _)| line >= *start && line <= *end);
            if let Some((start, _, path)) = location {
                let filename = path.as_str();
                bounded_text(
                    &format!("{filename}:{}: {error}", line.saturating_sub(*start) + 1),
                    MAX_EVENT_BYTES,
                )
            } else {
                bounded_text(&error.to_string(), MAX_EVENT_BYTES)
            }
        })?;
        for function in manifest.callbacks.values() {
            if !ast
                .iter_functions()
                .any(|f| f.name == function && f.params.len() == 1)
            {
                return Err(format!("missing one-argument callback {function}"));
            }
        }
        Ok(Self {
            request_id,
            manifest,
            ast,
            dependencies: Vec::new(),
            replace,
            id,
        })
    }

    pub(crate) fn set_dependencies(&mut self, dependencies: Vec<PreparedDependency>) {
        self.dependencies = dependencies;
    }

    pub(crate) fn into_dependency(self) -> PreparedDependency {
        PreparedDependency {
            id: self.id,
            manifest: self.manifest,
            ast: self.ast,
        }
    }
    /// Return the request identifier associated with this prepared package.
    pub fn request_id(&self) -> u64 {
        self.request_id
    }
    /// Return the validated package manifest.
    pub fn manifest(&self) -> &ScriptManifest {
        &self.manifest
    }
    pub(crate) fn into_parts(
        self,
    ) -> (
        ScriptId,
        AST,
        BTreeMap<ScriptPhaseId, String>,
        Option<String>,
        Vec<String>,
        PackageVersion,
        Vec<PackageRequirement>,
        Vec<PackageConflict>,
        Option<WorkbenchTabMetadata>,
        Vec<PreparedDependency>,
        bool,
    ) {
        let (callbacks, on_run) = callback_parts(&self.manifest);
        (
            self.id,
            self.ast,
            callbacks,
            on_run,
            self.manifest.capabilities,
            self.manifest.version,
            self.manifest.requires,
            self.manifest.conflicts,
            self.manifest.workbench_tab,
            self.dependencies,
            self.replace,
        )
    }
}

fn callback_parts(manifest: &ScriptManifest) -> (BTreeMap<ScriptPhaseId, String>, Option<String>) {
    let mut callbacks = BTreeMap::new();
    if let Some(function) = manifest.callbacks.get("game.post_sim") {
        callbacks.insert(ScriptPhaseId::GamePostSim, function.clone());
    }
    (callbacks, manifest.callbacks.get("on_run").cloned())
}

/// One validated, owned source file transferred to the package compiler.
#[derive(Debug, Clone)]
pub(crate) struct PackageSource {
    pub path: String,
    pub contents: String,
}

fn combine_sources(sources: &[PackageSource]) -> (String, Vec<(usize, usize, String)>) {
    let mut combined = String::new();
    let mut lines = Vec::with_capacity(sources.len());
    let mut next_line = 1usize;
    for source in sources {
        let count = source.contents.lines().count().max(1);
        let start = next_line;
        combined.push_str(&source.contents);
        if !source.contents.ends_with('\n') {
            combined.push('\n');
        }
        let end = start + count - 1;
        lines.push((start, end, source.path.clone()));
        next_line = end + 1;
    }
    (combined, lines)
}
