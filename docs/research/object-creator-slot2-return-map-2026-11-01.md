# CGameObjectCreator third-slot return map

Date: 2026-11-01  
Static program: `/MadMax/MadMax.exe`, image base `0x140000000`  
Decoded bytes: saved historical Ghidra program `/runtime_snapshots/MadMax_xpdata_runtime_slice_141C00000.bin`

## Scope and confidence

A Ghidra scan found all 286 `CGameObjectCreator<T>` TypeDescriptors, their associated COL-bounded vtable address points in `.data1_a`, and each table's third entry. Every target address was readable in the saved runtime slice. All 286 targets decode to exactly `MOV EAX, imm32; RET` (`B8 <4 bytes> C3`); no target was outside the captured slice or had a different byte shape. The returned values range from `0xD0` to `0x45E0`, with 99 distinct values.

For 26 representative creator types, the saved snapshot also contains a decoded creator entry. Each entry computes an argument and passes it to the same helper at `0x141B6D150`; in all 26 checked cases, that argument exactly matches the value returned by the corresponding third slot. The entries then use the helper result in an initialization path and form a shared-pointer-like result. This strongly corroborates that the third slot supplies an instance allocation size and that the helper is size-driven allocation/acquisition. The helper body itself is not present as a decoded function in this snapshot, so its exact contract is not named. For the remaining 260 types, the **constant-return behavior is directly verified**, while the “instance size” meaning is inferred from the shared table pattern and these 26 creator call paths.

This is a static/runtime-snapshot mapping only. The saved capture is not current game memory. No vtable was called; no creator, allocator, world registration, or destruction ABI was exercised. These callback names are analyst labels, not original symbols. The table records exact return values and function addresses, not a claim that it is safe to pass a value to any arbitrary allocator.

The first 19 call-path cross-checks are:

| Type | Creator entry | Slot-2 value | Value passed to `0x141B6D150` | Initializer target | Returned object adjustment |
|---|---:|---:|---:|---:|---:|
| `CRigidObject` | `0x141FB4120` | `0x2A0` | `0x2A0` | `0x142050FF0` | `+0x8` |
| `CScaledRigidObject` | `0x141FB41E0` | `0x2B0` | `0x2B0` | `0x142077B90` | `+0x8` |
| `CAnimatedRigidObject` | `0x141FB42A0` | `0x4A0` | `0x4A0` | `0x1420C20B0` | `+0x8` |
| `CDestructibleObject` | `0x141FB4410` | `0x258` | `0x258` | `0x14206BE80` | `+0x8` |
| `CDestructibleObjectKeyframed` | `0x141FB44D0` | `0x298` | `0x298` | `0x142CE6E70` (outside captured code) | `+0x8` |
| `CItemPropObject` | `0x141FB4D50` | `0x318` | `0x318` | `0x141F6DAC0` (captured bytes do not decode as a meaningful initializer) | `+0x8` |
| `CCharacter` | `0x141FB6910` | `0x32B0` | `0x32B0` | `0x141CE82A0` | `+0x8` |
| `CCar` | `0x141FB7690` | `0x4150` | `0x4150` | `0x1422014E0` | `+0x8` |
| `CHelicopter` | `0x141FB7750` | `0x4090` | `0x4090` | `0x1421FD400` | `+0x8` |
| `CAirPlane` | `0x141FB7810` | `0x45E0` | `0x45E0` | `0x142201110` | `+0x8` |
| `CMotorBike` | `0x141FB78D0` | `0x42A0` | `0x42A0` | `0x1421FDC60` | `+0x8` |
| `CBoat` | `0x141FB7990` | `0x40F0` | `0x40F0` | `0x142202450` | `+0x8` |
| `CSpawnPoint` | `0x141FBA740` | `0x238` | `0x238` | `0x141F40300` | `+0x10` |
| `CSpawnObjectCollection` | `0x141FC11E0` | `0x118` | `0x118` | `0x14233E580` (common constructor path; creator applies vptrs) | `+0x8` |
| `CEncounterSpawn` | `0x141FBA800` | `0x158` | `0x158` | `0x141E4D510` | `+0x8` |
| `CRayCasterObject` | `0x141FBAF10` | `0x130` | `0x130` | `0x14203F730` | `+0x8` |
| `CSkeletalAnimatedObject` | `0x141FB6340` | `0xF30` | `0xF30` | `0x142040210` | `+0x8` |
| `CFlameObject` | `0x141FB63F0` | `0x320` | `0x320` | `0x141E6C630` | `+0x8` |
| `CResourceObject` | `0x141FB6850` | `0xD0` | `0xD0` | `0x14233E580` (common constructor path; creator applies vptr) | `+0x8` |

The helper argument is assembled from per-call immediates (for example, `CRigidObject` adds `0x19006091` to `0xE6FFA20F`, producing `0x2A0`) before the call. The `+0x8`/`+0x10` adjustments are observed pointer arithmetic when forming the returned object/control-block pair; their precise base-interface meaning is not established. Treat those as creator-specific pointer adjustments, not a universal rule. The keyframed and item-prop initializer entries are included only to record the call target: their bodies are not decoded well enough here to describe their initialization behavior.

Seven more creator entries were recovered by following data references from their slot-1 code pointers back through the creator vtable's COL to the exact RTTI TypeDescriptor. Their target names and slot-2 callbacks are:

| Creator type | Slot-1 entry | Slot-2 callback | Value passed to `0x141B6D150` | Initialization path | Returned pointer adjustment |
|---|---:|---:|---:|---|---:|
| `CThunderStickLauncher` | `0x141FB4C90` | `0x141F69FA0` → `0x4A0` | `0x4A0` | `0x141E0B7F0` | `+0x8` |
| `CVehicleInstantRepairRefuelComponent` | `0x141FB7BB0` | `0x141F6E980` → `0x1B8` | `0x1B8` | `0x14221BD40` | `+0` |
| `CAiRoadFlagSetter` | `0x141FB8AC0` | `0x141F70180` → `0x1A0` | `0x1A0` | `0x141BD80B0` (target body unavailable) | `+0` |
| `CStateContainerStateHarpoonShoot` | `0x141FBBE70` | `0x141F753F0` → `0x148` | `0x148` | `0x141C9C0E0` (capture bytes do not decode meaningfully) | `+0` |
| `CMoveToGround` | `0x141FBD8B0` | `0x141F77F10` → `0x100` | `0x100` | `0x141EF4D30` | `+0x10` |
| `CMapIcon` | `0x141FC2F50` | `0x141F70C90` → `0x130` | `0x130` | Inline allocation, common constructor, and vptr initialization in creator | `+0` |
| `CUpgradeSystem` | `0x141FCC800` | `0x141F76C70` → `0x188` | `0x188` | `0x141FCB1F0` | `+0` |

For these seven, the slot-1 RTTI match and the arithmetic are separate checks: the slot-1 pointer is referenced by the RTTI-bounded creator table, and the immediate addition at the call site reproduces the corresponding slot-2 return. The pointer adjustment varies: these examples include direct base pointers, `+0x8`, and `+0x10`. This reinforces that the helper result is a raw allocation base and the creator's returned object pointer may be a type-specific subobject; it is not a universal `base + 8` rule. Two initializer targets remain unreadable in this snapshot, and no target was invoked.

## Results

| Creator type parameter | Slot-2 target | Observed return |
|---|---:|---:|
| `CAIControlObject` | `0x141F6FD70` | `0x1C8` |
| `CAiEntityBlackboardSetter` | `0x141F70720` | `0x148` |
| `CAIFactionInfluenceObject` | `0x141F6FDB0` | `0x1B0` |
| `CAiInfluenceMapModifierObject` | `0x141F708D0` | `0x178` |
| `CAiNoDriveZoneObject` | `0x141F70610` | `0x208` |
| `CAiRoadFlagSetter` | `0x141F70180` | `0x1A0` |
| `CAirPlane` | `0x141F6E480` | `0x45E0` |
| `CAiSensorEventEmitter` | `0x141F70AD0` | `0x130` |
| `CAiSetTreeArgumentObject` | `0x141F70800` | `0x158` |
| `CAiUserEdge` | `0x141F709E0` | `0x208` |
| `CAnimatedRigidObject` | `0x141F69400` | `0x4A0` |
| `CAnimatedTexture` | `0x141F6B7E0` | `0x138` |
| `CAnimRotateTowards` | `0x141F782E0` | `0x168` |
| `CAnimRotation` | `0x1422E2690` | `0x268` |
| `CAnimSpline` | `0x1422E25B0` | `0x478` |
| `CAnimTranslation` | `0x1422E2790` | `0x388` |
| `CAOVolumeObject` | `0x1422E2E80` | `0x1D0` |
| `CAreaDamage` | `0x141F6F490` | `0x1B0` |
| `CAreaOfOperationsObject` | `0x141F70520` | `0x1A8` |
| `CArenaSpawner` | `0x141F739B0` | `0x220` |
| `CArenaSpawnerWave` | `0x141F73AA0` | `0x130` |
| `CArmourWeakspot` | `0x141F768B0` | `0x230` |
| `CAssignNewSystemGroup` | `0x141F68610` | `0xF8` |
| `CAttachableGameObject` | `0x141F6F310` | `0x150` |
| `CBalloonControllerObject` | `0x141F694F0` | `0x1C0` |
| `CBoat` | `0x141F6E690` | `0x40F0` |
| `CBoneAttachedModel` | `0x141F6CD50` | `0xE8` |
| `CBoneAttachment` | `0x141F6CC60` | `0x168` |
| `CBoneOffsetAlias` | `0x141F6CAD0` | `0x1A8` |
| `CBoneTransformEventSender` | `0x141F6CBA0` | `0x108` |
| `CButtonTrigger` | `0x141F68EC0` | `0x108` |
| `CCameraObject` | `0x1422E33E0` | `0x600` |
| `CCar` | `0x141F6E260` | `0x4150` |
| `CCarCombatDirector` | `0x141F70260` | `0x6A0` |
| `CCarCombatMapObject` | `0x141F70340` | `0x190` |
| `CChallengesSystem` | `0x141F76BA0` | `0xA50` |
| `CCharacter` | `0x141F6C9F0` | `0x32B0` |
| `CCharacterDialoguePrompt` | `0x141F6B9B0` | `0x1C8` |
| `CCharacterUpgrade` | `0x141F6D030` | `0x4F8` |
| `CCloneFactory` | `0x141F6B150` | `0x1F8` |
| `CConditionalDialogueBlocker` | `0x141F6B500` | `0x110` |
| `CConstraint` | `0x141F68490` | `0x210` |
| `CContactListenerObject` | `0x141F73D60` | `0x108` |
| `CConvoyDataContainer` | `0x141F6F990` | `0x168` |
| `CCountTrigger` | `0x141F68CA0` | `0x168` |
| `CCreationTrigger` | `0x141F6C110` | `0xD8` |
| `CCreationTriggerTimed` | `0x141F6C4A0` | `0x108` |
| `CDamageableObject` | `0x141F6B8D0` | `0x2B8` |
| `CDamageController` | `0x141F68390` | `0x348` |
| `CDebugTextObject` | `0x141F67E00` | `0x200` |
| `CDestructibleObject` | `0x141F695E0` | `0x258` |
| `CDestructibleObjectKeyframed` | `0x141F696D0` | `0x298` |
| `CDestructibleSoundModulator` | `0x141F69790` | `0x118` |
| `CDestructionDamageController` | `0x141F69870` | `0x2C0` |
| `CDestructionDamageControllerGroup` | `0x141F69950` | `0x188` |
| `CDialogue` | `0x141F6B420` | `0x1E0` |
| `CDistanceTrigger` | `0x141F68FD0` | `0x1A0` |
| `CDoActOnEventObject` | `0x1422E0C80` | `0xF8` |
| `CDoorWeakspot` | `0x141F76720` | `0x258` |
| `CDSPParameterModulator` | `0x1422E1200` | `0x200` |
| `CDynamicLightFlasher` | `0x1422E30A0` | `0x2E0` |
| `CDynamicLightObject` | `0x1422E2DA0` | `0x2D0` |
| `CEconomyResource` | `0x141F70DC0` | `0xD8` |
| `CEffectPointEmitter` | `0x1422E2AA0` | `0x260` |
| `CEffectSettingsDependency` | `0x1422E2BA0` | `0x138` |
| `CEffectSpawner` | `0x1422E28C0` | `0x330` |
| `CEffectSpawnerInstance` | `0x1422E29A0` | `0x140` |
| `CEncampmentInventory` | `0x141F77710` | `0x1E8` |
| `CEncounterSpawn` | `0x141F734F0` | `0x158` |
| `CEnvironmentPresetCutscene` | `0x141F72850` | `0x270` |
| `CEnvironmentPresetDayCycle` | `0x141F72750` | `0x160` |
| `CEnvironmentPresetGlobal` | `0x141F72520` | `0x120` |
| `CEnvironmentPresetLocal` | `0x141F72950` | `0x268` |
| `CEnvironmentPresetLocalCollider` | `0x141F72A20` | `0xE8` |
| `CEnvironmentPresetTimeOfDayManager` | `0x141F72670` | `0x5A0` |
| `CEnvironmentPresetTrigger` | `0x141F72450` | `0x168` |
| `CEventAxis` | `0x141F6A920` | `0x1D0` |
| `CEventRouter` | `0x141F67EC0` | `0x118` |
| `CEventStealthRouter` | `0x141F67FB0` | `0x118` |
| `CEventTrigger` | `0x141F68BA0` | `0x158` |
| `CExplosives` | `0x141F6AC70` | `0x1E0` |
| `CExtrudeSplineObject` | `0x1422E24B0` | `0x220` |
| `CFadeComponent` | `0x141F69C50` | `0x140` |
| `CFadeScreen` | `0x1422E31A0` | `0x108` |
| `CFlameObject` | `0x141F6BE50` | `0x320` |
| `CFlamePulse` | `0x141F6BBA0` | `0x180` |
| `CForceArchetype` | `0x141F6EDF0` | `0xF0` |
| `CForceField` | `0x141F6BC80` | `0x1B8` |
| `CForcePulse` | `0x141F6BAD0` | `0x208` |
| `CFOVObject` | `0x141F6E100` | `0x160` |
| `CFueltankWeakspot` | `0x141F769B0` | `0x230` |
| `CGameObjectList` | `0x141F71E90` | `0xE0` |
| `CGameObjectOrderedList` | `0x141F71F70` | `0x100` |
| `CGarageConnection` | `0x141F6EC50` | `0x160` |
| `CGarageState` | `0x141F6ED30` | `0x2D60` |
| `CGraphScriptGameObject` | `0x141F69D20` | `0x1D8` |
| `CGuiMessage` | `0x141F71430` | `0x120` |
| `CHandGrenade` | `0x141F6AD70` | `0x220` |
| `CHarpoonChassisTarget` | `0x141F75BD0` | `0x328` |
| `CHarpoonHumanTarget` | `0x141F75DD0` | `0x338` |
| `CHarpoonTarget` | `0x141F75A30` | `0x320` |
| `CHarpoonVehicleTarget` | `0x141F75AF0` | `0x328` |
| `CHarpoonWheelTarget` | `0x141F75CC0` | `0x328` |
| `CHealthbar` | `0x141F77610` | `0x238` |
| `CHealthControl` | `0x141F68180` | `0x138` |
| `CHelicopter` | `0x141F6E370` | `0x4090` |
| `CHumanWeakspot` | `0x141F76390` | `0x200` |
| `CImpactPredictionTrigger` | `0x141F69170` | `0x1C0` |
| `CIndoorState` | `0x141F680B0` | `0xD0` |
| `CInputActionOverride` | `0x141F779B0` | `0x130` |
| `CInteractionContext` | `0x141F6D870` | `0x3E0` |
| `CIntroDataContainer` | `0x141F6FA80` | `0xF8` |
| `CItemClubObject` | `0x141F6A380` | `0x440` |
| `CItemDefenderShield` | `0x141F6A740` | `0x2E0` |
| `CItemDispenser` | `0x141F77E20` | `0x1D0` |
| `CItemExplosiveObject` | `0x141F6A460` | `0x558` |
| `CItemFlashlightObject` | `0x141F6A290` | `0x2F0` |
| `CItemMaskObject` | `0x141F6A1B0` | `0x338` |
| `CItemObjectUpgrade` | `0x141F6D100` | `0xF8` |
| `CItemPropObject` | `0x141F6A080` | `0x318` |
| `CItemScrotusPolePruner` | `0x141F6A830` | `0x508` |
| `CItemStoneObject` | `0x141F6A650` | `0x4E8` |
| `CItemThunderstickObject` | `0x141F6A520` | `0x4C8` |
| `CJunctionObject` | `0x1422E0A90` | `0x138` |
| `CKillTrigger` | `0x141F6F5B0` | `0x198` |
| `CLandmark` | `0x141F70BB0` | `0x150` |
| `CLegendSystem` | `0x141F76AB0` | `0x188` |
| `CLightCharacter` | `0x141F6D1F0` | `0xFA8` |
| `CLocationInfo` | `0x141F70E90` | `0xF8` |
| `CLocationInfoIntel` | `0x141F71260` | `0x128` |
| `CLocationInfoTrackedItem` | `0x141F711A0` | `0x118` |
| `CLogBookMessage` | `0x141F77530` | `0x160` |
| `CLookAtTrigger` | `0x141F68DC0` | `0x1C8` |
| `CLootBag` | `0x141F71360` | `0x1A0` |
| `CMapIcon` | `0x141F70C90` | `0x130` |
| `CMaterialOverrideShapeTriggerExtension` | `0x141F689A0` | `0x100` |
| `CMissionStatusRouter` | `0x141F6D2A0` | `0x108` |
| `CMotionTracker` | `0x141F6BF20` | `0x1B0` |
| `CMotorBike` | `0x141F6E560` | `0x42A0` |
| `CMoveToGround` | `0x141F77F10` | `0x100` |
| `CMusicTrack` | `0x1422E12F0` | `0x1B8` |
| `CNamedPoint` | `0x141F6F6A0` | `0x160` |
| `CNavMeshDynamicObstacle` | `0x141F6FFB0` | `0x180` |
| `CNavMeshExcluder` | `0x141F6FEB0` | `0x150` |
| `CNavMeshWalkableSurface` | `0x141F70090` | `0x158` |
| `COutline` | `0x1422E2CA0` | `0x140` |
| `CParticleCullingVolume` | `0x1422E3300` | `0x1E0` |
| `CPerimeterArmament` | `0x141F78180` | `0x1A0` |
| `CPerimeterSystemObject` | `0x141F78420` | `0x1C0` |
| `CPerimeterWarcryerObject` | `0x141F784E0` | `0x218` |
| `CPhysicsCollisionGroup` | `0x141F69A70` | `0xD0` |
| `CPictureModeFilterObject` | `0x141F72E60` | `0x138` |
| `CPictureModeSettingsObject` | `0x141F72B50` | `0xD8` |
| `CPlacedAvoidanceObstacle` | `0x141F6F7A0` | `0x148` |
| `CPlayerSettings` | `0x141F6C010` | `0x220` |
| `CPlayerStateObject` | `0x141F68290` | `0x108` |
| `CPoseCopyCoordinator` | `0x141F77370` | `0x138` |
| `CPoseCopyThrall` | `0x141F77450` | `0x118` |
| `CPostcardViewer` | `0x141F77260` | `0xF8` |
| `CRayCasterObject` | `0x141F73E50` | `0x130` |
| `CRaycastSounds` | `0x141F6B5F0` | `0x170` |
| `CReceiveEventsTrigger` | `0x141F690B0` | `0x110` |
| `CRelicMenuEntry` | `0x141F76E40` | `0x100` |
| `CResourceObject` | `0x141F6C6C0` | `0xD0` |
| `CRestartPoint` | `0x141F6D4D0` | `0xF0` |
| `CReverbController` | `0x1422E1A80` | `0x1E0` |
| `CRigidObject` | `0x141F69250` | `0x2A0` |
| `CRoadFileLoader` | `0x141F6C5A0` | `0x160` |
| `CRoadHelperObject` | `0x1422E09A0` | `0x170` |
| `CRoadMover` | `0x141F6F230` | `0x200` |
| `CRoadObject` | `0x1422E08A0` | `0x140` |
| `CRoadPathCache` | `0x141F6F880` | `0xF8` |
| `CRopeObject` | `0x141F71AB0` | `0x240` |
| `CScaledRigidObject` | `0x141F69320` | `0x2B0` |
| `CScoutableInformationObject` | `0x141F72380` | `0x1A8` |
| `CScoutCacheList` | `0x141F72080` | `0xE0` |
| `CScoutingObject` | `0x141F72170` | `0x198` |
| `CScriptGameObject` | `0x141F71DC0` | `0x198` |
| `CSecondaryMotionController` | `0x141F6CE30` | `0x148` |
| `CSecondaryMotionJointComponent` | `0x141F6CF40` | `0x130` |
| `CSequenceDebugObject` | `0x141F71CE0` | `0x180` |
| `CSequenceObject2` | `0x141F71BA0` | `0x550` |
| `CSetTransformListener` | `0x141F6DA70` | `0x148` |
| `CShapeTrigger` | `0x141F68700` | `0x2A0` |
| `CShotgun` | `0x141F69E70` | `0x500` |
| `CShotgunArmourTarget` | `0x141F76190` | `0x290` |
| `CShotgunHumanTarget` | `0x141F760A0` | `0x2B8` |
| `CShotgunTarget` | `0x141F75EA0` | `0x290` |
| `CShotgunVehicleTarget` | `0x141F75FB0` | `0x2A0` |
| `CSideMissionStarter` | `0x141F6D390` | `0x110` |
| `CSkeletalAnimatedObject` | `0x141F6BD70` | `0xF30` |
| `CSoundBoxEmitter` | `0x1422E1030` | `0x308` |
| `CSoundLineEmitter` | `0x1422E0E70` | `0x258` |
| `CSoundModuleControllerGO` | `0x1422E1B80` | `0x1B0` |
| `CSoundModuleEvents` | `0x1422E1D30` | `0x120` |
| `CSoundModuleFunctionModulator` | `0x1422E1E30` | `0x1F0` |
| `CSoundModuleSetFunction` | `0x1422E1C60` | `0x1D0` |
| `CSoundOcclusionBox` | `0x1422E14E0` | `0x1A8` |
| `CSoundOcclusionCylinder` | `0x1422E1730` | `0x168` |
| `CSoundOcclusionDisc` | `0x1422E1630` | `0x148` |
| `CSoundOcclusionModulator` | `0x1422E1930` | `0x200` |
| `CSoundOcclusionPlane` | `0x1422E13D0` | `0x140` |
| `CSoundOcclusionTerrainPlane` | `0x1422E1830` | `0xF8` |
| `CSoundParameterModulator` | `0x1422E1120` | `0x200` |
| `CSoundPlaneEmitter` | `0x1422E0F50` | `0x278` |
| `CSoundPointEmitter` | `0x1422E0D80` | `0x248` |
| `CSoundPropertyModifierBoxGO` | `0x1422E2070` | `0x1A8` |
| `CSoundPropertyModifierGO` | `0x1422E1F30` | `0x110` |
| `CSoundPropertyModifierShapeGO` | `0x1422E22E0` | `0x208` |
| `CSoundPropertyModifierSphereGO` | `0x1422E2170` | `0x1F0` |
| `CSpawnInfoDataContainer` | `0x141F736C0` | `0xD8` |
| `CSpawnObjectCollection` | `0x141F733A0` | `0x118` |
| `CSpawnPickup` | `0x141F735E0` | `0x1B8` |
| `CSpawnPoint` | `0x141F732C0` | `0x238` |
| `CSpawnPointInCameraCondition` | `0x141F73BA0` | `0xF8` |
| `CSpawnPointWeight` | `0x141F73C70` | `0xD8` |
| `CSplineFollowObject` | `0x141F6FC90` | `0x1E0` |
| `CSplineMover` | `0x141F719E0` | `0x1D8` |
| `CSplineObject` | `0x1422E23D0` | `0x198` |
| `CSplineObjectGame` | `0x141F71900` | `0x1D0` |
| `CStateContainer` | `0x141F740F0` | `0x130` |
| `CStateContainerData` | `0x141F741E0` | `0xD0` |
| `CStateContainerDataHarpoon` | `0x141F748A0` | `0x340` |
| `CStateContainerDataHighlightTargetting` | `0x141F744E0` | `0x100` |
| `CStateContainerDataSlowMotion` | `0x141F745E0` | `0xE8` |
| `CStateContainerDataTargetInfo` | `0x141F743F0` | `0x170` |
| `CStateContainerDataVehicleShotgun` | `0x141F747B0` | `0x140` |
| `CStateContainerDataWeapon` | `0x141F746C0` | `0x128` |
| `CStateContainerState` | `0x141F74320` | `0xD8` |
| `CStateContainerStateHarpoonAim` | `0x141F754B0` | `0x148` |
| `CStateContainerStateHarpoonAuto` | `0x141F75100` | `0x120` |
| `CStateContainerStateHarpoonExplode` | `0x141F75580` | `0x108` |
| `CStateContainerStateHarpoonHooked` | `0x141F75660` | `0x128` |
| `CStateContainerStateHarpoonHookedNoDriver` | `0x141F75730` | `0xF8` |
| `CStateContainerStateHarpoonIdle` | `0x141F74FF0` | `0x100` |
| `CStateContainerStateHarpoonPreShoot` | `0x141F75310` | `0x108` |
| `CStateContainerStateHarpoonReload` | `0x141F75200` | `0x118` |
| `CStateContainerStateHarpoonRollback` | `0x141F75830` | `0xF0` |
| `CStateContainerStateHarpoonShoot` | `0x141F753F0` | `0x148` |
| `CStateContainerStateVehicleShotgunAim` | `0x141F74EE0` | `0x180` |
| `CStateContainerStateVehicleShotgunAuto` | `0x141F749D0` | `0x118` |
| `CStateContainerStateVehicleShotgunIdle` | `0x141F74AD0` | `0xF8` |
| `CStateContainerStateVehicleShotgunQuickShoot` | `0x141F74CC0` | `0xE8` |
| `CStateContainerStateVehicleShotgunReload` | `0x141F74BC0` | `0x100` |
| `CStateContainerStateVehicleShotgunShoot` | `0x141F74E00` | `0x1D0` |
| `CStaticDecalObject` | `0x1422E2FA0` | `0x1A0` |
| `CStatsEventTrigger` | `0x141F78020` | `0x118` |
| `CStealthHandler` | `0x141F6B060` | `0x168` |
| `CStormConfig` | `0x141F77AC0` | `0xD0` |
| `CStormDebrisSpawner` | `0x141F737B0` | `0x280` |
| `CStormEnemySpawner` | `0x141F73890` | `0x188` |
| `CTargetGameObject` | `0x141F75930` | `0x268` |
| `CTargetTrackingProjectile` | `0x141F6B330` | `0x308` |
| `CTeleport` | `0x1422E0B80` | `0x190` |
| `CTeleportPoint` | `0x141F6D950` | `0x148` |
| `CThunderStickLauncher` | `0x141F69FA0` | `0x4A0` |
| `CThunderStickLauncherControllerObject` | `0x141F6AF60` | `0x198` |
| `CTimelapse` | `0x141F72A70` | `0x108` |
| `CTimeOfDayRelay` | `0x141F778C0` | `0x138` |
| `CTimeTrigger` | `0x141F68A90` | `0x148` |
| `CTooltipManager` | `0x141F76D60` | `0x520` |
| `CTrackEventRelay` | `0x141F77800` | `0x110` |
| `CTrajectoryWarning` | `0x141F73F40` | `0x178` |
| `CTransformLinker` | `0x141F6DF00` | `0x168` |
| `CTransformObject` | `0x141F6DFF0` | `0x158` |
| `CTriggeredExplosive` | `0x141F6AE70` | `0x280` |
| `CTurnTakerGameObject` | `0x141F6FBA0` | `0x108` |
| `CUpdateOrderObject` | `0x141F74010` | `0x158` |
| `CUpgradeSystem` | `0x141F76C70` | `0x188` |
| `CVantagePoint` | `0x141F72280` | `0xF8` |
| `CVegetationSounds` | `0x141F6B6E0` | `0x158` |
| `CVehicleEngineUpgrade` | `0x141F6E890` | `0x108` |
| `CVehicleExplosionDamageHandlerType` | `0x141F6EA60` | `0xD8` |
| `CVehicleExplosionDamageHandlerTypeThunderstick` | `0x141F6EB50` | `0x100` |
| `CVehicleInstantRepairRefuelComponent` | `0x141F6E980` | `0x1B8` |
| `CVehicleUpgrade` | `0x141F6E7A0` | `0xD0` |
| `CVehicleUpgradeManager` | `0x141F70420` | `0x180` |
| `CVehicleWeakspot` | `0x141F764E0` | `0x228` |
| `CWaterLevelShapeTriggerExtension` | `0x141F68870` | `0xE0` |
| `CWeakspotGameObject` | `0x141F76290` | `0x1E0` |
| `CWeapon` | `0x141F69E30` | `0x498` |
| `CWeaponHolder` | `0x141F6AA00` | `0x198` |
| `CWeaponSequencer` | `0x141F6AB00` | `0x160` |
| `CWheelWeakspot` | `0x141F765C0` | `0x230` |
| `CWindSuppressor` | `0x141F69B60` | `0x128` |
| `CWreckingBall` | `0x141F6B240` | `0x2D8` |

## Reproduction

1. Enumerate the decorated `CGameObjectCreator<T>` strings and recover each TypeDescriptor.
2. Find COL records whose type-descriptor RVA resolves to that address; verify each COL self-RVA.
3. Find the `.data1_a` pointer to each COL; the next address is the vtable address point. The target at byte offset `+0x10` is slot 2.
4. Read six bytes at that address from the saved decoded-code snapshot and check for `B8 <imm32> C3`; decode `imm32` little-endian.

The complete object-creation/lifecycle interpretation and open questions are in [object resource lifecycle](object-resource-lifecycle-2026-11-01.md).

