# Documentation refresh plan

## Purpose and audience

Present this repository as a Mad Max modloader and code-library foundation, not a gameplay/content mod. Give players a short path to the loader and sample scripts; give script authors the package contract and API reference; give contributors the architecture and build process. Preserve dated reverse-engineering records as evidence, clearly separated from current instructions.

## Canonical documentation

| File | Audience and purpose |
| --- | --- |
| `README.md` | Project identity, quick script onboarding, and links to the focused guides. |
| `docs/architecture.md` | High-level implementation, data flow, module ownership, and safety boundaries. |
| `docs/scripting.md` | Script package creation, manifest, callbacks, capabilities, console actions, and generation behavior. |
| `docs/rhai-api.md` | Generated API v1 method/value/limit reference, sourced from `api::rhai::catalog`. |
| `docs/development.md` | Build/verification and contributor boundaries. |
| `docs/research/` | Dated investigative evidence, not current onboarding or normative API documentation. |

## Cleanup actions

- Replace the oversized, stale README with a concise project entrypoint and correct current paths.
- Replace `docs/runtime-map.md` with the architecture guide and move the root `API.md` into the docs tree as generated `rhai-api.md`.
- Remove superseded dated implementation plans/specs from the published docs tree; their source history remains in Git. Keep the current decisions in the architecture guide and this plan.
- Label the research collection as historical, retain it because it records evidence for build-specific safety decisions, and remove stale research links from onboarding copy.
- Verify internal links, generated API output, build instructions, and current source/module names.

## Status

Completed with this architecture cleanup. Future API behavior changes must update the catalog and regenerate the reference through the build.
