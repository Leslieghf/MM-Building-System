# Mad Max Reverse-Engineering Playbook

This is the project's durable method for recovering game capabilities beyond the public community SDK. It is intended to prevent rediscovery and to keep speculative labels from silently becoming API contracts.

## Evidence boundary

For every capability, keep these sources distinct:

1. **Public SDK declaration:** the exact repository, commit, path, and declaration. A missing declaration means only “not in the checked snapshot”; it does not mean the game lacks the feature.
2. **Executable-static finding:** exact executable build fingerprint, RVA, disassembly range, caller/callee references, ABI evidence, and any RTTI/string evidence. A readable function body alone does not establish its C++ name or semantics.
3. **Live runtime observation:** exact installed plugin hash, process/run, user action, timestamp, sampled inputs/outputs, and whether the observation was bounded or filtered.
4. **Mod API contract:** only the subset supported by the first three layers and proven safe for the intended thread, lifecycle, reentrancy, ownership, and failure cases.

Never let a friendly function label substitute for evidence. Mark names as `SDK-declared`, `RTTI-confirmed`, `binary-inferred`, or `temporary-label`; retain the raw RVA either way.

## Capability investigation loop

1. **Pin the target.** Record executable version, PE timestamp/size/hash where available, module base assumptions, SDK repository commit, active source revision, candidate DLL hash, and install backup. An RVA or vtable slot is build-specific until proven otherwise.
2. **Inventory prior art.** Inspect the public SDK tree at a pinned commit and search declarations, samples, and history. Record the exact coverage checked and any absent symbol. Read existing project research before starting fresh.
3. **Start at a concrete behavior.** Choose a narrow user-visible action or SDK method. Identify its enclosing function, call instruction and return RVA, cross-references, and nearby code in Ghidra. Follow caller data into callees and follow return use back into the caller.
4. **Map the ABI before calling anything.** Record Win64 argument registers, stack arguments (including shadow space vs. true arguments), nonvolatile values relied on after the call, return registers, object/subobject offsets, vtable slot origin, and conditional branches. Keep unknown slots raw. Do not make a Rust function pointer from a guessed prototype.
5. **Instrument the smallest link.** Prefer a counter or signature-checked passive tap first. For detailed captures, keep hook work bounded and allocation-free; capture raw values alongside decoded interpretations; limit sample volume; preserve original registers/stack/return behavior; and avoid unrelated taps in the same experiment.
6. **Run a discriminating manual test.** Write down the exact action sequence before capture. Change one meaningful input at a time (for example ground vs. sky), record the user's result, then group records by call ordinal and compare input, return, output, and downstream caller branch. Missing samples can result from caps, filtering, or wrong tap placement; they are not proof that a path did not run.
7. **Separate observations from interpretations.** A Boolean, fraction, candidate count, or pointer is a raw observation until the caller's use and controlled contrasts support a meaning. State where a conclusion applies and list excluded callers/contexts.
8. **Promote only a proven slice.** Before exposing a Rust/Bevy resource or system, establish a typed contract, calling thread and phase, object lifetime/ownership, reentrancy behavior, safe failure result, and a minimal runtime test. Keep unsafe ABI details inside one adapter, not the mod-facing API.
9. **Append the record.** Do not replace prior findings when evidence changes. Add a dated section with the new build, source, action, result, changed interpretation, and unresolved items; link the raw log or stable artifact when available. Update a conclusion explicitly when new evidence supersedes it.

## Reusable finding record

Copy this block into the relevant dated research note:

```text
Capability / question:
Target executable fingerprint:
SDK repository + pinned commit + paths searched:
Candidate function / RVA / return RVA:
Name provenance: SDK-declared | RTTI-confirmed | binary-inferred | temporary-label
Static evidence: caller/callee sites, argument setup, return use, vtable/subobject basis
Runtime build: source revision, candidate hash, installed hash, backup path
User action and sequence:
Capture policy: counters, filters, quota, dropped/unavailable reasons
Observed inputs and outputs:
Controlled comparisons:
Supported conclusion and scope:
Confidence / evidence still missing:
Thread, lifetime, reentrancy, and ownership questions:
Next smallest discriminating experiment:
Related notes and artifacts:
```

## Current example: ammunition query

`CAmmunition::CheckCollisions` is absent from the checked `mm_sdk` snapshot. This project identified its caller in the Steam executable at return RVA `0x1DA5892`, examined its stack-argument setup and return continuation, then correlated bounded live records with alternating ground/sky shots. The latest four-shot run captured ten downward, true-return samples per ground shot and ten upward, false-return samples per sky shot. This is stronger than a name match: it connects controlled inputs to caller behavior. It still does not establish an arbitrary camera-ray API, target-object type, surface normal/material, or a safe cross-thread invocation model.

## Source of truth

The dated research notes under `docs/research/` hold evidence and current confidence; the design specs describe intended user-facing architecture; implementation plans track a bounded change. Keep all three linked, and prefer recorded artifacts over conversational recollection when resuming work.
