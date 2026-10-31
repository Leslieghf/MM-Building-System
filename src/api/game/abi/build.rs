//! Address metadata used by the build-specific adapter.
//!
//! # Address contract
//!
//! [`SDK_REVISION`] identifies the source snapshot for header-derived values.
//! [`AddressPair`] keeps distribution-specific absolute addresses separate;
//! the adapter resolves only addresses it actually uses. An address pair is
//! candidate metadata, not proof that the user's executable matches or that a
//! function's ABI/lifecycle is safe. Executable validation is performed by the
//! adapter before use.

/// Pinned source revision used to derive the address table.
pub const SDK_REVISION: &str = "f2dc640769c88aae9c3965a2aca0aa138f0e62ab";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutableFingerprint {
    /// PE header timestamp used to identify this executable build.
    pub pe_timestamp: u32,
    /// Exact executable size in bytes.
    pub file_size: u64,
    /// Expected hexadecimal SHA-256 string, compared case-insensitively.
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
    /// Compare a candidate executable's metadata against this fingerprint.
    pub fn matches(&self, pe_timestamp: u32, file_size: u64, sha256: &str) -> bool {
        self.pe_timestamp == pe_timestamp
            && self.file_size == file_size
            && self.sha256.eq_ignore_ascii_case(sha256)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdkBuild {
    /// GOG build 1.0.3.0.
    Gog1030,
    /// Steam build 1.0.3.0.
    Steam1030,
    /// Any build without an address mapping in this crate.
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressPair {
    /// Absolute address for the GOG build, or `None` when unavailable.
    pub gog: Option<usize>,
    /// Absolute address for the Steam build, or `None` when unavailable.
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

const fn pair(gog: usize, steam: usize) -> AddressPair {
    AddressPair {
        gog: if gog == 0 { None } else { Some(gog) },
        steam: if steam == 0 { None } else { Some(steam) },
    }
}

/// Character-manager singleton address pair from the SDK.
pub const CHARACTER_MANAGER_SINGLETON_PAIR: AddressPair = pair(0x1417_11B38, 0x1417_F5338);
/// Current-player-character accessor address pair from the SDK.
pub const GET_PLAYER_CHARACTER_PAIR: AddressPair = pair(0x1400_FBD30, 0x141C_91250);
/// Character-to-vehicle accessor address pair from the SDK.
pub const GET_CHARACTER_VEHICLE_PAIR: AddressPair = pair(0x1400_EE860, 0x141C_82910);
/// Camera-manager singleton address pair from the SDK.
pub const CAMERA_MANAGER_SINGLETON_PAIR: AddressPair = pair(0x1417_11B78, 0x1417_F5378);
/// Camera-matrix accessor address pair from the SDK.
pub const GET_CAMERA_MATRIX_PAIR: AddressPair = pair(0x1405_F3CB0, 0x1422_2DA60);
/// Spawn descriptor constructor address pair from the SDK.
pub const SPAWN_INFO_CTOR_PAIR: AddressPair = pair(0x1402_E19D0, 0x141E_B2410);
/// Spawn descriptor destructor address pair from the SDK.
pub const SPAWN_INFO_DTOR_PAIR: AddressPair = pair(0x1402_E2320, 0x141E_B2E10);
/// Spawn descriptor entity-file setter address pair from the SDK.
pub const SPAWN_SET_ENTITY_FILE_PAIR: AddressPair = pair(0x1405_48C60, 0x1421_71820);
/// Spawn ID accessor address pair from the SDK.
pub const SPAWN_GET_NEW_ID_PAIR: AddressPair = pair(0x1405_099D0, 0x1421_2B750);
/// Spawn submission address pair from the SDK.
pub const SPAWN_SUBMIT_PAIR: AddressPair = pair(0x1405_62EB0, 0x1421_8D5B0);
/// Spawn-system context singleton address pair from the SDK.
pub const SPAWN_SYSTEM_CONTEXT_PAIR: AddressPair = pair(0x1417_1B678, 0x1417_FEE78);
/// Device-manager singleton address pair; absent from the GOG build mapping.
pub const DEVICE_MANAGER_SINGLETON_PAIR: AddressPair = pair(0, 0x1417_F4EF0);

const fn steam_address(address: AddressPair) -> usize {
    match address.steam {
        Some(value) => value,
        None => 0,
    }
}

/// Steam 1.0.3.0 addresses consumed by the current adapter.
///
/// These absolute addresses are candidates for the inspected executable only.
pub mod steam {
    /// Preferred image base recorded for the inspected Steam executable.
    pub const IMAGE_BASE: usize = 0x14000_0000;

    use super::{
        CAMERA_MANAGER_SINGLETON_PAIR, CHARACTER_MANAGER_SINGLETON_PAIR,
        DEVICE_MANAGER_SINGLETON_PAIR, GET_CAMERA_MATRIX_PAIR, GET_CHARACTER_VEHICLE_PAIR,
        GET_PLAYER_CHARACTER_PAIR, SPAWN_GET_NEW_ID_PAIR, SPAWN_INFO_CTOR_PAIR,
        SPAWN_INFO_DTOR_PAIR, SPAWN_SET_ENTITY_FILE_PAIR, SPAWN_SUBMIT_PAIR,
        SPAWN_SYSTEM_CONTEXT_PAIR, steam_address,
    };

    /// Character-manager singleton address.
    pub const CHARACTER_MANAGER_SINGLETON: usize = steam_address(CHARACTER_MANAGER_SINGLETON_PAIR);
    /// Current-player-character accessor address.
    pub const GET_PLAYER_CHARACTER: usize = steam_address(GET_PLAYER_CHARACTER_PAIR);
    /// Character-to-vehicle accessor address.
    pub const GET_CHARACTER_VEHICLE: usize = steam_address(GET_CHARACTER_VEHICLE_PAIR);
    /// Camera-manager singleton address.
    pub const CAMERA_MANAGER_SINGLETON: usize = steam_address(CAMERA_MANAGER_SINGLETON_PAIR);
    /// Camera-matrix accessor address.
    pub const GET_CAMERA_MATRIX: usize = steam_address(GET_CAMERA_MATRIX_PAIR);
    /// Spawn descriptor constructor address.
    pub const SPAWN_INFO_CTOR: usize = steam_address(SPAWN_INFO_CTOR_PAIR);
    /// Spawn descriptor destructor address.
    pub const SPAWN_INFO_DTOR: usize = steam_address(SPAWN_INFO_DTOR_PAIR);
    /// Spawn descriptor entity-file setter address.
    pub const SPAWN_SET_ENTITY_FILE: usize = steam_address(SPAWN_SET_ENTITY_FILE_PAIR);
    /// Spawn ID accessor address.
    pub const SPAWN_GET_NEW_ID: usize = steam_address(SPAWN_GET_NEW_ID_PAIR);
    /// Spawn submission address.
    pub const SPAWN_SUBMIT: usize = steam_address(SPAWN_SUBMIT_PAIR);
    /// Spawn-system context singleton address.
    pub const SPAWN_SYSTEM_CONTEXT: usize = steam_address(SPAWN_SYSTEM_CONTEXT_PAIR);
    /// Device-manager singleton address.
    pub const DEVICE_MANAGER_SINGLETON: usize = steam_address(DEVICE_MANAGER_SINGLETON_PAIR);
}

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
