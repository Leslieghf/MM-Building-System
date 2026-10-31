//! Rhai resource limits, sandbox policy, and package compilation interface.
//!
//! # Security boundary
//!
//! The engine registers only the host-provided context and pure helpers. It
//! applies evaluation, operation, nesting, and data-size limits and does not
//! enable arbitrary filesystem, process, network, FFI, or dynamic-module
//! access. Package source is compiled on the loader worker, then invoked by the
//! callback-bound [`super::manager`].
use rhai::Engine;
/// Maximum Rhai operations per callback evaluation.
pub const MAX_OPERATIONS: u64 = 20_000;
/// Maximum nested Rhai function-call depth.
pub const MAX_CALL_LEVELS: usize = 16;
/// Maximum expression nesting depth.
pub const MAX_EXPRESSION_DEPTH: usize = 32;
/// Maximum function-expression nesting depth.
pub const MAX_FUNCTION_EXPRESSION_DEPTH: usize = 32;
/// Maximum variables in a compiled callback scope.
pub const MAX_VARIABLES: usize = 128;
/// Maximum bytes in one script string value.
pub const MAX_STRING_BYTES: usize = 4096;
/// Maximum items in one script array.
pub const MAX_ARRAY_ITEMS: usize = 256;
/// Maximum entries in one script map.
pub const MAX_MAP_ITEMS: usize = 128;
/// Maximum functions in one package source.
pub const MAX_FUNCTIONS: usize = 64;
/// Maximum Rhai source bytes accepted by package compilation.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
#[derive(Debug, Clone, Copy, Default)]
/// Factory for the host's constrained [`Engine`] configuration.
///
/// Limits are declared by the `MAX_*` constants in this module. The factory
/// installs no filesystem, process, or native-call functions.
pub struct RhaiEngineFactory;
impl RhaiEngineFactory {
    /// Create an engine with the host's API v1 limits and registered functions.
    pub fn create(self) -> Engine {
        Self::build()
    }
    /// Build an engine without owning a factory value.
    pub fn build() -> Engine {
        let mut engine = Engine::new();
        engine
            .set_max_operations(MAX_OPERATIONS)
            .set_max_call_levels(MAX_CALL_LEVELS)
            .set_max_expr_depths(MAX_EXPRESSION_DEPTH, MAX_FUNCTION_EXPRESSION_DEPTH)
            .set_max_variables(MAX_VARIABLES)
            .set_max_functions(MAX_FUNCTIONS)
            .set_max_string_size(MAX_STRING_BYTES)
            .set_max_array_size(MAX_ARRAY_ITEMS)
            .set_max_map_size(MAX_MAP_ITEMS);
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});
        crate::api::rhai::register_v1(&mut engine);
        engine
    }
}
