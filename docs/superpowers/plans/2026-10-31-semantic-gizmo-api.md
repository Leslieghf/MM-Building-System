# Stable Gizmo API Implementation Plan

**Goal:** Keep each useful, visually distinct gizmo once under a semantic Rhai name.

**Public surface:** RGB axes (position and transformed forms), line, arrow, three wire sphere modes, three wire box modes, solid cube, and solid 3D arrow.

**Keep:** slots 26–28, 31, 34–36, and 38. **Remove from live runtime dispatch:** old native-name Rhai aliases, duplicate solid variants at slots 37 and 39, and experimental bounds capabilities not represented by the retained cube/arrow. Keep the full reverse-engineering mapping in research documentation.

**Files:** Rhai bindings/catalog and primitive collection, native debug-draw adapter/config, generated API guide, maintained example, research/design docs. Preserve signatures and routing for all retained calls.

**Review:** Every listed Rhai method must map to its documented slot and visible identity. No aliases or duplicate variants should remain registered. The maintained example should demonstrate every distinct form once. Do not add or run tests unless requested.
