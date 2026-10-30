//! Source-order catalog for SDK virtual declarations.
//!
//! Derived from the pinned `game/go/*.h` and `game/events.h` files listed in
//! [`SDK_REVISION`](super::super::build::SDK_REVISION).
//!
//! These indices are ordinals within each header's class declaration, **not**
//! numeric vtable slots. MSVC destructors and multiple inheritance change the
//! actual vtable layout; use executable evidence before making a virtual call.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualApi {
    pub class_name: &'static str,
    pub methods_in_declaration_order: &'static [&'static str],
}

pub const VIRTUAL_APIS: &[VirtualApi] = &[
    VirtualApi {
        class_name: "CGameObject",
        methods_in_declaration_order: &[
            "GetTypeId",
            "IsType",
            "Init",
            "PostInit",
            "GetRequiredUpdates",
            "GetParallelizedUpdates",
            "SetProperties",
            "ExtendedSetProperties",
            "UpdateProperty",
            "ReadProperty",
            "UpdatePreSim",
            "UpdatePostSim",
            "UpdateRender",
            "GetUpdateOrder",
            "NeedsUpdate",
            "StayInUpdate",
            "StayInUpdatePreSim",
            "StayInUpdatePostSim",
            "StayInUpdateRender",
            "InitTransform",
            "SetTransform",
            "GetTransform",
        ],
    },
    VirtualApi {
        class_name: "IPfxGameObject",
        methods_in_declaration_order: &[
            "~IPfxGameObject",
            "GetIPfxInstance",
            "DoReactToHit",
            "OnBreak",
            "NotifyOutOfBroadphase",
            "OnOutOfBroadphase",
        ],
    },
    VirtualApi {
        class_name: "CPhysicsGameObject",
        methods_in_declaration_order: &[
            "~CPhysicsGameObject",
            "GetPhysicsAABB",
            "SetKeyframedMotionState",
            "SetNextKeyframeTransform",
            "HitByBullet",
            "HitByExplosion",
            "HitByFire",
            "CollideWithWater",
            "OnCollision",
            "OnCollisionSliding",
            "OnCollisionRolling",
            "OnCollisionGeneric",
            "IsDamageable",
            "SetMaterialOverride",
            "ClearMaterialOverride",
            "SetWaterLevel",
            "GetPointClosestTo",
            "InitBuoyancy",
        ],
    },
    VirtualApi {
        class_name: "CDamageable",
        methods_in_declaration_order: &[
            "~CDamageable",
            "InflictDamage",
            "CalculateDamage",
            "IsTargetable",
            "OnDestroyed",
            "SendOnDestroyEvent",
        ],
    },
    VirtualApi {
        class_name: "CVehicle",
        methods_in_declaration_order: &[
            "StopPartAnimation",
            "SetWheelSkew",
            "UpdatePfxPreSim",
            "UpdatePfxPostSimStep",
            "Update",
            "Clear",
            "SetVisibilityModelsAndEffects",
            "OnEntering",
            "OnEnter",
            "OnExit",
            "SetVelocity",
            "SetTargetHeight",
            "SetLOD",
            "GetNumberOfWheelsOnGround",
            "GetTrust",
            "GetTargetHeight",
            "GetAABB",
            "GetLocalAABB",
            "AttachObject",
            "FadeIn",
            "GetSpawnTypeId",
            "GetSpawnVariationIndex",
            "GetSpawnTeam",
            "KillPhysics",
            "CalculateLod",
            "OnEmerge",
            "OnSubmerge",
            "SetDirtyWheels",
            "UpdateAiEntityInstance",
            "GetAngularVelocity",
            "GetVelocity",
            "GetVelocityLastFrame",
            "GetAcceleration",
            "GetState",
        ],
    },
    VirtualApi {
        class_name: "CEventHandler",
        methods_in_declaration_order: &["~CEventHandler", "HandleEvent"],
    },
];

/// Return a source-order ordinal for a declaration without implying a vtable slot.
pub fn find_virtual_declaration(class_name: &str, method: &str) -> Option<usize> {
    let class = VIRTUAL_APIS
        .iter()
        .find(|candidate| candidate.class_name == class_name)?;
    class
        .methods_in_declaration_order
        .iter()
        .position(|candidate| *candidate == method)
}

/// Return the number of virtual declarations recorded for one SDK class.
pub fn virtual_declaration_count(class_name: &str) -> Option<usize> {
    VIRTUAL_APIS
        .iter()
        .find(|candidate| candidate.class_name == class_name)
        .map(|class| class.methods_in_declaration_order.len())
}

#[cfg(test)]
mod tests {
    use super::{VIRTUAL_APIS, find_virtual_declaration, virtual_declaration_count};

    #[test]
    fn catalog_covers_the_sdk_virtual_method_declarations_in_source_order() {
        assert_eq!(VIRTUAL_APIS.len(), 6);
        assert_eq!(
            find_virtual_declaration("CGameObject", "GetTransform"),
            Some(21)
        );
        assert_eq!(
            find_virtual_declaration("CVehicle", "SetVelocity"),
            Some(10)
        );
        assert_eq!(
            find_virtual_declaration("CVehicle", "GetAcceleration"),
            Some(32)
        );
        assert_eq!(
            find_virtual_declaration("CDamageable", "IsTargetable"),
            Some(3)
        );
        assert_eq!(virtual_declaration_count("CGameObject"), Some(22));
        assert_eq!(virtual_declaration_count("IPfxGameObject"), Some(6));
        assert_eq!(virtual_declaration_count("CPhysicsGameObject"), Some(18));
        assert_eq!(virtual_declaration_count("CDamageable"), Some(6));
        assert_eq!(virtual_declaration_count("CVehicle"), Some(34));
        assert_eq!(virtual_declaration_count("CEventHandler"), Some(2));
    }

    #[test]
    fn catalog_does_not_invent_numeric_vtable_slots() {
        assert_eq!(find_virtual_declaration("CVehicle", "NotInSdk"), None);
        assert_eq!(find_virtual_declaration("UnknownType", "GetVelocity"), None);
    }
}
