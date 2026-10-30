# Script Coordinate and Vector API Design

## Intent

Make MadMaxModding scripts use a consistent, useful spatial API. The script world follows the same axes as Bevy: **+X right, +Y up, -Z forward**. Add practical vector and direction operations so scripts can work with game positions, movement, and gizmos without hand-written math at every call site.

## Current state

- Mad Max `CCharacter` and `CVehicle` transforms are copied as row-major `Matrix4x4` values with translation in row 3.
- In-game `CDebugRenderer` comparison showed actor transform row 0 points right, row 1 points up, and row 2 points backward. The working actor/vehicle forward vector is the negated horizontal projection of row 2.
- Under Bevy's convention, that observed basis is already coherent: +Z is backward and -Z is forward. The blue +Z axis pointing opposite the green forward arrow is expected; the purple -Z arrow is forward.
- `ctx.player` and `ctx.vehicle` expose positions, Euler rotations, transforms, vectors, and derived horizontal forward vectors as Rhai maps/arrays. `ctx.gizmos` takes the same world coordinates and matrices. Vehicle velocity writes send a script vector back to the native setter.
- There is no general vector arithmetic API; scripts must currently implement common operations themselves.

## Coordinate contract

- Script-visible positions, vectors, transforms, and gizmo coordinates use Mad Max's world values directly, with +X right, +Y up, and -Z forward.
- Bevy-compatible `forward` means -Z in an identity-aligned frame. The observed actor/vehicle forward vector remains the normalized horizontal negative of transform row 2.
- No Z reflection is added. Reads, gizmo submissions, and vehicle velocity writes keep their current numeric coordinates.
- Matrix arrays remain row-major, with translation at indices 12–14. Existing Rhai map and array shapes remain compatible.
- Camera ray internals remain native and are not changed by this API work.

## Script vector and direction support

Add a small, consistent `ctx.math` surface over the existing `{x, y, z}` vector maps:

- `ctx.math.vec3(x, y, z)` constructs a vector map.
- `ctx.math.add(a, b)`, `subtract(a, b)`, and `scale(v, scalar)` return vector maps.
- `ctx.math.dot(a, b)`, `cross(a, b)`, `length(v)`, and `distance(a, b)` return numbers or vectors as appropriate.
- `ctx.math.normalize(v)` returns a unit vector or `()` for zero/invalid input.
- `ctx.math.direction(from, to)` returns the normalized vector from `from` toward `to`, or `()` when the points coincide or are invalid.
- `ctx.math.lerp(a, b, t)` performs component-wise interpolation; `t` must be finite.

These are ordinary vector operations; they do not silently flatten vectors or assume that a vector is a position, velocity, or direction. Existing `ctx.player.forward` and `ctx.vehicle.forward` continue to mean normalized horizontal forward in game/Bevy coordinates. This slice does not add a second direction type or change script values to a custom Rhai type.

## Validation and failure behavior

- Every vector input must be a map with exactly numeric `x`, `y`, and `z` components. Reject NaN, infinity, and components outside the existing spatial range. Scalar math inputs and scalar results must be finite; they do not use the spatial component bound. Vector-valued results remain subject to the component bound.
- All operations return fresh vector maps and do not mutate their arguments.
- Normalize and direction return `()` for near-zero magnitude rather than NaN or an invented direction.
- Cross product follows the right-handed +X/+Y/+Z algebra used by Bevy math. This is compatible with Bevy's axis convention; forward remains -Z.
- `lerp` accepts finite `t` in the documented range [0,1].

## Documentation and compatibility

- Generated `API.md` states the axis convention and describes vector helpers and zero-vector behavior.
- Existing script fields, vector maps, transform arrays, and gizmo call signatures do not change.
- The API version stays unchanged because this adds methods without changing existing values or behavior.
- The `debug_gizmo` and `debug_orientation` packages remain valid and continue to use the same world coordinates. Orientation axes display +Z as backward and -Z as forward.

## Non-goals

- Converting Mad Max world coordinates or FFI data.
- Adding camera raycasts, spawning, entity handles, or physics APIs.
- Replacing Rhai maps with a custom `Vec3` or `Dir3` type.
- Changing gizmo rendering internals or the native game's transform convention.

## Verification

- Unit-test vector construction, add/subtract/scale, dot/cross, length/distance, normalization, direction, and interpolation.
- Cover zero vectors, coincident points, malformed maps, non-finite values, and range failures.
- Check orientation math: right × up = backward (+Z), and the actor's `forward` value is -Z for an identity-aligned observed actor basis.
- Verify existing script context fields, velocity writes, and gizmo submissions preserve their prior numeric coordinates.
- Regenerate `API.md`, run the Rust test suite and release build, then verify the orientation and gizmo packages in-game.

## Decisions

- Coordinate contract: Bevy-compatible axes, -Z forward; no runtime conversion.
- Script representation: keep ordinary `{x,y,z}` maps and arrays for backward compatibility; add explicit stateless vector operations under `ctx.math`.
- Direction representation: return normalized vector maps, with `()` when no direction exists.
- Scalar validation ruling: keep the ±100000 range for vector components and vector-valued results, while finite scalar values such as dot products and scale factors may exceed that range; lerp `t` remains in [0,1].
