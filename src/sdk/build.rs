//! Address metadata from the pinned community SDK revision.
//!
//! These are candidate absolute addresses from the SDK's `ADDRESS` macros and
//! global definitions. Resolution is useful for inventory and diagnostics; it
//! does not validate an executable or make a native call safe.

/// Pinned source revision used to derive the address table.
pub const SDK_REVISION: &str = "f2dc640769c88aae9c3965a2aca0aa138f0e62ab";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutableFingerprint {
    pub pe_timestamp: u32,
    pub file_size: u64,
    pub sha256: &'static str,
}

/// Locally inspected Steam executable recorded in the existing binding design.
/// This static record does not re-check the user's current file at runtime.
pub const LOCAL_STEAM_TARGET: ExecutableFingerprint = ExecutableFingerprint {
    pe_timestamp: 0x5628_A448,
    file_size: 76_814_792,
    sha256: "CD719A721DB92575AA32FF01E07A590F47DD34B702B1BCD98EC698A07358BB0D",
};

impl ExecutableFingerprint {
    pub fn matches(&self, pe_timestamp: u32, file_size: u64, sha256: &str) -> bool {
        self.pe_timestamp == pe_timestamp
            && self.file_size == file_size
            && self.sha256.eq_ignore_ascii_case(sha256)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkBuild {
    Gog1030,
    Steam1030,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressPair {
    pub gog: Option<usize>,
    pub steam: Option<usize>,
}

impl AddressPair {
    /// Resolve an address declared for this SDK distribution, rejecting zero.
    pub const fn resolve(self, build: SdkBuild) -> Option<usize> {
        let address = match build {
            SdkBuild::Gog1030 => self.gog,
            SdkBuild::Steam1030 => self.steam,
            SdkBuild::Unsupported => None,
        };

        match address {
            Some(0) | None => None,
            Some(address) => Some(address),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolEvidence {
    HeaderDeclaration,
    SourceDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSymbol {
    pub name: &'static str,
    pub source_path: &'static str,
    pub address: AddressPair,
    pub evidence: SymbolEvidence,
}

const fn pair(gog: usize, steam: usize) -> AddressPair {
    AddressPair {
        gog: if gog == 0 { None } else { Some(gog) },
        steam: if steam == 0 { None } else { Some(steam) },
    }
}

pub const CHARACTER_MANAGER_SINGLETON_PAIR: AddressPair = pair(0x1417_11B38, 0x1417_F5338);
pub const GET_PLAYER_CHARACTER_PAIR: AddressPair = pair(0x1400_FBD30, 0x141C_91250);
pub const GET_CHARACTER_VEHICLE_PAIR: AddressPair = pair(0x1400_EE860, 0x141C_82910);
pub const CAMERA_MANAGER_SINGLETON_PAIR: AddressPair = pair(0x1417_11B78, 0x1417_F5378);
pub const GET_CAMERA_MATRIX_PAIR: AddressPair = pair(0x1405_F3CB0, 0x1422_2DA60);
pub const SPAWN_INFO_CTOR_PAIR: AddressPair = pair(0x1402_E19D0, 0x141E_B2410);
pub const SPAWN_INFO_DTOR_PAIR: AddressPair = pair(0x1402_E2320, 0x141E_B2E10);
pub const SPAWN_SET_ENTITY_FILE_PAIR: AddressPair = pair(0x1405_48C60, 0x1421_71820);
pub const SPAWN_GET_NEW_ID_PAIR: AddressPair = pair(0x1405_099D0, 0x1421_2B750);
pub const SPAWN_SUBMIT_PAIR: AddressPair = pair(0x1405_62EB0, 0x1421_8D5B0);
pub const SPAWN_SYSTEM_CONTEXT_PAIR: AddressPair = pair(0x1417_1B678, 0x1417_FEE78);
pub const DEVICE_MANAGER_SINGLETON_PAIR: AddressPair = pair(0, 0x1417_F4EF0);

const fn steam_address(address: AddressPair) -> usize {
    match address.steam {
        Some(value) => value,
        None => 0,
    }
}

/// Named Steam 1.0.3.0 SDK addresses already consumed by the native adapter.
/// They remain candidate addresses and must not be used on another build.
pub mod steam {
    /// Preferred image base used by the inspected Steam executable build.
    pub const IMAGE_BASE: usize = 0x14000_0000;

    use super::{
        CAMERA_MANAGER_SINGLETON_PAIR, CHARACTER_MANAGER_SINGLETON_PAIR,
        DEVICE_MANAGER_SINGLETON_PAIR, GET_CAMERA_MATRIX_PAIR, GET_CHARACTER_VEHICLE_PAIR,
        GET_PLAYER_CHARACTER_PAIR, SPAWN_GET_NEW_ID_PAIR, SPAWN_INFO_CTOR_PAIR,
        SPAWN_INFO_DTOR_PAIR, SPAWN_SET_ENTITY_FILE_PAIR, SPAWN_SUBMIT_PAIR,
        SPAWN_SYSTEM_CONTEXT_PAIR, steam_address,
    };

    pub const CHARACTER_MANAGER_SINGLETON: usize = steam_address(CHARACTER_MANAGER_SINGLETON_PAIR);
    pub const GET_PLAYER_CHARACTER: usize = steam_address(GET_PLAYER_CHARACTER_PAIR);
    pub const GET_CHARACTER_VEHICLE: usize = steam_address(GET_CHARACTER_VEHICLE_PAIR);
    pub const CAMERA_MANAGER_SINGLETON: usize = steam_address(CAMERA_MANAGER_SINGLETON_PAIR);
    pub const GET_CAMERA_MATRIX: usize = steam_address(GET_CAMERA_MATRIX_PAIR);
    pub const SPAWN_INFO_CTOR: usize = steam_address(SPAWN_INFO_CTOR_PAIR);
    pub const SPAWN_INFO_DTOR: usize = steam_address(SPAWN_INFO_DTOR_PAIR);
    pub const SPAWN_SET_ENTITY_FILE: usize = steam_address(SPAWN_SET_ENTITY_FILE_PAIR);
    pub const SPAWN_GET_NEW_ID: usize = steam_address(SPAWN_GET_NEW_ID_PAIR);
    pub const SPAWN_SUBMIT: usize = steam_address(SPAWN_SUBMIT_PAIR);
    pub const SPAWN_SYSTEM_CONTEXT: usize = steam_address(SPAWN_SYSTEM_CONTEXT_PAIR);
    pub const DEVICE_MANAGER_SINGLETON: usize = steam_address(DEVICE_MANAGER_SINGLETON_PAIR);
}

/// Address declarations explicitly present in the pinned SDK.
///
/// A resolved address remains a candidate until matched against the exact
/// executable and the function's calling convention and lifecycle are checked.
pub const SDK_NATIVE_SYMBOLS: &[NativeSymbol] = &[
    NativeSymbol {
        name: "CMatrix::operator*(CVector4f)",
        source_path: "math3d.h",
        address: pair(0x1409_DEA30, 0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CMatrix::RotationYawPitchRoll",
        source_path: "math3d.h",
        address: pair(0x1409_EA970, 0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CMatrix::ToEulerXYZ",
        source_path: "math3d.h",
        address: pair(0x1409_EA850, 0x1429_BE740),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CMatrix::FromEulerXYZ",
        source_path: "math3d.h",
        address: pair(0x1409_EA3F0, 0x1429_BE740),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CMatrix::ToEuler",
        source_path: "math3d.h",
        address: pair(0x1409_EC760, 0x1429_C0EE0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CQuaternion::FromMatrix4",
        source_path: "math3d.h",
        address: pair(0x1409_F4470, 0x1429_C91A0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CQuaternion::ToMatrix4",
        source_path: "math3d.h",
        address: pair(0x1409_E1270, 0x1429_B49A0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CQuaternion::FromEulerXYZ",
        source_path: "math3d.h",
        address: pair(0x1409_EF470, 0x1429_C3D30),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CQuaternion::FromEuler",
        source_path: "math3d.h",
        address: pair(0x1409_EF030, 0x1429_C38D0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CQuaternion::operator*(CVector3f)",
        source_path: "math3d.h",
        address: pair(0x1409_E1BA0, 0x1429_B5340),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "SSpawnInstanceInfo::constructor",
        source_path: "game/spawnsystem.h",
        address: SPAWN_INFO_CTOR_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "SSpawnInstanceInfo::destructor",
        source_path: "game/spawnsystem.h",
        address: SPAWN_INFO_DTOR_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "SSpawnInstanceInfo::SetEntityFile",
        source_path: "game/spawnsystem.h",
        address: SPAWN_SET_ENTITY_FILE_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "SpawnSystemGetNewSpawnId",
        source_path: "game/spawnsystem.h",
        address: SPAWN_GET_NEW_ID_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "SpawnSystemDespawn",
        source_path: "game/spawnsystem.h",
        address: pair(0x1405_30400, 0x1421_57080),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "SpawnSystemSpawn",
        source_path: "game/spawnsystem.h",
        address: SPAWN_SUBMIT_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CCharacterBase::RotateInstantly",
        source_path: "game/go/character.h",
        address: pair(0x1406_3F940, 0x1422_87340),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CCharacter::ForceNeutralState",
        source_path: "game/go/character.h",
        address: pair(0x1401_39A20, 0x141C_D69B0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CCharacter::GetVehiclePtr",
        source_path: "game/go/character.h",
        address: GET_CHARACTER_VEHICLE_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CCameraControlManager::GetCameraMatrix",
        source_path: "game/cameramanager.h",
        address: GET_CAMERA_MATRIX_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CCharacterManager::GetPlayerCharacter",
        source_path: "game/charactermanager.h",
        address: GET_PLAYER_CHARACTER_PAIR,
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CGame::IsGuiOccludingMainDraw",
        source_path: "game/game.h",
        address: pair(0x1402_11FD0, 0x141D_C8130),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "HashString",
        source_path: "core/hash.h",
        address: pair(0x140F_332C0, 0x142F_BD230),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CInputActionMap::GetValue",
        source_path: "core/input.h",
        address: pair(0x1409_27740, 0x1428_DA580),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "IInputDeviceManager::GetActionMap",
        source_path: "core/input.h",
        address: pair(0x1409_24140, 0x1428_D64B0),
        evidence: SymbolEvidence::HeaderDeclaration,
    },
    NativeSymbol {
        name: "CGraphicsEngine::m_Params",
        source_path: "core/graphics/graphicsengine.cpp",
        address: pair(0x1416_E9BF0, 0x1417_CB6E0),
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CGameState::m_InMainMenu",
        source_path: "game/game.cpp",
        address: pair(0x1417_1A32B, 0x1417_FDB2B),
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CGameState::m_State",
        source_path: "game/game.cpp",
        address: pair(0x1417_1A32C, 0x1417_FDB2C),
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CAvaSingle<CGraphicsEngine>::Instance",
        source_path: "core/avasingle.cpp",
        address: pair(0x1417_116D0, 0x1417_F4EC8),
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CAvaSingle<CDeviceManager>::Instance",
        source_path: "core/avasingle.cpp",
        address: pair(0x1417_11708, 0),
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CAvaSingle<CDeviceManager_STEAM>::Instance",
        source_path: "core/avasingle.cpp",
        address: DEVICE_MANAGER_SINGLETON_PAIR,
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CAvaSingle<CCharacterManager>::Instance",
        source_path: "core/avasingle.cpp",
        address: CHARACTER_MANAGER_SINGLETON_PAIR,
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "CAvaSingle<CCameraControlManager>::Instance",
        source_path: "core/avasingle.cpp",
        address: CAMERA_MANAGER_SINGLETON_PAIR,
        evidence: SymbolEvidence::SourceDefinition,
    },
    NativeSymbol {
        name: "s_SpawnSystemContext",
        source_path: "game/spawnsystem.cpp",
        address: SPAWN_SYSTEM_CONTEXT_PAIR,
        evidence: SymbolEvidence::SourceDefinition,
    },
];

#[cfg(test)]
mod tests {
    use super::{AddressPair, LOCAL_STEAM_TARGET, SDK_REVISION, SdkBuild, steam};

    #[test]
    fn address_pair_requires_a_matching_declared_distribution() {
        let address = AddressPair {
            gog: Some(0x1400_1234),
            steam: Some(0x1420_5678),
        };
        assert_eq!(address.resolve(SdkBuild::Gog1030), Some(0x1400_1234));
        assert_eq!(address.resolve(SdkBuild::Steam1030), Some(0x1420_5678));
    }

    #[test]
    fn zero_address_is_treated_as_absent_for_that_distribution() {
        let address = AddressPair {
            gog: Some(0),
            steam: Some(0x1420_5678),
        };
        assert_eq!(address.resolve(SdkBuild::Gog1030), None);
        assert_eq!(address.resolve(SdkBuild::Steam1030), Some(0x1420_5678));
    }

    #[test]
    fn unsupported_build_never_resolves_an_sdk_address() {
        let address = AddressPair {
            gog: Some(0x1400_1234),
            steam: Some(0x1420_5678),
        };
        assert_eq!(address.resolve(SdkBuild::Unsupported), None);
    }

    #[test]
    fn address_pairs_are_only_claimed_for_the_pinned_sdk_revision() {
        assert_eq!(SDK_REVISION, "f2dc640769c88aae9c3965a2aca0aa138f0e62ab");
    }

    #[test]
    fn bridge_addresses_resolve_from_the_same_sdk_address_pairs() {
        assert_eq!(
            steam::CHARACTER_MANAGER_SINGLETON,
            super::CHARACTER_MANAGER_SINGLETON_PAIR
                .resolve(SdkBuild::Steam1030)
                .unwrap()
        );
        assert_eq!(
            steam::GET_PLAYER_CHARACTER,
            super::GET_PLAYER_CHARACTER_PAIR
                .resolve(SdkBuild::Steam1030)
                .unwrap()
        );
        assert_eq!(
            steam::GET_CHARACTER_VEHICLE,
            super::GET_CHARACTER_VEHICLE_PAIR
                .resolve(SdkBuild::Steam1030)
                .unwrap()
        );
        assert_eq!(
            steam::CAMERA_MANAGER_SINGLETON,
            super::CAMERA_MANAGER_SINGLETON_PAIR
                .resolve(SdkBuild::Steam1030)
                .unwrap()
        );
        assert_eq!(
            steam::GET_CAMERA_MATRIX,
            super::GET_CAMERA_MATRIX_PAIR
                .resolve(SdkBuild::Steam1030)
                .unwrap()
        );
    }

    #[test]
    fn local_steam_fingerprint_matches_only_the_recorded_executable_identity() {
        assert!(LOCAL_STEAM_TARGET.matches(
            0x5628_A448,
            76_814_792,
            "cd719a721db92575aa32ff01e07a590f47dd34b702b1bcd98ec698a07358bb0d"
        ));
        assert!(!LOCAL_STEAM_TARGET.matches(
            0x5628_A448,
            76_814_792,
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }
}
