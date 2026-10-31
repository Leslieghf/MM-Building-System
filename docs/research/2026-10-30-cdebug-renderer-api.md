# CDebugRenderer facade API

Static mappings from the supported Steam executable and the `Live_CDdebugRenderer_Methods_20261030` / `Live_NGraphicsEngine_PrimitiveRendererImpl_20261030` Ghidra overlays. The methods below describe call shapes; they do not by themselves prove that the active backend renders queued primitives.

## Acquisition and dispatch

- Image base: `0x140000000` for the supported executable.
- Facade singleton pointer global: RVA `0x017CF0E8` (`0x1417CF0E8` at the observed image base).
- The static object pointed to by that global is at `0x1417CF190`. Its facade vtable is at `+0x00`, a secondary vtable-like pointer is at `+0x08`, and the selected backend pointer is at `+0x10`.
- Facade vtable: RVA `0x01346938` (`0x141346938`). Its wrappers forward through the backend pointer at `+0x10`.
- `SetCDebugRendererBackend(facade, backend)` selects a supplied backend, or restores a static fallback backend when passed null. The captured overlays do not establish which backend is active after game initialization.

## Confirmed line and arrow calls

All signatures are Windows x64 call shapes. `Vector3` is passed as a pointer to three floats. Each color pointer addresses four floats; RGBA channel meaning is inferred from the renderer helper names and use.

| Facade slot | Method | Call shape | Notes |
|---|---|---|---|
| 1 | `ProcessCDebugRendererEndpointColoredLineSlot01` | `(facade, start, end, startColor, endColor)` | Forwards to backend slot 1. |
| 2 | `ProcessCDebugRendererTwoPointLineSlot02` | `(facade, start, end, colorA, colorB)` | Alternate two-point line queue. |
| 3 | `ProcessCDebugRendererSecondaryEndpointColoredLineSlot03` | `(facade, start, end, startColor, endColor)` | Forwards to a distinct queue route. |
| 9 | `ProcessCDebugRendererArrowSlot09` | `(facade, start, end, shaftColor, headColor)` | Automatic width; the primitive builder uses `length * 0.075` as its width basis. |
| 11 | `ProcessCDebugRendererArrowSlot11` | `(facade, start, end, width, shaftColor, headColor)` | Explicit width. |

The primitive implementation queues records under different renderer offsets for the line variants and arrow. Queue consumption was not located in the captured overlays. The vtable and wrapper code prove these entry points exist and forward; they do not prove runtime visibility.

## Primitive backend details from the 2026-10-30 Ghidra overlays

- `ProcessCDebugRendererAxesSlot25` at overlay address `Live_CDdebugRenderer_Methods_20261030::0x14297A940` dispatches to backend vtable slot 25 (`+0xC8`) with `(facade, transform4x4, axisLength)`.
- `DrawPrimitiveRendererAxes` at `Live_NGraphicsEngine_PrimitiveRendererImpl_20261030::0x1428B79C0` reads the transform translation from float indices 12–14 and the three axis basis vectors from indices 0–2, 4–6, and 8–10. It submits three axis lines through `AppendColoredLineVertices`; the built-in colors are red, green, and blue. Its transform contract matches the existing row-major `Matrix4x4` translation layout.
- `ProcessPrimitiveRendererEndpointColoredLineSlot01` forwards to backend slot 1 (`+0x08`) and calls `AppendColoredLineVertices`. That helper enters the renderer synchronization object at `renderer+0x10`, appends two 0xE4-byte records to the queue vector at `renderer+0x630`, and leaves the synchronization object.
- `BuildPrimitiveRendererArrowGeometry` appends its generated shaft/head segments to a different queue vector at `renderer+0x1610`, also under the renderer synchronization object.
- These methods are queue submission paths. The overlays still do not show which render pass consumes the queues, so a valid submission is not yet equivalent to a visible gizmo.

## Current probe

The first installed diagnostic command, `debug_renderer_test`, submitted a short tick and auto-width arrow on the sampled camera ray; the user reported seeing no marker. That result disproves the assumption that successfully identifying and invoking the facade is enough to establish visible output.

The replacement experiment removes that one-off command and exposes an explicitly experimental `debug.draw` Rhai capability: `ctx.gizmos.axes(position, length)`, `ctx.gizmos.line(start, end, rgba)`, and `ctx.gizmos.arrow(start, end, rgba)`. A discoverable `debug_gizmo` example draws a 40-unit RGB axes marker and long colored lines two units above the copied player position, plus a short yellow arrow. The native adapter verifies the exact facade wrapper targets and the expected primitive-renderer backend targets before submission. Its first accepted batch is logged once to `MadMaxModding/bootstrap.log`, distinguishing a script/native submission failure from a missing visual result. The user confirmed on 2026-10-30 that the scripted axes, lines, and arrow render in-game. The automatic-width arrow initially looked extremely broad; the example endpoint has since been shortened to four world units. This confirms the visible primitive path, but does not establish persistent lifetime or the queue-consumer details.

## Additional mapped geometry paths (2026-10-30)

Ghidra overlays identify several more facade slots and their concrete primitive builders:

| Facade slot | Name | Observed call shape | What the implementation does |
|---|---|---|---|
| 26 | `ProcessCDebugRendererSphereSlot26` | `(facade, center, radius, rgba)` | Backend slot `+0xD0`; `DrawPrimitiveRendererSparseSphere` emits a low-density wire sphere as line segments. |
| 27 | `ProcessCDebugRendererSparseSphereRecordsSlot27` | `(facade, center, radius, rgba)` | Backend slot `+0xD8`; disassembly shows it forwards center/radius unchanged and copies the color, matching the sphere wrapper ABI. |
| 28 | `ProcessCDebugRendererTessellatedSphereSlot28` | `(facade, center, radius, rgba)` | Backend slot `+0xE0`; `DrawPrimitiveRendererTessellatedSphere` generates three circular wire rings at 18-degree increments (about 60 line segments per call). |
| 31 | `ProcessCDebugRendererAabbSlot31` | `(facade, minimum, maximum, rgba)` | Backend slot `+0xF8`; `DrawPrimitiveRendererAabb` submits the 12 edges of an axis-aligned box. |
| 34 | `ProcessCDebugRendererTransformedAabbSlot34` | `(facade, transform4x4, minimum, maximum, rgba)` | Backend slot `+0x110`; transforms the eight box corners, then submits the 12 edges. |
| 35 | `ProcessCDebugRendererTransformedAabbQueueSlot35` | `(facade, transform4x4, minimum, maximum, rgba)` | Separate queued route at backend slot `+0x118`; implementation submits the same 12 transformed edges to the secondary endpoint-colored-line queue. |
| 36–39 | bounds shape slots | `(facade, transformArg, minimum, maximum, rgba)` | Queue transformed debug-shape records for sphere (type 0), cylinder (type 1), capsule (type 4), and convex-vertices proxy (type 5), respectively. The call path builds a scale/center transform from min/max and combines it with `transformArg`. |

The low-density sphere and AABB helpers have direct, typed call shapes and are exposed alongside the tessellated sphere. The tessellated variant expands to about 60 line segments, so the script API charges it 60 units against a bounded per-invocation and per-callback geometry budget. Slots 34–35 accept the same 16-float transform layout already used by slot 25 axes. Shape slots 36–39 are also exposed experimentally: their wrappers declare an `undefined8` transform argument, and the builder composes that argument with local bounds. The Rhai adapter passes a pointer to a 16-float transform, but no game callsite has yet confirmed that concrete type. Their render behavior and lifetime need user testing.

The `debug_gizmo` example calls every mapped draw path together, placing shapes in rows above Max for comparison. It covers the three wire-sphere variants, direct and transformed boxes, and the four experimental bounds-shape slots. This scan found wire spheres and wire boxes, plus shape-bound proxy paths. It did **not** find evidence in the inspected `CDebugRenderer` methods for arbitrary mesh vertices or a general wire-mesh submission API. Slot 39 accepts bounds and a transform, not a vertex/index buffer; it is not evidence of a user-supplied model mesh API.

The renderer-side helper `ProcessPrimitiveRendererDebugShape` stores a shape type, 8-bit RGBA, and a copied 4x4 transform in an internal record while holding the renderer synchronization object. The exact consumer/render pass and practical capacity remain unverified.

### Bounds-shape type IDs cross-check (2026-10-31)

The four IDs passed by the bounds-shape builders are not arbitrary labels: they match Havok's `hkcdShapeType::ShapeTypeEnum` values exactly — `SPHERE = 0`, `CYLINDER = 1`, `CAPSULE = 4`, and `CONVEX_VERTICES = 5`. The same enum values and names appear in a public source mirror of Havok's `hkcdShape.h` ([source](https://codeviewer.zeldamods.org/uking/uking/lib/hkStubs/Havok/Geometry/Collide/Shapes/hkcdShape.h.html)). This external match corroborates the interpretation of the Mad Max builders' type IDs as requested Havok shape kinds. It does not prove that Mad Max uses the same Havok revision, nor does it identify the queue consumer.

**API selection:** the user chose visible, stable names for distinct output and asked to drop duplicate variants. Rhai therefore retains slot 36 as `solid_cube` and slot 38 as `solid_arrow`; slots 37 and 39 are visually redundant and are omitted from runtime dispatch. These names describe observed visuals, while the type-ID mapping above records that the game requests Havok sphere/capsule shapes. Keep the mismatch explicit in API docs. The consumer/transform cause remains unresolved. Slot 39 accepts bounds rather than a vertex buffer, so none of these calls imply arbitrary user-supplied convex meshes.

## Character and vehicle transform basis investigation (2026-10-30)

Ghidra's MSVC RTTI hierarchy descriptors confirm that both `CCharacter` and `CVehicle` have a `CGameObject` base subobject at byte offset `+8`:

- `CCharacter` type descriptor `0x141797910`, hierarchy `0x1414B1020`; its `CGameObject` base descriptor reports PMD member displacement `8`. The `CGameObject` subobject vftable address point is `0x141251188`, with slot 22 targeting `0x141C58BB0`.
- `CVehicle` type descriptor `0x1417B7928`, hierarchy `0x1414DA4C0`; its `CGameObject` base descriptor also reports PMD member displacement `8`. The subobject vftable address point is `0x1412D8868`, with slot 22 targeting `0x1421A15D0`.

The pinned SDK declaration in `game/go/go.h` identifies the last `CGameObject` virtual as `bool GetTransform(CMatrix*)`. Counting the inherited `CRtti` destructor entry followed by the 22 declared `CGameObject` virtuals places `GetTransform` at zero-based vtable slot 22. This matches both RTTI-derived vtable targets above, so slot 22 is now mapped to `CGameObject::GetTransform` for these base subobjects. The observed `+8` adjustment is still required before dispatch. The two target addresses lie in `.xpdata`; their static Ghidra instruction streams are invalid, so the method name and dispatch slot are mapped but the native bodies and returned-transform behavior remain unverified.

This confirms the `+8` adjustment used to reach the `CGameObject` subobject for both classes. The distinct slot-22 targets agree with class-specific overrides, but the supported executable's `.xpdata` bytes at those targets do not statically decompile into trustworthy bodies, so this does not by itself prove the virtual's runtime behavior or matrix convention.

The pinned community SDK revision defines `CMatrix4f::Right()` from row 0, `Forward()` from row 1, `Up()` from row 2, and `Position()` from row 3 (`math3d.h`, revision `f2dc640769c88aae9c3965a2aca0aa138f0e62ab`). The captured `DrawPrimitiveRendererAxes` implementation likewise draws the three transform rows and reads translation from elements 12–14. In-game visual comparison of both `CCharacter` and `CVehicle` transforms shows that the actor transform semantics differ from those generic SDK labels: row 0 is right, row 1 is up, and row 2 is backward. Therefore actor/vehicle visible forward is the **negative horizontal projection of row 2** (x/z components, with world Y removed). The camera matrix path remains separate and is not changed by this actor-transform mapping.

The `debug_orientation` diagnostic package exposes each actor's copied 16-float transform and submits it to the existing native RGB axes renderer. Red marks row 0, green row 1, and blue row 2; player axes are 4 units long and vehicle axes 8 units long. Its first explicit `ctx.*.forward` and negated-forward arrows were based on the old row-1 projection and appeared erratic in-game. That behavior is consistent with row 1 being vertical and its XZ projection being near zero; normalizing that noise caused the direction vectors to wander. The new row-2-derived actor-forward implementation still needs in-game verification.
