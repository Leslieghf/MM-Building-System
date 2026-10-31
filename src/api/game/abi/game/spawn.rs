//! Spawn-system declarations from the pinned community SDK.
//!
//! Source: `game/spawnsystem.h`. The C++ descriptor owns a game string and
//! custom vector and must be constructed/destroyed by the game. It is opaque
//! here by design; this module does not provide allocation or native calls.

use crate::api::game::abi::types::Matrix4x4;
use core::ffi::c_char;

/// Opaque `SSpawnInstanceInfo`; never construct or dereference this in Rust.
pub enum SpawnInstanceInfo {}

/// Opaque queue item whose layout includes engine and resource-loader state.
pub enum SpawnQueueItem {}

/// Opaque spawned entity record.
pub enum SpawnedEntity {}

/// Spawn priority values declared by `ESpawnPriority` in the SDK.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPriority {
    /// Lowest declared priority.
    Low = -2,
    /// Normal declared priority.
    Normal = -1,
    /// High declared priority.
    High = 1,
    /// Critical declared priority.
    Critical = 2,
    /// Medium declared priority; the SDK's numeric ordering is non-monotonic.
    Medium = 9,
}

/// `ESpawnPriority::E_SPAWN_PRIORITY_COUNT`; a count sentinel, not a priority.
pub const SPAWN_PRIORITY_COUNT: i32 = 5;

/// Outcomes the SDK's handler records in its last-frame arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnObservation {
    /// The handler recorded a spawn.
    Spawned,
    /// The handler recorded a despawn.
    Despawned,
    /// The handler recorded a failed operation.
    Failed,
}

/// `SSpawnInstanceInfo::SetEntityFile(const char*)` candidate signature.
pub type SetEntityFileFn = unsafe extern "system" fn(*mut SpawnInstanceInfo, *const c_char);

/// `SSpawnInstanceInfo` native constructor and destructor candidates.
pub type SpawnInfoConstructorFn = unsafe extern "system" fn(*mut SpawnInstanceInfo);
/// `SSpawnInstanceInfo` native destructor candidate.
pub type SpawnInfoDestructorFn = unsafe extern "system" fn(*mut SpawnInstanceInfo);

/// `SpawnSystemSpawn(SSpawnInstanceInfo*, CMatrix*)` candidate signature.
pub type SpawnFn = unsafe extern "system" fn(*mut SpawnInstanceInfo, *mut Matrix4x4) -> usize;

/// `CSpawnedEntityHandler::Spawn(...)` has an SDK-declared singleton path, but
/// the SDK uses a distribution-specific global and an unpaired direct address.
pub enum SpawnedEntityHandler {}

#[cfg(test)]
mod tests {
    use super::{SPAWN_PRIORITY_COUNT, SpawnPriority};

    #[test]
    fn spawn_priority_values_match_the_sdk_declarations() {
        assert_eq!(SpawnPriority::Low as i32, -2);
        assert_eq!(SpawnPriority::Normal as i32, -1);
        assert_eq!(SpawnPriority::High as i32, 1);
        assert_eq!(SpawnPriority::Critical as i32, 2);
        assert_eq!(SpawnPriority::Medium as i32, 9);
        assert_eq!(SPAWN_PRIORITY_COUNT, 5);
        assert_eq!(std::mem::size_of::<SpawnPriority>(), 4);
    }
}
