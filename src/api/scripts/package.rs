//! Script package identity, manifest validation, and compiled payload.
//!
//! # Package contract
//!
//! A package directory contains a manifest and its declared Rhai entry file.
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parsed `manifest.toml` contract for a Rhai package.
///
/// # Example
///
/// ```toml
/// id = "vehicle_watch"
/// api_version = 1
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
}
impl ScriptManifest {
    pub(crate) fn validate(&self, id: &ScriptId) -> Result<(), String> {
        if ScriptId::parse(self.id.clone()).as_ref() != Ok(id) {
            return Err("manifest ID does not match requested package".into());
        }
        if self.api_version != API_VERSION {
            return Err("unsupported script API version".into());
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
        }
    }
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
    replace: bool,
    id: ScriptId,
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
        manifest.validate(&id)?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err("source byte limit exceeded".into());
        }
        let ast = factory
            .create()
            .compile(source)
            .map_err(|e| bounded_text(&e.to_string(), MAX_EVENT_BYTES))?;
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
            replace,
            id,
        })
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
        bool,
    ) {
        let mut callbacks = BTreeMap::new();
        if let Some(function) = self.manifest.callbacks.get("game.post_sim") {
            callbacks.insert(ScriptPhaseId::GamePostSim, function.clone());
        }
        let on_run = self.manifest.callbacks.get("on_run").cloned();
        (
            self.id,
            self.ast,
            callbacks,
            on_run,
            self.manifest.capabilities,
            self.replace,
        )
    }
}
