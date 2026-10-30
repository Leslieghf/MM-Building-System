//! Rhai engine limits, sandbox configuration, and package compilation interface.
use rhai::Engine;
pub const MAX_OPERATIONS: u64 = 20_000;
pub const MAX_CALL_LEVELS: usize = 16;
pub const MAX_EXPRESSION_DEPTH: usize = 32;
pub const MAX_FUNCTION_EXPRESSION_DEPTH: usize = 32;
pub const MAX_VARIABLES: usize = 128;
pub const MAX_STRING_BYTES: usize = 4096;
pub const MAX_ARRAY_ITEMS: usize = 256;
pub const MAX_MAP_ITEMS: usize = 128;
pub const MAX_FUNCTIONS: usize = 64;
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
#[derive(Debug, Clone, Copy, Default)]
pub struct RhaiEngineFactory;
impl RhaiEngineFactory {
    pub fn create(self) -> Engine {
        Self::build()
    }
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
