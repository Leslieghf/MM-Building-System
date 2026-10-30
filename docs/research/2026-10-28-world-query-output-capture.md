# World Query Output Capture

**Date:** 2026-10-28  
**Executable:** Steam Mad Max 1.0.3.0  
**Status:** Runtime caller and filter mappings recovered; aim-ray hit path remains unproven.

### Overnight read-only watch started (2026-10-28 07:20 UTC)

Mad Max PID `20880` remained responsive. A scratch-only watcher now reads the existing helper-call counter and latest return-address cells every 30 seconds with the read-only probe; it stops if the game process exits. Its first interval recorded `2,410,486` then `2,413,888` calls (+3,402 in 30 seconds, about 113 calls/second), with latest return address `0x141FC0E0E` (`wind_occlusion`) at both samples. This latest-caller cell is not per-caller attribution; counts should not be assigned to wind queries. The watcher script and output are outside the repository, and no game memory is written.

At `07:23 UTC`, later samples showed the latest return address transition to `0x1421DC95A` (`vehicle_sound_occlusion`) and remain there across the next interval, while the counter advanced by about 115 calls/second. This confirms that the vehicle sound-occlusion callsite is active during this idle observation window; it still does not attribute the interval's total calls to that caller.

Subsequent samples through `07:29 UTC` alternated between the two latest-return values (`wind_occlusion` and `vehicle_sound_occlusion`) while the counter continued rising by roughly 110–116 calls per 30 seconds. This confirms both callsites remain active in the sampled periods; the counter deltas remain global totals, not per-caller rates.

At `07:30 UTC`, the global counter reached `2,478,494` and the latest return was again `wind_occlusion`; PID `20880` remained responsive. The detailed bootstrap log was still 19,086 bytes and last written at `02:26:57 +01`, so no new query sample records exist in this old-build run.

The watcher remained live through `07:34 UTC`; the total reached `2,505,698` and the latest caller remained `wind_occlusion` for four consecutive 30-second samples. Those intervals averaged about 115 helper calls/second globally. This is evidence of sustained helper activity during this observation window, not a caller-specific rate or a hit count.

The first watcher process exited without a process-ended record, while Mad Max PID `20880` remained responsive. I restarted only the read-only watcher as PID `13824`. Its initial sample at `07:35 UTC` was `2,512,578` (the prior gap prevents a local delta), and the next interval advanced by 3,448 with latest caller `wind_occlusion`. The detailed game log remains unchanged.

At `07:37 UTC`, the watcher remained responsive and the counter reached `2,529,408`; the latest return was `wind_occlusion` across the last four 30-second samples. The observed workload is therefore still active, but no new callsite-specific outputs were logged by the installed build.

Revalidation at `07:38 UTC` found Mad Max PID `20880` and watcher PID `13824` still responding. The counter advanced to `2,532,816`, with `wind_occlusion` still the latest return address; the detailed log remains unchanged. This extends the live activity window without changing the hit-semantics evidence.

The watcher remained active through `07:45 UTC`; the global total reached `2,580,160`, with `vehicle_sound_occlusion` as latest caller for two consecutive samples. The game process remains responsive. This is another point observation only; the earlier sampling limitation still applies.

### Spaced recapture policy prepared for the active occlusion callers (2026-10-28 07:28 UTC)

The current repeat-caller list in `src/game_probe.rs` does not include either `wind_occlusion` or `vehicle_sound_occlusion`, so each receives only its first-use sample under the default policy. These two callsites are active during the current run and their post-query scalar controls a `1.0f` threshold branch. I prepared a small source-only policy change to capture each at its own call ordinals `0`, `256`, and `2048`, spread over that caller's activity rather than taking three adjacent queries. The per-caller rate has not been measured, so those ordinals are deliberately not translated into elapsed seconds.

The revised budget is 32 ammo + 8 height + 16 script + 30 samples for the existing ten repeat callers + 6 spaced occlusion samples + one for each of the remaining 40 sites = 132 of 144 slots, leaving 12 for unexpected caller RVAs. The source change passed `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check`; no tests were run. The built candidate is 2,484,736 bytes with SHA-256 `561CF45849E8C7A211B01AEFF45EB5FF6AC6341B91EDEF7DBC1A1A16D7F7CE53` in the scratch target directory. It remains uninstalled. It must be inspected as part of the complete Rust diff before the next closed-game install; no new runtime behavior is claimed.

### Live-state revalidation (2026-10-28 06:55 UTC)

Read-only checks still find Mad Max PID `20880` responding. The bootstrap log remains `19,086` bytes with last write `2026-10-28 02:26:57` local time, so this session has produced no newer query samples to inspect. A fresh 96-byte read of the two stack addresses `0x7FFF6060` and `0x7FFF62F0` still returns the old sentinel-shaped records and self-pointers. Their continued readability does not make them live query results; they are stale stack data from the earlier samples.

### Camera script-hook route revalidated (2026-10-28 07:04 UTC)

Read-only reads of PID `20880` confirm the registered `RaycastStaticFromCamera` entry at `0x141EAC4C0` is detoured: its `E9` branch lands at relay `0x13FFF0F93`, whose indirect jump targets the loaded `MadMaxModding.asi` at `0x7FFFEC3E10F0`. The Rust hook source's counter increment and original-function load match the loaded hook's first instructions. The counter cell at `0x7FFFEC572F00` is currently zero, consistent with the log's “waiting for game-script calls” message; this API has not been observed running in this session.

The hook's saved original pointer at `0x7FFFEC572F38` is `0x13FFF0F80`, not the apparent `0x13FFF8000` from a mis-grouped wider dump. The target page is committed executable memory. Its trampoline contains the five-byte original prologue (`48 89 5C 24 08`) followed by an indirect jump to the still-unpatched continuation at `0x141EAC4C5`. Thus the saved original target is valid; the earlier concern that it pointed into a free page was a decoding error. This verifies the hook route without invoking it.

### Live helper-call counter and latest caller (2026-10-28 07:05 UTC)

The installed helper detour calls `record_world_query_call` at `0x7FFFEC3E4430`. Its first atomic increment targets `0x7FFFEC56CDE8` (`WORLD_QUERY_CALLS`); the following store writes the current return address to `0x7FFFEC5730B8` (`WORLD_QUERY_LAST_RETURN_ADDRESS`). Read-only samples across two seconds showed the total rise from `2,322,422` to `2,322,645` (+223, about 112 calls/second). Each of five half-second samples reported latest caller `0x141FC0E0E`, identified in the caller table as `wind_occlusion`. This confirms the shared helper remains active even though the detailed log is capped and unchanged. It does not prove every call in those intervals came from `wind_occlusion`; the latest-caller field is a single overwritten value, not a histogram.

A later point read at `07:07 UTC` found total `2,341,358`, the same latest caller, and camera-script tap counter `0`. The 19,086-byte log still had no detailed record for the wind-occlusion caller.

The old log has a caller-observed entry for `wind_occlusion` but no detailed sample for that caller, so its ray arguments and output remain unknown. The prepared 144-slot candidate samples the first call from each otherwise-unsampled helper caller, which should capture one `wind_occlusion` record on the next launch if that callsite runs.

### `wind_occlusion` caller setup from the live image (2026-10-28 07:07 UTC)

The function containing return RVA `0x1FC0E0E` has range `0x1FC0920..0x1FC0F59`. A read-only window around the call at `0x141FC0E09` resolves directly to the shared helper at `0x14252EDF0`. Its call setup establishes:

| Helper input | Caller setup | Confidence / remaining question |
|---|---|---|
| `RCX` | Loads a pointer from global slot `0x1417F4F08`; current value is `0x82588260` | Same owner pointer seen in older query samples; object type still unknown |
| `RDX` | RIP-relative address `0x14129D708`, containing `wind_occlusion\0` | Query label is directly observed |
| `R8` | Six-float local ray block at `[RBP-0x70]` | Origin/direction layout is supported by other callers; this caller's actual vector values were not logged |
| `XMM3` | `XMM7` | Raw float value not captured for this caller |
| Stack slot 5 | `XMM9` | Ray-range scalar; raw value not captured |
| Stack slots 6–7 | Output record `[RBP-0x40]`; pointer to a caller-local context area at `[RBP+0x10]` | The helper receives the address of this stack area, not the qword stored at its start |
| Stack slots 8–9 | Both zero | Exact raw values established at this call |
| Stack slots 10–11 | `R12D`, then `R12` | Shared register source; runtime value and pointee meaning remain unknown |

This narrows what the first candidate sample should resolve: actual ray vectors and range, both raw `R12` arguments, context changes, callback fractions, bounds results, and the caller-visible output. The static callsite alone does not establish a hit or collision meaning.

#### Context pointer clarification

A follow-up byte-level check of the live call setup disambiguates the context argument. The caller loads `0x141253B20` into `R11` and stores that value at `[RBP+0x10]`; later it uses `lea RAX, [RBP+0x10]` for helper stack argument 7. Therefore the helper sees a pointer to the caller's local area, while the first qword in that area contains `0x141253B20`. RTTI at this address identifies it as the vtable for `CIgnoreFilter`, not as the base of the context data. The helper prologue loads argument 7 into `RDI` and accesses fields relative to the passed stack pointer, so reading `0x141253B20 + offset` does not reveal the `CIgnoreFilter` instance fields. The earlier exploratory read at that vtable address must not be interpreted as a decoded query-context structure.

The caller first passes that local area to initializer `0x1424E5AA0`. Its live bytes write defaults at offsets `+0x08` (1.0), `+0x20` (1.0), `+0x24` and `+0x30` (0xFFFFFFFF), `+0x50` and `+0x60` (zero), then write the incoming `EDX`, `R8D`, `R9`, and `R10` values at `+0x70`, `+0x74`, `+0x78`, and `+0x80`. The caller subsequently replaces `+0x00` with the address `0x141253B20`. It then calls `0x14202A360` with `DL=1`; that short helper conditionally sets bit 8 in a global-derived value and writes it over `+0x70`. These are initialization facts, not semantic field names. The world-query helper later reads context-relative values at `+0x40`, `+0x44`, and `+0x48`; their runtime contents and meaning are still unknown. The prepared capture of the 168-byte context area is intended to resolve those values without guessing from the unrelated global object.

The candidate's `0xA8` length is a conservative raw stack-window capture, not a proven `sizeof` for a game type. In this `wind_occlusion` frame, the prologue saves `R12` with a `push`; later explicit stores copy `R12` to caller addresses `[RBP+0xA0]`, `[RBP+0xA8]`, and `[RBP+0xB0]`. Relative to the passed context pointer `[RBP+0x10]`, those are offsets `+0x90`, `+0x98`, and `+0xA0`. Since `CIgnoreFilter` vtable slot `+0x10` reads `[self+0x90]` and `[self+0x98]` as the ends of a pointer range, and this caller writes the same `R12` value to both, its observed ignore range is empty for this query unless another operation changes either field. The meaning of `+0xA0` remains unknown. Thus the final captured qwords may be logical object fields, not callee-save slots; preserve raw bytes and do not infer a structure size from the `0xA8` capture length.

The immediate continuation at return RVA `0x1FC0E0E` begins with `movss xmm5, [RBP-0x2C]`. Since the output record is `[RBP-0x40]`, this is exactly output `+0x14`. The caller then compares that scalar with preserved `XMM8` and branches when output `+0x14 >= XMM8`. This is direct evidence that `wind_occlusion` consumes the helper's segment scalar as a threshold and supports the fraction/range interpretation; the threshold is `1.0f`, while the resulting gameplay meaning remains unknown until a detailed sample.

The live function setup loads `XMM8` from `0x14122F8D8`; a read of that constant yields `1.0f`. Since the helper call preserves Win64 nonvolatile `XMM8`, the post-call comparison is against a 1.0 threshold unless some intervening caller code changes it. The branch condition is therefore `output +0x14 >= 1.0` versus `< 1.0`; values below the threshold enter the alternate path. This still does not equate helper `AL` or the scalar alone with a confirmed physical hit.

### `vehicle_sound_occlusion` caller setup from the live image (2026-10-28 07:22 UTC)

The call returns at RVA `0x21DC95A` and targets the same helper RVA `0x252EDF0`. Its label is independently confirmed from the RIP-relative string at `0x1412D6640`. The caller loads `RCX` through the same global slot `0x1417F4F08` used by `wind_occlusion`; the current slot value is `0x82588260`. The six-float ray block is at `[RBP-0x68]`, output at `[RBP-0x30]`, and the context-area pointer is `[RBP+0x20]`. Stack argument 5 is `XMM6`; `XMM3` receives `XMM8`; arguments 8–11 are zero. These are call-boundary observations, not identified game types.

Immediately after return, the caller reads output `+0x14` from `[RBP-0x1C]` and compares it against `XMM12`, then branches on `>=`. Earlier in the same function, `XMM12` is loaded from `0x14122F8D8`; a live read confirms the constant is `1.0f`, matching `wind_occlusion`'s threshold. Thus values `>= 1.0` take one branch and values `< 1.0` take the alternate branch. The exact gameplay meaning of those branches remains unknown; the candidate's raw XMM/context/output capture can connect them to runtime query records if this callsite is sampled.

### Live broadphase object and fraction reduction (2026-10-28 07:40 UTC)

A fresh read-only chain from the currently loaded owner confirms the concrete broadphase dispatch. Global slot `0x1417F4F08` contains owner pointer `0x82588260`; owner `+0x38` contains `0x08156B20`; that object's `+0x88` and `+0xC0` contain `0x0817E760` and `0x0817EDA0` respectively. The broadphase object at `0x0817E760` has vtable `0x1413E0A18`. Its preceding MSVC Complete Object Locator points to TypeDescriptor `0x1417DBA28`, whose name is `.?AVhk3AxisSweep@@`. Vtable slot `+0xC8` resolves to `0x142B2CD40`, matching the live `hk3AxisSweep::castRay` implementation previously identified from the image.

The helper's cast wrapper calls that slot with its stack ray input and collector. In the broadphase loop, the implementation calls the collector's virtual entry at vtable `+0x08`; after the call, it takes the returned `XMM0` float, compares it with a running float through `MINSS`, and stores the lower value back. This is direct static evidence that candidate callback returns participate in a running minimum/early-out fraction. The collector entry seen elsewhere returns the per-ray record's `+0x08` float, so the two code paths form a plausible candidate-fraction reduction chain.

The last link is still unproven: this broadphase accumulator has not been shown to be the same storage as caller context `+0x20` or final output `+0x14`. The prepared candidate records callback fractions, context before/after, and output before/after on one query so that relationship can be tested directly. No process memory was written and no thread was suspended during these reads.

The same live object chain identifies the filter at world `+0xC0` as an `hkpCustomGroupFilter`: its vtable pointer is `0x141325DA0`, its MSVC RTTI TypeDescriptor is `0x1417C9EA0`, and the name bytes decode to `.?AVhkpCustomGroupFilter@@`. Separately, the caller's argument-7 stack object starts with vtable pointer `0x141253B20`; its TypeDescriptor at `0x141797D40` decodes to `.?AVCIgnoreFilter@@`. These are distinct objects and types. The `CIgnoreFilter` label is a class identity only; it does not establish the meaning of its `+0x10..+0x80` fields or why the helper reads result-like fields from it.

One `CIgnoreFilter` vtable method is more specific: slot `+0x10` targets `0x142059ED0`, whose live bytes load a candidate-associated pointer from `[RDX+0x28]`, walk the range bounded by `[RCX+0x90]` and `[RCX+0x98]` in 8-byte steps, compare each entry, and return a Boolean equality result. This supports an ignore-membership/filter role for that method. Its source name, exact argument type, and relationship to the helper's result fields remain unknown; it does not show that the world query's chosen output is a physical hit.

## What the current run proves

The running game loaded the corrected stack-address-filter build. The log records the helper hook installing at runtime address `0x14252EDF0`, then records eight calls from `CAmmunition::CheckCollisions` at return RVA `0x1DA5892`. Those eight calls returned `1`. The current sampling cap stopped recording that caller after those eight calls, so later shotgun activity in this run is not represented in the detailed sample set.

The user reported firing four shotgun shells. The selected ammo caller generated eight detailed query samples. This establishes that the tap runs on the shotgun collision path; the sample count alone does not establish whether one call corresponds to one pellet, one ray, or another unit of work.

For sample `#8611`, the logged 96-byte block changed at its last qword from `0x3F80000000000000` to `0x3F7F6BBEBD7E8C40`. Caller stack layout now proves this aliases output bytes `+0x08..+0x0F`; the pair is two live output floats, but their geometric meaning is still unknown.

Decoded that qword across samples `#8610`–`#8617` as two little-endian `f32` values (low word first). Before each query they are `(1.0, 0.0)`; after the query, the pair varies, for example sample `#8611` is approximately `(-0.062145, 0.997738)`. The after-pairs have magnitudes from `0.998028` to `0.999899` in these eight samples. This supports that the output pair is a near-unit 2D component pair. Since these samples followed shots at the ground and the second component is near `1`, a ground-contact normal is a useful hypothesis, but it is not confirmed: output `+0x10` was not captured, and the helper result `1` does not itself prove collision. The separate `+0x28` tagged field also remains unclassified.

The current log also captures two stack addresses referenced by the 96-byte block. Their first twelve qwords share the same sentinel-like values, including `0xFFFFFFFFFFFFFFFF`, `0x00000000FFFFFFFF`, `0x3E8`, and a self-pointer. That is evidence of stack-backed records with a repeated header shape; it is not enough to name their C++ type.

## Verified call boundary

Live disassembly of `CAmmunition::CheckCollisions` identifies the call instruction at RVA `0x1DA588D`; its return address is the observed caller RVA `0x1DA5892`. Before the call, that function places a ray length and two pointers in Win64 stack argument slots. The two pointers target:

- an output record whose result fields through `+0x4C` are initialized by the caller before invoking the helper;
- a 0xA8-byte query context initialized through nearby helper calls.

The callee reads the output/context pointers from its fifth-and-later Win64 argument area and writes query results through the output pointer before returning a Boolean; the return value alone has not been proven to mean a physical collision. The current hook preserves the original stack and records the return value, but it does not capture these two pointed-to records. That is the most direct missing evidence for identifying the output values, caller-computed ray position, dispatch pointer, and any collision metadata.

## Output record layout recovered from live disassembly

Read-only disassembly of the helper at RVA `0x252EDF0` confirms its output writes directly. Its `RBX` register is the caller's output pointer (stack argument 6); `RDI` is the query context pointer (stack argument 7). The observed output record spans `0x50` bytes, with result fields written through `+0x4C`. The endpoint bounds predicate does not write into this record. This is code evidence for field relationships, not runtime evidence for their gameplay meaning.

| Output offset | Observed write | Evidence-based interpretation |
|---:|---|---|
| `0x00` | Pointer copied from a selected object-table entry; null on the observed fallback paths | Object/subobject-related pointer; exact type unknown |
| `0x08..0x10` | Three 32-bit values copied from context `+0x10`, `+0x14`, `+0x18` when context `+0x60` is non-null | Vector-shaped group; the ammo caller initializes it to `(1, 0, 0)`, and true-return samples' first two values are near-unit together. A normal/direction interpretation is plausible but unproven |
| `0x08..0x10` on fallback | Three floats assembled from helper locals | Fallback vector-like data; meaning unknown |
| `0x14` | Context `+0x20` on the populated path; constant `1.0f` on one fallback | Ammo caller multiplies it by ray length; `wind_occlusion` compares it to a caller threshold. This strengthens the segment-fraction interpretation, but does not alone prove a physical hit |
| `0x18` | Either zero or `object + sign_extend(*(i8*)(object + 0x20))`, selected by the byte at object `+0x28` | Conditional object-relative pointer |
| `0x20` | `object + sign_extend(*(i8*)(object + 0x20))` | Object-relative pointer derived from a signed one-byte offset |
| `0x28` | Pointer copied from context `+0x80`; null on fallback | Context-owned pointer; exact type unknown |
| `0x30..0x4C` | Eight 32-bit values copied from matching context offsets `+0x30..+0x4C` | Raw context values; types and meanings are not established |

### Query-context observations

These are offsets from the separate query-context pointer (stack argument 7), not from the 0x50-byte output record:

| Context offset | Observed use | Interpretation |
|---:|---|---|
| `+0x60` | The helper tests this field for null and takes different output paths | Pointer-like control field; pointed-to type unknown |
| `+0x64..+0x73` | An endpoint-check entry writes these bytes from XMM registers | Raw vector-sized data; the unusual overlapping call entry leaves their meaning unknown |

The candidate captures 0xA8 bytes from the query-context pointer, so these offsets and their before/after changes are in scope. No field meaning is assigned to the remaining context bytes without a traced read or write.

The helper also has a fallback where context `+0x60` is null: it clears output `+0x00`, `+0x18`, and `+0x20`, then copies context fields into `+0x08..+0x14`. A separate earlier fallback clears `+0x00`, `+0x18`, and `+0x28`, writes a vector-like fallback to `+0x08..+0x10`, and writes `1.0f` at `+0x14`; that branch does not visibly initialize `+0x20` or `+0x30..+0x4C`. Therefore the pre-call snapshot matters: a field not written on one path may retain the caller's prior value.

The `CAmmunition::CheckCollisions` continuation independently confirms the helper leaves the six ray floats available to the caller. On a true return, the caller reads output `+0x14`, multiplies it by the ray length, and computes `origin + direction * distance`; this position is passed to a later virtual callback. Since the ray length is multiplied by the output scalar, `+0x14` is strongly supported as a segment fraction/scale. The code does not establish that the value is clamped to `[0,1]`, so its range remains unknown. On a false return, the caller computes the full segment endpoint `origin + direction * ray_length` instead.

The same ammo caller reads output `+0x28` as a pointer. If non-null, it checks the byte at target `+0x10` against `0x1C` and routes to one of two ammo virtual callbacks. This establishes that `+0x28` supplies a target/descriptor pointer used to select collision handling; its exact C++ type and whether it always denotes the hit entity remain unproven. The true-return path also passes the output record and query context into the general callback, so those structures likely carry additional collision metadata, but their field meanings need the captured snapshots.

The helper’s live disassembly reads argument 9’s low byte as a selector, reads argument 10 as a 32-bit value, and tests argument 11 independently as an optional pointer. When argument 11 is non-null, the helper reads a 32-bit override through it; this dereference is **not** conditional on argument 9. Argument 8 has no direct read in the helper body found so far. Preserve all four values as raw words because callsites can supply different values, and argument 8’s possible use by callees or future code paths has not been established. At the sampled ammo callsite, arguments 8–11 are all zero. At the sampled `HeightOverGround` callsite, argument 8 is byte `1`, and arguments 9–11 are zero. The previous candidate sampled stack slots 5–10; the current candidate extends the tap frame to preserve slot 11 as well.

At both inspected callsites, slots 5–7 are ray length, output pointer, and query-context pointer. The helper prologue confirms these as Win64 arguments 5–7. At the `HeightOverGround` site, stack setup also shows `XMM3=0`, `RDI=0` (argument 9's byte selector), and `R8` pointing to the six-float ray block; its output/context pointers occupy slots 6 and 7. The context pointer targets a nearby stack record initialized before the call. Argument 11 is null at this site, so the helper does not dereference it.

An earlier analysis identified routine RVA `0x25267E0` as a point-in-bounds predicate: its body compares point±radius against per-axis limits. Corrected relative-call arithmetic confirms the helper calls this routine twice, with XMM2 set to zero. The helper tests each returned `AL` and can skip the broadphase cast when a test is false. The logger's endpoint coordinates are reconstructed inputs, not captured predicate results.

The ammo caller's setup also shows its output record's initial values: `+0x00=0`, `+0x08=1.0f`, `+0x0C=0`, `+0x10=0`, `+0x14=1.0f`, `+0x18=0`, `+0x20=0`, `+0x28=0`, and bytes `0xFF` throughout `+0x30..+0x4F`. This explains why the before snapshot is essential, especially on helper paths that leave some fields unwritten.

The same caller proves an alias between its ray block and output record: `R8` points to `[rsp+0x60]`, while the output argument points to `[rbp-0x50]`; from the caller's frame setup, the output starts at `R8+0x50`. Therefore the logged qword at ray-block offset `+0x58` is output bytes `+0x08..+0x0F`, not a separate ray-tail field. In all eight logged shotgun queries with helper return `1`, output `+0x00` was null and the two floats at `+0x08/+0x0C` were:

| Sample | output `+0x08` | output `+0x0C` |
|---:|---:|---:|
| `8610` | `-0.050183` | `0.997200` |
| `8611` | `-0.062145` | `0.997738` |
| `8612` | `-0.050183` | `0.997200` |
| `8613` | `-0.091132` | `0.995737` |
| `8614` | `-0.058421` | `0.997049` |
| `8615` | `-0.042389` | `0.997127` |
| `8616` | `-0.068529` | `0.997134` |
| `8617` | `-0.050183` | `0.997200` |

Those paired values look near-unit, and together with output `+0x10` form a vector-shaped group initialized to `(1, 0, 0)` before the query. That supports a normal/direction hypothesis, but output `+0x10` and the context were not captured in this run, so it is not yet a decoded surface normal. The qword's location is now known: it is output bytes `+0x08..+0x0F`, not a distinct ray-block field.

The two logged indirect addresses (`0x7FFF6060` and `0x7FFF62F0`) remain readable in the current process and still contain the same 0x60-byte sentinel pattern, including `0x3E8` at `+0x20`, `1` at `+0x28`, and a self-pointer at `+0x58`. `VirtualQueryEx` reports both as committed private memory. However, the old samples captured them only during calls; rereading their addresses later does not prove the data still belongs to those original calls. The ray block at `0x5F88B0` has already been reused. These old addresses are clues for record shape, not valid hit outputs.

## Caller coverage from the live image

A read-only scan of executable pages in the mapped game image searched for direct `E8 rel32` calls whose destination is helper RVA `0x252EDF0`. It found **55 unique direct callsites**. The current log contains **14 unique caller return addresses**, and all 14 match those direct callsites exactly. Therefore at least 41 statically present direct callsites did not run during the portion of this session represented by the log; this does not say whether they are dormant, gated by other gameplay, or reached only in other scenes.

The executable's `.pdata` exception table associates each observed return address with its containing function range. Those ranges are structural boundaries only; the functions have no recovered names yet.

| Observed caller RVA | Enclosing function RVA range |
|---:|---:|
| `0x1C26438` | `0x1C26260..0x1C264B4` |
| `0x1C284DA` | `0x1C282E0..0x1C28521` |
| `0x1CA3731` | `0x1CA3550..0x1CA390A` |
| `0x1CC6257` | `0x1CC5F70..0x1CC62F9` |
| `0x1D035A2` | `0x1D03390..0x1D03791` |
| `0x1DA5892` | `0x1DA56C0..0x1DA5A0F` |
| `0x1ECEBED` | `0x1ECEA80..0x1ECECAE` |
| `0x1F071AE` | `0x1F07030..0x1F07330` |
| `0x1F7C32D` | `0x1F7C1B0..0x1F7C3A7` |
| `0x1FC0E0E` | `0x1FC0920..0x1FC0F59` |
| `0x1FFBB53` | `0x1FFB7F0..0x1FFBEB3` |
| `0x20059EA` | `0x2005660..0x2005DA0` |
| `0x21DC95A` | `0x21DC540..0x21DCB28` |
| `0x22B95F3` | `0x22B9380..0x22B9958` |

The 41 direct callsite return RVAs that were present in executable code but absent from the current run's unique-caller log are:

`0x1C00C4D`, `0x1C5B949`, `0x1C7067A`, `0x1CA282D`, `0x1CA2AEC`, `0x1CA33FB`, `0x1CA59DD`, `0x1CAF3F1`, `0x1CB8C52`, `0x1D1B6C8`, `0x1D2906E`, `0x1D45092`, `0x1D963F7`, `0x1DA6DA3`, `0x1E1BE5B`, `0x1E9420E`, `0x1EAA014`, `0x1EAB0F1`, `0x1EAC351`, `0x1F066EB`, `0x1F18239`, `0x1F33118`, `0x1FA783D`, `0x1FA78B7`, `0x1FB0895`, `0x1FEC79F`, `0x200DCC8`, `0x200DEE4`, `0x2050E4E`, `0x205327F`, `0x20777E9`, `0x20C8899`, `0x217DA42`, `0x21AFA4C`, `0x21BD4FC`, `0x21BEC4A`, `0x21C18EE`, `0x21E1E4B`, `0x237BC19`, `0x24AB713`, `0x252F240`.

## Caller labels recovered from live memory

The current sample log includes `RDX` for each query. Bounded, read-only peeks at the two addresses recorded for the known callers yielded printable strings:

- At caller RVA `0x1DA5892`, `RDX=0x14125D848` points to `CAmmunition::CheckCollisions`.
- At caller RVA `0x1CA3731`, `RDX=0x14124D888` points to `HeightOverGround`.

This establishes that at least these two callers pass a stable textual query label in `RDX`. The local candidate now copies a 64-byte, null-terminated printable ASCII label from `RDX` at sample time. That should name many survey samples directly; labels from other sites remain to be verified.

The same live image scan found the latest `lea rdx, [rip+label]` before each direct call and resolved its target string. In 51 of 53 resolved cases, the instruction is exactly seven bytes before the call; the remaining two are 3 and 59 bytes before it. The two live-confirmed labels match this static pattern. Treat the other labels as strong candidates until the new runtime logger records them, especially the `FootPlacement` candidate with the larger gap.

| Caller return RVA | Candidate `RDX` label | Seen this run |
|---:|---|:---:|
| `0x1C00C4D` | `CCameraAutoFocusModifier::Raycast` | no |
| `0x1C26438` | `BaseToCameraSimpleRaycast` | yes |
| `0x1C284DA` | `CRayCastFanCondition::Evaluate` | yes |
| `0x1C5B949` | `CCharacter::HitBoneAt` | no |
| `0x1C7067A` | `CRayCastCondition::Evaluate` | no |
| `0x1CA282D` | `AlignWorldMatrix` | no |
| `0x1CA2AEC` | `AlignWorldMatrix` | no |
| `0x1CA33FB` | `CCharacter::UpdateMaterialBelow` | no |
| `0x1CA3731` | `HeightOverGround` | yes |
| `0x1CA59DD` | `CCharacterDialoguePrompt` | no |
| `0x1CAF3F1` | `CRayCastFanCondition::Evaluate` | yes (11:09) |
| `0x1CB8C52` | `CIsInDistanceOfObject::Evaluate` | no |
| `0x1CC6257` | `character_detection` | yes |
| `0x1D035A2` | `NFootPlacement::Raycast` | yes |
| `0x1D1B6C8` | `Selector` | yes (11:09) |
| `0x1D2906E` | `NScouting::Update` | no |
| `0x1D45092` | `NSpearAttack::Update` | no |
| `0x1D963F7` | `CExplosives::Detonate` | no |
| `0x1DA5892` | `CAmmunition::CheckCollisions` | yes |
| `0x1DA6DA3` | `CItemThunderStickObject::PhysicsCollision` | no |
| `0x1E1BE5B` | `CWeapon::GetRayWorldPosition` | no |
| `0x1E9420E` | `NodeCheckHeightOverGround` | no |
| `0x1EAA014` | `NodeIsVisibleObject` | no |
| `0x1EAB0F1` | `IntroHelperLineTestPassesTo` | no |
| `0x1EAC351` | `GraphScriptRaycastStatic` | no |
| `0x1ECEBED` | `ValidateTransformOnTerrain` | yes |
| `0x1F066EB` | `MoveToGround` | no |
| `0x1F071AE` | `raycast_sounds` | yes |
| `0x1F18239` | `StormDebrisSpawner` | no |
| `0x1F33118` | `ValidateRopeConnection` | no |
| `0x1F7C32D` | `CGameStateLoad::ForceOnPhysics` | yes |
| `0x1FA783D` | `CInteractionClearPathCondition::Evaluate` | no |
| `0x1FA78B7` | `CInteractionClearPathCondition::Evaluate` | no |
| `0x1FB0895` | unresolved | no |
| `0x1FC0E0E` | `wind_occlusion` | yes |
| `0x1FEC79F` | `CInteractionGraphEntry::GetPositionOnNavmeshWS` | no |
| `0x1FFBB53` | `CInteractionGraphEntry::CalculateExitTransform` | yes |
| `0x20059EA` | `CInteractionGraphEntry::CalculateEntryTransform` | yes |
| `0x200DCC8` | `NWarpedMotion::Update::GraphMoved::UpCheck` | no |
| `0x200DEE4` | `NWarpedMotion::Update::GraphMoved::ToUserCheck` | no |
| `0x2050E4E` | `CForcePulse::CheckPlayerRaycast` | no |
| `0x205327F` | `TrajectoryWarning` | no |
| `0x20777E9` | `CRayCasterObject::CastBulletRay` | no |
| `0x20C8899` | `CCharacterModule::CanSee` | no |
| `0x217DA42` | `CSpawnQueueItem::ForceAbovePhysics` | no |
| `0x21AFA4C` | `CAirVehicle::GetGroundHeight_Expensive` | no |
| `0x21BD4FC` | `LookAtTrigger` | no |
| `0x21BEC4A` | `RotorBlade` | no |
| `0x21C18EE` | `DebugSpawner` | no |
| `0x21DC95A` | `vehicle_sound_occlusion` | yes |
| `0x21E1E4B` | `CallForChum(Indoor test)` | no |
| `0x22B95F3` | `CEffectSystem::UpdateDecalEffect` | yes |
| `0x237BC19` | `CRayCastScheduler::Update` | no |
| `0x24AB713` | `FootPlacement` | no |
| `0x252F240` | unresolved | no |

This caller inventory spans character movement and perception, weapon/combat, script nodes, vehicle and sound occlusion, world alignment, explosions, weather/debris, interaction graphs, and decal effects. The helper is therefore shared infrastructure with a much broader reach than a camera-only or shotgun-only raycast.

## Additional static caller profiles

Read-only disassembly of two callers that have not appeared in the current log confirms the same Win64 argument layout as the observed ammo caller:

| Caller (static label candidate) | Call / return RVA | Slot 5 | Slots 6–7 | Slots 8–11 | XMM3 |
|---|---:|---|---|---|---|
| `CForcePulse::CheckPlayerRaycast` | call `0x2050E49`, return `0x2050E4E` | `XMM11 - XMM15`; `XMM11` was computed with `sqrtss`, and `XMM15` is loaded from image data | output pointer at `[RBP-0x20]`; context pointer at `[RBP+0x30]` | zero; `ESI` is cleared before call and supplies all four slots | zero; `XMM14` was cleared before call |
| `CRayCasterObject::CastBulletRay` | call `0x20777E4`, return `0x20777E9` | `XMM6`, computed with `sqrtss` from `XMM8` | output pointer at `[RBP-0x80]`; context pointer at `[RBP-0x20]` | zero; `EDI` is cleared before call and supplies all four slots | zero; `XMM7` is cleared before call |
| `CWeapon::GetRayWorldPosition` | call `0x1E1BE56`, return `0x1E1BE5B` | `XMM6` | output pointer at `[RBP-0x80]`; context pointer at `[RBP-0x30]` | slot 8=`1`, slot 9=`1`, slots 10–11=`0` (`R14` was cleared) | zero (`XMM8` was cleared) |
| `CRayCastCondition::Evaluate` | call `0x1C70675`, return `0x1C7067A` | `f32 [RBX+0x28]` | output pointer at `[RBP-0x50]`; context pointer at `[RBP]` | slot 8=`1`, slot 9=`1`, slot 10=`u32 [RSI+0x3184]`, slot 11=`0` (`RDI` was cleared) | `XMM6` |
| `GraphScriptRaycastStatic` | call `0x1EAC34C`, return `0x1EAC351` | computed `XMM6` (length from a squared vector distance, adjusted by `XMM10`, then clamped at zero) | output pointer at `[RBP-0x20]`; context pointer at `[RBP+0x30]` | zero; `EBX` is cleared and supplies all four slots | zero; `XMM9` is cleared |
| `CRayCastFanCondition::Evaluate` | call `0x1C284D5`, return `0x1C284DA` | `XMM0`, square root of the squared distance between two caller positions | output pointer at `[RBP-0x70]`; context pointer at `[RBP-0x20]` | zero; `EDI` is cleared and supplies all four slots | zero; `XMM3` is cleared |
| `NFootPlacement::Raycast` | call `0x1D0359D`, return `0x1D035A2` | `XMM8=1.0f` from global `0x14122F8D8` | output pointer at `[RBP-0x70]`; context pointer at `[RBP-0x20]` | slot 8=`1`, slot 9=`1`, slot 10=`R14D=0`, slot 11=`R14=0` | zero (`XMM7` was cleared before being copied to `XMM3`) |

An additional read-only live disassembly of `NFootPlacement::Raycast` narrows its call boundary. The helper call is at `0x1D0359D` (return `0x1D035A2`) and its relative target resolves exactly to RVA `0x252EDF0`. `R8` is `RSI+0xC8`; `RCX` is loaded from a global, `RDX` is set to a RIP-relative static address, and `R9` is not assigned in the immediate query setup; an intervening helper call follows its earlier zeroing, so its value at this boundary is unknown. The caller passes `XMM8` as slot 5; a live read confirms its source global `0x14122F8D8` currently contains `1.0f`. It uses local output/context records at `[RBP-0x70]` and `[RBP-0x20]`, sets slots 8 and 9 to one, and passes zero through slots 10 and 11. `XMM3` is zero because the caller clears `XMM7` before copying it. This callsite appears in the current run's unique-caller log but has no detailed sample in the installed build. Its six-float block's values and the meanings of the `RCX`/`RDX` inputs remain unknown; the prepared candidate's first-sample-per-caller policy will capture them on a future run. The true-return continuation also reads output `+0x08` at `[RBP-0x68]`, output `+0x10` at `[RBP-0x60]`, and output `+0x14` at `[RBP-0x5C]`, then uses the `+0x14` value in scalar comparisons and arithmetic before writing a vector-like result. This supports `+0x14` as a scalar consumed on this path and shows nearby output fields feed result construction, but it does not identify the exact semantics of `+0x08`, `+0x10`, or the resulting vector.


For the other caller profiles in the table (excluding `NFootPlacement::Raycast`), `R8` points to a local six-float query block according to static disassembly. The current log's unique-caller list includes `CRayCastFanCondition::Evaluate` return RVA `0x1C284DA`, but the installed build recorded only that address; it did not take a detailed sample, so the runtime value of its `XMM0` scalar and its pointed-to records remain unknown. Its second static callsite at `0x1CAF3F1` was not observed. The `CForcePulse::CheckPlayerRaycast`, `CRayCasterObject::CastBulletRay`, `CWeapon::GetRayWorldPosition`, `CRayCastCondition::Evaluate`, and `GraphScriptRaycastStatic` callsites in this profile were not observed in the current unique-caller list. `CRayCastCondition::Evaluate` uses a caller-owned scalar at `[RBX+0x28]` for slot 5 and passes a 32-bit field at `[RSI+0x3184]` through slot 10; that field's meaning remains unknown, but it is consumed by the helper's index construction. `GraphScriptRaycastStatic` and `CRayCastFanCondition::Evaluate` compute a segment length from their own caller data. The `CWeapon::GetRayWorldPosition` continuation initializes output `+0x14` to the `XMM7` value loaded from image address `0x14122F8D8`, which a read-only live-process read confirmed is `1.0f`. After the helper call it compares output `+0x14` with this sentinel; when the value is below `1.0f`, it multiplies by slot 5, computes `origin + direction * distance` from the six floats at `R8`, and writes the position to the function's output vector. The helper's observed fallback also writes `1.0f` at output `+0x14`. Together these code paths strongly support `+0x14` as a segment fraction, with `1.0f` representing the full segment/no earlier hit for this consumer; they do not prove the helper clamps all results to `[0,1]`. The inspected callers set slot 11 to null, so they do not exercise the optional override. Other callers may pass a non-null slot-11 pointer; the candidate snapshots its first `u32` at call time under a readable-page check.

The `CRayCasterObject::CastBulletRay` success continuation provides another output consumer. Its output pointer is `[RBP-0x80]`; after a true return, it copies output `+0x08..+0x10` into a newly obtained result object's `+0x00..+0x0C`, copies output `+0x14` into result `+0x10`, and copies output `+0x00` into result `+0x68`. This independently confirms that the three output words at `+0x08..+0x10` are consumed together as a three-float vector-shaped group and that `+0x14` is a scalar. The pointer at output `+0x00` is propagated as a pointer-sized result field. The continuation does not identify the vector as a normal/position or the pointer as a particular object class; those meanings remain hypotheses pending record captures or a typed consumer.

## Registered script ray-query APIs

A read-only scan of the live executable found a registration-table region at RVA `0x1E71CF0` containing repeated entries shaped as `LEA R8, function; LEA RDX, name; MOV RCX, registry; CALL registrar`. Resolving two entries' RIP-relative addresses showed:

| Registered name | Function pointer | Evidence |
|---|---:|---|
| `RaycastStatic` | RVA `0x1EAC3F0` | Registration entry resolves the name and pointer. Its function body directly calls RVA `0x1EAC0C0` at call RVA `0x1EAC478` (return RVA `0x1EAC47D`). `.pdata` gives the callee range `0x1EAC0C0..0x1EAC3D6`; that function contains the shared-helper call at `0x1EAC34C` (return RVA `0x1EAC351`). |
| `RaycastStaticFromCamera` | RVA `0x1EAC4C0` | Registration entry resolves the name and pointer. In the current process, the entry begins with a relative jump to a veneer at `0x13FFF0F93`; its indirect-jump slot contains `0x7FFFEC3E10F0`. Read-only module enumeration places that target inside the loaded `MadMaxModding.asi` at module offset `0x10F0`. The installed plugin hash matches the recorded working build. |

These are names registered in the game's script-function table, not proof that external mods can invoke them. `RaycastStatic` has a verified static call chain: the registered entry at RVA `0x1EAC3F0` calls `GraphScriptRaycastStatic` at RVA `0x1EAC0C0`, which calls the shared helper at RVA `0x1EAC34C`. The `GraphScriptRaycastStatic` label is still a candidate derived from its adjacent `RDX` string, but the code relationship is established independently of that label. The current run's logger did not record caller return RVA `0x1EAC351`, so runtime invocation remains unobserved. The function's parameter semantics and output mapping still need disassembly/capture. For `RaycastStaticFromCamera`, the live jump chain ties the registration entry to the already-installed `camera_raycast_hook` path: the code entry is detoured, its MinHook relay points into the loaded plugin, and the plugin's observer increments `CAMERA_RAYCAST_CALLS` before forwarding to MinHook's original trampoline. The observer captures no ray arguments or result beyond the count. The previous in-game status showed zero calls, meaning this separate camera API was not invoked during that observed play interval; this does not mean the lower-level world query was idle. Do not assume the two registered APIs share an implementation or parameter layout.

### `RaycastStatic` wrapper handoff

The live bytes at the `RaycastStatic` wrapper's call site (`0x1EAC478`) show it invoking `GraphScriptRaycastStatic` with this ABI-level setup:

| Callee input | Value at the call | What is established |
|---|---|---|
| `RCX` | Original wrapper-entry `RCX` (saved in `RDI`) | Forwarded unchanged; meaning unknown |
| `RDX` | Original wrapper-entry `RDX` (saved in `RBX`) | Forwarded unchanged; meaning unknown |
| `R8` | Pointer to three `f32` values at caller `[RSP+0x30..0x3B]` | The wrapper fills them from `[RSP+0x70]`, `[RSP+0x74]`, and `[RSP+0x78]` after a virtual call through `[RAX+0xB0]`. The call writes to a 64-byte buffer at `[RSP+0x40]`; these values are at buffer offsets `0x30`, `0x34`, and `0x38`. This matches the translation row of `Matrix4x4`, but the virtual method identity is not runtime-confirmed. |
| `R9` | `0xB773FFCF`, loaded via `MOV R9D, 0xC346A20F` at RVA `0x1EAC431` then `LEA R9D, [R9-0x0BD2A240]` at `0x1EAC437` | Exact low 32-bit value established; semantic role unknown |
| stack slot 5 | `0x2827198C` at caller `[RSP+0x20]` | Raw constant only |
| stack slot 6 | `0x5C26F215` at caller `[RSP+0x28]` | Raw constant only |

The wrapper first calls RVA `0x1E6AB20` with its original `RCX`, original `RDX`, and `R8D=0x8325AE7C`. That adapter calls RVA `0x14300B8D0`, which preserves the incoming 32-bit value in `R14D` and forwards it into subsequent lookup calls. This confirms the constant is an intentional lookup key/selector, not an incidental value, but its name and selected object type remain unknown. The adapter's returned pointer is then used for the virtual call. The wrapper calls an object virtual at vtable offset `+0xB0` before preparing the three-float block. The output buffer is 64 bytes, and the wrapper reads its final three `f32` values at offsets `0x30`, `0x34`, and `0x38`. This matches a 4x4 matrix's translation row. The repository's SDK port declares `CGameObject::GetTransform(CMatrix*)` with candidate vtable slot 22 (`22 * 8 = 0xB0`) and a 64-byte `Matrix4x4` output, so the virtual is likely `GetTransform` and the vector likely a transform translation. However, the SDK slot is explicitly unverified against this executable, and the caller's pointer type is unknown; therefore this remains a well-supported inference, not a decoded API contract. The wrapper reads this buffer before entering the graph query. Whether the transform translation becomes the final ray origin, endpoint, or another component is unknown. Its two incoming pointers and fixed stack values are not yet identified as user-facing script parameters. At the lower helper call in `GraphScriptRaycastStatic`, the separately recovered profile is `XMM3=0`, slot 5 computed from a vector distance, output/context pointers in slots 6–7, and zero slots 8–11. That helper profile describes the inner ray query, not the script wrapper's external parameter types.

### `GraphScriptRaycastStatic` body from the running image

I read and disassembled the running process's bytes for RVA `0x1EAC0C0` through `0x1EAC3D5`. The on-disk executable does not contain the same bytes at this RVA, so these observations specifically describe the live materialized image. The function retains its incoming `R8` three-float pointer, then resolves an object through helper calls using its original `RCX`/`RDX` and wrapper-provided constants. A vtable call at `+0xB0` fills a 64-byte transform buffer; the code reads the translation row at `+0x30..+0x38`.

The six-float block passed in `R8` is assembled exactly as follows: floats `0..2` copy the incoming `R8` vector; floats `3..5` are the resolved transform translation minus that incoming vector. Separately, the stack scalar is the Euclidean length of that difference minus a float value retrieved from a typed lookup, clamped to zero when negative. The tag check and float read are statically identified below, but the value's semantic role remains unknown. The live helper multiplies the second triple directly by `XMM3` and slot 5 to form its two precheck points; it does not normalize the triple before those calculations. Other callers use the six floats as ray-like origin/direction data, but this call site's unnormalized difference paired with a distance scalar does not prove conventional ray semantics; preserve the raw triples and verify their geometry at runtime. The transform method identity also remains an SDK-based inference.

At the shared-helper call at RVA `0x1EAC34C`, the function passes `XMM3=0`, the output and context pointers in slots 6–7, and the computed scalar in slot 5. It sets slot 10 to the low 32 bits of an intermediate resolved pointer and slot 11 to that same pointer. Since the helper can read a `u32` from slot 11, the next candidate's optional-pointee snapshot should be especially informative for this script path. Afterward, the function compares output `+0x14` against the live value at `0x14122F8D8`, which currently reads as `1.0f`, before its second lookup/dispatch. This matches the sentinel threshold used by `CWeapon::GetRayWorldPosition` and strengthens `+0x14` as a segment fraction with `1.0f` representing no earlier hit for those consumers. The function's observed non-null-object path returns `1`; the slot constants and lookup result types are still opaque, so this does not yet establish the public script signature or identify its user-facing inputs.

### Float-tagged lookup behind the GraphScript scalar

The live `GraphScriptRaycastStatic` body loads its candidate subtraction value into `XMM10` through a typed-lookup branch. It calls RVA `0x3005F50` with the resolved value pointer; that small predicate compares the `u32` at pointer `+0x04` with the `u32` at live address `0x141A01F04`, returning the equality result. The live global currently contains `0x4E0A188B`. The recovered zero-seed lookup3 `HashString` algorithm computes `HashString("float") = 0x4E0A188B`; adjacent words at that data block also match `HashString("vector") = 0x1CB5283C`, `HashString("bool") = 0xA8B28B19`, `HashString("int") = 0xB56363B5`, `HashString("int64") = 0xC2535A9C`, `HashString("uint32") = 0xE9F00A0C`, and `HashString("uint64") = 0x71AF5FAB`. These seven exact matches strongly corroborate the algorithm and identify the predicate's expected tag as `float`.

When the tag matches, the GraphScript body calls RVA `0x3005D90` with its two lookup objects and loads the returned pointer's first `f32` into `XMM10`. When it does not match, it calls RVA `0x30068B0` with a local `f32` output and then loads that fallback value into `XMM10`. So the subtracted value is a float-typed lookup result or fallback. Its gameplay meaning (radius, margin, padding, or something else) remains unknown. The next capture now includes `XMM10` and the resulting slot-5 scalar so we can measure their relationship at runtime.

This narrows the most useful next runtime comparison: correlate a call to registered `RaycastStatic` with a `GraphScriptRaycastStatic` sample, then compare its raw two vector triples, computed scalar, output `+0x14`, slot-11 pointee, and dispatch result. `RaycastStaticFromCamera` remains a separate registered function and should be analyzed independently.

The prepared candidate also preserves all eight Win64 nonvolatile registers and prints them by name. `GraphScriptRaycastStatic` saves its original `RCX` and `RDX` inputs in `RDI` and `RSI`, respectively, before the helper call; those registers survive to the shared-query tap. Therefore the next bounded script-ray samples should include the wrapper's two opaque pointer arguments as `RDI`/`RSI`, alongside the raw ray block and output/context data, without adding another hook.

The outer wrapper also branches on the byte result returned by `GraphScriptRaycastStatic`: it selects a different 32-bit value for a second lookup (`0x1CA7C4F6` when nonzero, `0x48D7A8AD` when zero), passes that value with the original `RCX`/`RDX`, and then returns `1`. The called lookup's output and those two selector values' meanings are still unknown. Thus the registered wrapper's final return value alone is not evidence that a ray hit; the inner output fraction and the conditional follow-up path are the relevant signals.

### Hash-string candidate check

The Steam build table in `src/api/game/abi/build.rs` labels RVA `0x2FBD230` as a candidate for the SDK's `HashString(const char*)`. Bounded live reads at that address show a NUL-terminated string-length wrapper (`repne scasb`) that sets `R8=0`, passes the string pointer and length to RVA `0x2FBC960`, and returns that routine's result. Read-only disassembly of the helper shows the exact initialization `0xDEADBEEF + length + seed`, little-endian 12-byte blocks, the six-stage `mix` rotation sequence, the `final` avalanche, and return of the final third state. This instruction pattern matches Bob Jenkins' `lookup3` `hashlittle` structure with seed zero. The identification is a strong code-pattern match, but it has not been checked against a known input/output pair.

The `RaycastStatic` wrapper passes `0x8325AE7C` as a 32-bit key to the lookup path at RVA `0x14300B8D0`. A nested lookup at `0x143006810` reads an element count from `[object+0x10]`, an element-array pointer from `[object+0x08]`, scans 0x20-byte records, compares each record's first `u32` with the key, and returns the matching record pointer. This establishes that the value is an intentional record-lookup key. It does **not** prove that this key is the output of `HashString` or reveal its source string. No match was found for a small exploratory set of obvious labels such as `RaycastStatic`, `Origin`, `Target`, `Camera`, `Position`, `Transform`, or `GameObject`; that set is not broad enough to meaningfully reverse the key by brute force.

## Decoded `HeightOverGround` query inputs

The old log also contains eight calls from caller RVA `0x1CA3731`, whose runtime label is `HeightOverGround`. Decoding each 64-bit word as two little-endian `f32` values shows that the first six floats in the `R8` block form an origin XYZ and direction XYZ pair. All eight directions are `(0, -1, 0)` and have magnitude `1.0`, consistent with a normalized vertical ground-height query direction. Seven helper calls returned Boolean `1`; sample `#7218` returned Boolean `0`. The seven true-return origins cluster around `(-3924, 478, -1336)` while the false-return sample used `(0, 501, 0)`. These are helper return values, not yet verified physical-hit/miss labels.

This is strong corroboration that `R8[0..24]` contains an origin/direction-shaped pair for both `HeightOverGround` and `CAmmunition::CheckCollisions`, despite their different caller behavior. Live disassembly of the `HeightOverGround` callsite shows argument 5 loaded from image address `0x14125D4A8`; a read-only live-process read returned `0.0f` there, and the caller also passes `XMM3=0`. The helper's two points formed from the origin/direction pair therefore coincide, so the downward direction does not affect the observed endpoint-call inputs. This suggests a point-style query on that path, but does not establish what the endpoint-associated routine does or whether later helper logic uses the direction. The scalar has not yet been runtime-captured at the exact call, so the prepared logger now decodes slot 5 for this caller too; the next run should confirm whether the source value remains zero when the query executes. The old false helper return at `(0, 501, 0)` does not by itself establish a bounds rejection or geometric miss. The before/after raw `R8` blocks are byte-for-byte identical for all eight old samples, so they expose no returned hit details through that block; only the helper Boolean distinguishes the observed true and false calls.

### Live bounds values behind the predicate (2026-10-28)

For the old query samples, `RCX=0x82588260`; a live read of its `+0x38` pointer yielded `0x8156B20`. The predicate's live code loads vectors at that target's `+0x400` and `+0x410`. Reading those values gives `(-4050, -1000, -4050, -4050)` and `(4050, 7100, 4050, 4050)` respectively. Their first three components are consistent with a broad axis-aligned minimum/maximum bounds pair. The helper subtracts its frame origin at `[RCX+0x40..0x48]` before calling the predicate. Those live fields currently decode to approximately `(-3923.68, 0, -1335.66)`. Using that current origin for illustration, the old `HeightOverGround` point `(0, 501, 0)` from sample `#7218` maps to local coordinates `(3923.68, 501, 1335.66)`, inside the broad bounds intervals; the seven origins near `(-3924, 478, -1336)` map close to `(0, 478, 0)`.

This makes a simple coarse-bounds rejection a poor explanation for sample `#7218`'s false helper return. The coordinate translation uses the presently readable frame origin with historical query points, not a sample-time origin snapshot; the old build also did not capture the predicate's own Boolean results, and the bounds data's exact type/role is not fully proven. The new candidate captures each helper-scoped predicate point, radius, and return byte, so a future run can confirm whether these endpoints pass that gate. The helper Boolean and physical-hit semantics remain separate questions.

Static inspection of the `HeightOverGround` caller continuation adds a reason not to equate the helper Boolean with its public query outcome. Immediately after return RVA `0x1CA3731`, the caller reads its output record's scalar at `[RBP+0x14]`, compares it with `XMM7`, and then inspects the output pointer at `[RBP+0x28]`; the visible continuation does not immediately test `AL`. In this same function, `XMM7` is loaded from `0x14122F8D8`, whose live value is `1.0f`, and the caller initializes output `+0x14` to that value before the helper call. The continuation branches away from the pointer-processing path when the returned scalar is at least `1.0f`; values below the sentinel proceed to inspect the pointer and its byte at `+0x10`. The same `+0x10 == 0x1C` tag check appears in the `CAmmunition::CheckCollisions` consumer of output `+0x28`, supporting a shared tagged-result/descriptor role for that pointer. The type and whether it always identifies a geometric hit remain unknown. This is strong evidence that `HeightOverGround` treats `+0x14` as a caller-visible sentinel/segment scalar and uses output metadata to decide whether to continue. It does not prove the helper Boolean is unused on every path or that a below-one value necessarily means a physical collision. Runtime output/context snapshots are still needed to connect the old true/false samples to the caller-visible result.
## Decoded shotgun ray inputs

The first six `f32` values in the 96-byte block passed in `R8` decode as origin XYZ followed by direction XYZ. All eight logged ammo calls use the same origin, `(-3940.409, 477.318, -1513.962)`. Their direction magnitudes range from `0.9999174` to `1.0000000` (maximum deviation from unit length about `0.0000827`), while direction components vary slightly, consistent with normalized spread rays. Each of the eight helper calls returned Boolean `1`; this is not by itself proof that all eight produced physical impacts:

| Sample | Direction XYZ |
|---:|---:|
| `8610` | `(-0.14606, -0.09761, -0.98445)` |
| `8611` | `(0.00662, -0.14700, -0.98903)` |
| `8612` | `(-0.14319, -0.09191, -0.98542)` |
| `8613` | `(-0.22033, -0.21882, -0.95050)` |
| `8614` | `(-0.10803, -0.11470, -0.98750)` |
| `8615` | `(-0.17987, -0.08670, -0.97985)` |
| `8616` | `(-0.07928, -0.15799, -0.98421)` |
| `8617` | `(-0.12094, -0.09826, -0.98778)` |

The ray block's qword at offset `0x58` changes in these samples, but it aliases output `+0x08/+0x0C` as established by caller stack layout above. The helper's Boolean return and these two output floats are runtime-observed; their geometric meaning is still unknown. The helper's output `+0x14` scalar and dispatch pointer at output `+0x28` are statically decoded from caller use but await the next build's runtime snapshots.

## Next diagnostic build

The source now extends the naked tap's saved-register frame to copy Win64 stack slots 5 through 11 and snapshots a printable `RDX` label where present. Sampling is capped at 32 ammo calls, 8 `HeightOverGround` calls, 16 `GraphScriptRaycastStatic` calls, and one first-use sample for each of the other 52 sites in the current 55-callsite map: at most 108 records, within the 128-slot capacity. The added 15 script-path samples are reserved for the newly confirmed `RaycastStatic` API chain and should expose changing inputs without recording every invocation. For every sampled caller, it snapshots the helper's 0x50-byte output record and 0xA8-byte query context immediately before and after the helper, using the shared ABI confirmed by live disassembly. The sampled ray block also yields up to two additional bounded snapshots for readable pointer candidates, alongside the raw 0x60-byte block. The candidate decodes slots 6 and 7 as the output and context pointers. For ammo samples, it decodes the first six `R8` floats as origin XYZ and direction XYZ, with slot 5 labeled ray length. For `GraphScriptRaycastStatic`, it preserves the first and second triples as raw vectors and labels slot 5 as a scale; it does not assume segment semantics or a normalized direction. `HeightOverGround` keeps an explicitly raw `slot5=` label. The hook snapshots three floats at `RCX+0x40` and logs the two local-space points that the bounds precheck tests, reconstructed directly from the six captured floats, `XMM3`, slot 5, and that frame offset. For ammo samples with a true helper return and a readable output snapshot, output `+0x14` is emitted as a raw output scalar alongside scalar-times-ray-length and the caller-computed position, following the formula observed in the caller; these labels do not claim a physical collision. Stack values are emitted with explicit `slot5=` through `slot11=` labels and remain raw unless caller-specific disassembly establishes their argument use. It also reads the dispatch pointer's byte at `+0x10` under a page-readability check, matching the caller's dispatch test. A false result logs the caller-computed segment endpoint. The hook path uses fixed atomics and bounded page checks; formatting remains on the existing background logger. This policy turns the 55-site static map into a bounded runtime survey while preserving extra detail for the known shotgun path.

This candidate has not been installed into the running game. The game process remains open (PID `20880`), and the installed binary has not been replaced. The candidate broadens pre/post output and context capture from the ammo caller to every sampled caller, because the shared helper uses the same argument slots and record writes. It copies stack slots 5–11, including slot 11 that the helper reads when non-null. Live callsite disassembly has verified argument profiles for ammo, `HeightOverGround`, Force Pulse, bullet-ray casting, and weapon ray-position callers. For both observed runtime callers it decodes slot 5 as the helper scalar and logs the origin/direction pair. Every sample records the first-argument frame offset and reports both local points used by the zero-radius bounds precheck. It also snapshots the optional `u32` pointee at slot 11 when readable, reporting null and unreadable separately. For ammo samples it logs the Boolean return, raw output `+0x14` scalar, scalar-times-ray-length, caller-computed position, dispatch pointer from output `+0x28`, and pointed-to byte `+0x10` used by the dispatch branch; a false return logs the caller-computed full-segment endpoint. Static continuation disassembly also shows `CAmmunition::CheckCollisions` stores the caller-computed ray position into its receiver (`RBX`) and dispatches through that receiver's vtable at `+0x58` or `+0x60`. The latest candidate therefore saves all eight Win64 nonvolatile registers for sampled calls and, for the ammo caller only, snapshots the receiver's vtable pointer and those two dispatch entries under readable-page checks. This may identify the consumer methods when the candidate is eventually run; it does not identify their behavior. `HeightOverGround`'s scalar has not yet been captured at runtime, and its current observed source-memory value is `0.0f`. The latest candidate passed `cargo fmt --check`, `git diff --check`, and `cargo build --release`: `target/release/madmax_bootstrap.dll`, 1,787,904 bytes, SHA-256 `2C1EF343BFCE8B88AD1E7FD90E12AF76D4BA8B9D289EBF363804DF4FFF548DEB`. The installed log remains unchanged at 19,086 bytes, last written 2026-10-28 02:26:57. A clean, user-observed next-session launch is still required before the candidate's expanded register capture, caller sweep, labels, and output/context records can be called runtime-verified.

## Evidence sources

### Static RTTI check for `CAmmunition`

The local executable contains the RTTI name string `.?AVCAmmunition@@` at file offset `0x573899`. Its neighboring bytes match the expected 64-bit `TypeDescriptor` shape (two pointer-sized fields followed by the decorated name). This image's section layout is unusual: that file offset falls in `.data1`, whose raw offset is zero and virtual address is `0x1000`; the name therefore maps to RVA `0x574899`, and the `TypeDescriptor` start maps to RVA `0x574889`. A full-file scan found no exact 32-bit occurrence of either the name RVA or the descriptor RVA. This route did not recover a Complete Object Locator or vtable, so it provides no static names for the receiver callbacks at vtable offsets `+0x58` and `+0x60`. The candidate's live receiver/vtable capture remains the next direct evidence source. The string's presence alone does not establish that usable RTTI metadata survives in this executable.

- `%LOCALAPPDATA%\MadMaxModding\logs\bootstrap.log`, current run, entries at Unix time `1793150817` and sample IDs `8610` through `8617`.
- Live disassembly of the mapped Steam executable, `CAmmunition::CheckCollisions`, RVA range `0x1DA56C0` onward; the helper call is at RVA `0x1DA588D`.
- Live disassembly of the world-query helper at RVA `0x252EDF0` onward, including its accesses through caller-provided output/context pointers, two local point constructions, and two calls to the point-in-bounds predicate at RVA `0x25267E0`.
- Live disassembly of a separate point-in-bounds predicate at RVA `0x25267E0`; its relation to the helper's direct endpoint calls is not established.
- Live disassembly of the `HeightOverGround` callsite at RVA `0x1CA372C`, plus read-only live-process reads confirming the argument-5 data source at `0x14125D4A8` currently contains `0.0f`.
- Live disassembly of `CForcePulse::CheckPlayerRaycast` at return RVA `0x2050E4E` and `CRayCasterObject::CastBulletRay` at return RVA `0x20777E9`; their runtime calls have not yet been observed.
- Live disassembly of the `CRayCasterObject::CastBulletRay` success continuation, showing copies from world-query output offsets `+0x00`, `+0x08..+0x10`, and `+0x14` into its constructed return object.
- Live disassembly of `CWeapon::GetRayWorldPosition` at return RVA `0x1E1BE5B`, showing output `+0x14` multiplied by slot 5 and used with the `R8` origin/direction block to compute the result position.
- Live disassembly of the `CAmmunition::CheckCollisions` continuation after its query call, showing that its receiver is retained in `RBX`, the computed caller position is stored into receiver fields `+0x70..+0x7C`, and the receiver dispatches through vtable entry `+0x58` or `+0x60`; the general path passes the full output record and query context to the `+0x60` callback. The current Rust parity port has no `CAmmunition` interface declaration, so its vtable slots cannot be named from the pinned SDK headers. The new candidate captures `RBX`, the other nonvolatile registers, and the two vtable entries for this caller, but their runtime values and method behavior remain unobserved.
- Read-only live-process read of `CWeapon::GetRayWorldPosition`'s `XMM7` initializer source at `0x14122F8D8`, which currently contains `1.0f` and is used as the output scalar's pre-call sentinel and post-call comparison threshold.
- Live disassembly of `CRayCastCondition::Evaluate`, `GraphScriptRaycastStatic`, and `CRayCastFanCondition::Evaluate`, identifying their stack-argument profiles and scalar construction. The `CRayCastFanCondition::Evaluate` return site `0x1C284DA` appears in the unique-caller list but was not sampled in detail; its second callsite and the other listed profiles are absent from the current caller list.
- Read-only live-process snapshot and disassembly of `GraphScriptRaycastStatic`, RVA `0x1EAC0C0..0x1EAC3D5`, recovering its vector-to-ray construction, computed length, slot-10/11 values, output-fraction branch, and outer-wrapper selector branch. A separate live read of `0x14122F8D8` returned bytes `00 00 80 3F` (`1.0f`). The current game log has not recorded this caller in this run, so all of these are static live-image findings pending a runtime sample.
- Read-only `VirtualQueryEx`/`ReadProcessMemory` scan of executable pages in the live image, finding direct relative calls to RVA `0x252EDF0`; `.pdata` ranges came from the local PE exception directory.
- Read-only scan of the live executable's script registration area at RVA `0x1E71CF0`, plus bounded reads of the `RaycastStatic` and `RaycastStaticFromCamera` function entries and their immediate jump veneer.
- Live disassembly of the `RaycastStatic` lookup adapter at RVA `0x1E6AB20` and its nested routine at RVA `0x14300B8D0`, showing the wrapper's `0x8325AE7C` constant is forwarded as a 32-bit key/selector input. Its semantic name remains unresolved.
- Read-only disassembly of the SDK's `HashString` address candidate (`0x142FBD230`) and its target (`0x142FBC960`), matching Bob Jenkins `lookup3` `hashlittle` structure with zero seed. Subsequent read-only checks found seven matching built-in type-name hashes in the live type-tag globals (`float`, `vector`, `bool`, `int`, `int64`, `uint32`, `uint64`), which strongly corroborates the implementation; `HashString` itself was not invoked in-process.
- Live disassembly of the GraphScript float-tag predicate at RVA `0x3005F50` and its two value retrieval/fallback helpers at `0x3005D90` and `0x30068B0`, plus a live read of type-tag data at VA `0x141A01F04`. The predicate compares the value's `+0x04` tag against `HashString("float")`; the GraphScript body loads the selected/fallback `f32` into `XMM10`.
- Read-only disassembly of the `RaycastStatic` lookup adapter at `0x14300B8D0` and its nested 32-byte-record lookup at `0x143006810`, establishing the `0x8325AE7C` value is used as a record key but leaving its label/type unresolved.
- A full scan of printable ASCII runs (3–160 bytes) in the local 76,814,792-byte `MadMax.exe`, hashing each with the recovered zero-seed lookup3 implementation, found no matches for the four observed opaque keys `0x2827198C`, `0xB773FFCF`, `0x5C26F215`, and `0x8325AE7C`. The scan implementation was cross-checked against `float` (`0x4E0A188B`) and `uint32` (`0xE9F00A0C`). A repeat over 98,729,984 bytes of readable committed pages in the live process also found no printable-ASCII labels hashing to those four values. These scans only rule out matching labels stored as printable ASCII in the scanned executable ranges; keys may originate in packed resources, non-ASCII/encoded data, or generated tables.
- Follow-up read-only scan of committed readable pages in the live MadMax image range `0x140000000..0x148000000` found two occurrences each of `0x2827198C` and `0x5C26F215`, one of `0x8325AE7C`, and none of `0xB773FFCF`. A bounded live read at `0x141EAC3F0` independently shows the `RaycastStatic` adapter loading `0x8325AE7C` into `R8D` before its resolver call, and later loading `0x5C26F215` and `0x2827198C` into its stack-built helper arguments. Their exact field semantics are still unknown. The live-memory scan reconciles the absence of these constants from the raw disk image: the loaded image contains runtime bytes not found by the corresponding file scan. Absence of `0xB773FFCF` is limited to the scanned readable pages and does not establish that it is unused.
- Cross-check against the local SDK port: `src/api/game/abi/game/object.rs` declares `CGameObject::GetTransform` at candidate slot 22 and a matrix-output signature; `src/api/game/abi/types.rs` defines a 64-byte `Matrix4x4` with translation in row 3. This corroborates the wrapper interpretation but does not validate the vtable slot for the executable.
- Read-only module enumeration confirms the `RaycastStaticFromCamera` relay's indirect target lies in the loaded `MadMaxModding.asi`; bounded live reads show the relay and target. The installed plugin file hash is `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`, matching the current-run record.
- Current implementation: `src/game_probe.rs`, world-query tap, per-thread return pairing, and background sample logger.

## Interpretation limits and next session

Raw words remain unlabeled. For the next game session, capture shotgun queries during known surface impacts on different materials and cases where the ray clearly passes without impact, retaining the first 32 ammo calls. Compare these gameplay-known outcomes separately from the helper Boolean return. Compare output-record fields across hit/miss and surface changes; map stable pointer fields to their owning objects only after checking their readable memory and lifetime. Preserve the full raw snapshot beside any decoded view. Do not infer a general-purpose public raycast API from this single ammo call site.
### Candidate refresh

On 2026-10-28 at 04:43 local time, the release candidate was rebuilt after changing only the diagnostic labels for `GraphScriptRaycastStatic`: its triples are now logged as raw `vectors=` and slot 5 as `slot5-scale=`, avoiding an unsupported origin/direction or segment-length interpretation. The resulting DLL is 1,787,904 bytes with SHA-256 `2C1EF343BFCE8B88AD1E7FD90E12AF76D4BA8B9D289EBF363804DF4FFF548DEB`. `cargo fmt --check`, `git diff --check`, and `cargo build --release` passed. The live process and installed plugin were left untouched; this candidate still requires a later user-started game session for runtime verification.

### XMM capture extension

The next rebuild expands only the already-bounded sample record: the naked tap now copies all ten Win64 nonvolatile SIMD registers (`XMM6`–`XMM15`) into its fixed record, and the background logger prints them by register name. This specifically captures the float-typed `XMM10` lookup result used in the GraphScript scalar calculation. The frame is `0x1E8` bytes and the saved-register record is `0x1B8` bytes; compile-time assertions check the saved record size and the inline-assembly offsets for stack arguments and nonvolatile registers. Existing sample caps are unchanged. `cargo fmt --check`, `git diff --check`, and `cargo build --release` passed. The current DLL is 1,789,440 bytes, SHA-256 `3A155C3710A13AE8F5E4FB0849ECFC86F4820BCFB067A9D06E1E7DB25A16BD95`. It remains uninstalled; the live process is still PID `20880` and the log remains 19,086 bytes at its 02:26 timestamp.

### GraphScript input snapshot extension

The next bounded capture also records the first `0x20` raw bytes behind each original GraphScript argument pointer. Static disassembly shows the wrapper preserves its original `RCX` argument in `RDI` and original `RDX` in `RSI` through the shared helper call; the tap already saves both registers. For only the existing first 16 `GraphScriptRaycastStatic` samples, the hook checks that each 32-byte span is readable, saves four raw 64-bit words per argument, and logs the pointer plus words. It does not decode a type or value layout. This may reveal the runtime type tags and operand payloads needed to name the opaque wrapper keys while keeping the hook's work bounded and avoiding a new detour.

`cargo fmt --check`, `git diff --check`, and `cargo build --release` succeeded after the change. The candidate DLL is 1,791,488 bytes with SHA-256 `05D6AB9E1EEB788803F66F08E410A9E3D521BF9CAFCAF0F968D33C142F77D598`. The game remains PID `20880`, the installed plugin remains SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`, and the capture log has not advanced since `2026-10-28 02:26:57 +01`. This candidate is built but uninstalled and has not been runtime-verified.

### Return-status label correction

A review of the caller consumers showed that the shared helper Boolean must not be equated with a physical collision without more evidence. `CWeapon::GetRayWorldPosition` treats output `+0x14 == 1.0f` as a sentinel and only constructs a position when that scalar is below `1.0f`; `CAmmunition::CheckCollisions` computes a position from `output +0x14 * ray_length` on a true helper return, while the false path uses the full segment endpoint. The false path can also follow a zero return from an endpoint-associated call whose semantics are unresolved, so “false” does not yet prove an ordinary geometric miss. The candidate's ammo diagnostic now emits the raw Boolean separately from `output +0x14`, scalar-times-ray-length, the caller-computed position or full endpoint, the pointer at output `+0x28` used for dispatch, and the pointed-to byte at `+0x10`. Labels no longer assert “hit” or “miss.” The byte label explicitly identifies its base as the dispatch pointer.

### Per-caller activity summary extension

At 05:19 local time, the candidate was extended with a fixed `AtomicU64` counter for each of the existing 256 caller-table slots. The tap performs one relaxed increment per query call. The existing background worker compares counters in five-second windows, skips empty windows, and writes one compact line with the total observed registered-site calls, number of active callers, and top twelve caller RVAs by count. Sorting, formatting, and log writes stay off the game thread; detailed sample caps and return capture are unchanged. This measures caller frequency but does not infer ray hits or count results for unsampled calls.

The release candidate is 1,808,896 bytes, SHA-256 `ECC1EC3599214C9B8C088B0A2949F1375188F639D256BD16C3EF844B63FC8830`. `cargo fmt --check`, `git diff --check`, and `cargo build --release` completed successfully. The game remains PID `20880`, responsive, with the older plugin loaded; the existing log remains 19,086 bytes and has not changed since 02:26:57 +01. This candidate is not installed or runtime-verified.

### Live-state revalidation (2026-10-28 05:21 +01)

Mad Max PID `20880` was still responsive with `MadMaxModding.asi` loaded from the Steam game `scripts` directory. The log remained 19,086 bytes with 16 detailed samples from two callers and its last write at `02:26:57 +01`; no new query behavior was observed. A fresh read-only `trace_xref_probe.exe 20880 --peek-any 14122F8D8 4` read bytes `00 00 80 3F` at the live image address `0x14122F8D8`, reconfirming the `1.0f` comparison/sentinel value used by the `CWeapon::GetRayWorldPosition` and `GraphScriptRaycastStatic` paths. This confirms the global value is still present in the running process; it does not reveal the helper's current output `+0x14` or prove collision semantics.

The revised candidate passed `cargo fmt --check`, `git diff --check`, and `cargo build --release`. It is 1,791,488 bytes with SHA-256 `823E4B644BCFE74C239F5F31ACE65660CDE74CC38981A15C84ACA6194E3FE810`. Mad Max remains responsive as PID `20880`; the log is still 19,086 bytes (last write 02:26:57 +01), and the installed plugin hash remains `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. The revised candidate remains uninstalled and has no runtime observations yet.

### Raw output pair cross-check (2026-10-28 05:25 +01)

A read-only decode of the eight existing `CAmmunition::CheckCollisions` samples confirms their first six `R8` floats are ordered as origin XYZ followed by direction XYZ. All eight helper return bytes are `1`. For the pair aliased at ray-block byte offset `0x58`, decode the qword as two little-endian `f32`s: the low 32-bit word is output `+0x08`, and the high word is output `+0x0C`. This memory-order detail matters because the log prints each qword most-significant half first.

| Sample | Output `+0x08` | Output `+0x0C` |
|---:|---:|---:|
| 8610 | -0.050183 | 0.997200 |
| 8611 | -0.062145 | 0.997738 |
| 8612 | -0.050183 | 0.997200 |
| 8613 | -0.091132 | 0.995737 |
| 8614 | -0.058421 | 0.997049 |
| 8615 | -0.042389 | 0.997127 |
| 8616 | -0.068529 | 0.997134 |
| 8617 | -0.050183 | 0.997200 |

In all eight records the pair changes from `(1.0, 0.0)` before the helper to the values above afterward. The helper Boolean is not confirmed to mean physical collision, and the two output floats have no established geometric meaning. This is a reproducible output mutation correlated with the eight queries, not a hit-normal or hit-position claim. The candidate already snapshots the separate output record, so a later candidate run can compare these bytes with the other output fields and gameplay-known controls.

Additional numeric check: the 2D magnitude of this pair ranges from `0.9980276` to `0.9998988` (mean `0.9989035`) across the eight samples. This is consistent with two components of a near-unit vector, but could also be another correlated pair of normalized values. The third component and neighboring fields must be inspected in the full output-record snapshot before assigning a vector or surface-normal interpretation.

### Old stack-address live-read check (2026-10-28 05:28 +01)

The current process is still PID `20880`. Read-only peeks at the stack addresses logged by sample `#8617` (`R8=0x5F88B0`; the caller's output record is at `R8+0x50`, `0x5F8900`) succeeded, but the bytes now differ from the recorded query inputs/output. The addresses remain readable, yet their contents have been reused or otherwise changed since the call; they are not a live query result and must not be interpreted as one. This confirms that output and context must be copied synchronously by the in-process tap while the caller frame is active. The current installed plugin does not capture those records; the uninstalled candidate does.


### Live Havok candidate collector path (2026-10-28 05:34 +01)

A read-only Capstone disassembly of the currently running image in PID `20880` confirms the shared game's world-query helper at RVA `0x252EDF0` submits a one-ray `hkpCastRayInput` to the live `hkp3AxisSweep::castRay` implementation (vtable slot `+0xC8`). The input sets `m_numCasts = 1`, supplies the transformed start in `m_from`, points `m_toBase` to the transformed endpoint, and sets `m_toStriding = 0x10`. This Havok broadphase operation enumerates AABB candidates; by itself it is not evidence of a shape-level hit.

The broadphase collector object has RTTI name `hkProxyRayCaster`. Its `addBroadPhaseHandle(handle, castIndex)` callback (RVA `0x2526AA0`) uses the handle's signed byte-relative object pointer, applies object/type gates (`[object+0x28] == 2` and a virtual `+0x30` result of `3` on the primary path), transforms the ray endpoints into object-local coordinates using object data at `+0x140..+0x170`, and invokes that object's virtual method at `+0x80` with the per-ray output record. It then returns the record's float at `+0x08` as the collector's hit fraction. Rejected candidates go through fallback RVA `0x2B0DC60`, which also transforms the ray to object-local coordinates and invokes virtual `+0x30`; the callback result is again read from per-ray output `+0x08`. The precise meanings of those object virtual methods and the fraction's edge cases are still unresolved.

The fallback function copies fields into the adapter's stack-local collector (`caster`): transformed vector to `caster+0x40`, the per-cast ray-input dword at `[caster+0x08] + 0x30*castIndex + 0x24` to `caster+0x50`, the handle-relative candidate pointer to `caster+0x60`, and the ray-input qword at `+0x28` to `caster+0x68`. Separately, the shared helper retains caller argument 7 in `RDI` and checks `caller_context+0x60` after the adapter returns. These are different base addresses: the collector is the local at helper-frame `RBP-0x70`, while caller context is the pointer supplied in stack argument 7. The inspected code has not established a store/copy from collector `+0x60` to caller-context `+0x60`. The helper's Boolean and output sentinel/copy branch are determined by the caller-context field; the collector candidate pointer alone does not explain that result. Neither Boolean nor callback fraction alone proves a physical collision. The next trace must snapshot both collector fields and caller-context bytes under the same sample ID, then find the missing bridge if they change together.

These findings were obtained by `OpenProcess(QUERY_INFORMATION | VM_READ)` and `ReadProcessMemory` against the existing process only. No thread was suspended, no game memory was written, and the process was neither closed nor restarted. At the end of the inspection PID `20880` remained responsive; the old installed plugin and unchanged log were still in place. The newly built capture candidate remains uninstalled. The old stack sample addresses are reused, so any further output semantics work must use the candidate's synchronous snapshots from a future run.

### Initial per-candidate callback capture candidate (2026-10-28 05:42 +01; superseded)

The next capture candidate adds a signature-checked tap at the live `hkProxyRayCaster::addBroadPhaseHandle` entry (RVA `0x2526AA0`). For a sampled query already on the existing thread-local return stack, it snapshots the per-ray record (six qwords / `0x30` bytes) immediately before and after the original callback, records the callback's returned `hkReal` fraction, cast index, handle and handle-relative object pointer, object/subobject vtables, and the virtual targets at `+0x30` and `+0x80`. These are raw observations: the callback fraction is not yet labeled as a physical collision result, and the raw record fields remain undecoded.

That initial revision retained at most eight records per sampled query: the first three callbacks in slots 0–2, a rolling window of the latest four in slots 3–6, and the lowest finite in-range fraction in slot 7. The returned-fraction slot could therefore retain a candidate from the middle of a long callback sequence. It forwarded the original callback's `f32` return unchanged. The later hot-path audit established that this revision bounded retained records but not deep-read work; it has now been superseded by the fixed-ordinal revision documented below.

The release candidate passed `cargo fmt --check`, `cargo build --release`, and `git diff --check`. Its SHA-256 is `FA005BF54475029DE7E3B771BA8F493C9608F2CE39C1FFF5E2E32B259AC4C6D8` (1,819,136 bytes). It remains uninstalled and has no runtime evidence. Mad Max remains PID `20880`, responsive; the installed ASI remains SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`, and the log is still 19,086 bytes with last write at `02:26:57 +01`. Thus the current process has not loaded this callback tap, and all recorded fraction/record semantics still need a future capture run.

### Live caller-side argument and result semantics (2026-10-28 05:47 +01)

Read-only live disassembly around the observed return addresses recovered three caller labels from their RDX constants and the immediate consumer logic:

| Caller RVA (return address) | RDX label | Input setup | Caller-side result use |
|---|---|---|---|
| `0x1DA5892` | `CAmmunition::CheckCollisions` | R8 points at a stack ray block with origin XYZ then direction XYZ; XMM3 is XMM7; stack arg 5 is XMM6 (segment length); stack args 6/7 point to output/context at `RBP-0x50` and `RBP`. | Tests AL. On true, computes `origin + direction * (output+0x14 * segmentLength)` and passes that position into subsequent collision processing. On false, uses the full segment endpoint `origin + direction * segmentLength`, then rejoins common post-processing. This is strong evidence that this caller treats true as a usable query result and false as no usable result; it does not establish a universal helper Boolean contract, since the helper also has an earlier bounds-failure path. |
| `0x1CA3731` | `HeightOverGround` | R8 points at a stack ray block; XMM3 is XMM9; stack arg 5 is XMM8 loaded from a global float; args 6/7 point at output/context at `RBP` and `RBP+0x50`. | Does not test AL here. It compares context `+0x14` to XMM7, then checks context `+0x28` and a byte at pointed-to `+0x10` before selecting later logic. It then uses the scalar again for threshold/range computation. |
| `0x1EAC351` | `GraphScriptRaycastStatic` | R8 points at a stack ray block; XMM3 is XMM9; stack arg 5 is XMM6; args 6/7 point at output/context at `RBP-0x20` and `RBP+0x30`. | Does not test AL here. It compares output/context scalar at `RBP-0x0C` (output `+0x14`) to XMM11, then consults script-side data and invokes further query/dispatch logic. |

The ammo caller's query data is a start point and direction, and its segment length is a separate stack argument. The `CAmmunition::CheckCollisions` descriptor is the actual RDX string at VA `0x14125D848`; the other strings are `HeightOverGround` at `0x14124D888` and `GraphScriptRaycastStatic` at `0x14125D458`. The function labels are observed binary strings, not inferred names.

This refines the earlier output interpretation: helper return AL is consumed as a hit-like usable-result gate by the ammo caller, while the HeightOverGround and script callers use the output scalar threshold and other context fields instead. Keep the exported Rust API conservative until the new snapshots establish how the context scalar behaves across true, false, and known scene controls. The candidate callback fractions and output vectors are still not semantically decoded.

### Existing sampled ray vectors, decoded with helper offsets (2026-10-28 05:49 +01)

The live helper disassembly reads exactly six consecutive `f32` values from R8: origin XYZ at offsets `0x00`, `0x04`, `0x08`, then direction XYZ at `0x0C`, `0x10`, `0x14`. It forms two segment endpoints as `origin + direction * XMM3` and `origin + direction * stack-argument-5`; therefore the query origin/direction are distinct from the two range/scaling inputs.

Decoding the eight captured `HeightOverGround` samples (`#7215`, `#7218`, `#7220`, `#7224`, `#7229`, `#7236`, `#7240`, `#7248`) yields direction `(0, -1, 0)` for all eight, consistent with a downward height probe. Seven recorded helper returns were `1`; sample `#7218` returned `0`. The caller's live code reads output/context `+0x14` and does not branch on AL at the immediate callsite, so this status variation alone does not establish whether the probe was a geometric miss or a bounds/filter path.

The eight captured `CAmmunition::CheckCollisions` samples (`#8610`–`#8617`) have a common origin around `(-3940.4, 477.3, -1514.0)`, with the direction changing by sample and magnitude between `0.999917` and `1.000000` (six-decimal rounded). Each recorded helper return was `1`. This confirms the ammo query block's first six floats decode cleanly as origin and normalized direction. The older log still lacks the external output/context snapshot and is not current live state; the new candidate is intended to capture those synchronously on a future run.


### Havok collector contract cross-check and historical candidate (2026-10-28 05:49 +01)

The public Havok `hkpBroadPhaseCastCollector` header declares `addBroadPhaseHandle` as returning `hkReal` and describes that return as the fraction where the primitive hits the collidable; it also says the fraction is updated within `[0,1]`. The `hkpBroadPhase` header says `castRay` calls the collector for each overlapping AABB. These are interface-contract facts from a third-party public mirror of old Havok SDK headers, not Mad Max source. They support the candidate callback's fraction return and the capture's finite `[0,1]` summary filter, while leaving the game's concrete sentinel/filter behavior unverified. References: [hkpBroadPhaseCastCollector.h](https://raw.githubusercontent.com/nitaigao/engine-showcase/master/etc/vendor/havok/Source/Physics/Internal/Collide/BroadPhase/hkpBroadPhaseCastCollector.h), [hkpBroadPhase.h](https://github.com/nitaigao/engine-showcase/blob/master/etc/vendor/havok/Source/Physics/Internal/Collide/BroadPhase/hkpBroadPhase.h).

The historical candidate then reserved one of its eight records for the lowest finite fraction in `[0,1]`, while keeping three early and four recent callbacks. That improved retention but did not bound metadata reads; the fixed-ordinal implementation below supersedes it. The historical release candidate passed `cargo fmt --check`, `cargo build --release`, and `git diff --check`; SHA-256 `C3F9ED0E739131D41E3B40FC3C564AA8038A087E0A4840D94D8D2CF48B303F24`, size 1,816,576 bytes. It was not installed or runtime-verified.


That historical candidate also retained the first valid 1.0 callback fraction when no closer fraction was observed, so a sequence with only no-hit/endpoint candidates still left one record. Its build SHA-256 was C3F9ED0E739131D41E3B40FC3C564AA8038A087E0A4840D94D8D2CF48B303F24.

### Provisional interpretation of the ammo output vector

The old ammo samples' output pair at `+0x08/+0x0C` has near-unit 2D magnitude (`0.9980`–`0.9999`). In those same samples, the input direction is mostly toward negative Z with only a small negative Y component, while the pair's second component is near `+1`. Given the game's observed downward `-Y` height-probe convention, this is consistent with a surface normal pointing mostly upward, so `output+0x08..+0x10` is a useful surface-normal hypothesis. It is still unconfirmed: the third float at `+0x10` and the candidate object's identity were not present in the old capture. Generic Havok shape-ray outputs do contain a normal and hit fraction, but their standard field offsets do not by themselves establish the layout of Mad Max's custom output record; the new capture deliberately preserves raw bytes for comparison. Reference: [CommonLibSSE-NG hkpShapeRayCastCollectorOutput layout](https://ng.commonlib.dev/hkp_shape_ray_cast_collector_output_8h_source.html).

### Havok world-ray-caster contract cross-check (2026-10-28 05:54 +01)

The public Havok `hkpWorldRayCaster` header describes the class as the adapter from broadphase ray traversal to `hkpShape::castRay`; its `addBroadPhaseHandle` implementation is the virtual callback used by that adapter. Its documentation distinguishes a callback for narrowphase objects from the broadphase's initial AABB enumeration. This supports the observed Mad Max callback's two-stage shape: the game's helper first enumerates broadphase candidates, while the callback then transforms into object-local coordinates and invokes an object virtual. The return value at that callback is contractually a fraction, not a boolean. Sources: [hkpWorldRayCaster.h](https://github.com/nitaigao/engine-game-factions/blob/master/etc/vendor/havok/Source/Physics/Collide/Query/CastUtil/hkpWorldRayCaster.h), [hkpBroadPhaseCastCollector.h](https://raw.githubusercontent.com/nitaigao/engine-showcase/master/etc/vendor/havok/Source/Physics/Internal/Collide/BroadPhase/hkpBroadPhaseCastCollector.h), [hkpShape.h](https://github.com/nitaigao/engine-showcase/blob/master/etc/vendor/havok/Source/Physics/Collide/Shape/hkpShape.h).

This remains corroborating SDK documentation, not proof that Mad Max's RTTI class `hkProxyRayCaster` is the same concrete class as `hkpWorldRayCaster`, nor that the game's custom output record has the standard Havok layout. The binary-derived facts remain narrower: Mad Max's helper invokes a broadphase `castRay`; its collector callback runs per candidate, transforms the segment to local space, calls the candidate's virtual method, and returns the candidate record's `+0x08` float. The next candidate run should correlate that raw fraction and record with caller output/context for controlled clear-ray and surface-ray scenarios before assigning hit, normal, shape, or material semantics.

No code or game state was changed in this research pass. Mad Max PID `20880` was still responsive. The old log remained 19,086 bytes with last write `02:26:57 +01`; the installed plugin remains unchanged, and the candidate callback-capture DLL remains uninstalled.

### Candidate object gate and local-transform capture (2026-10-28 05:58 +01)

The bounded callback sample now additionally snapshots the candidate object's raw 64-bit word at `object+0x28` and six raw 64-bit words spanning `object+0x140..+0x170`. The first includes the gate tested by the callback (`[object+0x28] == 2` in the primary branch); the second covers the observed 0x30-byte object-local transform region used to transform ray endpoints before the object's virtual query. Each span is checked with `readable_span` immediately before copying, and the log says `unreadable` if validation fails. Values remain raw hex: no type, matrix convention, or coordinate meaning is assumed. This uses the existing callback hook and per-query cap; it adds no new detour and does not read old/reused stack addresses.

Fresh verification after this change: `cargo fmt --check` passed; `git diff --check` passed; `cargo build --release` finished successfully. No tests were run. The new uninstalled release DLL is 1,819,648 bytes, SHA-256 `E1413EA61A30642B83CE50430C28268939176770B88DA7A13D67B17045F8A87E`. The installed ASI remains the older SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. Mad Max PID `20880` remained responsive; the live log stayed at 19,086 bytes, last written `02:26:57 +01`. This capture candidate has not been installed or runtime-verified.

### Standard Havok output-layout mismatch (2026-10-28 05:59 +01)

A reverse-engineered Havok layout reference for `hkpShapeRayCastCollectorOutput` places the 16-byte normal at offset `0x00` and `hitFraction` at `0x10` (with `extraInfo` at `0x14` and `shapeKey` at `0x18`). The Mad Max callback's binary-derived implementation returns a float read at `perRayRecord+0x08`. These offsets do not line up if the captured per-ray record is assumed to begin at the standard output struct. This does not contradict the broadphase collector's `hkReal` fraction contract: the passed record may be a wrapper or may use a different base/layout. It does rule out assigning the standard Havok output fields by analogy alone. Keep the complete 0x30-byte before/after raw record and caller output/context snapshots as the evidence needed to resolve the base and field mapping. Reference: [CommonLibSSE-NG hkpShapeRayCastCollectorOutput layout](https://ng.commonlib.dev/hkp_shape_ray_cast_collector_output_8h_source.html).

### Candidate base-address disambiguation extension (2026-10-28 06:02 +01)

Reviewing the callback pointer arithmetic showed three distinct bases in the tap: the address produced by the handle's signed relative offset (`handle_relative`), the pointer read from that address (`candidate_object`), and the derived subobject address (`object_subobject`). The disassembly notes describe fields as `object+0x28` and `object+0x140` but do not yet establish which of those three bases they mean. To avoid encoding that naming assumption, the next candidate now snapshots the raw qword at `+0x28` and six qwords at `+0x140` for all three bases. Each region is independently guarded by `readable_span`; each is labeled by its actual base in the log. The existing three early/four recent/one lowest-fraction record policy remains in place.

Fresh verification: `cargo fmt --check`, `git diff --check`, and `cargo build --release` succeeded. No tests were run. The expanded uninstalled DLL is 1,824,768 bytes with SHA-256 `C377F0EB229DA3A89112265B7FCADF8F334C1BAF404CC5CCC1B08E7E4E061642`. Mad Max is still responsive as PID `20880`; the installed ASI remains `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`; the live log remains 19,086 bytes, last modified `02:26:57 +01`. The new binary is not installed and has no runtime observations.

### Live direct-callsite inventory revalidation (2026-10-28 06:05 +01)

A fresh read-only scan of the running PID `20880` found 55 RIP-relative references to the world-query helper. For each candidate displacement, a one-byte `ReadProcessMemory` peek at the preceding address found opcode `E8` (direct near call). The return RVA is therefore the displacement address plus four bytes; the resulting 55 RVAs exactly match the documented caller inventory above. This independently validates that the table's 55 entries are actual direct callsites in the currently running Steam image, rather than merely byte-pattern candidates. It does not cover indirect calls or references from other modules.

The live logger still has no new detailed samples: `bootstrap.log` remains 19,086 bytes with last write `02:26:57 +01`; the running game remains PID `20880`, responsive. The installed ASI is still the old build without the candidate callback tap. The current uninstalled capture candidate remains SHA-256 `C377F0EB229DA3A89112265B7FCADF8F334C1BAF404CC5CCC1B08E7E4E061642`.

### Live profile for previously unresolved caller at 0x252F240 (2026-10-28 06:06 +01)

A fresh read-only peek of the live Mad Max image around RVA `0x252F23B` confirms a direct call to the world-query helper, returning at `0x252F240`. The surrounding instruction bytes decode to this Win64 argument setup:

- `RCX = RBP`
- `RDX = RSI` (forwarded; no label string or type identity has been established)
- `R8 = R12`
- `XMM3 = XMM6`
- stack argument 5 = `f32 [RSP+0x170]`
- stack argument 6 = `RDI`
- stack argument 7 = address `[RSP+0x70]`

The plausible `.pdata` function range containing the call is RVA `0x252F170..0x252F26B` (unwind info RVA `0x16956B0`). The short range is structurally consistent with this callsite; an unrelated malformed-looking large range emitted by the exploratory exception-table parser was discarded. The captured argument values should be treated as opaque until this callsite executes under the candidate tap. In particular, this profile does not establish whether `R12` points to the ray block or another query structure.

The bytes were read from PID `20880` using the existing read-only process handle. The game was not suspended, modified, closed, or relaunched. The installed plugin and its old log remain unchanged; the expanded capture DLL remains uninstalled.

### Static argument-shape survey for previously unobserved callsites (2026-10-28 06:20 +01)

A temporary Capstone decoder was run against the live executable's read-only image pages, using each call's `.pdata` function start to disassemble through the call boundary. The pass covered the 41 direct helper calls absent from the prior run's unique-caller log. It decoded setup windows reliably for 38 sites. Three (`0x1CC6257`, `0x1CB8C52`, `0x1D1B6C8`) still have verified direct-call bytes but the linear decode from the selected `.pdata` function start did not land on the instruction boundary at the call; their argument setup is left unresolved rather than guessed. This is a static disassembly limitation, not evidence that those calls are absent.

Across the 38 decoded callsites, the common layout is stable: Win64 slots 6 and 7 point to caller-owned output and context records, while R8 normally points to a caller-owned ray/query block. The caller varies slot 5 (range/scale input), XMM3, and helper options in slots 8–11. Many initialize option slots with constants or register values; some set slots 8 and 9 to one. The previously studied `CRayCastCondition::Evaluate` site passes a dynamic 32-bit slot-10 value from `[RSI+0x3184]`; the shared helper consumes slot 10 in its index construction, but this field's game meaning remains unknown. `CWeapon::GetRayWorldPosition` passes R8 directly from RDI rather than taking a nearby local block. The unresolved `0x252F240` profile passes R8 from R12. Thus the next tap's raw argument/register capture is important: a universal assumption that every R8 points to the same six-float ray layout would be unsafe until each active caller is sampled.

The scan confirmed that these are shared-helper callsites across character, weapon, script, interaction, weather, vehicle, and effect code (caller labels are catalogued above). It did not add hooks or read/write game memory. The installed game process was only queried/read; it was not suspended, modified, closed, or relaunched. The uninstalled capture DLL remains untested in-game.

### Backward-aligned recovery of three remaining caller profiles (2026-10-28 06:24 +01)

The three callsites that did not decode from the `.pdata` function start were re-read from the live image and disassembled backward from the verified `E8` call boundary. The resulting instruction sequences contain the complete contiguous helper setup, including all stack argument stores, so the three sites no longer need to be treated as opaque:

| Return RVA / caller label candidate | R8 query input | Slot 5 | XMM3 | Output / context (slots 6–7) | Slots 8–11 at call |
|---|---|---|---|---|---|
| `0x1CC6257` / `character_detection` | `[RSP+0x70]` | XMM8 | XMM6 | `[RBP-0x20]` / `[RBP+0x30]` | 8=0, 9=0, 10=EDI, 11=RDI |
| `0x1CB8C52` / `CIsInDistanceOfObject::Evaluate` | `[RSP+0x60]` | XMM0, computed by `sqrtss` from caller values | not assigned in the contiguous setup window | `[RBP+0x80]` / `[RBP+0xD0]` | 8=0, 9=0, 10=R13D, 11=R13 |
| `0x1D1B6C8` / `Selector` | `[RSP+0x70]` | XMM9 | XMM7 | `[RBP-0x20]` / `[RBP+0x30]` | 8=0, 9=0, 10=ESI, 11=RSI |

This completes argument-setup recovery for all 55 verified direct callsites, although some query-data semantics and a few earlier register values remain unknown. The three sites above demonstrate why the candidate snapshots the optional argument-11 pointee rather than assuming null: each supplies a register-derived value. Static dataflow does not yet prove those runtime values are valid pointers, and the helper checks argument 11 before dereferencing it. The sample capture's guarded pointee read is therefore needed to establish whether these callers exercise that path.

The backward-alignment search only read the live process image. No game code or data was modified; no hooks were added to the running process. The prepared capture candidate already records the relevant raw slots and performs a readable-span check before copying an argument-11 pointee.

### Reference scan boundary check (2026-10-28 06:28 +01)

Rebuilt and ran the existing read-only `trace_xref_probe` against PID `20880`, retaining the full output temporarily. It found 55 RIP-relative references to `LowerCameraQueryHelper` and zero absolute-pointer references to that helper in the main image. The 55 RIP-relative references are the direct E8 callsites already catalogued above (each preceding opcode was separately verified as E8). The probe's total 63 includes references to other ray/script wrapper names; it is not an additional eight world-query calls. This scan therefore found no static function-pointer-table or absolute-pointer route to the shared helper in Mad Max.exe. It does not rule out references in other loaded modules, runtime-created function pointers, or calls through a different wrapper.

The scan requested only process query/read rights and did not suspend or modify the game. PID `20880` remained responsive at the last check; the installed ASI hash and old log were unchanged. The expanded capture candidate remains uninstalled.

### Dynamically forwarded query label at caller 0x1FB0895 (2026-10-28 06:34 +01)

Full read-only disassembly of the enclosing function (`0x1FB0600..0x1FB08FB`) resolves the previously unknown RDX source. The helper receives RDX from R13, and the function initialized R13 at entry from its own incoming stack argument at entry-RSP `+0x40` (Win64 stack argument 8). Thus this caller forwards an outer caller-supplied query-label pointer; there is no fixed label string at this callsite to recover from the executable. The next candidate already copies a bounded printable string from RDX when the call executes, which should reveal its runtime label if the input is a string.

The helper inputs at this call are otherwise now understood structurally: R8 points to a local six-float block assembled from the two incoming 3D points (R14 and R12); slot 5 is a nonnegative distance derived from their separation and an incoming scalar; XMM3 is zero; slot 6 points to local output `[RBP-0x70]`; slot 7 points to local context `[RBP-0x20]`; slots 8–11 are zero. The exact semantics of the two points and scalar belong to the enclosing function and remain unnamed. This is evidence that at least one world-query caller is a reusable wrapper whose diagnostic label is supplied dynamically by its caller, not a compile-time static name.

Only the process image was read. No game state was changed; Mad Max remains PID `20880` and the candidate capture DLL remains uninstalled.

### Exact helper construction of its caller-supplied query selector (2026-10-28 06:38 +01)

The shared helper's stack-frame-adjusted disassembly resolves the option/index formula without assigning a game-level name to it. At helper entry, slot 9 is the byte at `[entry-RSP+0x40]`, slot 10 is the `u32` at `[entry-RSP+0x48]`, and slot 11 is the pointer at `[entry-RSP+0x50]`. The helper constructs a local 32-bit selector as:

```text
selector = (arg10 << 16) + (arg9 == 0 ? 0x12 : 0x10)
if arg11 != null:
    selector = *(u32*)arg11
```

The shift and arithmetic are performed in 32-bit registers. The optional arg11 pointee replaces the constructed selector outright; it is not combined with it. The helper directly reads no stack arg8 in this function body, though that does not exclude a meaning to the caller or downstream routines. This is direct binary evidence of how the option tuple affects the query's internal selector/index. The semantic names of its high and low portions, and the reason for the `0x10`/`0x12` distinction, remain unknown until correlated with active callers and object/query results.

This makes the capture's raw slot 9, slot 10, and guarded arg11-pointee snapshots particularly valuable. The three recovered callers at `0x1CC6257`, `0x1CB8C52`, and `0x1D1B6C8` pass register-derived arg11 values, so their pointee snapshots can test whether they override the base selector. No game state was changed during this analysis.

### Caller-side consumers of the shared output record (2026-10-28 06:45 +01)

Read-only disassembly of representative call continuations confirms that output field `+0x14` is used as a caller-specific scalar, and that the three words at `+0x08..+0x10` and pointer at `+0x00` travel together on the bullet-ray path:

| Caller | Verified post-call use | What this supports (and does not prove) |
|---|---|---|
| `CWeapon::GetRayWorldPosition` (`0x1E1BE5B`) | Reads output `+0x14`, compares with the caller's `1.0f` sentinel, and if lower computes `origin + direction * (fraction * segmentLength)` into the result vector. | Strong support that this consumer treats `+0x14` as a segment position fraction/scale and `1.0f` as the no-earlier-result sentinel. It does not establish universal clamping or identical behavior for all callers. |
| `GraphScriptRaycastStatic` (`0x1EAC351`) | Compares output `+0x14` with its caller threshold and, when lower, invokes a script-side callback using the original script arguments. | The scalar controls a thresholded script path; the downstream script meaning is unknown. |
| `CCharacterModule::CanSee` (`0x20C8899`) | Compares output `+0x14` (`[RBP-0x2C]` for this caller) with a caller-held threshold, then feeds the comparison result into a Boolean sent to a later method. | Supports a thresholded visibility/occlusion decision for this caller. The exact Boolean polarity still depends on the caller's earlier state and full result path. |
| `HeightOverGround` (`0x1CA3731`) | Initializes output `+0x14` to `1.0f`; after the helper, compares that field with `1.0f` and on a lower value inspects the pointer at `+0x28` and its byte at `+0x10`. It does not immediately test helper `AL`. | Independently supports `+0x14` as a no-result sentinel/segment scalar and `+0x28` as a tagged result/descriptor pointer. The caller's final height/result semantics and actual hit status remain unverified. |
| `CRayCasterObject::CastBulletRay` (`0x20777E9`) | On helper-true, creates a result and copies output `+0x08..+0x10` to result `+0x00..+0x0C`, output `+0x14` to result `+0x10`, and output `+0x00` to result `+0x68`. | Confirms these fields are consumed as one vector-shaped payload, one scalar, and one pointer-sized value. The vector and pointer types remain unidentified. |
| `CForcePulse::CheckPlayerRaycast` (`0x2050E4E`) | Immediately gates its continuation on helper AL. | This callsite's immediate continuation uses the Boolean; it does not make the helper Boolean's meaning universal. |

These consumers make `+0x14` the best-supported candidate for a hit/segment fraction, while maintaining the distinction between code-level use and verified geometric semantics. The capture records the output's `0x50` span because observed helper writes end by `+0x4C`; its `0xA8` context snapshots plus candidate callback records are needed to identify the selected object and resolve the vector payload.

### Cross-check against the public SDK (commit `f2dc640`)

The community SDK defines `CAABox` as private minimum and maximum `CVector3f` members in [`math3d.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/math3d.h), and declares `CPhysicsGameObject::GetPhysicsAABB(CAABox*)` in [`game/go/physgo.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/physgo.h). The local Rust port includes `GetPhysicsAABB` in its source-order catalog at [`methods.rs`](../../src/api/game/abi/game/methods.rs) but intentionally represents the physics base as an opaque marker in [`hierarchy.rs`](../../src/api/game/abi/game/hierarchy.rs); its catalog documentation explicitly says declaration ordinals are not numeric vtable slots. The SDK `CAABox` is two 12-byte vectors (24 bytes); the live predicate instead loads min and max from separate 16-byte vectors at `+0x400` and `+0x410`, with each fourth lane repeating Z. These are conceptually similar bounds, not binary-layout-compatible structures, so do not cast the live storage to `CAABox`. This still does not prove the predicate owner's data is a `CAABox`, nor that the helper's output pointer is a `CPhysicsGameObject`. If a captured output/candidate pointer is later identified as a physics game object, and its lifetime and base-subobject offset are understood, `GetPhysicsAABB` could cross-check that selected object's bounds. Do not call it on an unidentified pointer or infer a vtable slot from source order.

### Shared R8 input contract clarified (2026-10-28 07:02 +01)

The helper body itself resolves the six-float layout for every direct caller, including callers whose setup leaves R8 in a register. At entry it reads `f32` values at R8 offsets `0x00, 0x04, 0x08` and `0x0C, 0x10, 0x14`. The first triple is combined with each caller scalar (`XMM3` and stack argument 5) times the second triple to form two endpoints; the helper subtracts its frame-origin vector before the bounds checks. Therefore all 55 calls must provide a readable 24-byte pair of vectors in that layout. The callers differ in where the block lives and in the scalar/option inputs, not in which six input floats the shared helper consumes.

The safe common description is `origin-like vector + direction/displacement-like vector`, with two independent scales/endpoints. Ammo samples show a near-unit second vector, but that does not prove every caller normalizes it. Values beyond the first 24 bytes may be caller-specific data or aliases with other records; preserve them raw until each callsite is sampled. This tightens the earlier caution that a universal six-float interpretation was unproven: the layout is binary-proven, while the vector semantics and normalization are not.

This was established from helper instructions and caller setup in the live executable, not from fresh gameplay samples. The active log is still unchanged and the running process remains untouched.

### Repeat sampling for high-value output/control callers (2026-10-28 07:10 +01)

An earlier uninstalled capture revision recorded up to three invocations for ten callers whose output or control flow is especially useful for distinguishing query outcomes: `CForcePulse::CheckPlayerRaycast`, `CWeapon::GetRayWorldPosition`, `CRayCasterObject::CastBulletRay`, `CCharacterModule::CanSee`, `CRayCastCondition::Evaluate`, both `CRayCastFanCondition::Evaluate` callsites, both `CInteractionClearPathCondition::Evaluate` callsites, and `NodeIsVisibleObject`. The existing caps were 32 ammo, 8 HeightOverGround, and 16 GraphScriptRaycastStatic samples. Every other one of the 55 known direct callsites received one sample. That budgeted 128 samples; the fixed 144-entry array left 16 for unexpected caller addresses. That earlier candidate capped detailed callback records at eight; it was superseded by the fixed seven-ordinal candidate described in the overnight/current status sections below.

This improves the chance of seeing changing caller outputs on repeated activity without allowing a hot caller to fill the whole sample array. It cannot guarantee that three calls represent different geometry or states; runtime interpretation will still require the user to vary the scene and compare logs. The current process is not affected by the rebuilt file.

Verification: `cargo fmt --check`, `git diff --check`, and `cargo build --release` all succeeded; no tests were run. The refreshed candidate DLL is 1,825,280 bytes, SHA-256 `E7DC2CD5E137C7F1E3B929E0E866E9627C0114166CE42516A77D638516E5A807`. It is not installed. The ASI loaded by Mad Max PID `20880` remains SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`; the process remains responsive, and `bootstrap.log` is unchanged at 19,086 bytes, last written `02:26:57 +01`.

### Per-caller capture ordinal in repeat-sample logs (2026-10-28 07:18 +01)

The repeat-sampling policy now writes a zero-based `capture-ordinal` on every detailed sample line. For capped callers this records which allowed capture was selected (for example 0, 1, or 2 for a repeated caller); one-shot callers use ordinal 0. This makes repeated records directly distinguishable without reconstructing their order from the global query sequence or activity windows.

Verification after the change: `cargo fmt --check`, `git diff --check`, and `cargo build --release` succeeded; no tests were run. Candidate DLL: 1,825,280 bytes, SHA-256 `37A71B1B4C2F48309B2DA7C6627E132B484C7B3E0599D2B5A26381C13C7A71EA`. The process is still PID `20880`, responsive, running the unchanged installed ASI hash `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`; the 19,086-byte bootstrap log remains last modified at `02:26:57 +01`. The candidate is uninstalled and has no runtime evidence.

### `CCharacterModule::CanSee` threshold polarity from full caller body (2026-10-28 07:34 +01)

A full read-only disassembly of the caller at return RVA `0x20C8899` resolves the branch sequence. It initializes EBX to zero, loads a scalar constant into XMM8, and initializes output `+0x14` (`[RBP-0x2C]` for this caller) to the same value. A live read of the RIP-relative constant at VA `0x1412328D8` returned bytes `40 3C BB 41`, i.e. `23.4044189453125f`. After the helper, the caller compares output `+0x14` against XMM8; if the output is lower it sets EBX to one. It then uses `sete dil`, so the Boolean delivered to the next method is true when EBX remains zero and false when EBX was set. The exact branch contract at this caller is therefore:

```text
CanSee-result = (output_scalar >= 23.4044189453125f)
```

This supports the interpretation that an unchanged high sentinel means no nearer query result was reported, while a lower value selects the false path. It still does not prove that every lower value is a physical obstacle: the shared helper has a bounds-failure fallback that can write `1.0f`, and other fallback/context paths also affect the scalar. Runtime capture of caller output, helper return, and callback candidates remains necessary to separate clear rays, geometric hits, and rejected/out-of-bounds queries.

The code bytes and constant were read from live PID `20880` using query/read-only process access. The game was not suspended or modified.

### `CCharacterModule::CanSee` caller input is a normalized segment (2026-10-28 07:42 +01)

The live disassembly of the routine at VA `0x141BB0770`, called by `CCharacterModule::CanSee`, confirms it normalizes a three-float vector in place: it loads XYZ, computes the squared length and square root, computes an inverse-length scale, applies a small-vector branch, multiplies XYZ by the chosen scale, and writes the three floats back. In the caller, the vector is the difference between its two endpoint positions. The caller separately computes the Euclidean distance before normalization and passes that distance as helper stack argument 5; XMM3 is zero and R8 points to a block containing the start position followed by this normalized direction. Thus, for this caller, the helper's two endpoints are the start position and start plus normalized direction times the endpoint distance.

This is a concrete caller example of the shared six-float contract: R8 is origin plus normalized direction, while the range is a separate scalar. It supports segment-fraction interpretation for the output on this path but does not assign the vector output at `+0x08..+0x10` or identify every other caller's normalization policy. The helper and vector routine were read from PID `20880` only; no process code or memory was written.

### Overnight handoff: current live/candidate boundary and next capture protocol (2026-10-28)

Read-only revalidation found Mad Max PID `20880` still responsive, with `MadMaxModding.asi` loaded from the Steam `scripts` directory. The active log is still exactly 19,086 bytes and its final record is shotgun sample `#8617`, caller `0x1DA5892`, helper return `1`; it was last modified at `02:26:57 +01`. It contains none of the candidate callback/output/context fields described below. No fresh query observations have arrived since that run.

The installed ASI and the prepared capture artifact are different builds. The candidate `target/release/madmax_bootstrap.dll` is 1,825,280 bytes with SHA-256 `37A71B1B4C2F48309B2DA7C6627E132B484C7B3E0599D2B5A26381C13C7A71EA`; it is not installed and is not the image loaded by PID `20880`. The loaded ASI remains SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. Accordingly, the current live run cannot expose the new taps, and reading more old stack addresses from its old log would not recover the missing output/context state.

The candidate is bounded to 144 helper samples. Its caller policy reserves 32 samples for `CAmmunition::CheckCollisions`, 8 for `HeightOverGround`, 16 for `GraphScriptRaycastStatic`, 3 each for ten callers with useful result/control-flow consumers, and one for each of the remaining 42 direct callsites. This totals 128, leaving 16 slots for unexpected caller RVAs. Each logged query also has its per-caller `capture-ordinal`. The callback records include the original returned float and before/after bytes of the callback's per-ray record; the helper tap captures its Boolean, the 0x50 output span and 0xA8 context before/after, input vectors, and selector arguments. The current revision retains detailed callback metadata only at ordinals `0, 1, 2, 4, 8, 16, 32`; every callback observed on the query thread contributes to a scalar minimum and count. These fields are the minimum useful join between the callback's candidate fraction and the helper's selected caller-visible result.

For the next evidence session, the priority is controlled variation rather than more unlabelled activity: after the candidate is installed on a later closed-game boundary, collect a short clear shot, a shot into a nearby solid surface, and a shot into open distance, then stop firing and inspect the log before doing anything else. The `CAmmunition::CheckCollisions` cap now records the first 32 queries, so several shotgun shells should be represented even if each shell expands into multiple ray queries. Compare, per query, (1) helper Boolean, (2) output pointer at `+0x00`, scalar at `+0x14`, and context before/after, and (3) callback count and the minimum finite callback fraction in `[0,1]`. A repeated relation between a candidate fraction and output `+0x14`, alongside a non-null selected object/context pointer and the caller's changed response to the solid surface, would support a geometric-hit interpretation. A low fraction by itself is not proof: the callback can report candidates that are later rejected, and helper bounds/fallback paths can write caller-visible values without a selected geometric hit.

Do not install this candidate into the currently running process. The user requested that this instance stay open because it may not relaunch reliably. Applying the candidate requires a later moment when Mad Max is closed; the currently running instance remains untouched. No new runtime or gameplay observation was made during this revalidation.

### Candidate callback tap hot-path audit (2026-10-28)

Static review of `begin_world_query_candidate_capture` confirms an important distinction: the eight-slot array bounds storage, but it does not bound deep capture work. For every callback associated with a sampled query, the detour increments the count, chooses either a first-three or rolling slot, then follows the candidate handle/object/subobject pointers, probes vtables, reads gate words and three 0x30 transform blocks, and snapshots a 0x30 per-ray record before calling the original callback. Even callbacks whose samples are later overwritten in the rolling slots pay those reads. Thus a dense broadphase query can trigger substantially more than eight sets of pointer/transform reads. The minimum-fraction slot is also maintained only after each callback has already incurred the full pre-capture cost.

This does not change the game's returned fraction: the hook calls the original callback and returns that exact float. The performance concern is diagnostic overhead and is not measured in the current run. Before installing this candidate, revise the callback capture so deep metadata reads are limited to a fixed number of callbacks (for example, the first three plus a small recent set). If retaining the minimum-fraction candidate is important, compare the returned fraction for each callback with a per-query atomic minimum and only capture its detailed metadata after the original returns when it becomes the best candidate so far. That still observes each callback's float, but avoids chasing object pointers and transforms for every candidate. The post-callback best record would need to be labeled as such because its before-snapshot is unavailable unless a cheap fixed-size pre-record read is retained.

This is a code-path audit, not a runtime performance measurement. No code was changed and no game state was touched. The candidate remains uninstalled. The distinction above corrects earlier shorthand that described candidate capture as “capped at eight”; only retained records are capped today.

#### Callback association and concurrency limit

The tap associates a callback with the active helper sample through a thread-local return-frame stack. The helper detour pushes its sample index on the calling thread, and the callback detour reads the top frame from that same thread-local stack. This is a good fit for synchronous casts and nested helper calls, but it means callbacks dispatched to worker threads will not be attributed to the initiating query by this mechanism. The current count therefore means “callbacks observed on the query thread while its sample frame is active,” not yet a proven count of every broadphase candidate considered by the engine.

Callbacks that see the active thread-local query frame necessarily execute on that query thread; the old rolling-slot implementation could nevertheless reuse storage after ordinal 6. The fixed ordinal map removes that reuse: each captured ordinal has a unique slot. The larger remaining concurrency uncertainty is whether the engine dispatches some work to other threads, which this TLS-based counter would not attribute. No runtime thread-affinity measurement has been captured, so the per-query total remains a query-thread count, not a global candidate count.

#### Initial proposed bounded callback capture revision (superseded)

The first bounded proposal kept full callback-entry/exit snapshots only for ordinals 0–2 and retained a minimum candidate's pointers for follow-up reads. The confirmed helper-local lifetime makes that pointer-retention idea unsuitable. The implemented revision instead captures seven fixed ordinals and keeps the minimum scalar-only.

The minimum-candidate snapshot timing must account for the helper's stack lifetime. The caller-owned output/context and caller frame remain live at the return hook, but the callback's caster or per-ray scratch may be local to the helper. Read-only disassembly of the live helper resolves this: the collector is helper-local, so do not rely on it after helper return.

Do not defer selected-candidate metadata or per-ray record reads until helper return. All seven selected records retain callback-time before/after snapshots; scalar fraction/count tracking continues through all callbacks observed on the query thread.

#### Implemented lifetime-safe callback capture policy

The live helper's machine code establishes that the collector and its scratch records are helper-local. Preserve full before/after metadata only for fixed callback ordinals `0, 1, 2, 4, 8, 16, 32`, and count all callback invocations while tracking the lowest finite in-range fraction as scalar-only per-query metadata. This bounds deep snapshots to seven per query and keeps selected-candidate reads inside the callback detour, before control returns to the helper and its stack frame can unwind. Do not retain a candidate handle or caster pointer for later reads. Logged selected-callback records should include their ordinal; the aggregate callback count and minimum fraction should be separate fields so an unsampled minimum remains explicit.

This is the implemented first capture policy. It preserves representative early and sparse late candidates and tests whether the helper's `+0x14` output tracks the minimum raw callback fraction. Even a match remains supporting evidence only, because callback candidates can be filtered/rejected before the helper writes the final output.

This design intentionally distinguishes the minimum raw callback fraction from the helper's selected caller output. A minimum callback fraction may still belong to a candidate the helper rejects. Compare it with the helper's output scalar, selected object/context pointers, and helper Boolean; do not label the minimum alone as a hit. The confirmed helper-local lifetime means the minimum remains scalar-only; do not retain its handle or caster pointer for deferred reads.

#### Live helper stack-lifetime evidence (2026-10-28)

The running image's helper entry at `0x14252EDF0` is detoured by the installed ASI, so the original five-byte prologue was recovered read-only from the MinHook trampoline at `0x13FFF0F00`. The helper body was then disassembled from a read-only process-memory copy with those original bytes restored in a temporary file; the game process was not modified.

In that body, helper offset `+0x1CE` stores a pointer into local `[RBP-0x70]`; at `+0x1FA` it places that address in `RCX`, then calls the cast routine at `+0x1FE`. The helper later restores its stack and returns. This identifies the collector passed to the cast routine as helper-stack storage. Its lifetime ends as the helper returns, before a helper-return hook can safely inspect it. Capture caster/per-ray details inside the callback detour; helper-return capture remains appropriate only for caller-owned output and context records.

### Numeric decoding of the eight retained shotgun ray inputs (2026-10-28)

I decoded the six little-endian `f32` values from the old `R8-data-before[0..0x60]` records for samples `#8610`–`#8617`, using the first 24 bytes only. All eight have the same origin `(-3940.41, 477.32, -1513.96)`. Their second vectors differ and have magnitudes from `0.999917` to `1.000000` (rounded), so those eight ammo queries used effectively unit-length directions from one shared origin. This is direct runtime evidence for the ammo caller's normalized direction input, not just an inference from its setup code.

The varying directions are consistent with multiple rays around a shot, but the log does not record shell boundaries or input events; four user-reported shots producing eight helper calls does not tell us whether those were two queries per shot, eight pellets across some shots, or another scheduling pattern. The helper returned `1` on all eight. The old build did not record output/context snapshots or callback fractions, so this decoding still cannot tell whether any ray had a geometric hit. The decoded values and unit lengths identify exactly what the next capture should preserve and compare per sample.

### Bounded callback-capture build prepared (2026-10-28 06:46 +01)

The Rust candidate now limits deep callback metadata reads to fixed callback ordinals `0, 1, 2, 4, 8, 16, 32`. Every callback associated with a sampled query on the query thread still increments the per-query callback count and contributes its return value to a scalar minimum only when the value is finite and in `[0,1]`. Unscheduled ordinals do not read caster/handle pointers, vtables, transforms, gates, or the per-ray record. The former rolling slots and copied-best-candidate record are removed. The minimum is logged separately from the seven detailed records, so a minimum whose callback ordinal was not selected remains visible without retaining its pointer.

The detour still calls the original callback once and forwards its exact `f32` return. The callback tap is not attributed to worker-thread work because its query association remains thread-local; the logged count must therefore be read as observed callbacks on the query thread, not a proven global candidate count. Candidate identity and per-ray record data are captured in the callback detour before the helper's local collector can expire. Helper-return snapshots remain limited to caller-owned output/context data.

Verification: `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check -- src/game_probe.rs docs/research/2026-10-28-world-query-output-capture.md` succeeded. The fresh release DLL is 2,424,320 bytes with SHA-256 `19B757D163894960121FE56A101D3448C55EAE3EEB0F2F1D5A87F581BD2225E2`. The first Cargo build could not write to the repository's existing `target` directory, so the successful build used an allowed scratch target directory. No tests were run. The DLL remains uninstalled. Mad Max PID `20880` is still responsive; `bootstrap.log` remains 19,086 bytes with last write `02:26:57 +01`, so this build has no runtime evidence yet.

### Live candidate virtual call contract refined (2026-10-28)

A fresh read-only copy of 0x200 bytes beginning at live VA `0x142526AA0` was disassembled from PID `20880`. It confirms the callback computes the selected 0x30-byte per-ray record from the caster's output base and stride (`caster+0x18`, `caster+0x20`). It reads the input-array pointer at `caster+0x08`, uses `castIndex * 0x30` to select that ray's two 16-byte vectors, transforms them with the candidate object's matrix data at `+0x140..+0x170`, and writes the transformed vectors to collector-local addresses `caster+0x30` and `caster+0x40`.

Immediately before the game-specific virtual call, the callback copies the candidate entry dword at `[caster+0x08] + 0x30*castIndex + 0x24` to collector-local `+0x50`. The indirect call at callback offset `+0x148` uses `RCX=object_subobject`, `RDX=&caster[0x30]` (the transformed local-ray block), and `R8=per_ray_record`. On return, at offset `+0x14E`, it loads `XMM0` from `per_ray_record+0x08`; the callback returns that float. Thus the `+0x80` virtual is a candidate-local ray-query method taking a local-ray block and per-ray record, but the live code does not identify its C++ name or make it safe to call independently. Its `+0x08` output is definitively the callback return value; mapping that value to the helper's final `+0x14` remains unproven.

This is an additional static live-image observation, not a new gameplay callback sample. The ASI did not log any new query output/context data; process PID `20880`, its installed older ASI, and the 19,086-byte log are unchanged. This narrows the callback ABI and result source but does not resolve the custom record's semantics or hit-selection policy.

### Bounded candidate now captures the local-ray input (2026-10-28 06:56 +01)

The bounded callback capture was extended to preserve five qwords from `caster+0x30..+0x57` for the same seven fixed callback ordinals. This records the two transformed 16-byte vectors passed to the `object+0x80` virtual through `RDX`, plus the following raw qword that overlaps the collector's `+0x50` field. The values stay raw in the log so their exact vector convention and trailing dword meaning remain open. The copy happens after the original callback returns but before the detour returns to the helper, while the helper-local collector is still alive. Unscheduled callbacks do not read this block.

The updated release candidate passed `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` for the changed source and research files. The DLL is 2,473,984 bytes, SHA-256 `AE8D85572FC85B3BBC52BD1F3CEC05A50339943E303EDAC6946AF52543E1CBCE`, built in the allowed scratch target directory. No tests were run and the DLL remains uninstalled. Mad Max PID `20880` is responsive; the old log remains 19,086 bytes, last written `02:26:57 +01`. Thus the local-ray capture has compile-time but not runtime verification.

### Helper output fallback and broadphase gate from live disassembly (2026-10-28 07:01 +01)

A read-only copy of the helper body beginning at VA `0x14252EDF5` (the installed ASI detours the entry at `0x14252EDF0`) was disassembled from PID `20880`. Its prologue establishes `RBP = entry-RSP - 0x68`; it loads stack argument 6 into `RBX` and stack argument 7 into `RDI`. At the callsites already catalogued, those are the output-record pointer and context-record pointer respectively. This independently confirms the arg6/arg7 ordering used by the logger.

Before the physics cast, the helper forms both points `origin + direction * XMM3` and `origin + direction * stack-argument-5`, subtracts the world-frame origin at `[RCX+0x40..0x48]`, and calls RVA `0x25267E0` for each point. XMM2 is zeroed before both calls, supplying a zero radius to this point-in-bounds predicate. Each returned `AL` is tested; a false result jumps past the cast to common post-processing. Only when both calls return nonzero does the helper build its collector and call the broadphase at `[world+0x88]`, passing the related filter at `[world+0xC0]`.

The call target is VA `0x1425267E0` (RVA `0x25267E0`), the entry of the separately identified point-in-bounds predicate. The other candidate address, `0x1425277E0`, lies inside a different function and is not called from this helper. No evidence supports writes beyond the output record's `0x50`-byte span.

After that path, the helper checks the caller-owned query context `+0x60` (stack argument 7). If the field is zero, it writes a fallback into the caller's output record: pointer `+0x00 = null`, vector words `+0x08..+0x10 = (0, 1, 0)`, scalar `+0x14 = 1.0`, and nulls at `+0x18` and `+0x28`. The observed constant for the vector's middle component is `1.0f` at VA `0x14122F8D8`. This fallback does not explicitly overwrite output `+0x20` or `+0x30..+0x4C`, so those fields can retain caller initialization or earlier contents. It returns false on this fallback branch.

When caller-context `+0x60` is nonzero, the helper copies vector `caller_context+0x10..+0x18` and scalar `caller_context+0x20` into output `+0x08..+0x14`, carries pointers/metadata from context into output `+0x18..+0x4C`, searches a context-associated array for an entry with kind `1` to populate output `+0x00`, and returns true. The exact types and meaning of those context fields remain unknown; the disassembly shows data flow, not semantic names. The pointer is loaded twice, so a concurrent change between the check and use could take a different inner branch; the helper does not provide a synchronization guarantee in the code inspected.

This sharpens the scalar interpretation: `+0x14 == 1.0` is explicitly written on the null-result fallback, while callers such as `CWeapon::GetRayWorldPosition` treat values below one as earlier points along their segment. However, a zero return from either endpoint-associated call also skips the broadphase cast and reaches the same common processing, and caller-provided context may not necessarily be zero-initialized. Therefore `1.0` is a no-result/default sentinel on the observed fallback path, not proof by itself of a valid in-bounds geometric miss. The next capture should compare the logged endpoint inputs, context before/after, helper Boolean, output fields, and candidate fractions in each controlled scene.

All reads were query/read-only against the already-running PID `20880`; no thread suspension or process write occurred. At 07:01 +01 it remained responsive on the older installed ASI; the 19,086-byte log remained last written at `02:26:57 +01`. The updated candidate is still uninstalled and has no runtime evidence.

### Current overnight status and address correction (2026-10-28)

The candidate source captures the helper output as ten qwords (`0x50` bytes). Its seven-ordinal local-ray copy remains inside `finish_world_query_candidate_capture`, after the original callback returns. The callback's returned fraction is tracked for every callback associated with the active query on the same thread; the aggregate count/minimum are explicitly named query-thread-only. The detailed records preserve raw words without assigning a normal/vector semantic to their trailing data.

No new tap data appeared in the still-running process: PID `20880` remains on the older installed ASI, and its bootstrap log remains 19,086 bytes with last write `02:26:57 +01`. The corrected candidate details are recorded at the end of this document. It was not deployed. No tests were added or run, and there is no runtime proof of callback coverage, stability, hit selection, or output correlation. A future run should first inspect one tiny controlled sample set (clear ray, nearby solid, long/open ray), then expand only if logs are understandable and the game remains stable.

### Superseded output-span interpretation (2026-10-28)

This section records an incorrect interpretation, now superseded below. The helper does not call VA `0x1425277E0`, and there is no evidence that it writes to output offsets `+0x64..+0x73`.

I briefly expanded the snapshot based on that mistaken interpretation, then reverted it. The candidate output span is ten qwords (`0x50` bytes). Historical build hashes in this superseded entry do not describe the corrected candidate.

### Corrected endpoint-call identity and candidate state (2026-10-28)

Rechecking the live call bytes corrected the prior address arithmetic. At VA `0x14252EF65`, next IP `0x14252EF6A` plus signed displacement `-0x878A` equals `0x1425267E0` (RVA `0x25267E0`). At VA `0x14252EF7A`, next IP `0x14252EF7F` plus displacement `-0x879F` yields the same target. Both calls reach the standalone point-in-bounds predicate with XMM2 zeroed. The earlier `0x1425277E0` result was an arithmetic error of `0x1000`; that address is not called from this helper. The log key is `bounds-check` and reports endpoint inputs, not predicate return values.

The initial address-corrected build passed `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check`; it was superseded by the bounds-result observer described below. No tests were run. PID `20880` remains responsive and the 19,086-byte bootstrap log is still last written at `02:26:57 +01`.

### Bounded bounds-result observer candidate

The live entry bytes at `0x1425267E0` begin `48 83 EC 28 48 8B 41 38 ... F3 44 0F 10 0A`, then load bound data relative to `[RCX+0x38]` and read a scalar from `[RDX]`. Together with the helper's call setup, this supports a machine-level ABI candidate of `RCX=owner/bounds object`, `RDX=point pointer`, `XMM2=radius`, and `AL=Boolean result`; it is not yet a public typed API or a runtime-confirmed semantic. The helper passes radius `0.0` and calls twice per query.

The candidate now adds a signature-checked observer at this predicate. It calls the original exactly once and forwards its byte-sized result unchanged. It checks the current thread's sampled-query TLS frame before reading the point pointer; only an active sampled query can trigger a point read or record write. It stores up to two calls per helper sample in fixed storage: point XYZ copied before the original call, radius, and returned Boolean. The per-query count also records calls beyond the two detailed slots. The logger includes the count and captured tuples. Calls outside a sampled query pass through without pointer reads or record writes, and no game pointers are retained. An independent review caught and prompted the TLS-before-read correction; the corrected `cargo fmt --check`, `cargo build --release --locked`, and `git diff --check` passed. Candidate DLL: 2,484,224 bytes, SHA-256 `52576B685E02F8A07A15561B7F925596F7A7629490782088697D2405E136D604`; no tests were run.

The data capture is bounded, but the detour is installed at the predicate entry and therefore forwards **every** call through the hook, including the 17 direct callers outside the shared helper. Those callers' runtime rates and the hook's frame-time cost have not been measured. The two-record cap limits reads and log volume only; it does not bound detour invocation overhead. Treat performance as unverified in the first controlled install session. If the cost is measurable, prefer intercepting the two helper-local callsites or optimize the inactive-sample fast path based on profiling.

The exact 16-byte signature was independently read from the live PID `20880` with the repository's read-only `trace_xref_probe --peek-any 1425267E0 128`: `48 83 EC 28 48 8B 41 38 0F 29 74 24 10 44 0F 29`. The game remains responsive on the old installed ASI, whose hash remains `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`; the bootstrap log remains 19,086 bytes, last written `02:26:57 +01`. The new build has not been installed, so runtime ABI and stability are still unverified.

To work around the probe's failed Toolhelp module enumeration, I added a read-only `--image-range <base-hex> <size-hex>` scan option and added this predicate to the target list. Scanning the known main-image range `0x140000000..0x145D88000` in PID `20880` found 19 candidate callsite RVAs: `0x1C16777`, `0x1C167E6`, `0x1C173A8`, `0x1C1741C`, `0x1C9549C`, `0x227A3F9`, `0x227A467`, `0x227A797`, `0x25269F2`, `0x252EF65`, `0x252EF7A`, `0x2541EB8`, `0x2543055`, `0x254309E`, `0x25433FE`, `0x2543441`, `0x2543A79`, `0x2543ABF`, and `0x25491D5`. I then checked each hit's surrounding bytes and recomputed its signed relative target: all 19 are preceded by opcode `E8` and resolve to VA `0x1425267E0` (RVA `0x25267E0`). This confirms 19 direct machine-code calls, including 17 sites outside the shared query helper. Eighteen immediately test `AL` and branch on it; the remaining call at RVA `0x25269F2` returns `AL` through a small wrapper (`add rsp, 0x38; ret`). The exact containing-function names for the newly confirmed sites are still unknown. The captured output was written to the task scratch file `ray-xref-scan.txt`. This strengthens the evidence that the predicate result is meaningful to multiple game paths and supports keeping its observer scoped to an active sampled helper call.

This broad direct-call fan-out also gives a concrete follow-up: decode the containing functions around the 17 external call sites, group paired endpoint checks, and identify which ones are independent bounds utility calls versus shared ray-query helper traffic. Do not turn these into broad hooks yet. The present observer intentionally sees only predicate calls nested inside an already sampled world-query helper, so its two-result-per-query tuples remain interpretable and low-volume.

### Direct-call neighborhood inspection (2026-10-28)

Read-only 96-byte windows around the 17 external calls show a repeated local pattern at multiple sites: callers write three scalar floats to a stack-local XYZ buffer, pass its address in `RDX`, set the predicate's radius register, call the bounds predicate, then test `AL` and branch. Several sites occur in close pairs with another stack-point construction between the calls. The observed clusters are `0x1C16777/0x1C167E6`, `0x1C173A8/0x1C1741C`, `0x227A3F9/0x227A467` plus `0x227A797`, `0x2543055/0x254309E`, `0x25433FE/0x2543441`, and `0x2543A79/0x2543ABF`. Single external sites are `0x1C9549C`, `0x2541EB8`, and `0x25491D5`; `0x25269F2` is the result-forwarding wrapper. The shared ray-query helper's pair remains `0x252EF65/0x252EF7A`.

This machine-code pattern supports treating the predicate as a reusable point-vs-bounds Boolean gate. It does not establish that every pair checks ray endpoints, that the queried points are world positions, or what gameplay subsystem owns each containing function. The local constructions and immediate `AL` branches are static observations from live code bytes; they are not new runtime samples. Keep these sites out of ray-hit interpretation until their containing functions and caller data are decoded.

The PE exception directory provides exact runtime-function boundaries for those callsites. I parsed MadMax.exe's `.pdata` exception directory (RVA `0x1A0C000`, size `0x125580`; 100,128 runtime-function records) and verified that the entire directory matches the live process byte-for-byte. The containing ranges are:

| Function RVA range | Direct predicate call RVAs | Size |
|---|---|---:|
| `[0x1C16670, 0x1C16ADF)` | `0x1C16777`, `0x1C167E6` | `0x46F` |
| `[0x1C172E0, 0x1C176D0)` | `0x1C173A8`, `0x1C1741C` | `0x3F0` |
| `[0x1C95420, 0x1C95624)` | `0x1C9549C` | `0x204` |
| `[0x227A340, 0x227A6EE)` | `0x227A3F9`, `0x227A467` | `0x3AE` |
| `[0x227A700, 0x227A975)` | `0x227A797` | `0x275` |
| `[0x25269B0, 0x25269FC)` | `0x25269F2` (forwards predicate `AL`) | `0x4C` |
| `[0x252EDF0, 0x252F165)` | `0x252EF65`, `0x252EF7A` (shared query helper) | `0x375` |
| `[0x2541E30, 0x25420CD)` | `0x2541EB8` | `0x29D` |
| `[0x2542FA0, 0x2543324)` | `0x2543055`, `0x254309E` | `0x384` |
| `[0x2543340, 0x25439A1)` | `0x25433FE`, `0x2543441` | `0x661` |
| `[0x25439B0, 0x2544010)` | `0x2543A79`, `0x2543ABF` | `0x660` |
| `[0x2549150, 0x2549284)` | `0x25491D5` | `0x134` |

This resolves exact same-function groupings: the pair at `0x227A3F9/0x227A467` is in one function, but nearby `0x227A797` is in a different one. Function names remain unresolved. The on-disk `.xpdata` section is marked initialized data/read-only (`0x40000040`) and its bytes at these code RVAs differ from the live image; live `VirtualQueryEx` reports those code pages as `PAGE_EXECUTE_READWRITE` (`0x40`). Therefore the disk payload is not a reliable disassembly source for this region. The live `.pdata` match makes its function boundaries strong evidence for the running image, but it does not explain how the game materializes executable bytes or reveal source-level symbols.

This directly exposes whether the two observed endpoints pass the gate, which was previously inferred from helper control flow. The machine-level ABI remains a static candidate, not a public typed API: the expected entry bytes and call-site register setup support `RCX` owner, `RDX` point pointer, `XMM2` radius, and `AL` result, but live runtime validation is pending. The candidate must remain uninstalled until the game is closed; first runtime validation should confirm hook installation and exactly two captures for a selected helper query, then compare the recorded Boolean pair against whether the broadphase candidate callback ran.

### Candidate reproducibility and live-state check (2026-10-28 07:33 +01)

Rebuilt the current source with `cargo build --release --locked` into the task scratch target directory. It completed successfully and the resulting DLL still hashes to `52576B685E02F8A07A15561B7F925596F7A7629490782088697D2405E136D604`. `cargo fmt --check` and `git diff --check` also completed successfully; Git emitted only existing line-ending conversion warnings. No tests were run. The installed `scripts/MadMaxModding.asi` independently hashes to the recorded old build `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`, confirming the candidate is not deployed. PID `20880` remains responsive; the bootstrap log remains 19,086 bytes with last write at `02:26:57 +01`. No newer runtime observations are available, and the candidate has no in-game validation yet.

### Extended read-only watcher baseline (2026-10-28 08:49 +01)

The scratch watcher continued reading the same two cells every 30 seconds while Mad Max PID `20880` remained responsive. From `08:20:36` through `08:49:40 +01`, 57 positive-delta intervals accumulated 193,446 shared-helper calls over 1,743 seconds, about 111 calls/second (interval deltas ranged from 3,229 to 3,496). The latest-return cell showed `wind_occlusion` at 51 sample instants and `vehicle_sound_occlusion` at 6. These are point samples of whichever call wrote the cell most recently, not caller-specific counts or shares; do not interpret the 51/6 split as call volume. The running state's gameplay context was not tagged during this interval.

The detailed bootstrap log remains 19,086 bytes, last written at `02:26:57 +01`; no fresh helper-entry snapshots were produced by the installed build. The uninstalled source candidate in scratch target `mm-build-target` is the `08:29:11 +01` build, 2,484,736 bytes, SHA-256 `561CF45849E8C7A211B01AEFF45EB5FF6AC6341B91EDEF7DBC1A1A16D7F7CE53`. The separate `madmax-ray-capture-target` DLL is older (`07:12:05 +01`, SHA-256 `48BA6C7B089A1127A30BE7E32A21CC472CE8A580A8A219599BF88E0A1EAA8064`) and should not be mistaken for the latest candidate. No candidate was installed, and no process memory was written.

Rechecking the bootstrap log's unique-caller records found 14 distinct caller RVAs observed during this run: `BaseToCameraSimpleRaycast`, `CRayCastFanCondition::Evaluate`, `HeightOverGround`, `CAmmunition::CheckCollisions`, `character_detection`, `NFootPlacement::Raycast`, `ValidateTransformOnTerrain`, `raycast_sounds`, `CGameStateLoad::ForceOnPhysics`, `CInteractionGraphEntry::CalculateExitTransform`, `CInteractionGraphEntry::CalculateEntryTransform`, `CEffectSystem::UpdateDecalEffect`, `wind_occlusion`, and `vehicle_sound_occlusion`. These labels are cross-referenced in the caller-label table above. This establishes that the functions ran at least once in this game run; the installed build logged unique identities but not per-caller totals or rates. The candidate's five-second atomic summaries are intended to supply those missing rates.

### Live world/broadphase chain revalidated (2026-10-28 08:53 +01)

Fresh `--peek-any` reads from PID `20880` reconfirmed the live pointer chain: global slot `0x1417F4F08` -> owner `0x82588260`; owner `+0x38` -> world `0x08156B20`; world `+0x88` -> broadphase `0x0817E760`; world `+0xC0` -> filter `0x0817EDA0`. The broadphase vtable is `0x1413E0A18`; its slot `+0xC8` still points to `0x142B2CD40`. Its live MSVC RTTI chain resolves through COL `0x1415048B0` to TypeDescriptor `0x1417DBA28`, whose name is `.?AVhk3AxisSweep@@`. The separate world filter vtable is `0x141325DA0`; its COL `0x1414F2278` resolves to TypeDescriptor `0x1417C9EA0`, name `.?AVhkpCustomGroupFilter@@`. Thus the world-owned filter and the per-call `CIgnoreFilter` seen in one caller remain distinct objects. Mad Max PID `20880` was responsive at this read; these are object-identity/dispatch observations, not proof of any current ray's hit result.

### Candidate and live-process recheck (2026-10-28 08:56 +01)

The current `src/game_probe.rs` rebuilt successfully with the installed Cargo binary using `cargo build --release --locked` and scratch target `mm-build-target`; the output remains 2,484,736 bytes with SHA-256 `561CF45849E8C7A211B01AEFF45EB5FF6AC6341B91EDEF7DBC1A1A16D7F7CE53`. `cargo fmt --check` and `git diff --check` also pass; the formatter reports a non-fatal path-canonicalization warning for the user-profile path, and Git reports existing LF/CRLF conversion warnings. No tests were run. Mad Max PID `20880` and watcher PID `13824` are both responsive. The watcher count rose from `2,610,812` at `08:49:40` to `2,658,450` at `08:56:40 +01`, approximately 113 shared-helper calls/second over that interval; the latest-return cell was `wind_occlusion` at the last sample. This remains global call volume and a point-sampled latest caller, not a per-caller histogram. The bootstrap log remains unchanged at 19,086 bytes, last written `02:26:57 +01`.

### Candidate review finding: signed-zero fraction ordering (2026-10-28 08:59 +01)

Static review found that `candidate_min_fraction` starts at positive-infinity bits and uses `AtomicU32::fetch_min(fraction.to_bits())` after accepting finite values in the numeric range `[0,1]`. Unsigned bit ordering matches numeric ordering for positive finite fractions, but Rust's range check also accepts `-0.0`; its `0x80000000` bit pattern sorts above positive infinity's `0x7F800000`, so it cannot lower the initial sentinel (or any positive minimum). The per-candidate `returned_fraction` still records the original bits, so only the aggregate minimum can be wrong for a negative-zero callback result. No current runtime sample establishes whether Mad Max returns `-0.0`. Before installation, normalize either signed zero to `+0.0` for aggregation or replace bitwise minimum with a float-aware compare/update; keep the raw detailed fraction unchanged.

### Candidate review finding: unsampled nested-query masking (2026-10-28 09:04 +01)

Static inspection of `record_world_query_call` shows that it pushes a thread-local return frame and replaces the helper return address only when that call receives a detailed sample. `current_world_query_sample_index` reads the top frame but has no marker for an unsampled nested helper call. Therefore, if a sampled helper call synchronously triggers a nested, unsampled helper call on the same thread, bounds-predicate and candidate-callback hooks inside the nested call can see the outer sample at the top of the stack and be counted as part of it. The current evidence does not establish that Mad Max takes this nested path; this is an association risk, not an observed misattribution.

To guarantee correct association, the next candidate should push a frame for every helper entry, using an explicit no-sample marker for unsampled calls, and route each helper return through the pop thunk; only sampled frames should trigger output reads or increment `WORLD_QUERY_ACTIVE_SAMPLES`. That raises the return-thunk rate from sampled calls to all shared-helper calls. The read-only baseline is about 111–116 helper calls/second, but the additional runtime cost is unmeasured. Alternatively, establish from the candidate virtual's full call graph that it cannot re-enter the helper. Add a focused stack-association test for sampled → unsampled nested → sampled outer restoration before treating callback counts as query-complete.

### Overnight live cast-adapter and context-bridge findings (2026-10-28 09:48 +01)

A fresh read-only copy of the direct helper-call target at VA `0x142B0DDE0` (RVA `0x2B0DDE0`) was disassembled with `dumpbin /disasm`. The PE exception directory places it at the start of runtime-function range `0x2B0DDE0..0x2B0DEFF` (length `0x11F`). It is a small adapter, not the `hk3AxisSweep::castRay` function entry. The adapter stores `RBX=RCX` (helper-local collector), `RBP=RDX` (broadphase object), `RSI=R8` (ray input), and `RDI=R9` (filter), prepares collector fields and a local aligned ray block, then calls the broadphase object's virtual slot `+0xC8`. This confirms the path `shared helper -> cast adapter 0x2B0DDE0 -> broadphase virtual +0xC8`.

A careful frame recalculation corrects an earlier speculative note that the caller context was not forwarded. The adapter's prologue saves `RBX`, `RBP`, and `RSI` into the incoming Win64 home space; it pushes only `RDI` before subtracting `0x50` from RSP. Its working RSP is therefore entry RSP minus `0x58`. Adapter `[rsp+0x80]` reads incoming stack slot 5, and `[rsp+0x88]` reads incoming slot 6. The helper supplies null as adapter slot 5 and its caller-owned context pointer as slot 6. The adapter stores that slot-6 pointer into its collector at `+0x18`.

The live candidate callback reads `caster+0x18` as the per-ray output base, then selects a record using the collector stride at `+0x20` and `castIndex`. This establishes a concrete path from the helper's caller context pointer into the per-ray records passed to the object virtual at `+0x80`. The candidate callback already captures the selected `0x30`-byte record before/after the game call, while the helper-return capture snapshots the caller context `0xA8` bytes before/after. Those two records can now be compared directly under one sample ID. The exact field/type mapping, including which write populates context `+0x60`, remains unresolved until a future candidate run captures real before/after values.

The adapter's imported function pointer resolves to `KERNEL32!TlsGetValue`; the global TLS index is `0x49` (73). When the thread-local trace cursor is below its limit, the adapter writes a marker pointer and low 32 bits of `RDTSC` before and after the broadphase call. The start marker points to `TtRayCstCached`; adjacent strings are `TtRayCastFSP` and `TtRayCastGroup`. Three allocated thread trace buffers were copied read-only (about 5.5 MB total); none contains any of these three ray-cast marker pointers. Their cursors did not advance across a 35-second sample while the global helper counter increased by about 3,500. This is consistent with bounds checks skipping the cast during that idle window, but it does not prove it; the old installed ASI cannot log the two bounds predicate returns or correlate this profiler stream to individual queries.

All reads used process query/read access and thread query access only. No game thread was suspended and no process memory was written. Mad Max PID `20880` remained responsive on the old installed ASI; the candidate DLL remains uninstalled. A passive scratch watcher now tracks future per-thread TLS cursor advances and the three ray-cast markers. The next candidate capture should correlate, for one sampled helper call: both bounds Boolean results, adapter invocation, candidate callback count/records, caller-context bytes through `+0xA8`, helper Boolean/output, and thread ID. Do not infer a geometric hit from the helper Boolean or a broadphase callback fraction alone.

### Live candidate fallback path (2026-10-28 09:53 +01)

A fresh read-only disassembly at callback VA `0x142526AA0` confirms its primary path and reveals the exact alternate path target: `0x142B0DC60` (RVA `0x2B0DC60`). The callback computes the per-ray record as `caster+0x18 + caster_stride(+0x20) * castIndex`, resolves the candidate through a signed relative handle, gates on the candidate byte at `object+0x28 == 2`, then checks the object virtual at `+0x30` for result `3`. Passing both gates transforms the ray into candidate-local coordinates, calls the candidate virtual at `+0x80`, and returns per-ray record `+0x08` as its fraction.

When a gate fails, callback `0x2526AA0` calls fallback `0x2B0DC60` with the same caster, handle, and index. The fallback runs a collector filter virtual at `collector+0x10` / vtable slot `+0x08`; if accepted, it transforms the ray, writes the transformed vector at caster `+0x40`, copies per-cast ray-input `[caster+0x08] + 0x30*castIndex + 0x24` to caster `+0x50`, stores the handle-relative pointer at caster `+0x60`, copies ray-input `+0x28` to caster `+0x68`, and invokes the handle object virtual `+0x30`. Both accepted and rejected fallback paths return per-ray output record `+0x08`; rejection returns before these collector writes. This confirms a collector-local `+0x60` write, distinct from caller-context `+0x60`, and identifies the group-filter decision as a key discriminator.

The already-prepared candidate hook at `0x2526AA0` records callback ordinal, object gates/vtables, selected per-ray output before/after, and return fraction. The helper-return hook records caller-context bytes before/after. A later capture should snapshot collector `+0x60` and caller-context `+0x60` separately, and capture the group-filter Boolean; the collector word is an opaque handle-relative pointer, not a semantic hit flag. A callback fraction can include a fallback record value and remains unproven as a geometric hit until controlled clear/surface comparisons correlate it with helper output.

The live bytes were copied using `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ` from Mad Max PID `20880`; no process memory was written and no thread was suspended. The game remained responsive. The candidate DLL is still uninstalled.

## Proposed next capture build: one correlated query trace (draft)

### Goal

Prove the stage sequence and raw data flow for a small number of world queries, then use caller actions to classify outputs. Keep high-frequency counters aggregate-only. Do not label a fraction or helper Boolean as a physical hit without controlled correlation.

### One sample record

Give each detailed query a monotonically increasing sample ID and record thread ID plus per-thread nesting depth. For each sample, capture:

1. **Helper entry/return** — caller RVA, six ray floats, scalar/register/stack arguments used by the helper, its Boolean return, external output argument 6 before/after, and caller-context argument 7 before/after. Preserve raw bits and unreadable/null distinctions; these two caller pointers are separate even though the callback uses argument 7 as the per-ray output base.
2. **Bounds gate** — both points/radius/results, ordered and associated with the same query ID. Record whether the adapter ran after the gate.
3. **Cast adapter** — one entry/exit marker around `0x2B0DDE0`, broadphase object and vtable target, ray/filter pointers, cast status/return, and the forwarded caller-context pointer. This is the direct “bounds passed and cast dispatched” measurement; keep Havok-internal work out of scope until this is proven.
4. **Candidate callback** — total callbacks per query, sampled ordinals, raw 0x30-byte ray-input and per-ray output records, candidate identity pointers, gate byte, virtual targets, returned fraction, and running numeric minimum. The per-ray output base aliases caller-context argument 7 on this one-ray path; keep external output argument 6 separate. Preserve negative-zero correctly in the aggregate.
5. **Fallback and group filter** — count fallback entry/return at `0x2B0DC60`; separately hook the verified `hkpCustomGroupFilter` subobject virtual at `0x24E7580` to capture its returned Boolean while the stack output pointer is valid. Record ray-input `+0x24`, handle-relative candidate `+0x2C`, filter subobject, and decision. On accepted fallback candidates, capture local collector `+0x40..+0x68`; separately retain caller-context `+0x60` from the helper snapshot. On rejection, record that the collector writes were skipped. This links the opaque collector `+0x60` pointer to the actual group-filter branch without conflating it with caller context.
6. **Thread-local profiler** — cursor delta and marker pairs only when observed; do not treat an unchanged TLS stream as absence of a cast until the adapter tap agrees.

### Association and budget

Push a TLS frame for every helper entry, including an explicit no-sample marker. Route returns through one pop path. This prevents callbacks from an unsampled nested helper from attaching to a sampled outer query. Allocate nesting depth separately from the detailed sample budget. Keep per-caller sample quotas, reserve slots for unseen callers, and bound candidate detail to selected ordinals while always retaining callback totals and the aggregate minimum. The current global helper rate is about 112 calls/second, so the all-call path should only update counters/frame state; expensive memory snapshots and formatting stay on the bounded sample path and background logger.

### Evidence rules

Report raw fields and stage counts first. A helper false with no callback, a callback fraction, an accepted filter, and context `+0x60 != 0` are distinct observations. A likely geometric hit needs controlled comparisons (clear ray, ground/surface, and obstacle) with caller-side results or a verified output-field contract. Keep per-caller interpretations scoped to callsites already identified; the global last-return cell is only an instantaneous sample.

### Review and runtime sequence

Before installation, review every Rust line in the candidate and inspect generated diff/build identity. When the user next closes Mad Max, install only after that review; the user launches the game. First check hook signatures and low-overhead counters, then use a few user-controlled actions (shotgun at ground/obstacle, Force Pulse if available) and compare sample traces. Keep the current installed build intact until the new candidate has a clean startup and useful data; do not relaunch the game as part of overnight analysis.

This is a capture plan, not implemented code. The current process remains PID `20880`, responsive, and read-only observation continues.

### Live activity checkpoint (2026-10-28 09:56 +01)

The passive helper watcher advanced from `2,984,130` at `09:44:45` to `3,065,846` at `09:56:46` (+81,716 in 12m01s, approximately 113.4 shared-helper entries/second). Point-sampled latest return addresses were overwhelmingly `wind_occlusion` (`0x141FC0E0E`), with a brief `vehicle_sound_occlusion` sample (`0x1421DC95A`) at `09:54:16`; these are instantaneous last-writer observations and do not allocate the global delta by caller. The latter address was disassembled and confirmed as the known vehicle-sound caller, so it is not a new callsite.

The TLS trace watcher remained alive and emitted no cursor-advance records through `09:56`; this is not proof that no internal cast ran. Mad Max PID `20880` remained responsive. Installed plugin and bootstrap log are unchanged; no writes to game memory, no suspension, and no candidate install.

### Group-filter path and collector/context correction (2026-10-28 10:00 +01)

Fresh live disassembly separates the per-cast ray-input record from the per-ray output record, which were previously conflated. The per-ray output record is `caster+0x18 + stride(caster+0x20)*castIndex`. The per-cast ray-input record is independently addressed as `caster+0x08 + 0x30*castIndex` (the adapter stores the helper's ray-input pointer at `caster+0x08`). It supplies the `+0x24` dword and `+0x28` qword copied into the local collector; neither is from the per-ray output record nor from a broadphase candidate-entry array. The primary callback copies ray-input `+0x24` to collector `+0x50` before calling the object virtual at `+0x80`. The fallback copies the same ray-input `+0x24` to collector `+0x50`, stores its handle-relative candidate pointer at collector `+0x60`, and copies ray-input `+0x28` to collector `+0x68` after the filter accepts. Rejected candidates skip those fallback writes. Earlier descriptions calling this a candidate-entry record or caller-context write are corrected here.

The live `hkpCustomGroupFilter` object at world `+0xC0` contains the filter subobject vtable at `filter+0x28`; its slot `+0x08` resolves to `0x1424E7580`. That function reads a dword from ray-input record `+0x24` and a dword at handle-relative candidate pointer `+0x2C`, backs `RCX` from the subobject to the filter base (`-0x28`), and calls core routine `0x1424E73E0`. The core writes an explicit byte result to the supplied stack-local Boolean and returns its address; the fallback checks that byte. Its bit operations compare packed fields and consult filter matrix words. This is strong binary evidence for a candidate group-filter decision, but semantic names for each bit remain unassigned and no runtime accepted/rejected result was captured.

This exposes a concrete capture gap: the current candidate observer snapshots the per-ray output record and resolved object, but not the per-cast ray-input record or the group-filter Boolean. The next capture should record the six raw qwords at `caster+0x08 + 0x30*castIndex` and the decision byte while the fallback frame is active, then correlate them with collector `+0x50/+0x60/+0x68`, caller-context `+0x60`, and the final helper output. Keep the ray-input base, output-record base, collector address, and caller-context address separately named in logs and sample structs. These are read-only static byte observations; PID `20880` remained responsive and untouched.

The group-filter interpretation is also consistent with the Havok `hkpRigidBody` header: it describes collision filter info as an identifying value and says a group filter can encode collision and system groups ([source header](https://github.com/nitaigao/engine-showcase/blob/master/etc/vendor/havok/Source/Physics/Dynamics/Entity/hkpRigidBody.h)). This corroborates the binary inference but does not prove Mad Max uses an unmodified SDK implementation.

A final RTTI check on the filter subobject vtable pointer `0x141325DC8` reads its Complete Object Locator at `0x1414F2250`; the COL reports subobject offset `0x28` and the same TypeDescriptor `.?AVhkpCustomGroupFilter@@` as the main world filter. Thus the virtual at `0x1424E7580` is confirmed as a subobject method of this live custom group-filter object, rather than an unrelated global stub. This remains a pointer/type identity check, not a runtime filter-result sample.

### Adapter pointer alias: per-ray output is the caller context base (2026-10-28 10:07 +01)

Rechecking the exact adapter arguments resolves another part of the data flow. The helper passes caller stack argument 7 in the adapter's stack slot 6. Adapter `0x2B0DDE0` stores that pointer at `caster+0x18` and sets `caster+0x20` stride to zero. The callback computes its per-ray output address as `caster+0x18 + stride*castIndex`; for this one-ray cast (`castIndex=0`) that is exactly the caller-context base. The separate external output record is helper stack argument 6. Thus the callback's 0x30-byte per-ray record snapshot overlaps the first 0x30 bytes of the 0xA8-byte caller-context snapshot, and the candidate object's `+0x80` virtual receives that same caller-context base as its per-ray output pointer.

This is a concrete path by which the primary object virtual could write caller-context fields such as `+0x60`; the captured static instructions do not reveal its internal writes. It is separate from the fallback's `caster+0x60`, which is a field in the stack-local collector. Keep three addresses distinct in future records: external output arg6, caller-context/per-ray-output base arg7, and stack-local collector (`helper RBP-0x70`). Capture before/after around the virtual/callback while all three frames are live to identify which path populates caller-context `+0x60`.

### Live activity checkpoint (2026-10-28 10:11 +01)

The helper count reached `3,161,290` at `10:10:47`, up `177,160` from the `09:44:45` baseline (`2,984,130`) over 26m02s, about `113.5` shared-helper entries/second. Latest-return samples remain mostly `wind_occlusion`; `vehicle_sound_occlusion` has appeared only as brief point samples. This is still global count plus instantaneous latest caller, not per-caller volume. The TLS cursor watcher emitted no marker/cursor advances through `10:10`; that does not prove no cast. PID `20880` remains responsive, with both passive watchers alive. No game-memory write, suspension, close, or candidate install occurred.

### Candidate artifact and live-log recheck (2026-10-28 10:18 +01)

Read-only inspection at `10:18 +01` found Mad Max PID `20880` still responsive, with watcher PIDs `13824` and `17608` responsive. The installed `bootstrap.log` had last advanced at `10:12:46 +01` (19,246 bytes); its newest records are only caller observations at RVAs `0x1FB0895` and `0x237BC19`. Both addresses were already present in the static caller inventory: the former is the dynamic-label forwarding wrapper analyzed above, and the latter is `CRayCastScheduler::Update`. No later query sample, candidate callback record, or TLS watcher log was found in the checked local app-data and temp locations. Thus this recheck adds no runtime hit evidence.

The on-disk release DLL is SHA-256 `37A71B1B4C2F48309B2DA7C6627E132B484C7B3E0599D2B5A26381C13C7A71EA`, 1,825,280 bytes, last written `06:22:39 +01`; `src/game_probe.rs` was last written `10:15:32 +01`. A read-only ASCII-string check confirms this DLL contains the older candidate-tap code but not the newer `ray_input_record` or `collector_state` capture labels/fields. It predates the current source edits by nearly four hours and is not the current capture candidate. Do not install it as the next capture build. The separate installed ASI remains the known-good SHA-256 recorded above; no process memory was written, no thread suspended, and the game was not closed or relaunched.

### Static audit of the unbuilt capture edits (2026-10-28 10:19 +01)

Read-only source inspection found the current `src/game_probe.rs` edits are not a buildable capture candidate. `WorldQueryReturnStack::push` now returns `WorldQueryReturnPush` (lines 324–335), while `record_world_query_call` still assigns that result to `pushed` and uses it as a Boolean (lines 2719–2725). `cargo fmt --all -- --check` also reports formatting differences; it is not a type check. No build was run in this read-only pass.

The attempted overflow handling is not safe to repair by merely converting the enum to a Boolean: `push` increments `overflow_depth` and returns `OverflowTracked` when full, but the callsite only enables the return tap for a successful stored frame. Without that return tap, `pop` never decrements the overflow depth, so `current_sample_index` stays suppressed on that thread. Separately, the helper still pushes only inside the sampled-call branch; unsampled nested queries therefore still have no no-sample TLS marker and can expose the outer sampled frame to candidate taps. A `WorldQueryCandidateStack` is defined but no callsites use it. These are concrete implementation gaps to resolve before producing or installing a fresh DLL.

This is a static source audit only. The game and watchers remained responsive, and no process memory was changed.

### Newest available candidate DLL identity (2026-10-28 10:20 +01)

A second read-only artifact check found the newest available release candidate is the scratch-target DLL last written `08:29:11 +01`, SHA-256 `561CF45849E8C7A211B01AEFF45EB5FF6AC6341B91EDEF7DBC1A1A16D7F7CE53` (2,484,736 bytes). Its embedded diagnostic strings include the prior `local-ray@` output, but do not include the newer source logger labels `ray-input@` or `collector@`; the source has those labels and its timestamp is `10:15:32 +01`. The ordinary `target/release` DLL is older still (06:22:39, SHA-256 `37A71B1B4C2F48309B2DA7C6627E132B484C7B3E0599D2B5A26381C13C7A71EA`). This confirms neither available DLL contains the currently edited diagnostic output. Both remain uninstalled. The string comparison establishes stale logging format, not a full instruction-level audit of every embedded hook.

### Public SDK physics surface cross-check (pinned commit)

Fetched `game/go/physgo.h` from public `gigaHours/mm_sdk` at the pinned revision `f2dc640769c88aae9c3965a2aca0aa138f0e62ab`. `CPhysicsGameObject` declares physics-AABB access, collision callbacks, and `GetPointClosestTo`, but no ray-cast/world-query method. The pinned `game/go/vehicle.h` separately declares vehicle getters/setters (including velocity/acceleration) and likewise no ray-query method. This is limited to the checked public SDK interfaces, not proof that no internal ray API exists elsewhere in the game. It corroborates that the current reverse-engineered helper path extends beyond the documented SDK surface.

Sources: [pinned physgo.h](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/physgo.h), [pinned vehicle.h](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/vehicle.h).

### Loaded module identity confirmation (2026-10-28 10:23 +01)

Read-only enumeration of PID `20880`'s loaded modules confirms `MadMaxModding.asi` is loaded from `C:\Program Files (x86)\Steam\steamapps\common\Mad Max\scripts\MadMaxModding.asi`; its current on-disk SHA-256 is `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`, matching the known-good installed build. Neither available candidate DLL is the loaded module. PID `20880` remains responsive. This verifies which instrumentation build produced the current log; it does not add new raycast semantics.

### Live hook-state byte check (2026-10-28 10:24 +01)

Used the existing `trace_xref_probe.exe` `--peek-any` path against PID `20880`; its source requests only process query/read access. Current live bytes:

- Helper VA `0x14252EDF0`: `E9 1E 21 AC FD 57 48 8D 68 98 48 81 EC 50 01 00` — the first five bytes are a detour jump, consistent with the installed shared-helper hook.
- Candidate callback VA `0x142526AA0`: `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48` — matches the expected original entry signature, so the candidate callback hook is not installed.
- Group-filter method VA `0x1424E7580`: `48 83 EC 28 45 8B 49 2C 45 8B 40 24 48 83 C1 D8` — matches the expected original signature, so the group-filter hook is not installed.

Two other observed entrypoints are detoured in this run: camera-ray callback VA `0x141EAC4C0` begins `E9 CE 4A 14 FE ...`, and `CWeapon::GetRayWorldPosition` VA `0x141E1BBF0` begins `E9 5E 53 1D FE ...`. The bounds predicate VA `0x1425267E0` still begins with its expected original signature `48 83 EC 28 48 8B 41 38 0F 29 74 24 10 44 0F 29`; thus there is no installed bounds-result tap either.

This directly explains why current runtime logs can contain helper-level return samples but no candidate/filter records. It is not evidence that the game made no raycasts. No process memory was written and no thread was suspended.

### SDK reconstruction method and ray-query boundary (2026-10-28 10:43 +01)

The pinned public SDK commit `f2dc640769c88aae9c3965a2aca0aa138f0e62ab` gives direct evidence of an incremental interface-reconstruction workflow. In its `game/go/physgo.h` diff, generic virtual placeholders (`CPhysicsGameObject__Function01` through `Function10`) are replaced by named declarations such as `GetPhysicsAABB`, `SetKeyframedMotionState`, `HitByBullet`, and `GetPointClosestTo`. The same commit labels several `IPfxGameObject` signatures “invalid args, just for padding” while retaining their vtable positions, and adds a `CVehicle` class as a list of pure virtual declarations. Other changed headers use `CMETHOD` / `CMETHODV` with separate GoG and Steam addresses. This shows the SDK is a progressively refined map of class interfaces, vtable order, and build-specific entrypoints; it is not a full source reimplementation. The commit diff is available at https://github.com/gigaHours/mm_sdk/commit/f2dc640769c88aae9c3965a2aca0aa138f0e62ab.

For the ray-query target, the checked public `CPhysicsGameObject` and `CVehicle` interfaces expose no ray-cast method. Repository code searches for `RayCast`, `RaycastStaticFromCamera`, `CRayCasterObject`, `GetRayWorldPosition`, and `castRay` returned no matches; search indexing is not proof of absence from every file or branch. Separately, the already-mapped executable path is concrete: the shared game helper (`RVA 0x252EDF0`) builds a query context, calls the cast adapter (`RVA 0x2B0DDE0`), dispatches through the live `hk3AxisSweep::castRay` virtual at vtable offset `+0xC8`, and processes candidate callbacks plus an `hkpCustomGroupFilter`. Thus ray queries are an internal engine path beyond the checked SDK surface, not a missing method we can simply port from `mm_sdk`.

Process consequence: use the SDK’s style of evidence-based interface recovery—preserve unknown slots and exact per-build addresses, then name a method only when disassembly/callers support it. For this ray path, first produce a typed, evidence-backed map of the helper/context and the adapter’s collector/filter/candidate ABI. Keep each runtime experiment to validating a specific link in that map. The currently installed build still has only helper and camera/weapon taps; its log remains unchanged at 19,246 bytes (last write 10:12:46 +01), so this adds static/API-history evidence and no new runtime hit evidence.
### Fresh caller observations in bootstrap log (2026-10-28 11:25 +01)

The installed build's `bootstrap.log` advanced from 19,246 bytes (last known write 10:12:46 +01) to 19,406 bytes (last write 11:09:23 +01). Its new caller-only records are `0x1D1B6C8` at 11:09:06 and `0x1CAF3F1` at 11:09:23. The existing caller-label table maps these to `Selector` and `CRayCastFanCondition::Evaluate`, respectively; the table's `Seen this run` entries are updated. These are unique-caller observations, not per-caller counts, query-result captures, or evidence of a geometric hit.

At the 11:25 read-only check, the previously observed PID `20880` was absent from both `Get-Process` and the available `Win32_Process` enumeration. The process state could not be confirmed from this session. No process was launched, closed, attached, suspended, or written by this check.

### Candidate callback and group-filter capture build (2026-10-28 12:05 +01)

Repaired the bounded query-return stack so overflow disables active sample attribution, and unsampled helper calls now install no-sample return frames to prevent nested calls from inheriting an outer sample. Wired the already-discovered candidate callback into a per-thread candidate context. Added a signature-checked observer at group-filter RVA `0x24E7580`; it forwards the original call and return pointer, and only reads the raw ray/candidate filter dwords plus the output byte while a selected candidate callback is active. The logger prints the decision byte and raw words without assigning semantic names to their bit fields. The group-filter observation is labeled `unobserved` when the callback has no matching result.

`cargo check --locked --offline` and a Windows x64 release build completed successfully. No tests were run. Candidate DLL: `%LOCALAPPDATA%\Temp\MM-Cargo-RayCapture\release\madmax_bootstrap.dll`, 2,654,720 bytes, SHA-256 `055F99533BAD48AB53D9EC85787C4E4C1EDED46E1245A74277773130F105D120`. After the user closed Mad Max, the candidate was installed as `scripts\MadMaxModding.asi`; the previous known-good ASI is backed up as `MadMaxModding.asi.before-group-filter-capture-20261028` (SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`). Installed and candidate hashes match. This is not yet runtime-verified; launch and user-triggered ray activity are needed to confirm the callback and filter logs.

### Candidate rollback after startup crashes (2026-10-28 12:12 +01)

The candidate was launched after the install. The log confirms the helper, candidate-callback, bounds-predicate, and group-filter hooks installed. Windows Application Error events then recorded eight access violations in repeated launch attempts: five at `MadMax.exe+0x252efa9` (inside the shared world-query helper), one at `+0x25267f7` (inside the bounds predicate), and two at `+0x1fa5a28` (the `UpdatePostSim` area). The candidate is unsafe; the events do not isolate which hook or read caused the failures. The verified prior ASI backup has been restored to `scripts\MadMaxModding.asi` (SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`). Mad Max is closed. Do not ask the user to retry this candidate. Investigate the exact faulting instructions and narrow the experiment offline before any later runtime attempt.

### Exact helper crash instruction and rollback baseline retest (2026-10-28 13:40 +01)

Disassembled the saved helper snapshot around the repeated candidate-build fault at RVA `0x252EFA9`. The instruction sequence is:

```asm
0x14252EFA4  TEST R9,R9
0x14252EFA7  JZ   0x14252EFAF
0x14252EFA9  MOV  EAX,dword ptr [R9]
0x14252EFAC  MOV  dword ptr [RBP-0x7C],EAX
```

Thus the reported access violation is the game helper dereferencing its non-null fourth register argument. This identifies the faulting read, but without exception registers it does not establish whether the pointer was invalid at entry, became invalid concurrently, or was affected by instrumentation. The candidate build installed the candidate-callback, bounds-predicate, and group-filter hooks together with the revised return-stack handling, so the earlier crashes still do not isolate one change.

The restored ASI was retested by the user: after two failed launches, a launch at 13:25 reached the menu. Its on-disk SHA-256 remains `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. The successful run installed the helper tap and logged at least 36,332 helper samples by 13:31, including caller RVA `0x1DA5892`; these are query-helper observations and do not alone prove a geometric hit. The game is now closed. Next candidate should add the callback observer in isolation, leaving bounds and group-filter hooks disabled; retain separate crash records and compare before asking for another gameplay run.

### Bounds-hook register-preservation hypothesis (2026-10-28 13:42 +01)

Disassembly establishes that helper `0x14252EDF0` calls the bounds predicate at `0x1425267E0` (`helper+0x18A`), then later tests and dereferences `R9` at `helper+0x1B4..+0x1BC` (`MOV EAX,[R9]` at `+0x1B9`). The original bounds predicate body uses `RAX` and SIMD registers for its calculations and does not write `R9`. The added Rust detour calls Rust capture/TLS code before and after forwarding the predicate; as an ordinary Windows x64 function it may legally clobber volatile `R9`, unlike this observed original body. This is a concrete, plausible explanation for the repeated helper fault, not yet runtime-proven. Next build should leave the bounds and group-filter taps uninstalled and enable only the candidate callback observer, then check startup and callback records. If the helper fault disappears, keep the bounds observer off or replace it with a register-preserving assembly shim before attempting that capture again.


### Candidate callback-only isolation build (2026-10-28 13:56 +01)

Following the SDK's incremental interface-recovery approach, rebuilt the current bounded capture source with exactly one new tap enabled: the already identified `hkProxyRayCaster::addBroadPhaseHandle` candidate callback at RVA `0x2526AA0`. The existing shared-helper observation remains; the bounds-predicate and group-filter hooks are not installed. The install path logs this exact configuration at startup. This isolates the candidate callback from the previously combined bounds/group-filter experiment; it does not establish the callback's ABI semantics or prove geometric hits.

`cargo build --release --locked --offline` succeeded. Candidate DLL: `%LOCALAPPDATA%\Temp\MM-Cargo-RayCapture\release\madmax_bootstrap.dll`, 2,650,112 bytes, SHA-256 `37D96B62F89738C17753B409112CEAB892825CD202668C488E9A135718EA155A`. Before deployment, verified Mad Max was not running and the installed ASI matched known-good SHA-256 `9814AA43AAC199DA829C1320E2FD606F1799DEAA319E8D3A9ADB8903B4D73B2B`. Preserved that baseline as `scripts\MadMaxModding.asi.before-candidate-only-20261028`; installed candidate and verified its SHA-256 matches the DLL. Runtime test pending. If startup fails, stop retrying; after the game is closed, restore the named backup.

### Manual firing run with candidate callback observer (2026-10-28 14:02 +01)

The user launched the candidate build, loaded a save, and fired several shotgun shells at the ground. `bootstrap.log` contains the expected startup marker: “candidate callback tap only; bounds and group-filter taps are disabled.” The user is correct that this adds no visible in-game UI; all diagnostics are file-only.

The log contains 32 bounded samples labeled `CAmmunition::CheckCollisions`, sampled at 14:00:56–14:01:12 +01 during the run. In each sample the shared helper returned `1`; the candidate callback count was 10–13, five candidate rows were retained, and the minimum candidate fraction exactly matched the ammunition output fraction (sampled output fractions approximately 0.13–0.40). This links the isolated candidate callback activity with the ammunition query output during the user’s controlled firing run. It is strong correlation, not yet a proven semantic contract: the retained candidate row/slot fields and the relation to the actual ground impact still need controlled comparison and careful decoding. The game process was still running during log inspection; no process interaction was performed.

### Cross-caller fraction/result correlation from candidate-only run (2026-10-28 14:08 +01)

Reparsed the full current log, selecting only helper-return samples that carry the candidate-callback fields. Across 61 samples from the user’s run, the association is complete in this bounded set: all 48 helper returns of `1` have a candidate minimum below `1.0`, and all 13 helper returns of `0` have a candidate minimum exactly `1.0`. The 13 false/1.0 samples include `GraphScriptRaycastStatic`, `wind_occlusion`, `vehicle_sound_occlusion`, `CRayCastFanCondition::Evaluate`, and other identified callers; the 48 true/sub-1 samples include the 32 shotgun `CAmmunition::CheckCollisions` records.

For each of those 32 ammo samples, the `candidate-min-fraction-query-thread` exactly equals the caller’s `ammo-result=true-return-output-scalar` (range about 0.13–0.40). Static disassembly already established that the candidate callback invokes a game-specific candidate virtual and returns the per-ray record’s float at `+0x08`; the ammo caller separately consumes helper output `+0x14` to interpolate a position along the 15 m segment. The runtime equality strongly connects the callback fraction to the ammo caller’s selected segment fraction. The cross-caller return/minimum pattern also supports `1.0` as a no-closer-candidate sentinel on these sampled paths.

Scope limits: this is one live session with the present 144-sample bounded schedule; candidate callback attribution is thread-local and is not claimed as a global callback count. The user’s shotgun shots were aimed at the ground, so this run contains no controlled ammo miss. Do not yet generalize the Boolean/sentinel contract to every caller, call the nearest candidate a confirmed physical surface hit, or infer normal/material/object semantics. The next discriminating test is a single shotgun shot aimed high into clear sky, using the still-running candidate build; compare its ammo return, candidate count/minimum, and output scalar with the existing ground-directed shots. No restart or reinstall is needed for that read-only log capture.

### Sky-shot activity observed after ammo detail quota filled (2026-10-28 14:11 +01)

The user fired three shotgun shells aimed into the sky while the same candidate build remained loaded. The per-five-second aggregate caller summaries show `CAmmunition::CheckCollisions` activity of 10 calls and then 20 calls in successive windows around 14:10:35 and 14:10:40 +01, consistent with the manual test reaching the ammo query path. No detailed ammo samples followed because the build's fixed first-32-ammo-call capture budget had already been consumed by the ground-directed test. Therefore these sky shots provide activity confirmation only, not a miss result. No game/process changes were made. To capture the control, restart Mad Max with the same installed candidate (no rebuild/reinstall), load the save, and fire one clear-sky shell before any ground-directed shots so the new process's sample quota is available.

### Controlled clear-sky ammunition miss (2026-10-28 14:17 +01)

After restarting with the same candidate-only ASI, the user loaded the save and fired one shotgun shell into clear sky before any new ground shots. The fresh process captured ten `CAmmunition::CheckCollisions` samples (RVA `0x1DA5892`) at 14:17:02 +01. Their six-float ray inputs had upward directions with Y components about `+0.76..+0.85`, each segment length was 15.0, the helper returned `0`, nine query-thread candidate callbacks were observed per ray, and the minimum callback fraction was exactly `1.0` for all ten. Each log record took the caller's `false-return-endpoint` path, whose endpoint equals origin plus direction times the full 15 m segment.

This is the requested controlled contrast with the prior ground-directed run: 32/32 ground ammo samples returned `1` with candidate-minimum/output fractions below 1; 10/10 upward clear-sky samples returned `0`, candidate minimum `1.0`, and the endpoint path. Together with the static callback disassembly (candidate callback returns per-ray record `+0x08`) and the ammo caller's static use of output `+0x14` to interpolate along its segment, this validates an ammo-path ray-query contract: a fraction below 1 produces a usable along-segment result; the no-result path retains the 1.0 sentinel and uses the full endpoint. This is now runtime-confirmed for `CAmmunition::CheckCollisions` in this Steam build.

Scope: do not generalize the ammo helper's Boolean semantics to every caller, or claim that this query is the camera/aim ray used for spawn placement. The candidate object mapping, surface normal, material, and safe way to submit an arbitrary camera ray are still unresolved. The same game process remained open during log inspection; no in-process state was modified.

### Tenfold-budget ground/sky sequence and SDK coverage check (2026-10-28)

The user fired four shotgun shells in this order: ground, sky, ground, sky. The installed 320-sample build recorded 40 consecutive `CAmmunition::CheckCollisions` world-query calls, ordinals 0–39. The records divide into four ten-call groups matching the user-reported order:

| Shot | Sample ordinals | Mean ray direction Y | Helper return | Ammo caller branch |
|---|---:|---:|---:|---|
| Ground | 0–9 | -0.60 | 1 (10/10) | `true-return-output-scalar` (10/10) |
| Sky | 10–19 | +0.68 | 0 (10/10) | `false-return-endpoint` (10/10) |
| Ground | 20–29 | -0.78 | 1 (10/10) | `true-return-output-scalar` (10/10) |
| Sky | 30–39 | +0.81 | 0 (10/10) | `false-return-endpoint` (10/10) |

All sampled segments were 15 m. This controlled alternating sequence reproduces the previously observed ground/sky contrast in one process and shows that the larger budget captured later shots after ordinal 32. It does **not** exhaust the new 320-sample quota, establish one world-query call per pellet/ray, prove a universal helper contract, or identify the candidate object's type, normal, or material. Keep those as separate questions.

**Community SDK boundary:** inspected the complete recursive public `gigaHours/mm_sdk` tree at commit `f2dc640769c88aae9c3965a2aca0aa138f0e62ab`, including `game/go` headers, and searched that repository for `CAmmunition` and `CheckCollisions`; neither symbol was found. The SDK's checked gameplay object headers cover character, damageable, game object, PFX object, physics object, and vehicle interfaces, but not this ammunition caller. Therefore the ammunition callsite and behavior are binary-derived findings recovered in this project, not a port of a declared `mm_sdk` interface. This is a statement about that specific public repository snapshot, not a claim that no private/community notes exist elsewhere.

Public evidence: [pinned mm_sdk commit and complete tree](https://github.com/gigaHours/mm_sdk/tree/f2dc640769c88aae9c3965a2aca0aa138f0e62ab), [vehicle interface](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/vehicle.h), [game/go directory](https://github.com/gigaHours/mm_sdk/tree/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go).

Reusable methods and evidence-record conventions are in [`reverse-engineering-playbook.md`](reverse-engineering-playbook.md).

The same four groups recorded 10–11 candidate-callback invocations per ground ray group, with group minimum fractions ranging 0.122–0.163 and 0.083–0.104 respectively. Both sky groups had ten candidate callbacks per query and a minimum fraction of exactly 1.000 for every sample. These are callback/candidate measurements, not ten or eleven confirmed physical impacts; the min-fraction split agrees with the caller branch and ray direction in this controlled sequence.

### World-query dispatcher reaches Havok broadphase `castRay` (2026-10-28)

Re-examined the saved runtime function snapshots for `WorldQueryHelper` (`0x14252EDF0`, 874 captured bytes) and its callee `DispatchRayCastQuery` (`0x142B0DDE0`, 287 captured bytes). This produced the first direct link from the game query helper to a named Havok world-query virtual:

- `WorldQueryHelper` forms two endpoint vectors from the caller's six-float ray input and its start/end distance scalars, subtracting the three floats at `queryBase + 0x40`, `+0x44`, and `+0x48` from both. Those three values are therefore a shared query-space origin/offset, but their owning type and semantic name remain unresolved.
- It passes the generated endpoint block to `DispatchRayCastQuery`, along with a broadphase pointer loaded via `queryBase + 0x38`, then that object's `+0x88` pointer, and a filter-related pointer from `queryBase + 0x38` object's `+0xC0`.
- The dispatcher constructs a stack input matching public Havok `hkpBroadPhase::hkpCastRayInput`: a four-float `from` vector from the first endpoint, `numCasts = 1`, an endpoint-array pointer to the second vector, `toStriding = 0x10`, and the helper's `option5` passed as `m_aabbCacheInfo`.
- It calls the broadphase object's vtable byte offset `0xC8` (zero-based pointer-sized slot 25), with `(broadphase, &input, callbackState, 0)`. The final zero is the collector stride. This argument and record shape match the declared `hkpBroadPhase::castRay(const hkpCastRayInput&, hkpBroadPhaseCastCollector*, int)` in a public Havok header. This is strong binary-to-API correspondence, not proof that the game embeds the exact public header revision. The game identifies its Havok build as `2013.2.0.8.2.0.942983` in the earlier ABI inventory.
- The callback state has observed writes at offsets `+0x08` (ray endpoints), `+0x10` (filter-derived pointer or null), `+0x18` (query context), `+0x20` (zeroed field), and `+0x58` (conditional filter-derived pointer or null). Its first qword is initialized to `0x14132BCE0`. The object's class and remaining fields are not yet mapped; do not treat it as a Rust layout.

The callback object's class is now resolved: the MSVC complete-object locator immediately preceding vftable `0x14132BCE0` points to type descriptor RVA `0x017CAC80`, whose name is `.?AVhkProxyRayCaster@@`. The vftable's second pointer is `0x142526AA0`, matching the RTTI-vtable-derived `hkProxyRayCaster::addBroadPhaseHandle` mapping. Thus the chain is now directly connected: `WorldQueryHelper` → `DispatchRayCastQuery` → a stack-allocated `hkProxyRayCaster` callback → `hkProxyRayCaster::addBroadPhaseHandle`. The 0x08..0x58 writes belong to this observed callback state, but are still only partial offsets; they do not define a safe Rust/native object layout.

Added opaque Rust declarations for `HkpBroadPhase` and a 0x30-byte, 16-byte-aligned `HkpCastRayInput`, plus the `HkpBroadPhaseCastRayFn` ABI alias and build-specific vtable slot constants in `src/api/game/abi/game/physics.rs` and `src/api/game/abi/game/world_query.rs`. These are documentation/ABI declarations only: no call path was implemented. The input structure's offsets are supported by dispatcher writes and cross-checked against the [public hkpBroadPhase header](https://github.com/nitaigao/engine-showcase/blob/master/etc/vendor/havok/Source/Physics/Internal/Collide/BroadPhase/hkpBroadPhase.h); its cache pointer's lifetime and required world locking are still unknown.

**Next useful mapping:** identify the owning broadphase/filter objects behind `queryBase + 0x38` and its fields, then trace the callback's filter and narrow-phase path toward `addRayHit`. Do not invoke the `0xC8` virtual from a mod until callback object layout, owning world lifetime, query phase/thread, and reentrancy requirements are established.

### Proxy callback's candidate-to-query transition (2026-10-28)

Inspected the saved 384-byte runtime snapshot for `hkProxyRayCaster::addBroadPhaseHandle` (`0x142526AA0`). The callback:

1. Uses the candidate's broadphase-handle payload to locate a candidate object and checks a virtual type value (`candidate + 0x05`-selected nested record; virtual slot `+0x30`) equals `3`. The field/tag interpretation and meaning of type `3` are not established.
2. Computes a per-cast query-context pointer as `callback + 0x18 + callback[0x20] * castIndex`. In the observed dispatch path, the collector stride field at `+0x20` is set to zero, so every cast would share the same context pointer; only one cast is submitted in this call.
3. Transforms both endpoints into a candidate-local frame using float fields read from the candidate object, writes two four-float vectors plus an additional scalar into the callback state at `+0x30..+0x50`, then invokes that candidate object's virtual slot `+0x80`, passing the local ray-input block and query-context pointer.
4. Returns a float from `queryContext + 0x08`. If the candidate does not satisfy the observed tag/type checks, it delegates to `hkpWorldRayCaster::addBroadPhaseHandle` at `0x142B0DC60` and returns that result.

This bridges broadphase candidate enumeration to a candidate-specific local-space query and back to the same per-ray float that correlated with `CAmmunition` output in the controlled ground/sky runs. The object at the nested candidate pointer, virtual slot `+0x80`, local input field semantics, filter predicate, and returned fraction's universal scope remain unknown. Do not label slot `+0x80` as Havok shape casting yet: the three-argument indirect call in this snapshot does not match the public `hkpShape::castRayWithCollector(input, cdBody, collector)` prototype as written, and the receiver may be a game-side object/adapter. The public [Havok hkpShape header](https://github.com/nitaigao/engine-showcase/blob/master/etc/vendor/havok/Source/Physics/Collide/Shape/hkpShape.h) describes the collector-driven shape API, but it is only a comparison point.

This mapping is binary-derived from the captured runtime body and is consistent with the prior live candidate-fraction captures. No new game run was performed. The next static step is to identify the object reached through the broadphase-handle payload and inspect the method table used by virtual slot `+0x80`; the next live step, if static analysis cannot disambiguate the local-input/output fields, should capture a bounded before/after snapshot of that callback for one ground and one sky shot.

### Fallback collector reaches the shape ray-hit collector API (2026-10-28)

The 357-byte runtime snapshot of `hkpWorldRayCaster::addBroadPhaseHandle` (`0x142B0DC60`) provides a second, more direct narrow-phase path. It resolves a candidate subobject at `candidateHandle + *(i8 *)(candidateHandle + 5)`, reads its first pointer as a shape pointer, and calls that shape's vtable byte offset `0x30` with four arguments: the shape, a ray-input scratch block at callback state `+0x30`, the candidate subobject, and the per-ray context at callback state `+0x18 + stride * castIndex`. The call argument shape matches the public Havok `hkpShape::castRayWithCollectorImpl(const hkpShapeRayCastInput&, const hkpCdBody&, hkpRayHitCollector&)` declaration. This is a strong interface match and likely identifies slot `+0x30` as the shape's collector-driven ray implementation; the embedded game's exact target function, `hkpCdBody` interpretation of the candidate subobject, and layout of the 0x30-byte scratch input remain to be confirmed.

The data-flow now has a defensible layered map: broadphase `castRay` enumerates candidates; the `hkProxyRayCaster` override handles one game-specific candidate class through its unresolved virtual at `+0x80`, otherwise it delegates to the base world ray caster; the base path calls a shape-level collector ray method, whose collector argument is the per-ray context; but filter-context vtable functions must not be conflated with the hit-collector callbacks. A later live-code check found that at least two old static `addRayHit` labels were scalar deleting destructors; the concrete collector callback and its hit-record update path remain a separate unresolved link.

Added opaque `HkpShape` and `HkpShapeRayCastInput` markers and a descriptive `HkpShapeCastRayWithCollectorImplFn` alias to Rust. No shape input layout or fixed target address is claimed. These additions and the broadphase declarations are compile-checked as part of the crate; they remain non-callable documentation until object identity, world phase/thread access, and memory lifetime are proven.

### Concrete collector type recovered for `wind_occlusion` (2026-10-28)

The earlier caller note identified `0x141253B20` in the `wind_occlusion` query-context area's first qword as a vtable, but deliberately did not interpret the context as a collector at that point. This turn decoded its MSVC RTTI complete-object locator and base-class array: its type is `CIgnoreFilter`, with bases `CIgnoreHoleyFilter`, `CRaycastFilter`, `hkpClosestRayHitCollector`, and `hkpRayHitCollector`. The world-query candidate/fallback paths pass the same query-context pointer as the per-ray collector argument. For this caller, the collector identity is therefore RTTI-confirmed and its inheritance chain reaches the exact Havok hit-collector interface used by shape ray casts.

The `CIgnoreFilter` vtable also confirms a build-specific address (`0x141253B20`, RVA `0x01253B20`) and its slot `+0x08` currently points to `0x141D1B250`. That target remains unnamed because the static executable's `.xpdata` body is only a one-byte analysis stub. It may be an inherited or overridden `addRayHit` implementation; do not rename it until its relationship to `CIgnoreHoleyFilter` / `CRaycastFilter` vtables is resolved. The Rust SDK now contains an opaque `CIgnoreFilter` identity and the vtable RVA, without an invented layout or callback prototype.

This resolves one important part of the query-context ambiguity without overgeneralizing: `WorldQueryHelper` accepts caller-specific collector/context objects, and `CIgnoreFilter` is now confirmed for `wind_occlusion`; it is not asserted as the collector for every query caller. The next static step is compare the `CIgnoreFilter`, `CIgnoreHoleyFilter`, and `CRaycastFilter` vtable entries to name the `0x141D1B250` callback, then find which fields it updates and whether those correspond to the helper's returned output record.

### Camera script callback and GraphScript query activity observed (2026-10-28, 17:16–17:31 UTC)

The user reports that the in-game `Camera ray tap` counter reached 535. In this build, `camera_raycast_hook` increments that counter immediately on entry to the registered `RaycastStaticFromCamera` callback at RVA `0x1EAC4C0`, then forwards the original two-pointer callback and returns its `u32` result. The tap does not inspect either argument or the return value. Therefore 535 confirms 535 callback entries since this plugin instance installed the tap; it does not mean 535 successful geometric hits.

The current `bootstrap.log` independently records 16 bounded `WorldQueryHelper` samples for caller return RVA `0x1EAC351`, labeled `GraphScriptRaycastStatic`, at Unix time `1793207770` (2026-10-28 17:16:10 UTC). Their `capture-ordinal` values are 0–15; all 16 helper return values are 0, each observed 9–10 candidate callbacks on the query thread, and each candidate minimum fraction is `1.0`. The rays vary over the sample set: segment scale decreases from about 107.5 to 98.9, while the origin and direction change slightly. Five-second caller-activity summaries also contain `0x1EAC351`, including windows with 148 and 146 calls. This proves that the registered `RaycastStatic` internal body reached the shared helper during this session, superseding the earlier statement that its helper call had not been observed live. These samples show the no-closer-candidate pattern for this bounded group, but do not identify why the queries ran or whether the camera entry caused them.

The 535 camera-entry count and the `GraphScriptRaycastStatic` helper activity are separate instrumentation signals. They are compatible with related camera/script behavior but do not by themselves prove that each `RaycastStaticFromCamera` call enters `GraphScriptRaycastStatic`, nor that the 16 helper samples came from the camera entry. The helper caller logger identifies the latter path; the camera tap currently records no arguments, call stack, ray, or result. A next capture should add narrowly bounded argument/result records to the camera tap (and preserve the current pre-forward count) so the two paths can be joined directly. The game remained open; inspection was limited to the existing log and source, with no process interaction.

The user additionally reports that the camera counter rises quickly during some periods and remains unchanged during others, without a known triggering action. Nearby five-second helper summaries show a similarly intermittent rate for `GraphScriptRaycastStatic`: 12, 21, 78, and 19 calls in several windows, followed by 46, 54, 148, 146, 66, and 23 in later windows. `vehicle_sound_occlusion` (`0x21DC95A`) is also active in those windows, commonly around 144–183 calls per five seconds. This temporal pattern is consistent with multiple gameplay systems entering and leaving active phases, but the counters have different hook locations and no shared event key; it does not identify the camera callback's trigger or prove that the two streams have the same cause. In particular, `0x21DC95A` is the already identified vehicle-sound occlusion caller, not the camera callback. Treat the user's 535 count as a callback-entry count, not a hit count or ray result.

### RTTI identity for a broadphase ray candidate (2026-10-28)

The bounded candidate records contain a candidate object whose primary vftable pointer is `0x141325C18` (RVA `0x01325C18`). Read-only inspection of the MSVC complete-object locator immediately preceding the table gives TypeDescriptor `0x1417C9DF8`, whose decorated name is `.?AVCCustomSampledHeightFieldShape@@`. Thus at least one candidate object seen by `hkProxyRayCaster::addBroadPhaseHandle` is the game's `CCustomSampledHeightFieldShape`; this is RTTI-confirmed object identity, not an inference from its behavior. The related candidate record separately reports a nested subobject vftable at `0x1413FB788`, whose RTTI identifies `hkpRigidBody`; keep those two pointers and class identities distinct.

The `CCustomSampledHeightFieldShape` table has a target at slot byte offset `+0x30` of `0x1424F6880` and at `+0x80` of `0x142B1B0A0`. The previously captured `hkProxyRayCaster` body checks the candidate object's `+0x30` virtual result against `3` and has a candidate-specific `+0x80` path, but the candidate log records only the virtual target address, not the `+0x30` return value. Therefore this row alone does not prove that this particular shape took that branch. The capture's `virtual=first/second` diagnostic means candidate-object `+0x30` / nested-subobject `+0x80`; the second logged value is **not** the candidate shape's `+0x80` target. The shape table was read separately to resolve its own `+0x80` target correctly.

The target at `0x142B1B0A0` has no useful function body in the on-disk Ghidra import: its bytes do not decode as meaningful x64 code, consistent with the earlier executable-versus-materialized-runtime limitation. Ghidra bookmarks now preserve both the RTTI-confirmed shape table and the unresolved slot target. Rust contains only an opaque `CCustomSampledHeightFieldShape` marker and its vftable RVA; no size, layout, method name, or callable API is asserted. A runtime code snapshot of `0x142B1B0A0` is the next targeted step if the game is available again.

The user also reports that the camera callback counter begins climbing rapidly after crossing a particular in-world spatial boundary/distance, then stops after stepping back a small amount. The exact coordinates and trigger are unknown. This remains a user-observed clue about `RaycastStaticFromCamera`; it is not evidence that the candidate shape above causes the behavior. The game was accidentally closed before any further process inspection, and no relaunch was attempted.

### RTTI inheritance chain for sampled-height-field ray candidates (2026-10-28)

Decoded the MSVC `ClassHierarchyDescriptor` for `CCustomSampledHeightFieldShape` from the on-disk `MadMax.exe` Ghidra program. The hierarchy descriptor at `0x1414F1EB0` reports seven base-class entries and an array at `0x1414F1EC8`. The array points to seven `BaseClassDescriptor` records whose TypeDescriptors decode to this hierarchy:

```text
CCustomSampledHeightFieldShape
└─ hkpHeightFieldShape
   └─ hkpShape
      └─ hkpShapeBase
         └─ hkcdShape
            └─ hkpReferencedObject
               └─ hkBaseObject
```

The decorated type names were read from the corresponding TypeDescriptor strings in Ghidra; the descriptor entries report contained-base counts 6 through 0 in this order. This is a static RTTI mapping for the game binary. It confirms that the sampled-height-field candidate belongs to Havok's height-field/shape hierarchy; it does not establish the target of the candidate's virtual methods, the shape's memory layout, or whether it represents terrain for a particular observed ray. A Ghidra bookmark now marks the base-class array. Rust adds only zero-sized opaque identity markers for the newly mapped Havok base names and records the known ancestry on `CCustomSampledHeightFieldShape`.

Next useful static target: resolve the `CCustomSampledHeightFieldShape` vtable's `+0x30` shape ray-cast implementation (and determine whether the candidate-specific `+0x80` slot participates in the proxy path). The prior on-disk import does not contain meaningful code bytes for one slot target, so that particular body needs a runtime code snapshot; no game launch or process interaction has been attempted in this turn.

Follow-up vtable check: the shape's `+0x30` entry is `0x1424F6880`, and Ghidra reports that address as a one-byte `FUN_1424f6880` stub. Its on-disk bytes do not provide a usable implementation, just like the previously noted `+0x80` target `0x142B1B0A0`. Both targets now have Ghidra bookmarks. Recovering either body therefore requires a bounded runtime code snapshot while the game is running; static address labels alone are insufficient to assign method names.

To collect that snapshot without modifying the targets, the Rust probe now performs a one-time read-only check and 96-byte copy for each RVA after `UpdatePostSim` becomes executable. It logs a clear skip reason if a target is not in readable executable memory or the requested span crosses a page region. `cargo check` and a Windows x64 release build succeeded. The candidate is installed as `scripts/MadMaxModding.asi` (SHA-256 `1E4CAE40E1126C91560430BBD509448249990AAEF160877FB5AA1E9F5E395401`); the preceding verified plugin was preserved at `scripts/MadMaxModding.asi.before-shape-virtual-snapshot-20261028` (SHA-256 `257828A8D424B01F5FBB701E765172FA15F7A7EFA7480B9DBED9BF776B6B51E7`). The game has not been launched with this candidate, so snapshot output and startup behavior remain unverified.

### Live sampled-height-field ray path recovered at the main menu (2026-10-28)

The user launched the newly installed ASI and confirmed Mad Max reached the main menu. Its startup-time byte logger captured the same non-instruction-looking `.xpdata` bytes found in the on-disk import; those bytes are not a valid runtime code snapshot. A separate read-only `ReadProcessMemory` pass against the already-running game, after it reached the main menu, returned different, valid x64 code at the same addresses. This demonstrates a timing/materialization distinction for these `.xpdata` targets; the mechanism is not yet identified. The startup logger's first bytes must not be used as code evidence.

Captured 512 bytes at each sampled-height-field vtable target, 1 KiB at its dispatch helper, and 4 KiB at the helper's query kernel. Imported the raw dumps into Ghidra as separate programs under `/LiveSnapshots`, with their image bases set to the original process addresses. Disassembly and decompilation establish this chain:

```text
CCustomSampledHeightFieldShape vtable +0x30  (RVA 0x024F6880)
  -> wrapper calls 0x1424EE900 (RVA 0x024EE900)
       -> segment-query kernel 0x142467FD0 (RVA 0x02467FD0)
       -> on a true result, invokes a ray-hit collector callback
```

The kernel at `0x142467FD0` has a 0x472-byte recovered body. Its observed ABI is two pointers to three-float endpoints, an output pointer, and a byte-sized mode; it returns 0/1 in `AL`. It converts the segment's X/Z coordinates to a tiled lookup, computes an 8-by-8 table index with 0x38-byte entries, and calls helpers that traverse/query sampled-height-field geometry. The call to `CalculateSampledHeightFieldPoint` passes the heightfield singleton, the segment start, an output scratch pointer, a mode byte, and a null optional context. The traversal setup helper receives a scratch-state pointer followed by the segment endpoints. On the hit path, output words 0–2 are the normalized surface-gradient vector, word 3 is a normalized distance, and words 4–6 are an additional triple that remains unknown. The crossing helper computes the 3D distance from a stored origin vector to the candidate crossing divided by a stored segment length; the shape wrapper then copies words 0–2 into the collector's normal field and word 3 into its hit-fraction field. The wrapper ignores words 4–6. On the no-result path, the kernel writes `1.0` to word 3 and returns false. These meanings are specific to this sampled-height-field kernel; do not promote them to the world helper's universal output contract.

The wrapper at `0x1424EE900` copies six coordinate floats from a caller input into two `Vector3` endpoints, invokes the kernel with mode `1`, and, when the kernel returns true, forwards the first three result floats as the collector normal and the fourth as hit fraction, plus a collision-body pointer. The trailing three result floats are not forwarded. This is a concrete shape-to-kernel-to-collector bridge. The `CCustomSampledHeightFieldShape` vtable slot `+0x80` target (`0x142B1B0A0`, RVA `0x02B1B0A0`) independently disassembles to `or eax, 0xFFFFFFFF; ret`; its meaning at the prior proxy-callback callsite is still unknown.

One ABI question is now explicit and must remain open: the saved `hkProxyRayCaster::addBroadPhaseHandle` snapshot calls the candidate object's vtable slot `+0x30`, compares `EAX` with 3, and on the matching path later calls slot `+0x80`. The full caller assembly shows `RCX` is set to the candidate shape while `RDX` remains the candidate-handle pointer, `R8` remains the cast index, and `R9` is computed as a candidate-handle-relative pointer from signed byte offsets. The live `+0x30` body reads a float at `R9 + 8` and forwards query-shaped data to the height-field helper. This resolves why the earlier three-argument decompiler display was incomplete: the virtual call consumes live register values not expressed in its inferred prototype. It does not yet establish the semantic types of those extra values, why `EAX == 3` gates the path, or the slot `+0x80` contract. Do not call `+0x30` a pure type getter or directly invoke either slot until those questions are resolved. A Ghidra bookmark marks the virtual call at `0x142526AF7`; the runtime bytes and caller snapshot remain saved for follow-up.

Rust records the sampled-heightfield vtable, dispatcher, kernel, and recovered internal-helper RVAs in `world_query.rs`. `SampledHeightFieldRaycastOutput` now names the surface normal and hit fraction and retains the final three floats as unknown. The kernel ABI alias remains non-callable; no direct-call wrapper was added because the kernel consumes game-owned heightfield state and is only known from an in-game call chain.

Two downstream kernel helpers were then inspected from bounded, read-only live-code snapshots while the game remained at the main menu. At `0x14245CDA0` (RVA `0x0245CDA0`), `BuildHeightTraversalState` copies two 3D endpoints into scratch state, computes segment direction and length, applies a horizontal-span limit, maps X/Z coordinates to grid units, and initializes traversal boundary/step values. This is traversal setup for the sampled-height-field query, not a general world-query entry point; its full scratch layout and boundary-field meanings remain unresolved. A 4 KiB runtime snapshot was needed because the earlier 1 KiB capture truncated the function before its end.

At `0x14245C900` (RVA `0x0245C900`), `CalculateSampledHeightFieldPoint` converts an input point's X/Z coordinates using game globals, computes an 8-by-8 tile index with 0x38-byte entries, and delegates to one of two lower-level query routines depending on a mode/context pointer. It then converts returned integer-like words into normalized float outputs and writes a pointer selected from a global 0x80-byte-stride table. The meanings of these output values, mode/context, table entries, and lower-level callees are not established. Ghidra snapshots are saved under `/LiveSnapshots` as `HeightFieldRayTraversal_14245CDA0_4K.bin` (4 KiB), `HeightFieldRayTraversal_14245CDA0.bin` (1 KiB initial capture), and `HeightFieldTileLookup_14245C900.bin` (1 KiB); matching bookmarks were added in the main `MadMax.exe` program. Rust records only their RVAs with guarded descriptions; it does not expose either internal helper as a callable API.

The two point-query callees were captured next from the live process. `0x1429E00F0` (RVA `0x029E00F0`, `CalculateHeightFieldPointSample`) evaluates polynomial sample data and computes additional derivative-like values before combining them with shape scale/transform fields. `0x1429E03B0` (RVA `0x029E03B0`, `GetHeightFieldCachedTileSample`) selects a tile-indexed cache entry using packed indices; when the cached key differs, it builds that entry, then computes sample outputs. The cache entry stride is `0xC0` bytes. These are internal point-sampling helpers called by the ray kernel's heightfield path, not general raycast functions. Native field meanings, cache ownership/lifetime, and the exact interpolation model remain unresolved. Their 1 KiB live snapshots and main-image Ghidra bookmarks are saved under `/LiveSnapshots`; Rust records the RVAs without native-call wrappers.

### Grid traversal and candidate crossing helpers

The traversal loop's direct callees were captured from the same live process and renamed in their individual Ghidra snapshot programs. `InitializeHeightFieldGridTraversal` (`0x14244AFF0`, RVA `0x0244AFF0`) derives start/end grid coordinates, per-axis step directions, and traversal bounds. `AdvanceHeightFieldGridTraversal` (`0x14245D5C0`, RVA `0x0245D5C0`) advances one grid axis at a time, updates cell indices/counters, and refreshes neighboring sample indices. `CheckHeightFieldTraversalTile` (`0x14245D890`, RVA `0x0245D890`) validates the selected 8-by-8 tile entry and current vertical range. `FindHeightFieldTraversalLayer` (`0x14245D960`, RVA `0x0245D960`) samples eight nearby grid values and continues searching while their maximum remains outside the current vertical interval. `TraceHeightFieldGridSegment` (`0x14245DCB0`, RVA `0x0245DCB0`) performs the iterative traversal and delegates candidate crossings to the segment-intersection helper.

The setup path also uses `InitializeHeightFieldRaySample` (`0x14244B590`, RVA `0x0244B590`) to sample neighboring points through the tile cache and initialize interpolation bounds, plus `CalculateHeightFieldTraversalBounds` (`0x14244B420`, RVA `0x0244B420`) to derive integer grid-cell bounds. `CheckHeightFieldSegmentIntersection` (`0x14244B7F0`, RVA `0x0244B7F0`) tests a candidate crossing, computes 3D distance from a stored origin divided by stored segment length, and writes a three-component surface-gradient vector into the nested query state. Its direct callee at `0x141BB0770` is now confirmed from a live snapshot to normalize a three-component vector. The kernel's local stack layout maps that nested vector and scalar directly to output words 0–3, and the shape wrapper forwards them as collector normal and hit fraction. Thus Rust now names those four fields; only the trailing three remain raw and unknown.

The traversal and vector-helper snapshots are stored under `/LiveSnapshots/TraversalHelper_*` and `/LiveSnapshots/HeightFieldVectorHelper_141BB0770_512.bin`; the first 1 KiB initializer snapshot remains alongside its later 4 KiB complete capture. Bookmarks now mark each helper in the main `MadMax.exe` Ghidra program. Rust records their RVAs and cautious structural descriptions only.

### Weapon ray result path recovered

A bounded runtime-code snapshot of the body following the entry detour at 0x141E1BBF5 (RVA 0x01E1BBF5) decompiles as the body of the already identified CWeapon::GetRayWorldPosition at RVA 0x01E1BBF0. Its observed ABI is (weapon, output_position, ray6) -> output_position; the six input floats are read as origin (x,y,z) followed by direction (x,y,z). The function first writes origin + distance_scale * direction to the output. It then constructs a stack-local ray-query filter with vtable 0x141253B20 (CIgnoreFilter), invokes WorldQueryHelper at 0x14252EDF0, and, when the query-context fraction is below the 1.0 fallback, replaces the output with origin + (fraction * distance_scale) * direction. Thus the returned vector is a game-computed ray endpoint that becomes the nearer collision point on the observed hit branch. The native return pointer is not a hit/miss indicator.

The caller's ray record is corroborated by the existing passive observer: the first samples contain a unit-length direction and returned points approximately 10,000 world units from the origin, matching the full-range endpoint path. In the three detailed CWeapon::GetRayWorldPosition helper samples in this run, the helper returned false, the candidate callback count was zero, and the output fraction remained at its 1.0 fallback. Those records show only full-range results; they do not validate the collision branch.

A separate live snapshot at 0x141E1BF10 shows a wrapper reading three floats from its receiver at offsets +0x318..+0x320, combining them with a supplied vector, normalizing the resulting direction through 0x141BB0770, and calling CWeapon::GetRayWorldPosition. The wrapper's owning class and user-visible role are not identified, so this is not yet labeled as the player reticle path. Rust now records the recovered weapon receiver, six-float ray input, function RVA, and ABI in src/api/game/abi/game/weapon.rs; the function pointer remains unsafe, with no invocation wrapper.

Next validation: from loaded gameplay, aim at nearby ground or a solid wall and compare the logged returned point with the full-range endpoint; then aim into open sky as a miss control. The existing passive observer is already installed in the running process, so this comparison requires no plugin reinstall or game restart.
### Runtime camera-ray path mapped through GraphScriptRaycastStatic (2026-10-28)

A read-only snapshot of the live body after the entry detour for `RaycastStaticFromCamera` (RVA `0x01EAC4C0`, body `0x141EAC4C5`) shows it loading three floats from global state `0x1417F5378 + 0x2A0` and passing them into the helper at `0x141EAC0C0` along with opaque script selectors. The helper's runtime body is 784 bytes in this build (snapshot starts at `0x141EAC0C5`). It obtains a target vector through a virtual call at vtable slot `+0xB0`, copies the supplied three-float position as the ray start, subtracts start from target, and normalizes the resulting direction with `0x141BB0770`.

The helper then calls `WorldQueryHelper` at `0x14252EDF0` with the context string `GraphScriptRaycastStatic` (`0x14125D458`) and a stack query record containing that start and normalized direction. This is a direct code-level link from the named camera script callback to the shared world-query subsystem; it is the strongest current candidate for a camera/aim ray path. After the query it checks a query-state float against its initialized `1.0` fallback and conditionally dispatches a further handler at `0x141EA8D60`. The exact query-state field semantics and final script output are not yet resolved.

The latest log contains 16 bounded `GraphScriptRaycastStatic` samples from caller return RVA `0x01EAC351`. All have `result=0`, candidate minimum fraction `1.0`, and zero captured bounds results. They confirm live invocation and ray construction, but do not yet demonstrate a collision or establish that the target vector is exactly the visible reticle point. The current test therefore advances the path mapping, not the hit behavior.

The callback body snapshot is `C:\Users\David Jackson\AppData\Local\Temp\MadMaxLiveCameraRayCallback_141EAC4C5.bin`; the full helper snapshot is `C:\Users\David Jackson\AppData\Local\Temp\MadMaxLiveCameraQueryHelper_141EAC0C5_full.bin`. Both were captured with read-only `ReadProcessMemory`; no game memory was modified.

Next: identify the query record layout and the meaning of the fraction/output fields, then capture a camera ray deliberately aimed into a nearby solid surface and compare the query record before/after. Existing capture instrumentation records candidate details but may need to expose the returned record fields before a hit can be confirmed.

### Shared query helper boundary mapped from live code

A read-only snapshot of the live `WorldQueryHelper` body at `0x14252EDF5` (RVA `0x0252EDF5`, after the entry detour) shows a second layer beneath the caller-facing helper. It takes the six-float ray data (start xyz, direction xyz), a range value passed on the caller stack, a world/context pointer, and output/context records. It computes both segment endpoints, subtracts the world frame origin at `world + 0x40`, and validates those endpoints through `0x1425267E0`. When valid, it builds the lower-level query object and calls the SDK-named `DispatchRayCastQuery` at `0x142B0DDE0`.

The helper then copies query-state fields into a 0x50-byte output record. Its no-context branch explicitly initializes the result fraction to `1.0` and result vectors to zero, matching the unchanged `output@...` and `context@...` fields in the camera samples. This gives us a concrete separation: `WorldQueryHelper` owns world-space conversion, bounds gating, dispatch, and output marshalling; `DispatchRayCastQuery` is the next function to inspect for the actual geometry traversal/collector update.

This path is not yet a confirmed hit result: the captured `GraphScriptRaycastStatic` calls had candidate minimum fraction `1.0` and no bounds result, so the early no-hit state remains. The runtime body snapshot is `C:\Users\David Jackson\AppData\Local\Temp\MadMaxLiveWorldQueryHelper_14252EDF5.bin`.

### Ray-query service target identified through the live vtable

The live `DispatchRayCastQuery` body at `0x142B0DDE5` packages its ray record and invokes a virtual method at byte offset `+0xC8` on the query-service object. During this session, read-only pointer inspection resolved the active object chain to world `0x82585260` -> subobject `0x8132C90` -> query service `0x816EE00`, with vtable `0x1413E0A18`; slot `+0xC8` points to `0x142B2CD40` (RVA `0x02B2CD40`). These are runtime addresses for this process/build, not stable persisted pointers.

A 2.6 KiB live snapshot of that method shows it transforming ray coordinates through per-axis scale/offset values, clamping and quantizing them, traversing three sorted spatial candidate lists, invoking the supplied collector's virtual method at `+0x08`, and keeping the minimum returned fraction. This supports the inferred Ghidra name `TraverseRaySpatialIndex`; the owning class and exact index representation remain unknown. The static `MadMax.exe` Ghidra function was renamed and bookmarked with this caveat. The snapshot is `C:\Users\David Jackson\AppData\Local\Temp\MadMaxLiveRayQueryServiceVtableC8_142B2CD40.bin`.

The current path is now mapped as: `RaycastStaticFromCamera` -> helper at `0x141EAC0C0` -> `WorldQueryHelper` with `GraphScriptRaycastStatic` -> `DispatchRayCastQuery` -> query-service vtable slot `+0xC8` -> spatial candidate traversal and collector callbacks. Existing camera samples still have minimum fraction `1.0`, so the remaining proof is a candidate returning a fraction below `1.0` and the resulting hit data reaching the camera callback's output.

### SDK Havok ray-caster callback linked to spatial traversal

The live query-service vtable is `0x1413E0A18`; its `+0xC8` entry is `TraverseRaySpatialIndex` at `0x142B2CD40`. That traversal calls its supplied collector at vtable slot `+0x08`. The camera query wrapper initializes this collector with vtable `0x14132BCE0`, whose `+0x08` entry resolves to the SDK-named `hkProxyRayCaster::addBroadPhaseHandle` at `0x142526AA0`.

The live `hkProxyRayCaster::addBroadPhaseHandle` body confirms the callback role: it accepts `(this, broadPhaseHandle, castIndex)`, handles broad-phase-handle type `2` with shape type `3` specially by transforming the ray endpoints into the shape's local coordinates and invoking a shape virtual at `+0x80`, then returns the float from the per-cast record at `record + 0x08`. Other candidates go through the SDK-named generic fallback `hkpWorldRayCaster::addBroadPhaseHandle` at `0x142B0DC60`. This ties the query-service grid traversal to the SDK's Havok ray-caster classes and to a per-candidate fraction result. The shape virtual's identity and the exact per-cast record layout still need recovery.

The `MadMax.exe` Ghidra database now has an evidence-backed `TraverseRaySpatialIndex` name plus bookmarks for the runtime vtable slot and both SDK-named Havok callbacks. The inferred traversal name is explicitly marked provisional; the SDK names were already present in the binary analysis database.

### Primitive shape hit path recovered

The generic `hkpWorldRayCaster::addBroadPhaseHandle` path calls a candidate adapter at `0x142B13810`; the adapter invokes the candidate shape's vtable slot `+0x28`, transforms successful hit data through the shape transform, and submits it to a collector. For the observed candidate vtable `0x1413D2EB8`, slot `+0x28` is `0x142B0E7D0`. Its live body is a local-space segment/sphere intersection: it reads the sphere radius from `shape + 0x20`, uses the local origin as sphere center, checks the nearest-fraction bound, and on success writes a hit flag plus vector/fraction-like output fields. The constant center is corroborated by the zero-valued constants at `0x14138D430..0x14138D438`.

The static function is now named `CheckLocalSphereRayIntersection` with an explanatory Ghidra bookmark. This is a recovered primitive-shape mapping only; it does not establish that terrain or every environment object uses this shape implementation. The runtime snapshot is `C:\Users\David Jackson\AppData\Local\Temp\MadMaxLiveShapeLocalRayTest_142B0E7D0.bin`.

### Camera query reaches Mad Max's sampled terrain ray caster

The captured `GraphScriptRaycastStatic` sample at Unix time `1793212372` (caller return RVA `0x01FB0895`) enumerated a candidate whose virtual ray method resolves to `0x1424F6880`. In the Ghidra snapshots, that method is a forwarding wrapper into `0x1424EE900`; the helper calls `0x142467FD0`, a terrain-specific segment ray routine. That routine queries sampled height data through tile lookup/query helpers (`0x14245C900`, `0x1429E00F0`, `0x1429E03B0`) and traverses the heightfield grid (`0x14245CDA0`, plus traversal helpers). When it reports a hit, the helper submits hit data through the supplied collector callback. The addresses and descriptive names here are build-specific and the names are analysis labels, not asserted original source symbols.

This is a significant path connection: the camera `GraphScriptRaycastStatic` query can reach Mad Max's custom sampled-heightfield terrain implementation through the broad-phase candidate callback. However, the captured candidate's per-cast fraction remained `1.0`, and the query's minimum candidate fraction was also `1.0`; this sample proves terrain dispatch, not a terrain hit. The live log's later entries through Unix time `1793214131` contain no newer `GraphScriptRaycastStatic` record, so the most recent aiming action has not yet been observed in this callback log.

Clarification: the camera callback caller return RVA `0x01EAC351` itself has a bounded set of 16 detailed samples (capture ordinals 0–15). Candidate slot 1 in both the first and last samples uses vtable `0x141325C18` and ray virtual `0x1424F6880`, the sampled-heightfield ray-cast wrapper described above. Thus the camera callback's query does enumerate and dispatch to the terrain ray caster. Those bounded samples all still show per-cast fraction `1.0` and query minimum fraction `1.0`; the capture cap means absence of later detailed camera lines cannot establish whether the just-completed aiming action invoked it again.

### Heightfield hit fraction semantics

`RaycastHeightFieldSegment` writes a hit parameter into result slot 3. In the direct height-crossing branch it computes `(rayStartY - sampledHeight) / (rayStartY - rayEndY)`, which is the segment parameter `t` for the 3D ray at the heightfield crossing; grid traversal likewise writes its candidate `t` into that result slot. Thus `0` means at the segment start, `1` means at its endpoint, and values strictly between them identify an interior hit. The per-cast record starts with its nearest-hit cap at `1.0`; if no candidate supplies a nearer valid intersection, it remains `1.0`. This interpretation is from the captured heightfield code and query-record behavior.

### Confirmed terrain hit from HeightOverGround query

Correction to any broad reading of the earlier no-hit statements: the camera callback samples remain misses, but the capture already contains successful terrain results from the separate `HeightOverGround` query. Sample #1008 (caller return RVA `0x01CA3731`) reports `result=1`, a 1000-unit downward ray from `(1879.871, 481.955, 6215.952)`, and per-cast/minimum fraction bits `0x3A348106` = approximately `0.0006885681`. Applying `start + t * (end - start)` gives an approximate hit point `(1879.871, 481.2664, 6215.952)` in the logged world coordinates. This confirms that Mad Max's terrain query can return a sub-1 fraction and that the fraction plus ray endpoints reconstructs a hit position. It does not yet prove that the camera-aim ray returns a hit or that Rust can call this query safely.

### HeightOverGround caller builds a per-query filter context

A runtime-code snapshot of `EvaluateHeightOverGroundQuery` (entry `0x141CA3550`, helper call return RVA `0x01CA3731`) clarifies the context lifetime and caller setup. On each invocation, the caller reserves a local 0x90-byte query-context record and calls `InitializeWorldQueryContext` at `0x1424E5AA0` with zero arguments. The initializer writes a base vtable (`0x141324960`), initializes default float/word fields, writes its three explicit inputs at offsets `0x38`, `0x3C`, and `0x78`, and returns the same storage pointer. The caller then replaces the vtable with `0x141238388` before invoking the world-query helper. The caller also passes a distinct 0x50-byte output record and a ray block; the downward segment endpoint is built from the sampled position and the configured distance.

This is direct evidence that this call site's context is per-invocation stack state, not a long-lived cache. The helper is still stateful in the broader sense: it receives the shared world-query base loaded from global slot `0x1417F4F08`, dispatches through game-owned world/query objects, and consumes the caller's filter context and output record. We have not established whether those shared objects cache spatial data internally, nor whether arbitrary external callers can safely invoke the helper outside its normal game thread/schedule.

The installed vtable identity is confirmed from the on-disk RTTI data: vtable `0x141238388` has an x64 Complete Object Locator at `0x1414A7E00`, whose type descriptor names `.?AVCCharacterRaycastFilter@@` at `0x141793A70`. Adjacent RTTI-confirmed vtables are `0x1412383A8` for `CCCameraCollisionRaycastFilter` and `0x1412383C8` for `CPlayerAimRaycastFilter`. The RTTI/data mapping is solid, but slot names must come from live code, not the old static decompiler labels.

The camera query provides a useful contrast: it also calls the common context initializer, then installs the RTTI-identified `CStaticOnlyRaycastFilter` vtable (`0x14123D6A8`) and calls `SetRaycastFilterOption(context, true)` at `0x14202A360`. The setter writes a 32-bit value at context offset `0x70`: `0x100` when false, or `global_0x1417AABDC | 0x100` when true. The meaning of this flag/global remains unknown. The `HeightOverGround` call site does not call this setter in the recovered body; it passes zero for the helper's trailing selector/control values. This difference supports treating the context as caller-specific filter/configuration state, not a generic cache object whose fields can be guessed from one caller.

Ghidra runtime snapshots and symbols used for this mapping: `/LiveSnapshots/MadMaxHeightOverGroundCaller_20261028.bin` (`EvaluateHeightOverGroundQuery`), `/LiveSnapshots/MadMaxQueryContextInitializer_1424E5AA0.bin` (`InitializeWorldQueryContext`), `/LiveSnapshots/MadMaxStaticOnlyFilterConfigure_14202A360.bin` (`SetRaycastFilterOption`), plus `/MadMax.exe` for RTTI strings and SDK callback names. The runtime snapshots are required for these executable regions; static `MadMax.exe` bytes at corresponding `.xpdata` addresses were previously shown to be non-code before runtime materialization.


The Rust raw mapping now includes C_CHARACTER_RAYCAST_FILTER_VFTABLE_RVA = 0x0123_8388 alongside the camera's static-only filter vtable. This constant identifies a filter object used by an existing engine caller; it does not yet make constructing or invoking the world query safe from Rust.


### Runtime vtable slot and owner-context correction

While Mad Max remained open (PID 10804), read-only `ReadProcessMemory` snapshots were captured for `0x141C12540` and `0x14202A3B0`; the live vtable bytes at `0x141238388` matched the on-disk pointer table. The primary `CCharacterRaycastFilter` table entries are `+0x00 -> 0x1424F6380`, `+0x08 -> 0x141C12540`, and `+0x10 -> 0x14202A3B0`. The runtime body at `0x141C12540` writes the `hkpRayHitCollector` vtable `0x141238008`, tests the deleting flag in `DL`, and conditionally calls a release/deallocation virtual. It is a scalar deleting destructor, not `addRayHit`; the old static Ghidra function label at this address was wrong.

The live body at `0x14202A3B0` is the candidate predicate reached through `CCharacterRaycastFilter` vtable slot `+0x10`. It calls a candidate object's vtable method at `candidate+0x48` with selector arguments `1` and, if needed, `3`. If neither test passes it returns false; otherwise it returns whether the pointer at `candidate+0x28` differs from the pointer at `filterContext+0x90`. This establishes that filter context carries caller/owner identity in addition to initialized query fields.

In `EvaluateHeightOverGroundQuery`, the initialized context begins at the stack storage named `local_10f8` (0x90 bytes), and the adjacent qword `local_1068` is assigned `param_1` immediately before the world-query call. That qword is exactly `context+0x90`, which the live predicate reads. The effective local storage reserved by this caller is therefore 0x98 bytes, with 0x90 bytes initialized by the generic constructor and an extra owner pointer supplied by the caller. This is direct evidence for per-query filtering state; it still does not prove whether the shared world-query service internally caches spatial data.

Adjacent RTTI vtables include `CCCameraCollisionRaycastFilter` at `0x1412383A8` and `CPlayerAimRaycastFilter` at `0x1412383C8`. Their `+0x08` entries (`0x141C12590` and `0x141C125E0`) have now been captured live and match the same scalar deleting destructor pattern. Their `+0x10` candidate-method bodies have also been captured and analyzed, below. The old `addRayHit` labels at all three `+0x08` addresses in this vtable run are wrong; unrelated filter callback labels remain unaudited.

Ghidra's `/MadMax.exe` program now names `0x14202A3B0` `TestCharacterRaycastCandidate`, renames `0x141C12540` to `DeleteCharacterRaycastFilter`, and has bookmarks documenting the RTTI table and runtime findings. Snapshots are saved in `/LiveSnapshots/MadMaxLiveCharacterRaycastFilterAddRayHit_141C12540_1K.bin` and `/LiveSnapshots/MadMaxLiveCharacterRaycastFilterVirtual_14202A3B0_1K.bin`. The first snapshot filename retains its original capture label; its recovered role is destructor.

### Camera static-only filter candidate test

The camera helper writes `3` to context offset `+0x90` immediately after installing `CStaticOnlyRaycastFilter`'s vtable (`0x14123D6A8`). A live vtable read shows slot `+0x08` points to `0x141C7FC10`; a read-only runtime snapshot of that address matches the scalar deleting destructor pattern, so the prior static label `CStaticOnlyRaycastFilter::addRayHit` was wrong. The actual vtable `+0x10` target is `0x142039DB0` (RVA `0x02039DB0`), captured in `/LiveSnapshots/MadMaxLiveStaticOnlyFilterPredicate_142039DB0_1K.bin` and named `TestStaticOnlyRaycastCandidate` in that snapshot database. It has a four-argument Win64-style ABI: filter context, candidate object, and two additional values that it forwards into candidate classification.

The predicate calls the candidate's vtable `+0x48` with selectors `1` and `3`, setting bit `2` or bit `1` respectively when those checks pass. It rejects when the resulting category bits do not intersect the 32-bit mask at `filterContext+0x90`. The camera caller stores mask `3`, so both recognized categories pass that first gate. It then applies additional candidate type and owner/material checks; those semantics are only partially decoded. This shows that the filter context is caller-configured state: `HeightOverGround` stores an owner pointer at `+0x90`, while the camera's static-only filter stores a category mask at the same offset. The same generic initializer can therefore be followed by type-specific extension data, and the context is not safely constructible from a single universal Rust struct yet.

The vtable `+0x08` target at `0x141C7FC10` and candidate predicate at `0x142039DB0` were also renamed/bookmarked in `/MadMax.exe` as `DestroyStaticOnlyFilterInstance` and `EvaluateStaticOnlyWorldCandidate`. The latter name was accepted with a naming-quality warning; its descriptive bookmark and runtime snapshot are the stronger evidence.


### Camera-collision and player-aim filter predicates

The live `CCCameraCollisionRaycastFilter` candidate predicate at `0x142066D10` first asks the candidate vtable `+0x48` about selectors `1`, `3`, and `0x14`; candidates matching none are rejected. It then reads candidate metadata through virtuals at `+0x10`, scans a type/instance table for IDs `0x1B` and `0x13`, calls a helper on the object, and rejects candidates based on filters in its context. Observed context offsets include `+0x90`, `+0x98`, `+0xA0`, `+0xA8`, and `+0xB0`; exact field types and semantic names remain only partly resolved. Snapshot: `/LiveSnapshots/MadMaxLiveCameraCollisionFilterVirtual_142066D10_1K.bin`.

The live `CPlayerAimRaycastFilter` predicate at `0x142039E90` obtains an entity pointer from a game global and rejects a candidate whose `candidate+0x28` equals that player entity. It accepts candidates whose candidate vtable `+0x48` responds to selector `2`; otherwise it tests the related object through another virtual, then accepts only candidates matching at least one of selectors `1`, `3`, or `0x15`. These are recovered branch conditions; the selector meanings and object classes are not named. Snapshot: `/LiveSnapshots/MadMaxLivePlayerAimFilterVirtual_142039E90_1K.bin`.

This gives us three distinct context/filter behaviors: `CCharacterRaycastFilter` carries an owner pointer at `+0x90`; `CStaticOnlyRaycastFilter` carries a category mask at `+0x90`; and `CCCameraCollisionRaycastFilter` consults multiple caller-configured fields. `CPlayerAimRaycastFilter` instead reads the player identity from a global. These filter classes should have separate Rust context adapters, not one guessed shared struct. The two new candidate predicate aliases are raw ABI declarations only; no Rust call wrapper or ownership/thread-safety guarantee exists.

### Current caller-activity window (2026-10-28 20:53–20:54 CEST)

The live log's five-second caller summary reported only `wind_occlusion` (`0x1FC0E0E`) and `vehicle_sound_occlusion` (`0x21DC95A`) during the final sampled windows, about 275–287 calls per caller per window. No camera/aim caller appeared in those windows. This is a short activity observation only; the game state was not independently established, and it does not show that camera queries never run.

### Camera query separates world output from its object-bounds result

The saved runtime body for `RaycastStaticFromCamera` (`MadMaxLiveCameraRayCallback_141EAC4C5.bin`) passes the return from `ProcessCameraRayQuery` into a two-way mapping that selects one of two opaque script result tokens. It then reports that token through a separate script callback. This is not the return from `WorldQueryHelper`: that call's return is ignored here, and the token values remain opaque.

The camera query helper (`MadMaxLiveCameraQueryHelper_141EAC0C5_full.bin`) has a second, independent result path. It initializes the output record's `+0x14` float to `1.0`, calls `WorldQueryHelper`, then tests that same field against `1.0`. Only when the value is strictly below `1.0` and a follow-up object is available does it invoke the downstream camera-ray handler with the output record, ray data, computed range, and object. This shows a sub-1 output scalar gates camera-specific postprocessing for this caller; it is not by itself a universal hit flag.

The camera ray itself is built from the supplied start position and a target returned by a virtual call at slot `+0xB0`. The helper subtracts start from target, normalizes that direction, and computes a nonnegative range from their distance minus a caller-derived margin. The world query therefore receives the expected origin/direction/range shape, with a camera-specific static-only filter context. The generic request API can represent those geometric inputs, but the adapter must retain the selected filter and caller-specific result interpretation.

This narrows the wrapper contract: keep helper status, output fraction, and any caller-specific postprocessing as separate values. The current camera capture has no confirmed live hit. Static analysis confirms the fraction-to-point formula, but not the output record's entity/normal semantics.

### Camera candidate handler reconstructs a point, then tests object bounds

While Mad Max remained open (PID 10804), a 2 KiB read-only snapshot was captured from `0x141EA8D60` and imported as `/LiveSnapshots/MadMaxCameraRayHitHandler_141EA8D60_2K.bin`. The function at that address is named `TestCameraRayPointAgainstObjectBounds` in that snapshot program; this is an analysis label, not a recovered source symbol.

The camera caller supplies four observed arguments: the world-query output record in `RCX`, the six-float ray in `RDX`, the query range in `XMM2`, and a caller-owned object in `R9`. After checking that object and its virtual `+0x10` predicate, the handler reads `output+0x14` and computes each component of `origin + direction * fraction * range`. It then passes the point through conversion helpers and asks the object's virtual `+0xF8` callback to fill six bound values. If that callback accepts, it calls `TestPointInBounds3D` (`0x1429ADE20`), which returns true only for `minX <= x < maxX`, `minY <= y < maxY`, and `minZ <= z < maxZ`. The object type and the point's exact coordinate space remain unresolved.

This confirms the camera consumer reconstructs a ray-derived candidate point from the fraction and ray, without reading output `+0x00` or vector `+0x08..+0x10` in this handler. The returned Boolean is an object-specific bounds result, not a general world-ray hit result. The caller reaches it only when the fraction is below `1.0` and its separate `R9` object is available; this does not identify a hit entity or prove the vector is a normal.

For a Rust-facing query result, `position` can be derived from origin, normalized direction, range, and the caller-interpreted fraction. `entity` and `normal` still require their own proven mappings. The function is not a general raycast entry point and no direct-call binding is added.

The object-specific `TestPointInBounds3D` analysis mapping is recorded at `CAMERA_POINT_IN_BOUNDS_TEST_RVA`; it is not part of the generic raycast API.

### WorldQueryHelper Boolean is a query-context state flag

The complete `WorldQueryHelper` runtime snapshot ends with an explicit `AL` assignment before its stack-cookie check. When `queryContext+0x60` is non-null, the helper copies result-state fields to the external output record and sets `AL=1`; when that pointer is null, it writes the no-state fallback (including fraction `1.0`) and sets `AL=0`. The called cookie routine at `0x143039610` compares its `RCX` argument with the global cookie and returns without changing `RAX`, so this status byte survives the epilogue. The runtime bytes are in `/LiveSnapshots/MadMaxLiveWorldQueryHelper_14252EDF5.bin` and `/LiveSnapshots/MadMaxSecurityCookieCheck_143039610_256.bin`.

Therefore the Boolean is exactly the post-call non-null test for the context field at `+0x60`. Its pointee type and the conditions under which each query family populates it are still unknown. A successful `HeightOverGround` sample had both this status and a sub-1 fraction, but that correlation does not establish that the pointer means a physical collision for every caller. The camera caller ignores the Boolean and gates its own postprocessing from the output fraction.

Keep this as a low-level `has_query_state` status in any future bridge. Derive an optional geometric hit only under a caller/filter contract whose fraction and hit semantics have been verified; do not expose this Boolean directly as `hit`.

### Camera detail capture budget raised for deliberate gameplay samples

The first camera-call detail budget (16) was consumed before a deliberate aiming sample could be collected. A first expansion raised this to 160 but kept all callers in one shared sample array. In the next process, caller-activity summaries showed `GraphScriptRaycastStatic` calls after startup, but no detail records appeared; the 16 camera detail records in the log were from the earlier process at Unix time `1793207770`. The shared global sample index could therefore starve the camera records when other callers consumed the pool first.

The corrected capture build keeps the same 576 fixed sample records and reserves indices `0..159` for the 160 camera-script records; non-camera callers use `160..575` (416 slots: 404 expected and 12 reserve). Other caller quotas are unchanged. The installed ASI SHA-256 is `4E1103DE0C8E445B63744A35C8C9C96E638A36BC29CD07022CE4E3C9BEC88DFB`; the prior shared-pool build is backed up as `scripts/MadMaxModding.asi.before-camera-capture-reserved-buffer-20261028`. This is capture-only instrumentation.

### `RaycastStaticFromCamera` is a script callback, not the live view ray

In the latest run, the user aimed at nearby geometry and then at open sky; the overlay's `Camera ray tap` remained `installed / 0 calls`. The new process log contains no `GraphScriptRaycastStatic` details, and its recent caller-activity windows do not include that caller. This distinguishes the script callback from an always-running player view ray: the test did not invoke this callback, so the reserved camera sample region has not yet been exercised.

The runtime snapshot `MadMaxLiveCameraRayCallback_141EAC4C5.bin` shows the registered callback reading three floats from the camera-manager singleton at offsets `+0x2A0`, `+0x2A4`, and `+0x2A8`, passing them to the script-ray helper at `0x141EAC0C0` with three fixed opaque constants, then reporting one of two opaque tokens through a separate callback. The helper's world-query caller is `GraphScriptRaycastStatic`. The name and implementation are consistent with a script graph node that performs a camera-position query when requested; the zero tap count during ordinary aim confirms that treating this entry as the player-aim hook was a wrong lead.

The separate RTTI-confirmed `CPlayerAimRaycastFilter` has a live candidate predicate at `0x142039E90`; it rejects the player entity and applies candidate-type checks. It is the better next lead for actual player/weapon aiming, but no sampled world-query context has yet shown its vtable (`0x1412383C8`). Next tracing should identify the owner/caller that constructs this filter and test specifically in weapon-aim mode. No change to ray-query behavior or direct-call API is justified yet.

### Weapon-aim confirmation does not exercise the camera script callback

The user confirmed the latest surface/sky test was performed while holding the weapon aim button. The `Camera ray tap` still reported `installed / 0 calls`, and the latest process log still contains no `GraphScriptRaycastStatic` details. This removes the earlier ambiguity that the test might only have moved the camera: weapon aim itself does not invoke this camera script callback in the tested state. The reserved 160-slot camera capture partition therefore remains unexercised; increasing it further would not address the missing aim query.

The RTTI-confirmed `CPlayerAimRaycastFilter` vtable at `0x1412383C8` places `TestPlayerAimRaycastCandidate` at slot `+0x10` (`0x1412383D8` -> `0x142039E90`). Its live body rejects the player entity and applies candidate-type predicates. This is a narrower next diagnostic target than the camera callback: a count-only entry tap can tell us whether weapon aim reaches this filter at all, without assuming it is the ray submission function. If it stays at zero, the aim system uses another path; if it increments, the caller and candidate records become the next trace target. The predicate name is descriptive/inferred, not an original SDK symbol.

### Current-process weapon aim did not call `CWeapon::GetRayWorldPosition`

The current ASI process installed its passive `CWeapon::GetRayWorldPosition` observer at entry `0x141E1BBF0` (RVA `0x01E1BBF0`) at Unix time `1793219381`. The only eight `Weapon ray sample` records in the accumulated log are timestamped `1793207788` through `1793208515`, before this process. No sample was emitted in the current process while the user held weapon aim and looked at geometry/sky. The observer records calls 1–8 and then every 30th call, so this is evidence that the method was not reached in the current held-aim interval; it does not show the weapon aim ray itself is absent. A single aimed shot is the next discriminating check for whether this method is a firing path.

### `CWeapon::GetRayWorldPosition` is a collision-aware endpoint query

The runtime snapshot `MadMaxLiveRayFunc_141E1BBF0_Restored.bin` decompiles `CWeapon::GetRayWorldPosition` at `0x141E1BBF0` as `(weapon, outPosition[3], ray6[6]) -> outPosition`. It reads the query range from `*(weapon + 0x3E0) + 0x270`, first writes `origin + range * direction` to the output, initializes a world-query context using weapon-owned state, and calls `WorldQueryHelper` at `0x14252EDF0` with the six-float ray. If the returned fraction in the local result record is below the no-hit sentinel, it overwrites the output with `origin + direction * (range * fraction)`. This establishes a collision-aware endpoint calculation in this weapon method. The weapon context/filter fields and whether this method is called during aim preview remain unproven.

The hook's eight logged outputs from the older process match the no-hit endpoint formula at range 10,000 (within float rounding). The current process had no new weapon-method samples during held aim. One shot while aiming will test whether the observed method belongs to firing/impact processing or participates in aim preview. The call is not yet exposed as a Rust SDK function because its hidden weapon-owned query context and thread expectations have not been established as safe for arbitrary mod calls.

### Shot test: pellet collision queries are separate from aim preview

Clarification from the later read of the same process log: one `GraphScriptRaycastStatic`-labeled world-query sample occurred at Unix time `1793219437`, well before the aimed-shot sample at `1793220037`. Thus the camera script path did run once earlier in this process, but it was not observed during the user's held-aim/shot interval; the overlay camera callback tap still reported zero calls during that test. This distinguishes the sampled helper caller from the separate callback-entry counter.

The user's single aimed shotgun discharge produced ten captured `CAmmunition::CheckCollisions` queries at Unix time `1793220037`. All ten originate from approximately `(-5500.818, 472.592, -2886.887)`, use 15-unit segments, and have distinct spread directions. Eight return true with scalar distances from 5.043 to 8.807 units; two return false and preserve the full 15-unit endpoint. This is the shotgun projectile/pellet collision path, not a single aim-preview ray. The current process still has zero `CWeapon::GetRayWorldPosition` samples. The shot therefore proves that the ammo path reaches `WorldQueryHelper` and returns collision distances, but it does not identify the live aim-preview query.

### Shotgun pellet ground/sky control pair confirms caller-specific query semantics

A deliberate two-shot comparison in the same running process produced a useful positive/negative pair for `CAmmunition::CheckCollisions` (caller return RVA `0x01DA5892`). The ground shot at Unix time `1793220833` generated ten distinct 15-unit pellet rays from `(-6869.031, 507.769, 66.580)`. All ten `WorldQueryHelper` calls returned `1`; each ammo result reported a scalar between `0.151887` and `0.294973`. The caller multiplies this scalar by the 15-unit segment length, giving collision distances of approximately 2.278–4.425 units. Their directions had negative Y components, consistent with the surface-directed shot.

The open-sky shot at Unix time `1793221394` generated ten 15-unit rays from `(-8240.424, 491.603, -114.508)`. All ten helper calls returned `0`; each ammo routine retained its full 15-unit endpoint. Their directions had positive Y components from approximately `0.883` to `0.950`, consistent with the upward shot. Thus the two result branches were reproduced in-game with a clear geometric control: for this caller, helper status `1` plus a sub-1 output scalar corresponds to a pellet collision, while status `0` follows the full-segment miss path. This is now a confirmed caller-specific interpretation, not a universal `WorldQueryHelper` return contract.

This makes the pellet path a valuable reference for query preparation and result consumption, but it is not a placement API because `CAmmunition::CheckCollisions` mutates projectile state and dispatches projectile callbacks. The call at `0x1431CCB30` is a runtime dispatch trampoline into the vector-assignment routine used to prepare `CIgnoreFilter`’s per-query ignored-object list; it is not itself a raycast API, and neither internal routine is proposed as a Rust call target.
### Ammunition ignore-vector mapping

The `CAmmunition::CheckCollisions` query context uses the RTTI-confirmed `CIgnoreFilter` vtable. Its caller creates a three-pointer MSVC vector header at context offset `+0x90` and copies the vector stored at `ammo+0x20` into it before the world query. The 64-bit header fields are `begin`, `end`, and `capacity_end`; the query filter's candidate predicate walks the `[begin, end)` range in pointer-sized steps and compares each entry with the candidate object's pointer at `candidate+0x28`. The ammunition-owned vector is therefore a per-query ignored-object list.

The call at `0x1431CCB30` is a runtime dispatch trampoline, not a ray-query API. In this live image it computes the target `0x141DA39D0` from a 15-byte runtime code sequence and a relocated global, then transfers into a 323-byte vector-assignment routine. That routine copies qword elements into the caller-provided three-pointer vector. Its caller then invokes the shared world-query helper; after the query, it destroys the temporary vector when its begin pointer is non-null. Ghidra snapshot `/LiveSnapshots/MadMaxAmmoRedirectTarget_141DA39D0_live.bin` names the observed body `AssignQwordVectorContents`.

The Rust declaration now represents only the confirmed layout: an opaque 0x90-byte prefix followed by a raw three-pointer vector header. It intentionally exposes no constructor, ownership operation, or callable wrapper. The target selected by the trampoline is runtime-dependent, so neither address is a stable Rust entry point.
### Live shared script-ray helper produces candidate fractions (2026-10-28 21:58–22:00 CEST)

A focused reread of the current run's shared GraphScriptRaycastStatic helper records (helper-call return RVA 0x01EAC351) found 99 detailed samples, correcting the earlier 16-sample observation that had contained only no-result cases. Eleven samples returned helper status 1; each had a candidate minimum between 0.930935 and 0.937657, and the output record's scalar_14 matched that minimum within 5e-7. The other 88 samples returned 0, had candidate minimum 1.0, and retained scalar_14 = 1.0.

This confirms that WorldQueryOutputRecord::scalar_14 is used as a segment fraction in both the sampled shared script-ray helper and CAmmunition::CheckCollisions. The helper's callers check the value against the 1.0 fallback before entering caller-specific follow-up logic; the observed sub-1 values therefore reach those paths. This is a real in-game script ray query with candidate traversal and a fraction-bearing output record.

Do not assign these 99 records specifically to RaycastStaticFromCamera: both registered script entrypoints share this helper, and the camera-specific callback tap remained at zero. The sample set also does not prove that ordinary weapon aim invokes the helper or that its input is the visible reticle ray. The CPlayerAimRaycastFilter count remained zero in the latest observation windows, and the weapon ray endpoint observer has no corresponding current-run samples. No general-purpose raycast call wrapper is justified yet. The older 16-sample all-no-result observation remains accurate for that earlier capture; this larger run adds fraction-bearing candidate cases rather than invalidating it.

Related code: WorldQueryOutputRecord::scalar_14 in src/api/game/abi/game/world_query.rs. The sample evidence remains in the user's local bootstrap.log; no game memory was written.
### Script ray helper builds a player-aim-filtered segment from origin to target (runtime snapshot, 2026-10-28)

A 256-byte read-only snapshot of the live `RaycastStatic` callback at `0x141EAC3F0` was imported into Ghidra as `MadMaxLiveRaycastStatic_141EAC3F0_20261028.bin` and rebased to its runtime address. The callback resolves a script-selected object through `0x141E6AB20`, calls its virtual slot `+0xB0`, and extracts a position vector. That vector is passed as the **ray origin** to `GraphScriptRaycastStatic` at `0x141EAC0C0`; it is not the ray target.

The registered `RaycastStaticFromCamera` callback passes the camera-manager position (`+0x2A0/+0x2A4/+0x2A8`) as that same origin. In both cases, the shared helper separately resolves a script-selected target object, calls its virtual `+0xB0` to obtain the target position, computes and normalizes `target - origin`, then submits the resulting finite segment. This corrects the earlier wording that described the generic callback's vector as a target.

The helper initializes a query context with `0x1424E5AA0`, installs the `CStaticOnlyRaycastFilter` vftable (`0x14123D6A8`), writes category mask `3` at context `+0x90`, and calls the observed filter option setter `0x14202A360` with `true`. The vtable's `+0x10` candidate predicate is `0x142039DB0`; it reads the category mask at `context+0x90`. The separate `CPlayerAimRaycastFilter` vftable is `0x1412383C8`, whose `+0x10` predicate is `0x142039E90` and does not read a category-mask field. The helper then calls `WorldQueryHelper` using the world-query base global. Its maximum segment distance is derived from the origin-to-target distance and another context-derived scalar; it subsequently uses the query fraction and a candidate-bounds handler to choose a script result token. This is a static-only script ray path, not a player-aim-filter path.

This is a meaningful building block for a reticle-equivalent API: the game already has code that constructs a static-only filtered finite segment and returns collision-dependent results. It is **not yet a general callable raycast**. The helper depends on live script execution context and script-key object resolution; an arbitrary endpoint vector cannot currently be supplied through this callback. A mod-side ray can likely reuse `WorldQueryHelper` with a constructed compatible filter, but ownership, thread phase, full context initialization, and direct-call behavior still need to be established before exposing it as a Rust API. The captured callback bytes are a local snapshot of the running process; no game memory was written.
### `GetCameraMatrix` supplies the geometric reticle-ray origin and direction

The community SDK declares `CCameraControlManager::GetCameraMatrix(CMatrix4f&)` at runtime address `0x14222DA60` (`game/cameramanager.h`, revision `f2dc640769c88aae9c3965a2aca0aa138f0e62ab`). Its `CMatrix4f` helpers define `Right()` as row 0, `Forward()` as row 1, `Up()` as row 2, and `Position()` as row 3 (`math3d.h` at the same revision). The Rust bridge already binds this getter and has matching `Matrix4x4::{right, forward, up, translation}` accessors.

A read-only live snapshot of the getter shows it selects the matrix at `CCameraControlManager + 0x270` unless the state getter at `manager + 0x480` returns `3`; in the sampled frame that state field (`manager + 0x880`) was `-1`, so the `+0x270` matrix was selected. The camera manager's position fields at `+0x2A0/+0x2A4/+0x2A8` are the translation row of this matrix. The three 3D basis rows in that live matrix were mutually orthogonal and changed after the user turned the camera, confirming that this is dynamic camera-frame data rather than an unrelated static transform.

This gives a concrete geometric reticle-ray construction: use the camera matrix translation as origin and compare the signed `Forward()` axis against the visible center-screen aim. The user observed the unsigned row pointing upward while aiming at the ground; after the prototype negated it, the displayed direction pointed down and forward, but the query still returned the no-hit sentinel. The matrix getter itself is already used by the probe; the signed axis now appears plausible, while arbitrary-ray query-context compatibility remains unvalidated.

### Reticle prototype no-hit cause: mismatched static-only filter context

The initial reticle prototype used vtable `0x14123D6A8`, but labeled it `CPlayerAimRaycastFilter`, and wrote its category mask at `context+0xC0`. Ghidra's vtable bytes at `0x14123D6A8` instead contain `0x142039DB0` at slot `+0x10`; that function is the static-only candidate predicate and reads the category mask from `context+0x90`. The actual `CPlayerAimRaycastFilter` vtable is `0x1412383C8`, whose `+0x10` entry is `0x142039E90`; that predicate does not read a category-mask field.

The captured `GraphScriptRaycastStatic` helper independently installs `0x14123D6A8` and writes `3` at `queryContext+0x90`. The first reticle prototype therefore paired the static-only vtable with a zeroed `+0x90` mask, while putting `3` at an unrelated `+0xC0` offset. Since the static-only predicate checks `(mask & candidateFlags) != 0`, a zero mask rejects every candidate before shape intersection. This explains the no-hit result despite the downward-looking ray and broad-phase candidate callbacks. The corrected adapter now pairs the static-only vtable with mask `3` at `+0x90`; the runtime result still needs a controlled ground/sky check.


### Weapon-aim hold does not invoke the player-aim filter (2026-10-29)

During the user's weapon-aim hold check, the passive `CPlayerAimRaycastFilter` predicate counter remained at zero in every reported five-second window. The world-query caller tap was active, so this is evidence that ordinary weapon aim did not invoke that predicate in this capture; it does not rule out other game modes or code paths.

The same log contains high query activity from already identified, separate callers: `NFootPlacement::Raycast` at RVA `0x01D035A2`, `wind_occlusion` at `0x01FC0E0E`, and `vehicle_sound_occlusion` at `0x021DC95A`. These are not evidence that the reticle query ran. In particular, the foot-placement ray is a roughly one-unit downward segment and is not a candidate reticle ray.

The next experiment compares two mod-issued queries on the same camera ray and frame: the existing `CStaticOnlyRaycastFilter` configuration against the RTTI-confirmed `CPlayerAimRaycastFilter` vtable (`0x1412383C8`). The latter's predicate explicitly rejects the current player character and has no static-only category-mask read. The comparison should establish whether changing only the filter moves the near-player hit; it does not imply that native weapon aiming calls this filter.
### Mod-issued player-aim filter query traverses candidates (2026-10-29)

After installing the paired-filter build, the passive `TestPlayerAimRaycastCandidate` counter advanced steadily at roughly 119–126 calls per five-second window. In the earlier build it remained zero while the user held native weapon aim. This confirms the new mod-issued player-aim-filtered query reaches the candidate predicate; the prior zero was specific to ordinary weapon aiming, not a dead predicate hook.

The user observed that aiming the camera down at Max's head makes the player-aim-filtered hit position separate from the static-only result by roughly two or three world units, with the separation growing as the view moves onto his head. This supports the hypothesis that the static-only query accepts a nearer player candidate while the player-aim predicate rejects Max and continues to a farther surface. The query records currently expose only the fraction-derived point, however, so the exact accepted object in the static-only result is not yet identified.

Next confirmation should correlate the filter decision with candidate identity (the player-aim predicate compares `candidate+0x28` with `CCharacterManager::GetPlayerCharacter`) or perform a controlled aim sweep across Max and adjacent ground while recording exact fraction/position pairs. Do not describe the static-only result as a confirmed Max hit until that identity check is captured.
### User-observed A/B behavior with Max in the ray (2026-10-29)

With the comparison build running, the user reports that the static-only hit point looks like the ray intersects Max, while the player-aim-filtered point separates by about 2–3 world units as the camera points more directly at his head. The observed filter-dependent displacement is consistent with the player-aim predicate rejecting Max and allowing the query to continue to a farther candidate.

This is strong behavioral support for the self-ignore hypothesis, but remains user-visual interpretation: the overlay reconstructs positions from the returned fraction and does not yet show candidate object identity. A same-session control aimed at a nearby surface with Max outside the ray should show whether both filters converge when the player is not a candidate.

### RTTI raycast filter family and live candidate predicates (2026-10-29)

An RTTI inventory of the executable identified the CRaycastFilter family and its vftables. Runtime code snapshots were read from the still-running game and imported into Ghidra; no process memory was written.

The shared CRaycastFilter vftable is 0x141324960. Every RTTI-identified derived filter inspected shares slot +0x00 at 0x1424F6380. Its live body checks candidate flags against filter-context masks at +0x70 and +0x74, forwards a surviving candidate to a closest-hit collector helper, and stores the accepted leaf candidate pointer at context offset +0x80 when the result becomes closer. The full record layout and helper semantics remain partial. Existing captured GraphScriptRaycastStatic query contexts contain this +0x80 pointer; one points to an object whose vftable is CCustomSampledHeightFieldShape at 0x141325C18, demonstrating that the field can identify a terrain shape candidate.

The derived filter vftables and slot +0x10 candidate predicates are:

| RTTI class | Vftable | Predicate | Observed behavior |
|---|---:|---:|---|
| CCharacterRaycastFilter | 0x141238388 | 0x14202A3B0 | Checks candidate selectors 1 and 3, then rejects the context owner at +0x90. |
| CCCameraCollisionRaycastFilter | 0x1412383A8 | 0x142066D10 | Candidate predicate is mapped; selector behavior is not yet documented here. |
| CPlayerAimRaycastFilter | 0x1412383C8 | 0x142039E90 | Rejects the current player character, then applies candidate classifications including selectors 1, 2, 3, and 0x15. |
| CIgnoreCharactersRaycastFilter | 0x14123D688 | 0x14202A420 | Checks selectors 1 and 3; when candidate+0x28 has an owner, rejects it if the owner passes the CCharacter interface check. Selector meanings are unresolved. |
| CStaticOnlyRaycastFilter | 0x14123D6A8 | 0x142039DB0 | Reads its category mask from context+0x90. |
| CCharacterStaticOnlyRaycastFilter | 0x1412A73A0 | 0x14202A510 | Requires selector 1, rejects candidate owner equality with context+0x90, then applies helper 0x1425232A0. |
| CIgnoreCharactersAndVehiclesRaycastFilter | 0x1412D38C0 | 0x14202A4C0 | Rejects candidates when selector 2 or 4 is set; accepts otherwise. |
| CVideoModeCameraCollisionRaycastFilter | 0x1412E6430 | 0x14229D2C0 | Checks selectors 1, 3, 0x14, and 2 before consulting a context-owned entry table; the remaining branch is unresolved. |
| CFootPlacementRaycastFilter | 0x1413207C8 | 0x142493670 | Checks selectors 1 and 3 and rejects owner equality with context+0x90. |

For these derived filters, slot +0x08 is the deleting destructor, not addRayHit. Live bodies at 0x141C7FBC0, 0x1421ADC20, 0x142039BE0, and 0x14227A110 reset the vtable and free a 0x70-byte object when the deleting flag is set. The former addRayHit labels at the first three addresses were incorrect. Rust constants now use the verified destructor and predicate roles.

One paired mod-issued reticle capture (samples #5 and #6) stored the same accepted shape pointer, 0x8129250, for both static-only and player-aim filters. A read-only inspection of that object found vtable 0x141325C18, the RTTI-confirmed CCustomSampledHeightFieldShape vftable. For that captured ray, both filters therefore accepted terrain geometry rather than Max. This is specific to that captured sample and does not explain every user-observed divergent result.

The release candidate now also issues a third query using CIgnoreCharactersAndVehiclesRaycastFilter on the same reticle ray and records each query's accepted shape address and vtable. The known sampled-heightfield vtable is rendered as terrain in the debug panel; unknown vtables remain visible as addresses. The candidate was installed as `MadMaxModding.asi` on 2026-10-29 after preserving the previous file as `MadMaxModding.asi.before-reticle-shape-comparison-20261029`. It has not yet been gameplay-validated. Its third filter is expected to reject candidates classified by selectors 2 or 4; this does not guarantee a terrain hit or establish those selector names.

### Follow-up: generic shape-cast slot and ray-segment contract (2026-10-29)

A Ghidra comparison of the generic `CheckRayShapeIntersection` body (`0x142B13810`) with the RTTI-confirmed `CCustomSampledHeightFieldShape` vtable resolves two entries that had previously been conflated:

- The generic shape path calls virtual slot `+0x28`, whose live target is `0x1424F6800` (RVA `0x024F6800`). This wrapper reaches `ProcessHeightFieldRayCast` at `0x1424EE900`.
- Slot `+0x30` targets a distinct wrapper at `0x1424F6880` (RVA `0x024F6880`). It also reaches the heightfield helper, but uses different argument setup. Its caller-specific ABI remains unresolved, so it must not be treated as the generic `+0x28` cast entry.

The captured `WorldQueryHelper` implementation confirms the ray input layout used by the existing reticle prototype: six floats for origin XYZ followed by direction XYZ, plus start and end distances. It forms both endpoints as `origin + direction * distance`, subtracts the query origin at `queryBase + 0x40`, then dispatches the segment. The Rust reticle path already passes the six-float ray and the `0..maxDistance` interval, so no extra caller-side rebasing is indicated by this finding. The candidate position remains reconstructed as `origin + direction * (maxDistance * fraction)`; this does not validate filter semantics or prove the current candidate hit is the reticle-visible surface.

A separate camera obstruction helper (`0x141EAC0C5`) builds a segment from a supplied point toward the camera and ends short of the camera by a configured offset. It uses `WorldQueryHelper`, but this is not evidence of an existing forward reticle query.

Ghidra bookmarks record these distinctions in `MadMax.exe`, `MadMaxLiveRayGraph.bin`, and the camera-query snapshots. The Rust constants now identify `+0x28` as the generic shape-cast target and retain `+0x30` under a separate secondary-wrapper name. Runtime behavior was not changed or tested in this follow-up.

### Follow-up: query-context closest fraction versus exported fraction (2026-10-29)

The captured `InitializeWorldQueryContext` at `0x1424E5AA0` initializes both `context+0x08` and `context+0x20` to `1.0`. The shared `CRaycastFilter::addRayHit` body at `0x1424F6380` reads `context+0x08`, calls hit-update helper `0x142B14540`, and only updates the accepted-shape pointer at `context+0x80` when the value at `+0x08` strictly decreases. This identifies `+0x08` as the closest-hit bound/current candidate fraction used by the shared collector path.

`WorldQueryHelper` independently copies `context+0x20` to `WorldQueryOutputRecord::scalar_14` at `outRecord+0x14`. The recovered hit callback does not visibly write `+0x20`; it calls `0x142B14540`, now identified as `hkpClosestRayHitCollector::addRayHit` by its RTTI vtable slot and its use from the shared filter callback. The method body is not recovered from the on-disk Ghidra program, whose bytes at that address do not decompile reliably. Although captured caller results and user-facing ray results can appear consistent with closest-hit fractions, the recovered code does not yet prove that `+0x20` always aliases or copies the collector bound at `+0x08`. Rust now names these offsets separately and preserves that uncertainty. Ghidra bookmarks were saved in the context-initializer, filter, and world-query programs. No runtime behavior was changed or tested.

### Follow-up: ray-hit callback labels were vtable-slot misreads (2026-10-29)

A direct read of the RTTI-backed vftables corrected four earlier function labels. `CStaticOnlyRayHitCollector` (`0x14125A168`), `CStaticAndKeyFramedOnlyRayHitCollector` (`0x1412750F8`), and `CDefaultClosestMaterialRayHitCollector` (`0x1412875B0`) each have the shared `CRaycastFilter::addRayHit` target `0x1424F6380` at slot `+0x00`. Their slot `+0x08` targets are respectively `0x141D83480`, `0x141EA7F70`, and `0x141F61B00`; captured bodies show deleting-destructor behavior, including resetting the vtable to the RTTI-confirmed `hkpRayHitCollector` table and conditionally freeing a 0x70-byte object. These three targets are not class-specific hit callbacks.

The `hkpClosestRayHitCollector` table (`0x141320790`) points from slot `+0x00` to `0x142B14540` and from slot `+0x08` to `0x14249F5A0`. The latter's captured body is a deleting destructor; the former is identified as `hkpClosestRayHitCollector::addRayHit` because the shared filter callback invokes it with its context, candidate record, and hit data. Its body remains unresolved, so we cannot yet say which fields it writes or whether it synchronizes the closest bound at `+0x08` with the exported fraction at `+0x20`. Rust constants now distinguish the callable method entry from the destructor. Ghidra bookmarks were added and saved. No runtime behavior was changed or tested.
