# Script Coordinate and Vector API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep script coordinates in the game's Bevy-compatible +X-right, +Y-up, -Z-forward convention and add safe, reusable vector operations to the Rhai API.

**Architecture:** Preserve current vector maps and transform arrays. Add a stateless `ctx.math` host object with bounded operations over `{x,y,z}` maps, register it in the v1 context, and add the new signatures and coordinate convention to the generated API catalog.

**Tech Stack:** Rust 2024, Rhai 1.27, existing unit-test setup, `build.rs` generated `API.md`.

**Spec:** `docs/superpowers/specs/2026-10-30-script-coordinate-space-design.md`

## Global Constraints

- Script values use +X right, +Y up, and -Z forward; no Z reflection is performed.
- Existing `{x,y,z}` map and 16-number row-major transform shapes remain compatible.
- Vector inputs must contain exactly numeric `x`, `y`, and `z` components, finite and in `[-100000,100000]`.
- Invalid or zero-length normalization/direction returns `()` where specified; it must not produce NaN or an invented direction.
- All vector operations are pure and return fresh maps; they do not mutate arguments.
- The script API version remains 1 because existing API values and behavior do not change.

## Review Focus

- Malformed maps, missing/extra components, and non-numeric components fail with a clear Rhai error; test parser rejection.
- NaN, infinity, and out-of-range components fail; test all three invalid classes.
- Zero and near-zero normalization/direction return `()`; test exact zero and the chosen tolerance boundary.
- Vector operations that produce components outside the supported spatial range fail instead of returning unusable maps; test add/scale/cross overflow cases.
- Bevy forward remains -Z while cross product follows right-handed axis math; test `X × Y = +Z` and identity-aligned observed actor forward = -Z.

---

### Task 1: Implement bounded vector math and tests

**Files:**
- Create: `src/scripting/vector.rs`
- Modify: `src/scripting/mod.rs`

**Interfaces:**
- Produces `pub(crate) struct ScriptMath` and Rhai methods `vec3(x, y, z)`, `add(a, b)`, `subtract(a, b)`, `scale(v, scalar)`, `dot(a, b)`, `cross(a, b)`, `length(v)`, `distance(a, b)`, `normalize(v)`, `direction(from, to)`, and `lerp(a, b, t)`.
- Vector arguments and vector results use Rhai `Map` values with exactly `x`, `y`, and `z` numeric components. Scalar methods accept `rhai::FLOAT` and return `rhai::FLOAT`.
- Vector components and vector-valued results remain finite and bounded to `[-100000,100000]`. Other scalar inputs/results must be finite but have no spatial bound; `t` remains in `[0,1]`. Near-zero magnitude threshold for normalize/direction is `1e-12`.

- [ ] Write unit tests for each operation, right-handed cross product, fresh-map behavior, lerp endpoints, zero/near-zero cases, malformed maps, non-finite values, component limits, and output overflow.
- [ ] Run `cargo test --lib scripting::vector::tests` and confirm the new tests fail because `ScriptMath` is not implemented.
- [ ] Implement `ScriptMath` operations and common vector parsing/validation in `src/scripting/vector.rs`; expose the module from `src/scripting/mod.rs`.
- [ ] Run `cargo test --lib scripting::vector::tests` and confirm the tests pass.

### Task 2: Register `ctx.math`, document the API, and verify integration

**Files:**
- Modify: `src/scripting/api.rs`
- Modify: `src/scripting/api_catalog.rs`
- Generated: `API.md`

**Interfaces:**
- Consumes `ScriptMath` from Task 1.
- Produces `ctx.math.<method>(...)` in `on_post_sim` and `on_run` callbacks. The math object is stateless and requires no capability.

- [ ] Add a Rhai integration test that builds the real v1 engine/context and evaluates vector construction, addition, direction, and normalization through `ctx.math`; assert that a map scaled from the native forward direction remains negative Z.
- [ ] Run `cargo test --lib scripting::api::` and confirm the new integration test fails because `ctx.math` is not registered.
- [ ] Register `ScriptMath` as `MadMaxMath`, add the `ctx.math` getter, and register each named operation as a method on the math object.
- [ ] Update the generated API catalog with the coordinate convention, `ctx.math` value, all signatures, input/output limits, and `()` behavior for zero-length results.
- [ ] Run `cargo test --lib scripting::api::` and confirm the integration tests pass; run `cargo fmt --check` and `cargo build --release`; confirm `API.md` contains the new entries.

## Completion

- Preserve all existing uncommitted work in the checkout; stage or commit only this feature if a commit is requested separately.
- Do not install or deploy an ASI as part of this plan. In-game checks require the user to install/run the build after reviewing it.
