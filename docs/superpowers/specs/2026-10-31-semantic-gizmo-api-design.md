# Stable Gizmo API Design

## Intent

Keep a concise script-facing gizmo API containing each useful, visually distinct output once. Script authors should choose by appearance and behavior; the game-facing ABI remains native-oriented and documented separately.

## Retained script API

- `axes(position, length)` and `transformed_axes(transform, length)` — RGB basis markers, either world-aligned or using a supplied transform.
- `line(start, end, color)` and `arrow(start, end, color)` — one-frame line and native arrow.
- `wire_sphere(center, radius, color)` — radial wire sphere.
- `xray_sphere(center, radius, color)` — the through-wall variant of the radial wire sphere.
- `surface_wire_sphere(center, radius, color)` — surface-following wire rings.
- `wire_aabb(minimum, maximum, color)` — world-axis-aligned wire box.
- `wire_box(transform, minimum, maximum, color)` — transformed wire box.
- `xray_wire_box(transform, minimum, maximum, color)` — transformed through-wall wire box.
- `solid_cube(transform, minimum, maximum, color)` — observed solid cube, submitted through native slot 36 (Havok sphere shape type).
- `solid_arrow(transform, minimum, maximum, color)` — observed solid 3D arrow, submitted through native slot 38 (Havok capsule shape type).

Slots 37 and 39 produced the same respective solid cube and solid arrow visuals as slots 36 and 38, so the duplicate routes are removed from script execution. Old native-mirroring Rhai names are also removed instead of retained as aliases. The `debug_gizmo` example will use one call for each retained visual and remove overlapping duplicate markers.

## Layer boundary

- **Game ABI:** retain exact signatures and mappings for the methods still called. Keep unexposed native mappings in the research note; do not delete useful reverse-engineering knowledge.
- **Rhai API:** register only the semantic names above; validation, catalogs, help/API docs, and script examples use the same terminology.
- **Native dispatch:** slots 36 and 38 are the only retained bounds-shape paths. Slots 37 and 39 are no longer validated or invoked by the runtime.

## Approaches considered

1. **Keep every call and relabel aliases:** rejected because duplicate visuals and old names keep the API noisy.
2. **Remove all bounds-shape calls:** rejected because the user wants to keep the solid cube and solid 3D arrow.
3. **Keep one proven native path per distinct visible solid form:** chosen. It preserves both requested forms and avoids presenting duplicate or confusing variants.

## Constraints and uncertainty

The solid names reflect confirmed in-game appearance, while the underlying type IDs are Havok sphere and capsule. Document both facts so scripts are predictable and the native mapping is not hidden. The other native bounds routes remain in `docs/research/2026-10-30-cdebug-renderer-api.md` but are not available through Rhai. Existing limits and call timing remain unchanged.

## Acceptance criteria

- Rhai exposes only the semantic API listed above; old aliases and the duplicate slots 37/39 are absent from the live dispatch path.
- Generated `docs/rhai-api.md` documents the solid appearance and corresponding native type for slots 36 and 38.
- The maintained gizmo example demonstrates each retained visual without repeating equivalent markers.
- Research notes retain all four original native shape mappings.
